//! RLM continuation-executor tests (SPEC Q4/Q5): suspension, paged walks,
//! out-of-order ask-each rejoin, meter exhaustion with inline continuation,
//! and value-cap truncation.

use kittens_code_core::rlm::exec::{AskResult, Bound, Executor, Page, PageRecord, StepOutcome};
use kittens_code_core::rlm::ir::{Binding, BoundValue, FinalValue, Instr, Ref, Sel};
use kittens_code_protocol::budgets::Budgets;

fn rec(seq: u64, text: &str) -> PageRecord {
    PageRecord {
        seq,
        text: String::from(text),
    }
}

fn ask_result(index: u32, answer: &str) -> AskResult {
    AskResult {
        index,
        answer: String::from(answer),
        wall_clock_ms: 0,
        tokens: 0,
    }
}

fn instr_line(slot: u32, instr: Instr) -> Binding {
    Binding {
        slot,
        value: BoundValue::Instr(instr),
    }
}

#[test]
fn grep_walks_multiple_pages_then_binds_matches() {
    // Line 1: grep "fn" over the whole transcript. Line 2: final %1.
    let query = vec![
        instr_line(
            1,
            Instr::Grep {
                pattern: String::from("fn"),
                sel: Sel::Whole,
                ctx: 0,
                kind: None,
            },
        ),
        instr_line(
            2,
            Instr::Final {
                value: FinalValue::Ref(Ref::new(1)),
            },
        ),
    ];
    let mut exec = Executor::new(query, Budgets::default());

    // First step: the grep line requests its first page.
    let StepOutcome::NeedPages(req) = exec.step() else {
        panic!("expected a page request");
    };
    assert!(req.cursor.is_none());

    // Page 1 of 2: one match, one miss; cursor continues.
    let outcome = exec.provide_pages(Page {
        records: vec![rec(1, "fn a()"), rec(2, "let x")],
        next_cursor: Some(2),
    });
    assert!(matches!(outcome, StepOutcome::NeedPages(_)));

    // Page 2, final: one more match, no cursor.
    let outcome = exec.provide_pages(Page {
        records: vec![rec(3, "fn b()")],
        next_cursor: None,
    });
    let StepOutcome::Line { slot, bound } = outcome else {
        panic!("expected the grep line to bind");
    };
    assert_eq!(slot, 1);
    let Bound::Records(hits) = bound else {
        panic!("grep binds records");
    };
    assert_eq!(hits.len(), 2, "both fn lines matched across pages");

    // Final resolves to the rendered grep result.
    let done = exec.step();
    let StepOutcome::Done { answer } = done else {
        panic!("expected done");
    };
    assert!(answer.contains("fn a()") && answer.contains("fn b()"));
}

#[test]
fn ask_each_windows_parallel_subcalls_and_rejoins_out_of_order() {
    // Line 1: partition whole into byte chunks of 1 record.
    // Line 2: ask-each over %1. Line 3: final %2.
    let query = vec![
        instr_line(
            1,
            Instr::Partition {
                sel: Sel::Whole,
                by: kittens_code_core::rlm::ir::By::Bytes,
                size: Some(1),
                pattern: None,
            },
        ),
        instr_line(
            2,
            Instr::AskEach {
                chunks: Ref::new(1),
                question: String::from("summarize"),
            },
        ),
        instr_line(
            3,
            Instr::Final {
                value: FinalValue::Ref(Ref::new(2)),
            },
        ),
    ];
    let mut budgets = Budgets::default();
    budgets.parallel_subcalls = 2;
    let mut exec = Executor::new(query, budgets);

    // Partition: one page with five records, no cursor -> 5 chunks.
    assert!(matches!(exec.step(), StepOutcome::NeedPages(_)));
    let outcome = exec.provide_pages(Page {
        records: vec![
            rec(1, "a"),
            rec(2, "b"),
            rec(3, "c"),
            rec(4, "d"),
            rec(5, "e"),
        ],
        next_cursor: None,
    });
    assert!(matches!(outcome, StepOutcome::Line { slot: 1, .. }));

    // Only the initial width-two window is dispatched.
    let StepOutcome::NeedAsk(requests) = exec.step() else {
        panic!("expected ask batch");
    };
    assert_eq!(
        requests
            .iter()
            .map(|request| request.index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    let mut outstanding = vec![0, 1];
    assert!(outstanding.len() <= 2);

    // Complete index 1 before 0. Each completion releases one permit and
    // dispatches exactly one queued chunk, keeping width at two.
    let mid = exec.provide_ask(vec![ask_result(1, "B")]);
    outstanding.retain(|index| *index != 1);
    let StepOutcome::NeedAsk(requests) = mid else {
        panic!("completion should open the next window slot");
    };
    assert_eq!(requests[0].index, 2);
    outstanding.push(2);
    assert!(outstanding.len() <= 2);

    let mid = exec.provide_ask(vec![ask_result(0, "A")]);
    outstanding.retain(|index| *index != 0);
    let StepOutcome::NeedAsk(requests) = mid else {
        panic!("completion should dispatch partition 3");
    };
    assert_eq!(requests[0].index, 3);
    outstanding.push(3);
    assert!(outstanding.len() <= 2);

    let mid = exec.provide_ask(vec![ask_result(3, "D")]);
    outstanding.retain(|index| *index != 3);
    let StepOutcome::NeedAsk(requests) = mid else {
        panic!("completion should dispatch the final queued partition");
    };
    assert_eq!(requests[0].index, 4);
    outstanding.push(4);
    assert!(outstanding.len() <= 2);

    let mid = exec.provide_ask(vec![ask_result(4, "E")]);
    outstanding.retain(|index| *index != 4);
    assert!(matches!(mid, StepOutcome::AwaitingMore));
    assert!(outstanding.len() <= 2);

    // Partition 2 is deliberately last despite being dispatched earlier.
    let done = exec.provide_ask(vec![ask_result(2, "C")]);
    outstanding.retain(|index| *index != 2);
    assert!(outstanding.is_empty());
    let StepOutcome::Line { slot: 2, bound } = done else {
        panic!("ask-each should bind once all results arrive");
    };
    let Bound::DigestList(list) = bound else {
        panic!("ask-each binds a digest list");
    };
    // Rejoined in partition-index order despite out-of-order arrival.
    assert_eq!(list, vec!["A", "B", "C", "D", "E"]);
}

#[test]
fn scanned_bytes_exhaustion_binds_inline_error_and_continues() {
    // A tiny scanned-bytes budget; grep line errors, then final still runs.
    let mut budgets = Budgets::default();
    budgets.scanned_bytes = 3;
    let query = vec![
        instr_line(
            1,
            Instr::Grep {
                pattern: String::from("x"),
                sel: Sel::Whole,
                ctx: 0,
                kind: None,
            },
        ),
        instr_line(
            2,
            Instr::Final {
                value: FinalValue::Literal(String::from("recovered")),
            },
        ),
    ];
    let mut exec = Executor::new(query, budgets);
    assert!(matches!(exec.step(), StepOutcome::NeedPages(_)));
    // A page whose text exceeds the 3-byte scan budget.
    let outcome = exec.provide_pages(Page {
        records: vec![rec(1, "way too many bytes here")],
        next_cursor: None,
    });
    let StepOutcome::Line { slot: 1, bound } = outcome else {
        panic!("expected the grep line to bind an error");
    };
    assert!(
        matches!(
            bound,
            Bound::Error(kittens_code_protocol::error::VerbErrorCause::Budget)
        ),
        "scanned-bytes exhaustion binds an inline budget error"
    );
    // The script continues to `final`.
    let StepOutcome::Done { answer } = exec.step() else {
        panic!("query should continue past the inline error");
    };
    assert_eq!(answer, "recovered");
}

#[test]
fn ask_digest_is_capped() {
    let mut budgets = Budgets::default();
    budgets.ask_digest_bytes = 4;
    let query = vec![
        instr_line(
            1,
            Instr::Ask {
                sel: Sel::Whole,
                question: String::from("q"),
                sample_k: None,
            },
        ),
        instr_line(
            2,
            Instr::Final {
                value: FinalValue::Ref(Ref::new(1)),
            },
        ),
    ];
    let mut exec = Executor::new(query, budgets);
    let StepOutcome::NeedAsk(_) = exec.step() else {
        panic!("ask requests a sub-model call");
    };
    let StepOutcome::Line { bound, .. } = exec.provide_ask(vec![AskResult {
        index: 0,
        answer: String::from("this is far too long"),
        wall_clock_ms: 0,
        tokens: 0,
    }]) else {
        panic!("ask binds a digest");
    };
    let Bound::Digest(d) = bound else {
        panic!("ask binds a digest");
    };
    assert_eq!(d.len(), 4, "digest capped to the ask_digest_bytes budget");
}

#[test]
fn continuation_memory_exhaustion_binds_inline_error() {
    let mut budgets = Budgets::default();
    budgets.continuation_memory_bytes = 3;
    let query = vec![
        instr_line(1, Instr::Slice { sel: Sel::Whole }),
        instr_line(
            2,
            Instr::Final {
                value: FinalValue::Literal(String::from("continued")),
            },
        ),
    ];
    let mut exec = Executor::new(query, budgets);

    assert!(matches!(exec.step(), StepOutcome::NeedPages(_)));
    let StepOutcome::Line { slot: 1, bound } = exec.provide_pages(Page {
        records: vec![rec(1, "four")],
        next_cursor: None,
    }) else {
        panic!("the oversized retained record should bind an error");
    };
    assert!(matches!(
        bound,
        Bound::Error(kittens_code_protocol::error::VerbErrorCause::Budget)
    ));
    let StepOutcome::Done { answer } = exec.step() else {
        panic!("the script should continue after an inline memory error");
    };
    assert_eq!(answer, "continued");
}

#[test]
fn ask_wall_clock_exhaustion_binds_inline_error() {
    let mut budgets = Budgets::default();
    budgets.ask_wall_clock_ms = 9;
    let query = vec![
        instr_line(
            1,
            Instr::Ask {
                sel: Sel::Whole,
                question: String::from("q"),
                sample_k: None,
            },
        ),
        instr_line(
            2,
            Instr::Final {
                value: FinalValue::Literal(String::from("continued")),
            },
        ),
    ];
    let mut exec = Executor::new(query, budgets);

    assert!(matches!(exec.step(), StepOutcome::NeedAsk(_)));
    let StepOutcome::Line { bound, .. } = exec.provide_ask(vec![AskResult {
        index: 0,
        answer: String::from("answer"),
        wall_clock_ms: 10,
        tokens: 0,
    }]) else {
        panic!("wall-clock exhaustion should bind an inline error");
    };
    assert!(matches!(
        bound,
        Bound::Error(kittens_code_protocol::error::VerbErrorCause::Budget)
    ));
}

#[test]
fn ask_token_exhaustion_binds_inline_error() {
    let mut budgets = Budgets::default();
    budgets.ask_tokens = 4;
    let query = vec![instr_line(
        1,
        Instr::Ask {
            sel: Sel::Whole,
            question: String::from("q"),
            sample_k: None,
        },
    )];
    let mut exec = Executor::new(query, budgets);

    assert!(matches!(exec.step(), StepOutcome::NeedAsk(_)));
    let StepOutcome::Line { bound, .. } = exec.provide_ask(vec![AskResult {
        index: 0,
        answer: String::from("answer"),
        wall_clock_ms: 0,
        tokens: 5,
    }]) else {
        panic!("token exhaustion should bind an inline error");
    };
    assert!(matches!(
        bound,
        Bound::Error(kittens_code_protocol::error::VerbErrorCause::Budget)
    ));
}

#[test]
fn recursion_depth_guard_rejects_the_current_query_depth() {
    let mut budgets = Budgets::default();
    budgets.recursion_depth = 0;
    let query = vec![instr_line(
        1,
        Instr::Final {
            value: FinalValue::Literal(String::from("unreachable")),
        },
    )];
    let mut exec = Executor::new(query, budgets);

    assert!(matches!(
        exec.step(),
        StepOutcome::Failed {
            cause: kittens_code_protocol::error::VerbErrorCause::Budget
        }
    ));
}

#[test]
fn prelowered_error_line_surfaces_and_script_continues() {
    // The lowerer flagged line 1 as an inline parse error; the executor
    // still surfaces it and runs the final line (Q9).
    let query = vec![
        Binding {
            slot: 1,
            value: BoundValue::Error(kittens_code_core::rlm::ir::VerbError {
                verb: String::from("bogus"),
                cause: kittens_code_protocol::error::VerbErrorCause::Parse,
            }),
        },
        instr_line(
            2,
            Instr::Final {
                value: FinalValue::Literal(String::from("ok")),
            },
        ),
    ];
    let mut exec = Executor::new(query, Budgets::default());
    let StepOutcome::Line { slot: 1, bound } = exec.step() else {
        panic!("the pre-lowered error surfaces as a line");
    };
    assert!(matches!(
        bound,
        Bound::Error(kittens_code_protocol::error::VerbErrorCause::Parse)
    ));
    let StepOutcome::Done { answer } = exec.step() else {
        panic!("script continues to final");
    };
    assert_eq!(answer, "ok");
}

#[test]
fn byte_partition_fills_to_budget_not_record_count() {
    // Four 3-byte records, byte budget 7: greedily fills each chunk until
    // the next record would exceed 7 bytes -> chunks of [aaa,bbb],[ccc,ddd].
    let query = vec![
        instr_line(
            1,
            Instr::Partition {
                sel: Sel::Whole,
                by: kittens_code_core::rlm::ir::By::Bytes,
                size: Some(7),
                pattern: None,
            },
        ),
        instr_line(
            2,
            Instr::Final {
                value: FinalValue::Ref(Ref::new(1)),
            },
        ),
    ];
    let mut exec = Executor::new(query, Budgets::default());
    assert!(matches!(exec.step(), StepOutcome::NeedPages(_)));
    let StepOutcome::Line { bound, .. } = exec.provide_pages(Page {
        records: vec![rec(1, "aaa"), rec(2, "bbb"), rec(3, "ccc"), rec(4, "ddd")],
        next_cursor: None,
    }) else {
        panic!("partition binds chunks");
    };
    let Bound::Chunks(chunks) = bound else {
        panic!("partition binds a chunk list");
    };
    // Budget-fill: two chunks of two records each, not four of one.
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].len(), 2);
    assert_eq!(chunks[1].len(), 2);
}

#[test]
fn oversized_record_gets_its_own_byte_chunk() {
    // A single record larger than the budget still forms one chunk (never
    // dropped).
    let query = vec![
        instr_line(
            1,
            Instr::Partition {
                sel: Sel::Whole,
                by: kittens_code_core::rlm::ir::By::Bytes,
                size: Some(2),
                pattern: None,
            },
        ),
        instr_line(
            2,
            Instr::Final {
                value: FinalValue::Literal(String::from("x")),
            },
        ),
    ];
    let mut exec = Executor::new(query, Budgets::default());
    assert!(matches!(exec.step(), StepOutcome::NeedPages(_)));
    let StepOutcome::Line { bound, .. } = exec.provide_pages(Page {
        records: vec![rec(1, "way bigger than two bytes"), rec(2, "ok")],
        next_cursor: None,
    }) else {
        panic!("partition binds chunks");
    };
    let Bound::Chunks(chunks) = bound else {
        panic!("chunks");
    };
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].len(), 1, "oversized record is its own chunk");
}
