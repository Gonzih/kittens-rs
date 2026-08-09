#![forbid(unsafe_code)]
//! Tokio driver for the kittens-code core (SPEC v0.8 section 3, L-D1).
//!
//! This is the std host: it owns the `kittens::reactor!` loop, the log
//! appender, the model clients (deterministic jail plus, behind `live`, an
//! Anthropic-dialect endpoint), and tool discharge over the filesystem. The
//! core stays sans-io; everything here is the effect world.
//!
//! KC0 uses the owned-task + funnel topology (SPEC L-D1): owned tasks run
//! model calls and tools and push their terminals into ONE completion
//! channel; the driver's run loop pulls completions, feeds the core, and
//! discharges the returned actions. The full `kittens::reactor!` wiring
//! (interrupt prefix, delta funnel with drain + `yields_to`) lands with the
//! streaming model client; KC0's jail-driven loop exercises the same
//! CoreInput/CoreAction contract deterministically for eval E1.

pub mod appender;
#[cfg(feature = "live")]
mod live;
pub mod model;
pub mod runner;
pub mod tools;
