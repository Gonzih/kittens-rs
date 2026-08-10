//! Embedded rendering/interaction profile for the Kittens reactor kernel.
//!
//! **Stage: K2R-0A closed with host + portable-link + exact-Xtensa-link
//! scope; K2R-0 and K2R-1 remain gated** (`SPEC.md` revision 12: section 6 is
//! the normative host surface, `K2R0A-LOG.md` records the selection history,
//! and `TRACE-MANIFEST.md` maps every required oracle to its status). The
//! kernel-admitted inline completion source and real `reactor!` integration
//! are closed with host + portable-link scope. The sealed blocking-region path
//! is separately closed with host + exact-Xtensa-link scope. The branded
//! single-payload async-region row is separately closed with host + exact-
//! Xtensa-reactor-link scope. Bilateral seam co-sign and sealing
//! `FlightStarter`/`OwnedTransfer` gate the K2R-0 breaking freeze. A real target
//! executor, board coordinator, target-runtime observations, and board
//! HIL/silicon delivery belong to K2R-1.
//!
//! The local normalized packaged-source + registry-HAL Xtensa consumer row is
//! selected but remains open until its clean-package matrix passes. The
//! repository's exact-git fixture is not a control for that source-identity
//! boundary. Upload, index availability, and exact-version download for a
//! future correctly versioned release remain human-ordered publication work;
//! none is implied by K2R-0A closure.
//!
//! The crate core is `#![no_std]`, no-alloc, `#![forbid(unsafe_code)]`. Host
//! tests model the HAL boundary. The pinned exact-HAL Xtensa compile/link probe
//! is closed with scope: it proves feasibility, not behavior on silicon. The
//! optional, default-off `embedded-graphics` feature adds a no-alloc RGB565
//! stripe draw target while preserving `no_std`; the default-off target-only
//! `esp32s3-sh8601-blocking` feature adds the exact-HAL blocking adapter; the
//! target-only `esp32s3-sh8601-async` feature adds the branded interrupt-
//! backed adapter. Feature-off core has no normal dependencies.

#![no_std]
#![forbid(unsafe_code)]

pub(crate) mod async_region;
pub mod blocking;
pub mod demand;
#[cfg(feature = "embedded-graphics")]
pub mod draw_target;
#[cfg(all(feature = "esp32s3-sh8601-blocking", target_arch = "xtensa"))]
pub mod esp32s3_sh8601;
#[cfg(all(feature = "esp32s3-sh8601-async", target_arch = "xtensa"))]
pub mod esp32s3_sh8601_async;
pub mod geometry;
pub mod sweep;
pub mod touch;
pub mod transfer;
