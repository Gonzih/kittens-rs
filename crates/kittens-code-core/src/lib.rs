#![no_std]
#![forbid(unsafe_code)]
//! Sans-io coding-agent engine (kittens-code SPEC v0.8, sections 6–9).
//!
//! This crate is a synchronous state machine: drivers feed it `CoreInput`s
//! and discharge the bounded `CoreAction` batches it returns. It owns turn
//! and session state, the context engine, the RLM query engine, and budget
//! law. It has no IO, no clock, no entropy, no async runtime, and no
//! dependency on the kittens kernel — the reactor loop lives in drivers
//! (rule T2; the driver law is SPEC L-D1).
//!
//! Module map (SPEC section 3): [`caps`] sealed budget cap-types, [`window`]
//! the typed post-compaction layout, [`compact`] compaction decisions,
//! [`tokens`] deterministic token accounting, [`prompts`] the versioned
//! prompt pack. The record model, RLM IR/lowering (blind-slice modules), the
//! turn engine, and the tool modules land in subsequent commits of the KC0
//! slice.

extern crate alloc;

pub mod caps;
pub mod compact;
/// Turn engine: CoreInput/CoreAction state machine (SPEC section 6).
pub mod engine;
pub mod prompts;
/// Codec-independent transcript record model and crash-replay scan
/// (blind-slice module; provenance input 18).
pub mod record;
/// Typed RLM instruction representation and text lowering (blind-slice
/// module; provenance input 18).
pub mod rlm;
pub mod tokens;
pub mod window;
