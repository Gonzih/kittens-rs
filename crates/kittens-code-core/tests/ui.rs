//! Compile-fail gate for the P6 cap-type law (SPEC gate G3): code that
//! bypasses `Capped`'s truncating constructors must not compile.

#[test]
fn cap_bypass_is_a_compile_error() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/cap_struct_literal_bypass.rs");
    t.compile_fail("tests/ui/cap_field_mutation_bypass.rs");
}
