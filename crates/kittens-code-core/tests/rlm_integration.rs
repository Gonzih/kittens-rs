//! Turn-engine integration tests for the Q4/Q8 recall continuation seam.

use kittens_code_core::engine::{
    CoreAction, CoreInput, EffectSpec, EffectTerminal, Engine, ModelOutcome, ProposedToolCall,
};
use kittens_code_core::rlm::exec::{AskResult, Page, PageRecord};
use kittens_code_protocol::config::SessionConfig;
use kittens_code_protocol::event::{Event, ToolOutcome};
use kittens_code_protocol::ids::{EffectId, SubmissionId, TurnEpoch};
use kittens_code_protocol::op::{Op, Submission};

fn started_effects(actions: &[CoreAction]) -> Vec<(EffectId, TurnEpoch, EffectSpec)> {
    actions
        .iter()
        .filter_map(|action| match action {
            CoreAction::StartEffect { id, epoch, spec } => Some((*id, *epoch, spec.clone())),
            _ => None,
        })
        .collect()
}

fn start_turn(engine: &mut Engine) -> (EffectId, TurnEpoch) {
    let actions = engine
        .handle(CoreInput::ClientOp(Submission {
            id: SubmissionId(1),
            op: Op::UserInput {
                text: String::from("remember x"),
            },
        }))
        .actions;
    let (id, epoch, spec) = started_effects(&actions).remove(0);
    assert!(matches!(spec, EffectSpec::ModelCall(_)));
    (id, epoch)
}

fn propose_recall(
    engine: &mut Engine,
    model: EffectId,
    epoch: TurnEpoch,
    script: &str,
) -> Vec<CoreAction> {
    engine
        .handle(CoreInput::EffectFinished {
            id: model,
            epoch,
            terminal: EffectTerminal::Model(ModelOutcome {
                text: String::new(),
                tool_calls: vec![ProposedToolCall {
                    name: String::from("recall"),
                    args_json: serde_json::json!({ "script": script }).to_string(),
                }],
                usage: None,
            }),
        })
        .actions
}

#[test]
fn recall_pages_resolve_as_a_capped_tool_result_then_resample() {
    let mut engine = Engine::new(SessionConfig::default(), 1);
    let (model, epoch) = start_turn(&mut engine);
    let actions = propose_recall(&mut engine, model, epoch, "grep \"x\"\nfinal %1");
    let (page_id, page_epoch, spec) = started_effects(&actions).remove(0);
    assert!(matches!(
        spec,
        EffectSpec::StoreReadPage { cursor: None, .. }
    ));

    let page_terminal = EffectTerminal::Pages(Page {
        records: vec![PageRecord {
            seq: 7,
            text: String::from("record with x"),
        }],
        next_cursor: None,
    });
    let actions = engine
        .handle(CoreInput::EffectFinished {
            id: page_id,
            epoch: page_epoch,
            terminal: page_terminal.clone(),
        })
        .actions;
    let (_, _, spec) = started_effects(&actions).remove(0);
    let EffectSpec::ModelCall(window) = spec else {
        panic!("recall completion must resample the root model");
    };
    assert!(window.verbatim_tail.iter().any(|item| matches!(
        item,
        kittens_code_core::window::TailItem::ToolResult { text, .. }
            if text.as_str().contains("record with x")
    )));
    assert!(actions.iter().any(|action| matches!(
        action,
        CoreAction::Publish(Event::ToolTerminal {
            outcome: ToolOutcome::Succeeded,
            ..
        })
    )));

    // The page child was already terminal. Its duplicate is ledger-dropped:
    // one trace commit and no state-changing publish/effect action.
    let duplicate = engine
        .handle(CoreInput::EffectFinished {
            id: page_id,
            epoch: page_epoch,
            terminal: page_terminal,
        })
        .actions;
    assert_eq!(duplicate.len(), 1);
    assert!(matches!(duplicate[0], CoreAction::Commit(_)));
}

#[test]
fn recall_ask_runs_as_a_child_effect_and_returns_its_answer() {
    let mut engine = Engine::new(SessionConfig::default(), 1);
    let (model, epoch) = start_turn(&mut engine);
    let actions = propose_recall(
        &mut engine,
        model,
        epoch,
        "slice\nask %1 \"summarize\"\nfinal %2",
    );
    let (page_id, page_epoch, _) = started_effects(&actions).remove(0);
    let actions = engine
        .handle(CoreInput::EffectFinished {
            id: page_id,
            epoch: page_epoch,
            terminal: EffectTerminal::Pages(Page {
                records: vec![PageRecord {
                    seq: 1,
                    text: String::from("context"),
                }],
                next_cursor: None,
            }),
        })
        .actions;
    let (ask_id, ask_epoch, spec) = started_effects(&actions).remove(0);
    let EffectSpec::SubModel { requests } = spec else {
        panic!("ask must leave as a sub-model effect");
    };
    assert_eq!(requests.len(), 1);
    assert!(requests[0].context.contains("context"));

    let actions = engine
        .handle(CoreInput::EffectFinished {
            id: ask_id,
            epoch: ask_epoch,
            terminal: EffectTerminal::Ask(vec![AskResult {
                index: 0,
                answer: String::from("digest answer"),
                wall_clock_ms: 0,
                tokens: 0,
            }]),
        })
        .actions;
    let (_, _, spec) = started_effects(&actions).remove(0);
    let EffectSpec::ModelCall(window) = spec else {
        panic!("finished ask must resolve recall and resample");
    };
    assert!(window.verbatim_tail.iter().any(|item| matches!(
        item,
        kittens_code_core::window::TailItem::ToolResult { text, .. }
            if text.as_str() == "digest answer"
    )));
}

#[test]
fn recall_ask_each_waits_for_already_started_child_effects() {
    let mut engine = Engine::new(SessionConfig::default(), 1);
    let (model, epoch) = start_turn(&mut engine);
    let actions = propose_recall(
        &mut engine,
        model,
        epoch,
        "partition --by=bytes --size=1\nask-each %1 \"summarize\"\nfinal %2",
    );
    let (page_id, page_epoch, _) = started_effects(&actions).remove(0);
    let actions = engine
        .handle(CoreInput::EffectFinished {
            id: page_id,
            epoch: page_epoch,
            terminal: EffectTerminal::Pages(Page {
                records: vec![
                    PageRecord {
                        seq: 1,
                        text: String::from("a"),
                    },
                    PageRecord {
                        seq: 2,
                        text: String::from("b"),
                    },
                ],
                next_cursor: None,
            }),
        })
        .actions;
    let mut asks = started_effects(&actions);
    assert_eq!(asks.len(), 2);
    let (first_id, first_epoch, first_spec) = asks.remove(0);
    let (second_id, second_epoch, second_spec) = asks.remove(0);
    let EffectSpec::SubModel {
        requests: first_requests,
    } = first_spec
    else {
        panic!("first ask-each child must be a sub-model effect");
    };
    let EffectSpec::SubModel {
        requests: second_requests,
    } = second_spec
    else {
        panic!("second ask-each child must be a sub-model effect");
    };

    let actions = engine
        .handle(CoreInput::EffectFinished {
            id: second_id,
            epoch: second_epoch,
            terminal: EffectTerminal::Ask(vec![AskResult {
                index: second_requests[0].index,
                answer: String::from("B"),
                wall_clock_ms: 0,
                tokens: 0,
            }]),
        })
        .actions;
    assert!(
        started_effects(&actions).is_empty(),
        "AwaitingMore must not dispatch another ask"
    );

    let actions = engine
        .handle(CoreInput::EffectFinished {
            id: first_id,
            epoch: first_epoch,
            terminal: EffectTerminal::Ask(vec![AskResult {
                index: first_requests[0].index,
                answer: String::from("A"),
                wall_clock_ms: 0,
                tokens: 0,
            }]),
        })
        .actions;
    let (_, _, spec) = started_effects(&actions).remove(0);
    let EffectSpec::ModelCall(window) = spec else {
        panic!("the final ask child must resolve and resample");
    };
    assert!(window.verbatim_tail.iter().any(|item| matches!(
        item,
        kittens_code_core::window::TailItem::ToolResult { text, .. }
            if text.as_str() == "A\nB"
    )));
}

#[test]
fn interrupt_discards_recall_and_cancels_its_child_effect() {
    let mut engine = Engine::new(SessionConfig::default(), 1);
    let (model, epoch) = start_turn(&mut engine);
    let actions = propose_recall(&mut engine, model, epoch, "grep \"x\"\nfinal %1");
    let (page_id, page_epoch, _) = started_effects(&actions).remove(0);

    let actions = engine
        .handle(CoreInput::ClientOp(Submission {
            id: SubmissionId(2),
            op: Op::Interrupt,
        }))
        .actions;
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, CoreAction::CancelEffect { id } if *id == page_id))
    );

    let late = engine
        .handle(CoreInput::EffectFinished {
            id: page_id,
            epoch: page_epoch,
            terminal: EffectTerminal::Pages(Page::default()),
        })
        .actions;
    assert_eq!(late.len(), 1);
    assert!(matches!(late[0], CoreAction::Commit(_)));
}

#[test]
fn shutdown_discards_recall_and_cancels_its_child_effect() {
    let mut engine = Engine::new(SessionConfig::default(), 1);
    let (model, epoch) = start_turn(&mut engine);
    let actions = propose_recall(&mut engine, model, epoch, "grep \"x\"\nfinal %1");
    let (page_id, _, _) = started_effects(&actions).remove(0);

    let actions = engine
        .handle(CoreInput::ClientOp(Submission {
            id: SubmissionId(2),
            op: Op::Shutdown,
        }))
        .actions;
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, CoreAction::CancelEffect { id } if *id == page_id))
    );
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, CoreAction::Publish(Event::ShuttingDown)))
    );
}

#[test]
fn suspended_query_budget_rejects_excess_recall_as_a_tool_failure() {
    let mut config = SessionConfig::default();
    config.budgets.suspended_queries = 1;
    let mut engine = Engine::new(config, 1);
    let (model, epoch) = start_turn(&mut engine);
    let args = serde_json::json!({ "script": "grep \"x\"\nfinal %1" }).to_string();
    let actions = engine
        .handle(CoreInput::EffectFinished {
            id: model,
            epoch,
            terminal: EffectTerminal::Model(ModelOutcome {
                text: String::new(),
                tool_calls: vec![
                    ProposedToolCall {
                        name: String::from("recall"),
                        args_json: args.clone(),
                    },
                    ProposedToolCall {
                        name: String::from("recall"),
                        args_json: args,
                    },
                ],
                usage: None,
            }),
        })
        .actions;

    assert_eq!(
        started_effects(&actions)
            .iter()
            .filter(|(_, _, spec)| matches!(spec, EffectSpec::StoreReadPage { .. }))
            .count(),
        1
    );
    assert!(actions.iter().any(|action| matches!(
        action,
        CoreAction::Publish(Event::ToolTerminal {
            outcome: ToolOutcome::Failed { message },
            ..
        }) if message.contains("cause:budget")
    )));
}

#[test]
fn recall_lowering_error_resolves_as_a_failed_tool() {
    let mut engine = Engine::new(SessionConfig::default(), 1);
    let (model, epoch) = start_turn(&mut engine);
    let actions = propose_recall(&mut engine, model, epoch, "bogus\nfinal \"unused\"");

    assert!(
        started_effects(&actions)
            .iter()
            .all(|(_, _, spec)| matches!(spec, EffectSpec::ModelCall(_)))
    );
    assert!(actions.iter().any(|action| matches!(
        action,
        CoreAction::Publish(Event::ToolTerminal {
            outcome: ToolOutcome::Failed { message },
            ..
        }) if message == "verb_error{verb:bogus,cause:parse}"
    )));
}
