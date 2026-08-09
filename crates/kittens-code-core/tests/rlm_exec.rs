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
fn ask_each_rejoins_out_of_order_results() {
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
    let mut exec = Executor::new(query, Budgets::default());

    // Partition: one page with three records, no cursor -> 3 chunks.
    assert!(matches!(exec.step(), StepOutcome::NeedPages(_)));
    let outcome = exec.provide_pages(Page {
        records: vec![rec(1, "a"), rec(2, "b"), rec(3, "c")],
        next_cursor: None,
    });
    assert!(matches!(outcome, StepOutcome::Line { slot: 1, .. }));

    // ask-each: three requests.
    let StepOutcome::NeedAsk(requests) = exec.step() else {
        panic!("expected ask batch");
    };
    assert_eq!(requests.len(), 3);

    // Provide results out of order (index 2, then 0, then 1).
    let mid = exec.provide_ask(vec![AskResult {
        index: 2,
        answer: String::from("C"),
    }]);
    assert!(matches!(mid, StepOutcome::AwaitingMore));
    let mid = exec.provide_ask(vec![AskResult {
        index: 0,
        answer: String::from("A"),
    }]);
    assert!(matches!(mid, StepOutcome::AwaitingMore));
    let done = exec.provide_ask(vec![AskResult {
        index: 1,
        answer: String::from("B"),
    }]);
    let StepOutcome::Line { slot: 2, bound } = done else {
        panic!("ask-each should bind once all results arrive");
    };
    let Bound::DigestList(list) = bound else {
        panic!("ask-each binds a digest list");
    };
    // Rejoined in partition-index order despite out-of-order arrival.
    assert_eq!(list, vec!["A", "B", "C"]);
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
    }]) else {
        panic!("ask binds a digest");
    };
    let Bound::Digest(d) = bound else {
        panic!("ask binds a digest");
    };
    assert_eq!(d.len(), 4, "digest capped to the ask_digest_bytes budget");
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
