#![no_std]
#![forbid(unsafe_code)]
#![doc = include_str!("crate_docs.md")]

// Lets generated code use one absolute path both inside the library target and
// from examples/integration tests that belong to this package.
extern crate self as kittens;

#[cfg(all(feature = "tokio", not(target_os = "none")))]
extern crate alloc;

/// Control values returned by non-terminal reactor handlers.
pub mod reactor;
/// Persistent, selection-loss-preserving reactor sources.
pub mod source;

#[cfg(feature = "macros")]
pub use kittens_macros::reactor;

/// Implementation details used by generated code.
///
/// This module is public only because procedural-macro output crosses a crate
/// boundary. Its contents are not a supported API.
#[doc(hidden)]
pub mod __private {
    use crate::reactor::Control;
    use crate::source::{BacklogSource, DrainableSource, HasReadiness, ReactorSource, Readiness};

    #[cfg(all(feature = "tokio", not(target_os = "none")))]
    pub use tokio;

    #[cfg(feature = "macros")]
    pub use kittens_macros::{
        reactor_event, reactor_slots, reactor_tokio_event, reactor_tokio_slots,
    };

    /// Anchors source-admission failures in rustc output.
    #[allow(non_snake_case)]
    pub fn assert_SRC001_reactor_source_is_admitted__repair_use_retained_or_channel<
        S: ReactorSource,
    >(
        _: &S,
    ) {
    }

    /// Anchors exact readiness-marker failures in rustc output.
    #[allow(non_snake_case)]
    pub fn assert_KTR006_declared_readiness_matches<R, S>(_: &S)
    where
        R: Readiness,
        S: HasReadiness<R>,
    {
    }

    /// Anchors requests to drain a source without the capability.
    #[allow(non_snake_case)]
    pub fn assert_KTR009_source_is_drainable<S: DrainableSource>(_: &S) {}

    /// Anchors buffered-yield targets without a backlog probe.
    #[allow(non_snake_case)]
    pub fn assert_KTR010_yield_target_has_backlog_probe<S: BacklogSource>(_: &S) {}

    /// Gives guard type errors a stable generated helper name.
    #[allow(non_snake_case)]
    pub const fn assert_KTR019_guard_result_is_bool(value: bool) -> bool {
        value
    }

    /// Keeps continuing-handler result requirements local in diagnostics.
    #[allow(non_snake_case)]
    pub fn assert_KTR013_continuing_handler_result<T, E>(
        value: Result<Control<T>, E>,
    ) -> Result<Control<T>, E> {
        value
    }

    /// Keeps terminal-handler result requirements local in diagnostics.
    #[allow(non_snake_case)]
    pub fn assert_KTR013_terminal_handler_result<T, E>(value: Result<T, E>) -> Result<T, E> {
        value
    }

    /// Keeps phase result requirements local in diagnostics.
    #[allow(non_snake_case)]
    pub fn assert_KTR013_phase_result<E>(value: Result<(), E>) -> Result<(), E> {
        value
    }
}
