/// Directs a non-terminal reactor handler to continue or stop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Control<T> {
    /// Begin the next reactor arbitration after the optional `after_event`
    /// phase completes.
    Continue,
    /// Stop the reactor with the contained exit value. `after_event` is not
    /// run for this service window.
    Stop(T),
}
