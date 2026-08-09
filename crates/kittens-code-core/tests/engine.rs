//! Turn-engine law tests (SPEC L-T1..L-T4, L-A1; gates G4b-adjacent at the
//! core layer — driver-level cancellation propagation is tested in the
//! driver crate).

use kittens_code_core::engine::{
    CoreAction, CoreInput, EffectSpec, EffectTerminal, Engine, ModelOutcome, ProposedToolCall,
};
use kittens_code_protocol::config::SessionConfig;
use kittens_code_protocol::error::ErrorCode;
use kittens_code_protocol::event::{Event, ToolOutcome, TurnEnd};
use kittens_code_protocol::ids::{EffectId, SubmissionId, TurnEpoch};
use kittens_code_protocol::op::{Op, Submission};
use kittens_code_protocol::policy::{ApprovalPolicy, ApprovalVerdict};

fn engine() -> Engine {
    Engine::new(SessionConfig::default(), 1)
}

fn user_input(engine: &mut Engine, text: &str) -> Vec<CoreAction> {
    engine
        .handle(CoreInput::ClientOp(Submission {
            id: SubmissionId(1),
            op: Op::UserInput {
                text: String::from(text),
            },
        }))
        .actions
}

fn started_effects(actions: &[CoreAction]) -> Vec<(EffectId, TurnEpoch, EffectSpec)> {
    actions
        .iter()
        .filter_map(|a| match a {
            CoreAction::StartEffect { id, epoch, spec } => Some((*id, *epoch, spec.clone())),
            _ => None,
        })
        .collect()
}

fn published(actions: &[CoreAction]) -> Vec<&Event> {
    actions
        .iter()
        .filter_map(|a| match a {
            CoreAction::Publish(e) => Some(e),
            _ => None,
        })
        .collect()
}

#[test]
fn user_input_starts_a_turn_and_a_model_call() {
    let mut e = engine();
    let actions = user_input(&mut e, "hello");
    let started = started_effects(&actions);
    assert_eq!(started.len(), 1);
    assert!(matches!(started[0].2, EffectSpec::ModelCall(_)));
    assert!(
        published(&actions)
            .iter()
            .any(|ev| matches!(ev, Event::TurnStarted { .. }))
    );
    // The window carries the standing RLM reminder (C6).
    if let EffectSpec::ModelCall(window) = &started[0].2 {
        assert!(!window.reminders.is_empty());
    }
}

#[test]
fn message_only_terminal_ends_the_turn() {
    let mut e = engine();
    let actions = user_input(&mut e, "hi");
    let (model_id, epoch, _) = started_effects(&actions)[0].clone();
    let actions = e
        .handle(CoreInput::EffectFinished {
            id: model_id,
            epoch,
            terminal: EffectTerminal::Model(ModelOutcome {
                text: String::from("done"),
                tool_calls: vec![],
                usage: None,
            }),
        })
        .actions;
    assert!(published(&actions).iter().any(|ev| matches!(
        ev,
        Event::TurnEnded {
            reason: TurnEnd::Completed,
            ..
        }
    )));
    assert!(started_effects(&actions).is_empty());
}

#[test]
fn tool_call_round_trip_resamples() {
    let mut e = engine();
    let actions = user_input(&mut e, "read a file");
    let (model_id, epoch, _) = started_effects(&actions)[0].clone();
    let actions = e
        .handle(CoreInput::EffectFinished {
            id: model_id,
            epoch,
            terminal: EffectTerminal::Model(ModelOutcome {
                text: String::new(),
                tool_calls: vec![ProposedToolCall {
                    name: String::from("read"),
                    args_json: String::from("{\"path\":\"a.rs\"}"),
                }],
                usage: None,
            }),
        })
        .actions;
    let started = started_effects(&actions);
    assert_eq!(started.len(), 1);
    let (tool_id, tool_epoch, spec) = started[0].clone();
    assert!(matches!(spec, EffectSpec::Tool(_)));

    let actions = e
        .handle(CoreInput::EffectFinished {
            id: tool_id,
            epoch: tool_epoch,
            terminal: EffectTerminal::Tool {
                outcome: ToolOutcome::Succeeded,
                output: String::from("file contents"),
            },
        })
        .actions;
    // All tools terminal -> resample: a fresh model call starts.
    let started = started_effects(&actions);
    assert_eq!(started.len(), 1);
    assert!(matches!(started[0].2, EffectSpec::ModelCall(_)));
    // The resample window's tail pairs the call with its result.
    if let EffectSpec::ModelCall(window) = &started[0].2 {
        assert!(window.verbatim_tail.len() >= 2);
    }
}

#[test]
fn duplicate_terminal_is_dropped_with_trace_only() {
    let mut e = engine();
    let actions = user_input(&mut e, "x");
    let (model_id, epoch, _) = started_effects(&actions)[0].clone();
    let finish = CoreInput::EffectFinished {
        id: model_id,
        epoch,
        terminal: EffectTerminal::Model(ModelOutcome {
            text: String::from("a"),
            tool_calls: vec![],
            usage: None,
        }),
    };
    let _ = e.handle(finish.clone());
    let actions = e.handle(finish).actions;
    // Ledger drop: no events, no effects — only the trace commit.
    assert!(published(&actions).is_empty());
    assert!(started_effects(&actions).is_empty());
    assert!(actions.iter().any(|a| matches!(a, CoreAction::Commit(_))));
}

#[test]
fn stale_epoch_terminal_is_dropped() {
    let mut e = engine();
    let actions = user_input(&mut e, "x");
    let (model_id, _epoch, _) = started_effects(&actions)[0].clone();
    let actions = e
        .handle(CoreInput::EffectFinished {
            id: model_id,
            epoch: TurnEpoch(999),
            terminal: EffectTerminal::Model(ModelOutcome {
                text: String::from("stale"),
                tool_calls: vec![],
                usage: None,
            }),
        })
        .actions;
    assert!(published(&actions).is_empty());
}

#[test]
fn interrupt_cancels_pending_and_ends_turn() {
    let mut e = engine();
    let actions = user_input(&mut e, "x");
    let (model_id, _, _) = started_effects(&actions)[0].clone();
    let actions = e
        .handle(CoreInput::ClientOp(Submission {
            id: SubmissionId(2),
            op: Op::Interrupt,
        }))
        .actions;
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, CoreAction::CancelEffect { id } if *id == model_id))
    );
    assert!(published(&actions).iter().any(|ev| matches!(
        ev,
        Event::TurnEnded {
            reason: TurnEnd::Interrupted,
            ..
        }
    )));
}

#[test]
fn stationarity_guard_ends_a_doom_loop() {
    let mut e = engine();
    let mut actions = user_input(&mut e, "loop");
    let mut guard_fired = false;
    // Feed identical tool proposals until the guard fires. Each cycle is
    // two transitions (model terminal, tool terminal), and the default
    // threshold is 16 identical proposals.
    for _ in 0..40 {
        let Some((id, epoch, spec)) = started_effects(&actions).into_iter().next() else {
            break;
        };
        match spec {
            EffectSpec::ModelCall(_) => {
                actions = e
                    .handle(CoreInput::EffectFinished {
                        id,
                        epoch,
                        terminal: EffectTerminal::Model(ModelOutcome {
                            text: String::new(),
                            tool_calls: vec![ProposedToolCall {
                                name: String::from("noop"),
                                args_json: String::from("{}"),
                            }],
                            usage: None,
                        }),
                    })
                    .actions;
            }
            EffectSpec::Tool(_) => {
                actions = e
                    .handle(CoreInput::EffectFinished {
                        id,
                        epoch,
                        terminal: EffectTerminal::Tool {
                            outcome: ToolOutcome::Succeeded,
                            output: String::from("same"),
                        },
                    })
                    .actions;
            }
            _ => break,
        }
        if published(&actions).iter().any(|ev| {
            matches!(
                ev,
                Event::TurnEnded {
                    reason: TurnEnd::Failed,
                    ..
                }
            )
        }) {
            guard_fired = true;
            break;
        }
    }
    assert!(guard_fired, "stationarity guard never fired");
}

#[test]
fn ask_policy_holds_the_tool_until_approved() {
    let mut config = SessionConfig::default();
    config
        .approval_defaults
        .insert(String::from("exec"), ApprovalPolicy::Ask);
    let mut e = Engine::new(config, 1);
    let actions = user_input(&mut e, "run something");
    let (model_id, epoch, _) = started_effects(&actions)[0].clone();
    let actions = e
        .handle(CoreInput::EffectFinished {
            id: model_id,
            epoch,
            terminal: EffectTerminal::Model(ModelOutcome {
                text: String::new(),
                tool_calls: vec![ProposedToolCall {
                    name: String::from("exec"),
                    args_json: String::from("{\"argv\":[\"ls\"]}"),
                }],
                usage: None,
            }),
        })
        .actions;
    // No tool starts yet; an approval request is published instead.
    assert!(started_effects(&actions).is_empty());
    let request = published(&actions)
        .iter()
        .find_map(|ev| match ev {
            Event::ApprovalRequested { request, .. } => Some(*request),
            _ => None,
        })
        .expect("approval requested");

    let actions = e
        .handle(CoreInput::ClientOp(Submission {
            id: SubmissionId(3),
            op: Op::Approve {
                request,
                verdict: ApprovalVerdict::Approve,
            },
        }))
        .actions;
    assert_eq!(started_effects(&actions).len(), 1);
}

#[test]
fn tool_output_is_capped_with_log_pointer() {
    let mut e = engine();
    let actions = user_input(&mut e, "big output");
    let (model_id, epoch, _) = started_effects(&actions)[0].clone();
    let actions = e
        .handle(CoreInput::EffectFinished {
            id: model_id,
            epoch,
            terminal: EffectTerminal::Model(ModelOutcome {
                text: String::new(),
                tool_calls: vec![ProposedToolCall {
                    name: String::from("read"),
                    args_json: String::from("{}"),
                }],
                usage: None,
            }),
        })
        .actions;
    let (tool_id, tool_epoch, _) = started_effects(&actions)[0].clone();
    let huge = "z".repeat(50_000);
    let actions = e
        .handle(CoreInput::EffectFinished {
            id: tool_id,
            epoch: tool_epoch,
            terminal: EffectTerminal::Tool {
                outcome: ToolOutcome::Succeeded,
                output: huge,
            },
        })
        .actions;
    let (_, _, spec) = started_effects(&actions)[0].clone();
    if let EffectSpec::ModelCall(window) = spec {
        let result_text = window
            .verbatim_tail
            .iter()
            .find_map(|item| match item {
                kittens_code_core::window::TailItem::ToolResult { text, .. } => Some(text),
                _ => None,
            })
            .expect("tool result in tail");
        assert!(result_text.len() < 10_000, "window copy was not capped");
        assert!(result_text.contains("full output at log seq"));
    } else {
        panic!("expected resample");
    }
}

#[test]
fn persist_failure_is_fatal_and_cancels() {
    let mut e = engine();
    let actions = user_input(&mut e, "x");
    let (model_id, _, _) = started_effects(&actions)[0].clone();
    let actions = e
        .handle(CoreInput::PersistFailed {
            at_seq: 5,
            message: String::from("disk gone"),
        })
        .actions;
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, CoreAction::CancelEffect { id } if *id == model_id))
    );
    assert!(published(&actions).iter().any(|ev| matches!(
        ev,
        Event::Error(err) if matches!(err.code, ErrorCode::StoreIo)
    )));
    // A fresh user input after failure starts nothing.
    let actions = user_input(&mut e, "again");
    assert!(started_effects(&actions).is_empty());
}
