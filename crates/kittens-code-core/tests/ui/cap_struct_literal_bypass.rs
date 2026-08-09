//! Gate G3 (SPEC P6): the only way to obtain a `Capped` is through its
//! truncating constructors. A struct literal that bypasses truncation must
//! not compile, because every field is private.

use kittens_code_core::caps::{Capped, VerbOutput};

fn main() {
    // Attempting to fabricate an untruncated Capped by naming its fields.
    let _forged: Capped<VerbOutput> = Capped {
        text: String::from("this bypasses the truncating constructor"),
        applied_limit: 8,
        truncation: None,
    };
}
