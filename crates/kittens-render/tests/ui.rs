#![allow(missing_docs)]

#[test]
fn proof_boundaries_and_negative_controls() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
    tests.pass("tests/ui-pass/*.rs");
}
