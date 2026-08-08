#![allow(missing_docs)]

#[test]
fn draw_permit_is_exclusive() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
