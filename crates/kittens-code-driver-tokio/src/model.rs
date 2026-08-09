//! Model clients: the trait the reactor's owned pump tasks call, plus the
//! deterministic jail (SPEC L-D3, D14). The live Anthropic-dialect client
//! ships behind the `live` feature (gate G10).

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use kittens_code_core::engine::{ModelOutcome, ProposedToolCall, Usage};
use kittens_code_core::window::{TailItem, WindowLayout};
use kittens_code_protocol::error::ErrorCode;
use serde::Deserialize;

/// A model-call failure the engine can classify.
pub type ModelError = (ErrorCode, String);

/// Boxed completion future so the trait stays object-safe without macros.
pub type ModelFuture = Pin<Box<dyn Future<Output = Result<ModelOutcome, ModelError>> + Send>>;

/// The seam every model backend implements (root and sub tiers alike).
pub trait ModelClient: Send + Sync {
    /// One completion over an assembled window.
    fn complete(&self, window: WindowLayout) -> ModelFuture;
}

/// One scripted jail step (D14: ordinal matching for KC0).
#[derive(Clone, Debug, Deserialize)]
pub struct JailStep {
    /// Assistant text to return.
    #[serde(default)]
    pub text: String,
    /// Tool calls to propose, in order: `[name, args_json]` pairs.
    #[serde(default)]
    pub tool_calls: Vec<(String, String)>,
    /// Optional scripted provider usage `(prompt_tokens, prompt_bytes)`.
    #[serde(default)]
    pub usage: Option<(u64, u64)>,
    /// When set, fail with this scripted transport error instead.
    #[serde(default)]
    pub fail: Option<String>,
}

/// The deterministic mock-model jail: scripted responses consumed in
/// order; running past the script is a hard scenario failure (D14).
pub struct JailClient {
    steps: Vec<JailStep>,
    cursor: Mutex<usize>,
    /// Captured request summaries (window sizes) for behavioral diffing.
    capture: Mutex<Vec<String>>,
}

impl JailClient {
    /// Builds a jail from scripted steps.
    #[must_use]
    pub fn new(steps: Vec<JailStep>) -> Self {
        Self {
            steps,
            cursor: Mutex::new(0),
            capture: Mutex::new(Vec::new()),
        }
    }

    /// The captured request summaries, for scenario diffing.
    ///
    /// # Panics
    ///
    /// If the capture mutex was poisoned by a panicking caller.
    #[must_use]
    pub fn captured(&self) -> Vec<String> {
        self.capture.lock().expect("capture lock").clone()
    }
}

/// Renders a compact deterministic request fingerprint for capture.
fn fingerprint(window: &WindowLayout) -> String {
    let tail_kinds: String = window
        .verbatim_tail
        .iter()
        .map(|item| match item {
            TailItem::Message(_) => 'm',
            TailItem::ToolCall { .. } => 'c',
            TailItem::ToolResult { .. } => 'r',
            _ => '?',
        })
        .collect();
    format!(
        "q={} tail={} summary={}",
        window.last_user_query.len(),
        tail_kinds,
        window.summary.len()
    )
}

impl ModelClient for JailClient {
    fn complete(&self, window: WindowLayout) -> ModelFuture {
        self.capture
            .lock()
            .expect("capture lock")
            .push(fingerprint(&window));
        let step = {
            let mut cursor = self.cursor.lock().expect("cursor lock");
            let step = self.steps.get(*cursor).cloned();
            *cursor += 1;
            step
        };
        Box::pin(async move {
            let Some(step) = step else {
                return Err((
                    ErrorCode::Internal,
                    String::from("jail scenario exhausted: unexpected model call"),
                ));
            };
            if let Some(message) = step.fail {
                return Err((ErrorCode::ModelTransport, message));
            }
            Ok(ModelOutcome {
                text: step.text,
                tool_calls: step
                    .tool_calls
                    .into_iter()
                    .map(|(name, args_json)| ProposedToolCall { name, args_json })
                    .collect(),
                usage: step.usage.map(|(prompt_tokens, prompt_bytes)| Usage {
                    prompt_tokens,
                    prompt_bytes,
                }),
            })
        })
    }
}
