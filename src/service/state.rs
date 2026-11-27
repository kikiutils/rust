#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    /// Also kept after a start was dropped midway, or a stop was dropped or timed out after it began:
    /// starting is rejected until a stop finishes.
    Stopping,
}
