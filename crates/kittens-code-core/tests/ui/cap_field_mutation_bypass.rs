//! Gate G3 (SPEC P6): a legitimately constructed `Capped` cannot have its
//! text swapped for an oversized value afterward, because `text` is private.

use kittens_code_core::caps::{Capped, VerbOutput};

fn main() {
    let mut capped = Capped::<VerbOutput>::head("small", 8, None);
    // Reaching into the private field to enlarge the capped text.
    capped.text = String::from("now far larger than the applied limit allowed");
    let _ = capped;
}
