//! Model clients: the trait the reactor's owned pump tasks call, plus the
//! deterministic jail (SPEC L-D3, D14). The live Anthropic-dialect client
//! ships behind the `live` feature (gate G10).

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use kittens_code_core::engine::{ModelOutcome, ProposedToolCall, Usage};
use kittens_code_core::rlm::exec::AskRequest;
use kittens_code_core::window::{TailItem, WindowLayout};
use kittens_code_protocol::error::ErrorCode;
use serde::Deserialize;

#[cfg(feature = "live")]
pub use crate::live::{LiveClient, LiveConfig, RetryConfig};

/// A model-call failure the engine can classify.
pub type ModelError = (ErrorCode, String);

/// Boxed completion future so the trait stays object-safe without macros.
pub type ModelFuture = Pin<Box<dyn Future<Output = Result<ModelOutcome, ModelError>> + Send>>;

/// The seam every model backend implements (root and sub tiers alike).
pub trait ModelClient: Send + Sync {
    /// One completion over an assembled window.
    fn complete(&self, window: WindowLayout) -> ModelFuture;

    /// One RLM sub-model completion.
    ///
    /// The default maps the ask context and question into a small
    /// [`WindowLayout`] and reuses [`Self::complete`]. Backends that resolve
    /// `SessionConfig::model_sub` separately can override this method without
    /// changing the core/driver effect contract.
    fn complete_submodel(&self, request: AskRequest) -> ModelFuture {
        self.complete(submodel_window(&request))
    }
}

fn submodel_window(request: &AskRequest) -> WindowLayout {
    let sampling = request.sample_k.map_or_else(String::new, |count| {
        format!("\nRequested samples: {count}.")
    });
    WindowLayout::new(
        String::from("Answer from the supplied transcript context only."),
        String::new(),
        String::new(),
        format!(
            "Context:\n{}\n\nQuestion:\n{}{}",
            request.context, request.question, sampling
        ),
        Vec::new(),
        String::new(),
        Vec::new(),
    )
    .expect("an empty verbatim tail is always atomic")
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

#[cfg(test)]
mod tests {
    use super::*;
    use kittens_code_core::caps::Capped;
    use kittens_code_protocol::ids::EffectId;

    fn window_with_tail() -> WindowLayout {
        WindowLayout::new(
            String::from("system"),
            String::new(),
            String::new(),
            String::from("question"),
            vec![
                TailItem::Message(String::from("message")),
                TailItem::ToolCall {
                    call: EffectId(9),
                    text: String::from("read {}"),
                },
                TailItem::ToolResult {
                    call: EffectId(9),
                    text: Capped::head("result", 64, None),
                },
            ],
            String::from("summary"),
            Vec::new(),
        )
        .expect("valid paired tail")
    }

    #[tokio::test]
    async fn jail_success_reports_usage_tool_calls_and_capture() {
        let client = JailClient::new(vec![JailStep {
            text: String::from("answer"),
            tool_calls: vec![(String::from("read"), String::from("{\"path\":\"x\"}"))],
            usage: Some((17, 99)),
            fail: None,
        }]);
        let outcome = client
            .complete(window_with_tail())
            .await
            .expect("scripted success");
        assert_eq!(outcome.text, "answer");
        assert_eq!(outcome.tool_calls[0].name, "read");
        assert_eq!(
            outcome.usage,
            Some(Usage {
                prompt_tokens: 17,
                prompt_bytes: 99,
            })
        );
        assert_eq!(
            client.captured(),
            vec![String::from("q=8 tail=mcr summary=7")]
        );
    }

    #[tokio::test]
    async fn jail_fail_step_and_script_exhaustion_are_classified() {
        let client = JailClient::new(vec![JailStep {
            text: String::new(),
            tool_calls: Vec::new(),
            usage: None,
            fail: Some(String::from("transport down")),
        }]);
        assert_eq!(
            client.complete(window_with_tail()).await,
            Err((ErrorCode::ModelTransport, String::from("transport down")))
        );
        let exhausted = client
            .complete(window_with_tail())
            .await
            .expect_err("second call exhausts the one-step script");
        assert_eq!(exhausted.0, ErrorCode::Internal);
        assert!(exhausted.1.contains("scenario exhausted"));
        assert_eq!(client.captured().len(), 2);
    }

    #[tokio::test]
    async fn complete_submodel_wraps_context_question_and_sampling() {
        let client = JailClient::new(vec![
            JailStep {
                text: String::from("sampled"),
                tool_calls: Vec::new(),
                usage: None,
                fail: None,
            },
            JailStep {
                text: String::from("plain"),
                tool_calls: Vec::new(),
                usage: None,
                fail: None,
            },
        ]);
        let sampled = AskRequest {
            index: 3,
            question: String::from("why?"),
            context: String::from("evidence"),
            sample_k: Some(4),
        };
        let sampled_window = submodel_window(&sampled);
        assert_eq!(
            sampled_window.system,
            "Answer from the supplied transcript context only."
        );
        assert!(
            sampled_window
                .last_user_query
                .contains("Context:\nevidence")
        );
        assert!(sampled_window.last_user_query.contains("Question:\nwhy?"));
        assert!(
            sampled_window
                .last_user_query
                .contains("Requested samples: 4.")
        );
        assert!(sampled_window.verbatim_tail.is_empty());

        assert_eq!(
            client
                .complete_submodel(sampled)
                .await
                .expect("sampled completion")
                .text,
            "sampled"
        );
        let plain = AskRequest {
            index: 4,
            question: String::from("what?"),
            context: String::from("facts"),
            sample_k: None,
        };
        assert!(
            !submodel_window(&plain)
                .last_user_query
                .contains("Requested samples")
        );
        assert_eq!(
            client
                .complete_submodel(plain)
                .await
                .expect("plain completion")
                .text,
            "plain"
        );
    }
}
