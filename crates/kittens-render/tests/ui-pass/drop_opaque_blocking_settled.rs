//! Negative control: safe code may accept and explicitly drop an opaque
//! blocking result without gaining a constructor or extracting its settlement.
//!
//! Handler interiors, including synchronous blocking work, remain ordinary
//! Rust as already pinned by
//! `crates/kittens/tests/ui-pass/constraint_erasure_boundaries.rs`; this
//! profile control deliberately does not invent a host-side target HAL bypass.

use kittens_render::blocking::BlockingSettled;

fn drop_without_extracting<T, P, E>(settled: BlockingSettled<T, P, E>) {
    drop(settled);
}

fn main() {}
