//! Embedded rendering/interaction profile for the Kittens reactor kernel.
//!
//! **Stage: K2R-0 host slice** (`SPEC.md` revision 7: section 6 is the
//! normative host surface; the K2R-0A experiment that selected it is
//! recorded in `K2R0A-LOG.md`, and `TRACE-MANIFEST.md` maps every required
//! oracle to its status). Not yet frozen: `FlightStarter`/`OwnedTransfer`
//! sealing, the kernel-admitted source carrier, the Xtensa probe, and board
//! HIL are named open gates.
//!
//! The crate core is `#![no_std]`, no-alloc, `#![forbid(unsafe_code)]`. Host
//! tests model the HAL boundary; the exact-HAL target compile probe is a
//! separately gated step (Xtensa toolchain). The optional, default-off
//! `embedded-graphics` feature adds a no-alloc RGB565 stripe draw target while
//! preserving `no_std`; feature-off core has no normal dependencies.

#![no_std]
#![forbid(unsafe_code)]

pub mod demand;
#[cfg(feature = "embedded-graphics")]
pub mod draw_target;
pub mod geometry;
pub mod sweep;
pub mod touch;
pub mod transfer;
