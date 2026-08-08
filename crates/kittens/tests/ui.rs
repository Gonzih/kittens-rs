#![allow(missing_docs)]

#[test]
fn reactor_mutations_and_negative_controls() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
    tests.pass("tests/ui-pass/*.rs");
}
