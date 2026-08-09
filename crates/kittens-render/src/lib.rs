//! Embedded rendering/interaction profile for the Kittens reactor kernel.
//!
//! **Stage: K2R-0A feasibility experiment** (`SPEC.md` section 7). Nothing in
//! this crate is a frozen public API: the spec's section 6 is a provisional
//! candidate surface, and this crate currently contains the candidate probes
//! that decide it. `K2R0A-LOG.md` is the experiment record.
//!
//! The crate core is `#![no_std]`, no-alloc, `#![forbid(unsafe_code)]`. Host
//! tests model the HAL boundary; the exact-HAL target compile probe is a
//! separately gated step (Xtensa toolchain).

#![no_std]
#![forbid(unsafe_code)]

pub mod demand;
pub mod geometry;
pub mod sweep;
pub mod touch;
pub mod transfer;
