//! Feature-gated Anthropic Messages API client (SPEC M1/M2, gate G10).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use kittens_code_core::engine::{ModelOutcome, ProposedToolCall, Usage};
use kittens_code_core::window::{TailItem, WindowLayout};
use kittens_code_protocol::error::ErrorCode;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderValue, RETRY_AFTER};
use reqwest::{Client, StatusCode, Url};
use serde_json::{Value, json};

use crate::model::{ModelClient, ModelError, ModelFuture};

const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_ERROR_MESSAGE_CHARS: usize = 2_048;

/// Bootstrap-only settings for an Anthropic-dialect live model endpoint.
///
/// The API key is intentionally not `Debug` and this configuration is not
/// part of the replayable protocol configuration.
#[derive(Clone)]
pub struct LiveConfig {
    /// Endpoint base URL; the client appends `/v1/messages`.
    pub endpoint_base_url: String,
    /// Value sent in the `x-api-key` header.
    pub api_key: String,
    /// Exact provider model identifier.
    pub model: String,
    /// Maximum number of output tokens requested from the provider.
    pub max_output_tokens: u32,
    /// Retry and circuit-breaker settings.
    pub retry: RetryConfig,
}

/// Bounds for the exponential retry ladder and failure-count breaker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryConfig {
    /// Total HTTP attempts, including the first attempt.
    pub max_attempts: u32,
    /// Delay before the first retry; later delays double from this value.
    pub base_delay: Duration,
    /// Maximum exponential delay before applying a longer `Retry-After`.
    pub max_delay: Duration,
    /// Maximum elapsed time the retry ladder may schedule across.
    pub max_elapsed: Duration,
    /// Consecutive terminally failed calls that open the breaker.
    pub breaker_failure_threshold: u32,
    /// How long an open breaker fails calls fast before permitting a probe.
    pub breaker_cooldown: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(8),
            max_elapsed: Duration::from_secs(30),
            breaker_failure_threshold: 3,
            breaker_cooldown: Duration::from_secs(30),
        }
    }
}

/// Anthropic Messages SSE client, available only with the `live` feature.
///
/// Dropping the future returned by [`ModelClient::complete`] drops the
/// in-flight request or Tokio backoff sleep, making both paths cooperatively
/// cancellation-aware.
#[derive(Clone)]
pub struct LiveClient {
    http: Client,
    endpoint: Url,
    api_key: HeaderValue,
    model: Arc<str>,
    max_output_tokens: u32,
    retry: RetryConfig,
    breaker: Arc<CircuitBreaker>,
    jitter: Arc<Jitter>,
}

impl LiveClient {
    /// Builds a live client from bootstrap-only endpoint settings.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::ConfigInvalid`] for an invalid URL, header value,
    /// output-token limit, or retry bound, and [`ErrorCode::ModelTransport`]
    /// if the HTTP client cannot be constructed.
    pub fn new(config: LiveConfig) -> Result<Self, ModelError> {
        validate_config(&config)?;
        let endpoint_text = format!(
            "{}/v1/messages",
            config.endpoint_base_url.trim_end_matches('/')
        );
        let endpoint = Url::parse(&endpoint_text).map_err(|error| {
            (
                ErrorCode::ConfigInvalid,
                format!("invalid model endpoint URL: {error}"),
            )
        })?;
        if !matches!(endpoint.scheme(), "http" | "https") {
            return Err((
                ErrorCode::ConfigInvalid,
                String::from("model endpoint URL must use http or https"),
            ));
        }
        let mut api_key = HeaderValue::from_str(&config.api_key).map_err(|error| {
            (
                ErrorCode::ConfigInvalid,
                format!("invalid model API-key header value: {error}"),
            )
        })?;
        api_key.set_sensitive(true);
        let http = Client::builder()
            .user_agent(concat!(
                "kittens-code-driver-tokio/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|error| {
                (
                    ErrorCode::ModelTransport,
                    format!("could not construct model HTTP client: {error}"),
                )
            })?;
        Ok(Self {
            http,
            endpoint,
            api_key,
            model: Arc::from(config.model),
            max_output_tokens: config.max_output_tokens,
            retry: config.retry,
            breaker: Arc::new(CircuitBreaker::new(
                config.retry.breaker_failure_threshold,
                config.retry.breaker_cooldown,
            )),
            jitter: Arc::new(Jitter::new(0x4b43_304c_4956_4501)),
        })
    }

    async fn complete_owned(self, window: WindowLayout) -> Result<ModelOutcome, ModelError> {
        let body = build_request_body(&window, &self.model, self.max_output_tokens)?;
        if let Some(error) = self.breaker.before_call(Instant::now()) {
            return Err(error);
        }

        let started = Instant::now();
        let mut attempt = 0;
        loop {
            match self.send_attempt(&body).await {
                Ok(outcome) => {
                    self.breaker.record_success();
                    return Ok(outcome);
                }
                Err(failure) => {
                    let elapsed = started.elapsed();
                    let backoff =
                        exponential_delay(self.retry.base_delay, attempt, self.retry.max_delay);
                    let jitter = self.jitter.next(backoff / 2);
                    match retry_decision(&self.retry, attempt, &failure, elapsed, jitter) {
                        RetryDecision::Retry(delay) => {
                            attempt += 1;
                            tokio::time::sleep(delay).await;
                        }
                        RetryDecision::GiveUp => {
                            let error = failure.into_model_error();
                            self.breaker.record_failure(Instant::now(), &error);
                            return Err(error);
                        }
                    }
                }
            }
        }
    }

    async fn send_attempt(&self, body: &[u8]) -> Result<ModelOutcome, AttemptFailure> {
        let response = self
            .http
            .post(self.endpoint.clone())
            .header("x-api-key", self.api_key.clone())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "text/event-stream")
            .body(body.to_vec())
            .send()
            .await
            .map_err(AttemptFailure::transport)?;

        if !response.status().is_success() {
            return Err(http_failure(response).await);
        }
        parse_stream(response, body.len()).await
    }
}

impl ModelClient for LiveClient {
    fn complete(&self, window: WindowLayout) -> ModelFuture {
        let client = self.clone();
        Box::pin(async move { client.complete_owned(window).await })
    }
}

fn validate_config(config: &LiveConfig) -> Result<(), ModelError> {
    if config.endpoint_base_url.trim().is_empty() {
        return Err((
            ErrorCode::ConfigInvalid,
            String::from("model endpoint base URL must not be empty"),
        ));
    }
    if config.api_key.is_empty() {
        return Err((
            ErrorCode::ConfigInvalid,
            String::from("model API key must not be empty"),
        ));
    }
    if config.model.trim().is_empty() {
        return Err((
            ErrorCode::ConfigInvalid,
            String::from("model id must not be empty"),
        ));
    }
    if config.max_output_tokens == 0 {
        return Err((
            ErrorCode::ConfigInvalid,
            String::from("max output tokens must be greater than zero"),
        ));
    }
    if config.retry.max_attempts == 0 || config.retry.breaker_failure_threshold == 0 {
        return Err((
            ErrorCode::ConfigInvalid,
            String::from("retry attempts and breaker threshold must be greater than zero"),
        ));
    }
    if config.retry.base_delay > config.retry.max_delay {
        return Err((
            ErrorCode::ConfigInvalid,
            String::from("retry base delay must not exceed its maximum delay"),
        ));
    }
    Ok(())
}

fn build_request_body(
    window: &WindowLayout,
    model: &str,
    max_output_tokens: u32,
) -> Result<Vec<u8>, ModelError> {
    let system = [
        Some(window.system.as_str()),
        Some(window.rules_reminder.as_str()),
    ]
    .into_iter()
    .chain(window.reminders.iter().map(String::as_str).map(Some))
    .flatten()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");

    let mut messages = Vec::new();
    let mut leading = Vec::new();
    if !window.user_info.is_empty() {
        leading.push(format!("[user_info]\n{}", window.user_info));
    }
    if !window.summary.is_empty() {
        leading.push(format!("[summary]\n{}", window.summary));
    }
    leading.push(window.last_user_query.clone());
    push_content(
        &mut messages,
        "user",
        json!({"type": "text", "text": leading.join("\n\n")}),
    );

    for item in &window.verbatim_tail {
        match item {
            TailItem::Message(text) => {
                let (role, text) = if let Some(text) = text.strip_prefix("[user] ") {
                    ("user", text)
                } else if let Some(text) = text.strip_prefix("[assistant] ") {
                    ("assistant", text)
                } else {
                    ("assistant", text.as_str())
                };
                push_content(&mut messages, role, json!({"type": "text", "text": text}));
            }
            TailItem::ToolCall { call, text } => {
                if let Some((name, input)) = parse_rendered_tool_call(text) {
                    push_content(
                        &mut messages,
                        "assistant",
                        json!({
                            "type": "tool_use",
                            "id": wire_tool_id(call.0),
                            "name": name,
                            "input": input,
                        }),
                    );
                } else {
                    push_content(
                        &mut messages,
                        "assistant",
                        json!({"type": "text", "text": format!("[tool_call {}] {text}", call.0)}),
                    );
                }
            }
            TailItem::ToolResult { call, text } => push_content(
                &mut messages,
                "user",
                json!({
                    "type": "tool_result",
                    "tool_use_id": wire_tool_id(call.0),
                    "content": text,
                }),
            ),
            _ => {}
        }
    }

    serde_json::to_vec(&json!({
        "model": model,
        "max_tokens": max_output_tokens,
        "stream": true,
        "system": system,
        "messages": messages,
    }))
    .map_err(|error| {
        (
            ErrorCode::Internal,
            format!("could not serialize model request: {error}"),
        )
    })
}

fn push_content(messages: &mut Vec<Value>, role: &str, block: Value) {
    if let Some(last) = messages.last_mut()
        && last.get("role").and_then(Value::as_str) == Some(role)
        && let Some(content) = last.get_mut("content").and_then(Value::as_array_mut)
    {
        content.push(block);
        return;
    }
    messages.push(json!({"role": role, "content": [block]}));
}

fn parse_rendered_tool_call(text: &str) -> Option<(&str, Value)> {
    let (name, args) = text.split_once(' ')?;
    if name.is_empty() {
        return None;
    }
    let input: Value = serde_json::from_str(args).ok()?;
    input.is_object().then_some((name, input))
}

fn wire_tool_id(effect: u64) -> String {
    format!("kc0_effect_{effect}")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureClass {
    Retryable,
    NonRetryable,
}

#[derive(Debug)]
struct AttemptFailure {
    code: ErrorCode,
    message: String,
    class: FailureClass,
    retry_after: Option<Duration>,
}

impl AttemptFailure {
    fn transport(error: impl std::fmt::Display) -> Self {
        Self {
            code: ErrorCode::ModelTransport,
            message: error.to_string(),
            class: FailureClass::Retryable,
            retry_after: None,
        }
    }

    fn terminal(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            class: FailureClass::NonRetryable,
            retry_after: None,
        }
    }

    fn overloaded(message: impl Into<String>, retry_after: Option<Duration>) -> Self {
        Self {
            code: ErrorCode::ModelOverloaded,
            message: message.into(),
            class: FailureClass::Retryable,
            retry_after,
        }
    }

    fn into_model_error(self) -> ModelError {
        (self.code, self.message)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetryDecision {
    Retry(Duration),
    GiveUp,
}

fn retry_decision(
    config: &RetryConfig,
    attempt: u32,
    failure: &AttemptFailure,
    elapsed: Duration,
    jitter: Duration,
) -> RetryDecision {
    if failure.class == FailureClass::NonRetryable || attempt + 1 >= config.max_attempts {
        return RetryDecision::GiveUp;
    }
    let delay = exponential_delay(config.base_delay, attempt, config.max_delay)
        .saturating_add(jitter)
        .min(config.max_delay);
    let delay = failure
        .retry_after
        .map_or(delay, |retry_after| delay.max(retry_after));
    if elapsed.saturating_add(delay) > config.max_elapsed {
        RetryDecision::GiveUp
    } else {
        RetryDecision::Retry(delay)
    }
}

fn exponential_delay(base: Duration, attempt: u32, maximum: Duration) -> Duration {
    let multiplier = 1_u32.checked_shl(attempt.min(31)).unwrap_or(u32::MAX);
    base.checked_mul(multiplier).unwrap_or(maximum).min(maximum)
}

async fn http_failure(response: reqwest::Response) -> AttemptFailure {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers().get(RETRY_AFTER));
    let body = response
        .text()
        .await
        .unwrap_or_else(|error| error.to_string());
    let (provider_type, provider_message) = provider_error(&body);
    let message = truncate_message(provider_message.as_deref().unwrap_or(&body));

    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
        || matches!(
            provider_type.as_deref(),
            Some("authentication_error" | "permission_error")
        )
    {
        AttemptFailure::terminal(ErrorCode::ModelAuth, message)
    } else if status == StatusCode::PAYLOAD_TOO_LARGE
        || is_context_error(provider_type.as_deref(), &message)
    {
        AttemptFailure::terminal(ErrorCode::ModelContextLength, message)
    } else if status == StatusCode::TOO_MANY_REQUESTS
        || status.as_u16() == 529
        || provider_type.as_deref() == Some("overloaded_error")
    {
        AttemptFailure::overloaded(message, retry_after)
    } else if status.is_server_error() {
        AttemptFailure {
            code: ErrorCode::ModelTransport,
            message,
            class: FailureClass::Retryable,
            retry_after,
        }
    } else {
        AttemptFailure::terminal(ErrorCode::ModelTransport, message)
    }
}

fn provider_error(body: &str) -> (Option<String>, Option<String>) {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return (None, None);
    };
    let error = value.get("error").unwrap_or(&value);
    (
        error.get("type").and_then(Value::as_str).map(str::to_owned),
        error
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_owned),
    )
}

fn is_context_error(provider_type: Option<&str>, message: &str) -> bool {
    if provider_type == Some("request_too_large") {
        return true;
    }
    let message = message.to_ascii_lowercase();
    message.contains("context window")
        || message.contains("context length")
        || message.contains("prompt is too long")
        || message.contains("too many input tokens")
        || message.contains("maximum context")
}

fn truncate_message(message: &str) -> String {
    message.chars().take(MAX_ERROR_MESSAGE_CHARS).collect()
}

fn parse_retry_after(value: Option<&HeaderValue>) -> Option<Duration> {
    let value = value?.to_str().ok()?;
    if let Ok(seconds) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let deadline = httpdate::parse_http_date(value).ok()?;
    Some(
        deadline
            .duration_since(SystemTime::now())
            .unwrap_or_default(),
    )
}

async fn parse_stream(
    mut response: reqwest::Response,
    prompt_bytes: usize,
) -> Result<ModelOutcome, AttemptFailure> {
    let mut decoder = SseDecoder::default();
    let mut state = StreamState::default();
    while let Some(chunk) = response.chunk().await.map_err(AttemptFailure::transport)? {
        for data in decoder.push(&chunk).map_err(AttemptFailure::transport)? {
            state.accept(&data)?;
        }
    }
    for data in decoder.finish().map_err(AttemptFailure::transport)? {
        state.accept(&data)?;
    }
    state.finish(prompt_bytes)
}

#[derive(Default)]
struct SseDecoder {
    pending: Vec<u8>,
    data_lines: Vec<String>,
}

impl SseDecoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, String> {
        self.pending.extend_from_slice(bytes);
        let mut events = Vec::new();
        while let Some(newline) = self.pending.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = self.pending.drain(..=newline).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            self.accept_line(&line, &mut events)?;
        }
        Ok(events)
    }

    fn finish(&mut self) -> Result<Vec<String>, String> {
        let mut events = Vec::new();
        if !self.pending.is_empty() {
            let line = std::mem::take(&mut self.pending);
            self.accept_line(&line, &mut events)?;
        }
        self.dispatch(&mut events);
        Ok(events)
    }

    fn accept_line(&mut self, line: &[u8], events: &mut Vec<String>) -> Result<(), String> {
        let line = std::str::from_utf8(line)
            .map_err(|error| format!("model SSE contained invalid UTF-8: {error}"))?;
        if line.is_empty() {
            self.dispatch(events);
        } else if !line.starts_with(':') {
            let (field, value) = line.split_once(':').unwrap_or((line, ""));
            if field == "data" {
                self.data_lines
                    .push(value.strip_prefix(' ').unwrap_or(value).to_owned());
            }
        }
        Ok(())
    }

    fn dispatch(&mut self, events: &mut Vec<String>) {
        if !self.data_lines.is_empty() {
            events.push(self.data_lines.join("\n"));
            self.data_lines.clear();
        }
    }
}

#[derive(Default)]
struct StreamState {
    saw_start: bool,
    saw_stop: bool,
    text: String,
    prompt_tokens: Option<u64>,
    open_tools: BTreeMap<u64, ToolBlock>,
    tool_calls: BTreeMap<u64, ProposedToolCall>,
}

struct ToolBlock {
    name: String,
    initial_input: Value,
    partial_json: String,
}

impl StreamState {
    fn accept(&mut self, data: &str) -> Result<(), AttemptFailure> {
        let event: Value = serde_json::from_str(data).map_err(|error| {
            AttemptFailure::transport(format!("malformed model SSE JSON: {error}"))
        })?;
        match event.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                self.saw_start = true;
                self.prompt_tokens = event
                    .pointer("/message/usage/input_tokens")
                    .and_then(Value::as_u64);
            }
            Some("content_block_start") => self.content_start(&event)?,
            Some("content_block_delta") => self.content_delta(&event)?,
            Some("content_block_stop") => self.content_stop(&event)?,
            Some("message_stop") => self.saw_stop = true,
            Some("error") => return Err(stream_error(&event)),
            _ => {}
        }
        Ok(())
    }

    fn content_start(&mut self, event: &Value) -> Result<(), AttemptFailure> {
        let Some(index) = event.get("index").and_then(Value::as_u64) else {
            return Err(AttemptFailure::transport(
                "content_block_start omitted its index",
            ));
        };
        let block = event.get("content_block").ok_or_else(|| {
            AttemptFailure::transport("content_block_start omitted its content block")
        })?;
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    self.text.push_str(text);
                }
            }
            Some("tool_use") => {
                let name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AttemptFailure::transport("tool_use block omitted its name"))?;
                if self
                    .open_tools
                    .insert(
                        index,
                        ToolBlock {
                            name: name.to_owned(),
                            initial_input: block.get("input").cloned().unwrap_or_else(|| json!({})),
                            partial_json: String::new(),
                        },
                    )
                    .is_some()
                {
                    return Err(AttemptFailure::transport(
                        "duplicate tool_use content-block index",
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn content_delta(&mut self, event: &Value) -> Result<(), AttemptFailure> {
        let Some(delta) = event.get("delta") else {
            return Err(AttemptFailure::transport(
                "content_block_delta omitted its delta",
            ));
        };
        match delta.get("type").and_then(Value::as_str) {
            Some("text_delta") => {
                if let Some(text) = delta.get("text").and_then(Value::as_str) {
                    self.text.push_str(text);
                }
            }
            Some("input_json_delta") => {
                let index = event
                    .get("index")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| AttemptFailure::transport("tool delta omitted its index"))?;
                let tool = self.open_tools.get_mut(&index).ok_or_else(|| {
                    AttemptFailure::transport("tool delta named an unopened content block")
                })?;
                if let Some(partial) = delta.get("partial_json").and_then(Value::as_str) {
                    tool.partial_json.push_str(partial);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn content_stop(&mut self, event: &Value) -> Result<(), AttemptFailure> {
        let Some(index) = event.get("index").and_then(Value::as_u64) else {
            return Err(AttemptFailure::transport(
                "content_block_stop omitted its index",
            ));
        };
        let Some(tool) = self.open_tools.remove(&index) else {
            return Ok(());
        };
        let input = if tool.partial_json.is_empty() {
            tool.initial_input
        } else {
            serde_json::from_str(&tool.partial_json).map_err(|error| {
                AttemptFailure::transport(format!("malformed streamed tool input: {error}"))
            })?
        };
        let args_json = serde_json::to_string(&input).map_err(|error| {
            AttemptFailure::transport(format!("could not preserve streamed tool input: {error}"))
        })?;
        self.tool_calls.insert(
            index,
            ProposedToolCall {
                name: tool.name,
                args_json,
            },
        );
        Ok(())
    }

    fn finish(self, prompt_bytes: usize) -> Result<ModelOutcome, AttemptFailure> {
        if !self.saw_start || !self.saw_stop || !self.open_tools.is_empty() {
            return Err(AttemptFailure::transport(
                "model SSE ended before a complete message_stop sequence",
            ));
        }
        let prompt_bytes = u64::try_from(prompt_bytes).unwrap_or(u64::MAX);
        Ok(ModelOutcome {
            text: self.text,
            tool_calls: self.tool_calls.into_values().collect(),
            usage: self.prompt_tokens.map(|prompt_tokens| Usage {
                prompt_tokens,
                prompt_bytes,
            }),
        })
    }
}

fn stream_error(event: &Value) -> AttemptFailure {
    let provider_type = event.pointer("/error/type").and_then(Value::as_str);
    let message = event
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("provider returned an SSE error");
    match provider_type {
        Some("overloaded_error" | "rate_limit_error") => AttemptFailure::overloaded(message, None),
        Some("authentication_error" | "permission_error") => {
            AttemptFailure::terminal(ErrorCode::ModelAuth, message)
        }
        Some("invalid_request_error" | "request_too_large")
            if is_context_error(provider_type, message) =>
        {
            AttemptFailure::terminal(ErrorCode::ModelContextLength, message)
        }
        Some("invalid_request_error") => {
            AttemptFailure::terminal(ErrorCode::ModelTransport, message)
        }
        _ => AttemptFailure::transport(message),
    }
}

struct Jitter {
    state: AtomicU64,
}

impl Jitter {
    const fn new(seed: u64) -> Self {
        Self {
            state: AtomicU64::new(seed),
        }
    }

    fn next(&self, maximum: Duration) -> Duration {
        let mut current = self.state.load(Ordering::Relaxed);
        loop {
            let mut next = current;
            next ^= next << 13;
            next ^= next >> 7;
            next ^= next << 17;
            match self.state.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    let maximum_millis = u64::try_from(maximum.as_millis()).unwrap_or(u64::MAX);
                    let millis = if maximum_millis == u64::MAX {
                        next
                    } else {
                        next % (maximum_millis + 1)
                    };
                    return Duration::from_millis(millis);
                }
                Err(observed) => current = observed,
            }
        }
    }
}

struct CircuitBreaker {
    failure_threshold: u32,
    cooldown: Duration,
    state: Mutex<BreakerState>,
}

#[derive(Default)]
struct BreakerState {
    consecutive_failures: u32,
    open_until: Option<Instant>,
    last_error: Option<ModelError>,
}

impl CircuitBreaker {
    fn new(failure_threshold: u32, cooldown: Duration) -> Self {
        Self {
            failure_threshold,
            cooldown,
            state: Mutex::new(BreakerState::default()),
        }
    }

    fn before_call(&self, now: Instant) -> Option<ModelError> {
        let mut state = lock_unpoisoned(&self.state);
        if let Some(open_until) = state.open_until {
            if now < open_until {
                let (code, message) = state.last_error.clone().unwrap_or((
                    ErrorCode::ModelTransport,
                    String::from("model circuit is open"),
                ));
                return Some((
                    code,
                    format!("model circuit is open after consecutive failures: {message}"),
                ));
            }
            *state = BreakerState::default();
        }
        None
    }

    fn record_success(&self) {
        *lock_unpoisoned(&self.state) = BreakerState::default();
    }

    fn record_failure(&self, now: Instant, error: &ModelError) {
        let mut state = lock_unpoisoned(&self.state);
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        state.last_error = Some(error.clone());
        if state.consecutive_failures >= self.failure_threshold {
            state.open_until = Some(now + self.cooldown);
        }
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kittens_code_protocol::ids::EffectId;

    fn retry_config() -> RetryConfig {
        RetryConfig {
            max_attempts: 4,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(2),
            max_elapsed: Duration::from_secs(20),
            breaker_failure_threshold: 3,
            breaker_cooldown: Duration::from_secs(5),
        }
    }

    #[test]
    fn overload_retries_exponentially_and_honors_retry_after() {
        let config = retry_config();
        let overload = AttemptFailure::overloaded("busy", None);
        assert_eq!(
            retry_decision(
                &config,
                0,
                &overload,
                Duration::ZERO,
                Duration::from_millis(7),
            ),
            RetryDecision::Retry(Duration::from_millis(107))
        );
        assert_eq!(
            retry_decision(&config, 2, &overload, Duration::ZERO, Duration::ZERO,),
            RetryDecision::Retry(Duration::from_millis(400))
        );

        let rate_limited = AttemptFailure::overloaded("limited", Some(Duration::from_secs(3)));
        assert_eq!(
            retry_decision(&config, 0, &rate_limited, Duration::ZERO, Duration::ZERO,),
            RetryDecision::Retry(Duration::from_secs(3))
        );
    }

    #[test]
    fn terminal_failures_and_exhausted_bounds_give_up() {
        let config = retry_config();
        for code in [ErrorCode::ModelAuth, ErrorCode::ModelContextLength] {
            let failure = AttemptFailure::terminal(code, "terminal");
            assert_eq!(
                retry_decision(&config, 0, &failure, Duration::ZERO, Duration::ZERO,),
                RetryDecision::GiveUp
            );
        }
        let overload = AttemptFailure::overloaded("busy", None);
        assert_eq!(
            retry_decision(
                &config,
                config.max_attempts - 1,
                &overload,
                Duration::ZERO,
                Duration::ZERO,
            ),
            RetryDecision::GiveUp
        );
    }

    #[test]
    fn breaker_opens_at_threshold_and_resets_after_cooldown_or_success() {
        let config = retry_config();
        let breaker =
            CircuitBreaker::new(config.breaker_failure_threshold, config.breaker_cooldown);
        let now = Instant::now();
        let error = (ErrorCode::ModelOverloaded, String::from("busy"));
        breaker.record_failure(now, &error);
        breaker.record_failure(now, &error);
        assert!(breaker.before_call(now).is_none());
        breaker.record_failure(now, &error);
        assert_eq!(
            breaker.before_call(now).unwrap().0,
            ErrorCode::ModelOverloaded
        );
        assert!(breaker.before_call(now + config.breaker_cooldown).is_none());

        breaker.record_failure(now, &error);
        breaker.record_success();
        assert!(breaker.before_call(now).is_none());
    }

    #[test]
    fn request_lowering_preserves_window_roles_and_tool_pairs() {
        let window = WindowLayout::new(
            String::from("system"),
            String::from("workspace"),
            String::from("rules"),
            String::from("question"),
            vec![
                TailItem::Message(String::from("[assistant] thinking")),
                TailItem::ToolCall {
                    call: EffectId(7),
                    text: String::from("read {\"path\":\"src/lib.rs\"}"),
                },
                TailItem::ToolResult {
                    call: EffectId(7),
                    text: String::from("contents"),
                },
                TailItem::Message(String::from("[user] continue")),
            ],
            String::from("earlier summary"),
            vec![String::from("remember")],
        )
        .unwrap();
        let body = build_request_body(&window, "claude-test-model", 777).unwrap();
        let request: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(request["model"], "claude-test-model");
        assert_eq!(request["max_tokens"], 777);
        assert_eq!(request["stream"], true);
        assert_eq!(request["system"], "system\n\nrules\n\nremember");
        assert!(request.get("tools").is_none());
        assert_eq!(request["messages"][0]["role"], "user");
        assert!(
            request["messages"][0]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("question")
        );
        assert_eq!(request["messages"][1]["role"], "assistant");
        assert_eq!(request["messages"][1]["content"][1]["type"], "tool_use");
        assert_eq!(request["messages"][1]["content"][1]["id"], "kc0_effect_7");
        assert_eq!(request["messages"][2]["role"], "user");
        assert_eq!(request["messages"][2]["content"][0]["type"], "tool_result");
        assert_eq!(request["messages"][2]["content"][1]["text"], "continue");
    }

    #[test]
    fn streamed_sse_collects_text_tools_and_usage_across_chunk_boundaries() {
        let wire = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":42}}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"read\",\"input\":{}}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"src/lib.rs\\\"}\"}}\n\n",
            "data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
            "data: {\"type\":\"future_additive_event\"}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let mut decoder = SseDecoder::default();
        let mut state = StreamState::default();
        for chunk in wire.as_bytes().chunks(7) {
            for data in decoder.push(chunk).unwrap() {
                state.accept(&data).unwrap();
            }
        }
        for data in decoder.finish().unwrap() {
            state.accept(&data).unwrap();
        }
        let outcome = state.finish(1234).unwrap();

        assert_eq!(outcome.text, "hello");
        assert_eq!(
            outcome.tool_calls,
            vec![ProposedToolCall {
                name: String::from("read"),
                args_json: String::from("{\"path\":\"src/lib.rs\"}"),
            }]
        );
        assert_eq!(
            outcome.usage,
            Some(Usage {
                prompt_tokens: 42,
                prompt_bytes: 1234,
            })
        );
    }
}
