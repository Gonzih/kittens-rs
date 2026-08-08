#![no_std]
#![forbid(unsafe_code)]
//! Wire contract for the kittens-code harness family.
//!
//! This crate is the only artifact frontends and external clients link
//! (kittens-code SPEC v0.8, rule T3). It contains pure serde data: client
//! submissions ([`op::Op`]), harness events ([`event::Event`]), the error
//! taxonomy ([`error::ErrorEvent`]), approval/sandbox policies as data
//! ([`policy`]), the patchable session configuration ([`config`]), budget
//! numbers ([`budgets`]), and plain identity types ([`ids`]).
//!
//! Deliberate absences (SPEC section 4): no engine types, no `WindowLayout`
//! or cap-types (those are `kittens-code-core` law), no bootstrap
//! configuration (endpoints/auth/TLS/store paths are driver-only and never
//! logged), and no `uuid`/`semver`/checksum dependencies — identities,
//! versions, and digests are plain arrays and integers (rule P9).
//!
//! Wire evolution is additive-only within v0.x; every enum is
//! `#[non_exhaustive]` and decoders must tolerate unknown fields. Persisted
//! compatibility is governed by the log header's `schema_epoch`, not crate
//! semver (rule P7).

extern crate alloc;

pub mod budgets;
pub mod config;
pub mod error;
pub mod event;
pub mod ids;
pub mod op;
pub mod policy;
