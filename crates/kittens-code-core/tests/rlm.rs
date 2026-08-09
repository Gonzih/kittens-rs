//! G7d golden, rejection, and binding-law coverage for RLM text lowering.

use kittens_code_core::rlm::{
    Binding, BoundValue, By, FinalValue, Instr, Out, Query, RangeUnit, Sel, lower_script,
    lower_script_with_verb_limit,
};
use kittens_code_protocol::error::VerbErrorCause;

fn instruction(value: &BoundValue) -> &Instr {
    match value {
        BoundValue::Instr(instruction) => instruction,
        BoundValue::Error(error) => panic!("unexpected inline error: {error:?}"),
    }
}

fn cause(value: &BoundValue) -> VerbErrorCause {
    match value {
        BoundValue::Instr(instruction) => panic!("unexpected instruction: {instruction:?}"),
        BoundValue::Error(error) => error.cause,
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn golden_script_covers_every_verb_and_declared_output() {
    let query = lower_script(
        "grep \"panic\" --ctx=2 --kind=model_delta\n\
         head 4 %1\n\
         slice %2\n\
         tail 2 %3\n\
         count \"panic\" %4\n\
         partition %4 --by=regex \"fn \"\n\
         ask %4 \"summarize\" --sample-k=2\n\
         ask-each %6 \"summarize each chunk\"\n\
         final %8\n",
    );

    assert_eq!(query.len(), 9);
    assert_eq!(
        query.iter().filter_map(Binding::output).collect::<Vec<_>>(),
        vec![
            Out::Records,
            Out::Records,
            Out::Records,
            Out::Records,
            Out::Count,
            Out::Chunks,
            Out::Digest,
            Out::DigestList,
            Out::Answer,
        ]
    );

    match instruction(&query[0].value) {
        Instr::Grep {
            pattern,
            sel: Sel::Whole,
            ctx,
            kind: Some(kind),
        } => {
            assert_eq!(pattern, "panic");
            assert_eq!(*ctx, 2);
            assert_eq!(kind.as_str(), "model_delta");
        }
        other => panic!("wrong grep lowering: {other:?}"),
    }
    match instruction(&query[1].value) {
        Instr::Head {
            sel: Sel::Ref(reference),
            n,
        } => {
            assert_eq!(reference.line(), 1);
            assert_eq!(*n, 4);
        }
        other => panic!("wrong head lowering: {other:?}"),
    }
    match instruction(&query[2].value) {
        Instr::Slice {
            sel: Sel::Ref(reference),
        } => assert_eq!(reference.line(), 2),
        other => panic!("wrong slice lowering: {other:?}"),
    }
    match instruction(&query[3].value) {
        Instr::Tail {
            sel: Sel::Ref(reference),
            n,
        } => {
            assert_eq!(reference.line(), 3);
            assert_eq!(*n, 2);
        }
        other => panic!("wrong tail lowering: {other:?}"),
    }
    match instruction(&query[4].value) {
        Instr::Count {
            pattern: Some(pattern),
            sel: Sel::Ref(reference),
        } => {
            assert_eq!(pattern, "panic");
            assert_eq!(reference.line(), 4);
        }
        other => panic!("wrong count lowering: {other:?}"),
    }
    match instruction(&query[5].value) {
        Instr::Partition {
            sel: Sel::Ref(reference),
            by: By::Regex,
            size: None,
            pattern: Some(pattern),
        } => {
            assert_eq!(reference.line(), 4);
            assert_eq!(pattern, "fn ");
        }
        other => panic!("wrong partition lowering: {other:?}"),
    }
    assert!(matches!(
        instruction(&query[6].value),
        Instr::Ask {
            sel: Sel::Ref(_),
            question,
            sample_k: Some(2),
        } if question == "summarize"
    ));
    assert!(matches!(
        instruction(&query[7].value),
        Instr::AskEach { chunks, question }
            if chunks.line() == 6 && question == "summarize each chunk"
    ));
    assert!(matches!(
        instruction(&query[8].value),
        Instr::Final {
            value: FinalValue::Ref(reference),
        } if reference.line() == 8
    ));
}

#[test]
fn golden_ranges_partitions_and_string_escapes() {
    let query = lower_script(
        "slice seq:4..9\n\
         partition %1 --by=turns --size=2\n\
         partition %1 --by=bytes --size=1024\n\
         partition --by=regex \"fn \"\n\
         grep \"a\\\"b\\\\c\"\n\
         final \"done\"\n",
    );

    assert!(query.iter().all(|binding| binding.output().is_some()));
    assert!(matches!(
        instruction(&query[0].value),
        Instr::Slice {
            sel: Sel::Range(range),
        } if range.unit == RangeUnit::Seq && range.start == 4 && range.end == 9
    ));
    assert!(matches!(
        instruction(&query[1].value),
        Instr::Partition {
            by: By::Turns,
            size: Some(2),
            pattern: None,
            ..
        }
    ));
    assert!(matches!(
        instruction(&query[2].value),
        Instr::Partition {
            by: By::Bytes,
            size: Some(1024),
            pattern: None,
            ..
        }
    ));
    assert!(matches!(
        instruction(&query[3].value),
        Instr::Partition {
            sel: Sel::Whole,
            by: By::Regex,
            pattern: Some(pattern),
            ..
        } if pattern == "fn "
    ));
    assert!(matches!(
        instruction(&query[4].value),
        Instr::Grep { pattern, .. } if pattern == "a\"b\\c"
    ));
}

#[test]
fn each_protocol_verb_error_cause_is_reachable() {
    assert_eq!(
        cause(&lower_script("slice %1\n")[0].value),
        VerbErrorCause::BadRef
    );
    assert_eq!(
        cause(&lower_script("slice byte:9..2\n")[0].value),
        VerbErrorCause::BadRange
    );
    assert_eq!(
        cause(&lower_script("grep \"x\" --ctx=1 --ctx=2\n")[0].value),
        VerbErrorCause::BadFlag
    );
    assert_eq!(
        cause(&lower_script("unknown\n")[0].value),
        VerbErrorCause::Parse
    );
    assert_eq!(
        cause(&lower_script_with_verb_limit("slice\n", 0)[0].value),
        VerbErrorCause::Budget
    );
}

#[test]
fn rejection_suite_covers_types_flags_arity_ranges_and_strings() {
    let cases = [
        ("slice %2\nslice\n", 0, VerbErrorCause::BadRef),
        ("count\nslice %1\n", 1, VerbErrorCause::BadRef),
        ("slice\nask-each %1 \"q\"\n", 1, VerbErrorCause::BadRef),
        ("slice %0\n", 0, VerbErrorCause::BadRef),
        ("slice row:1..2\n", 0, VerbErrorCause::BadRange),
        (
            "slice seq:18446744073709551616..2\n",
            0,
            VerbErrorCause::BadRange,
        ),
        ("slice --wat=1\n", 0, VerbErrorCause::BadFlag),
        ("partition --by=turns\n", 0, VerbErrorCause::BadFlag),
        (
            "partition --by=regex --size=1 \"x\"\n",
            0,
            VerbErrorCause::BadFlag,
        ),
        (
            "partition --by=bytes --size=4294967296\n",
            0,
            VerbErrorCause::BadFlag,
        ),
        ("grep \"x\" --ctx\n", 0, VerbErrorCause::BadFlag),
        ("grep \"x\" --kind other\n", 0, VerbErrorCause::BadFlag),
        (
            "grep \"x\" --ctx=999999999999999999999\n",
            0,
            VerbErrorCause::BadFlag,
        ),
        ("grep\n", 0, VerbErrorCause::Parse),
        ("final answer\n", 0, VerbErrorCause::Parse),
        ("grep \"bad\\n\"\n", 0, VerbErrorCause::Parse),
        ("grep \"(?i)bad\"\n", 0, VerbErrorCause::Parse),
    ];

    for (script, error_index, expected) in cases {
        let query = lower_script(script);
        assert_eq!(query.len(), script.lines().count(), "script: {script:?}");
        assert_eq!(
            cause(&query[error_index].value),
            expected,
            "script: {script:?}"
        );
    }
}

#[test]
fn syntax_errors_are_not_masked_by_the_verb_budget() {
    let query = lower_script_with_verb_limit("unknown\n", 0);
    assert_eq!(cause(&query[0].value), VerbErrorCause::Parse);
}

#[test]
fn escaped_literal_parenthesis_is_not_an_inline_regex_flag() {
    let query = lower_script("grep \"\\\\(?i\\\\)\"\n");
    assert!(matches!(
        instruction(&query[0].value),
        Instr::Grep { pattern, .. } if pattern == r"\(?i\)"
    ));
}

#[test]
fn typed_query_round_trips_through_serde() {
    let query = lower_script(
        "slice\npartition %1 --by=regex \"fn \"\nask-each %2 \"summarize\"\nfinal %3\n",
    );
    let encoded = serde_json::to_string(&query).expect("query serializes");
    let decoded: Query = serde_json::from_str(&encoded).expect("query deserializes");
    assert_eq!(decoded, query);
}

#[test]
fn raw_newline_in_string_is_illegal_and_recovery_continues() {
    let query = lower_script("ask \"line one\nline two\"\nslice\n");
    assert_eq!(query.len(), 3);
    assert_eq!(cause(&query[0].value), VerbErrorCause::Parse);
    assert_eq!(cause(&query[1].value), VerbErrorCause::Parse);
    assert!(matches!(instruction(&query[2].value), Instr::Slice { .. }));
}

#[test]
fn inline_errors_bind_slots_and_do_not_stop_lowering() {
    let query = lower_script("slice %2\nslice\nslice %2\nfinal %3\n");
    assert_eq!(query.len(), 4);
    assert_eq!(cause(&query[0].value), VerbErrorCause::BadRef);
    assert_eq!(query[0].slot, 1);
    assert!(matches!(instruction(&query[1].value), Instr::Slice { .. }));
    assert!(matches!(instruction(&query[2].value), Instr::Slice { .. }));
    assert!(matches!(instruction(&query[3].value), Instr::Final { .. }));
    assert_eq!(
        query.iter().map(|binding| binding.slot).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
}

#[test]
fn every_accepted_nonempty_line_has_one_instruction_and_slot() {
    let scripts = [
        "slice\n",
        "slice turn:0..0\ncount %1\nfinal %2\n",
        "slice\npartition %1 --by=turns --size=1\nask-each %2 \"q\"\nfinal %3\n",
        "grep \"needle\"\nhead 1 %1\ntail 1 %2\nask %3 \"q\"\nfinal %4\n",
    ];

    for script in scripts {
        let query = lower_script(script);
        let nonempty_lines = script
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
        assert_eq!(query.len(), nonempty_lines);
        for (index, binding) in query.iter().enumerate() {
            assert_eq!(
                usize::try_from(binding.slot).expect("small test slot"),
                index + 1
            );
            assert!(
                matches!(binding.value, BoundValue::Instr(_)),
                "script: {script:?}"
            );
        }
    }
}
