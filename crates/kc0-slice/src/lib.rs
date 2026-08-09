#![no_std]
#![forbid(unsafe_code)]
//! Isolated KC0 evidence for transcript records and RLM text lowering.
//!
//! The crate contains no I/O. Drivers remain responsible for record framing,
//! durable appends, and execution of lowered RLM instructions.

extern crate alloc;

/// Codec-independent transcript record model and crash-replay scan.
pub mod record;
/// Typed RLM instruction representation and text lowering.
pub mod rlm;
