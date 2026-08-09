//! Embedded rendering/interaction profile for the Kittens reactor kernel.
//!
//! **Stage: experimental 0.1.x evidence release of the K2R-0 host slice**
//! (`SPEC.md` revision 8: section 6 is the normative host surface; the K2R-0A
//! experiment that selected it is recorded in `K2R0A-LOG.md`, and
//! `TRACE-MANIFEST.md` maps every required oracle to its status). Publication
//! is not a protocol freeze. `FlightStarter`/`OwnedTransfer` sealing, the
//! kernel-admitted source and real `reactor!` fixture, bilateral seam co-sign,
//! blocking `write_region`, and board HIL/silicon delivery are named open
//! gates.
//!
//! The crate core is `#![no_std]`, no-alloc, `#![forbid(unsafe_code)]`. Host
//! tests model the HAL boundary. The pinned exact-HAL Xtensa compile/link probe
//! is closed with scope: it proves feasibility, not behavior on silicon. The
//! optional, default-off `embedded-graphics` feature adds a no-alloc RGB565
//! stripe draw target while preserving `no_std`; feature-off core has no normal
//! dependencies.

#![no_std]
#![forbid(unsafe_code)]

pub mod demand;
#[cfg(feature = "embedded-graphics")]
pub mod draw_target;
pub mod geometry;
pub mod sweep;
pub mod touch;
pub mod transfer;
