#![no_std]

/// Forces the `kittens/tokio` feature to participate in workspace feature
/// unification without using a host-only adapter.
pub const TOKIO_FEATURE_ENABLED: bool = true;
