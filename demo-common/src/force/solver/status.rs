//! What the system is doing, and what one step of it did.

/// What the system is doing, as the panel reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// At rest, and asking for no frames.
    Sleeping,
    /// Free particles are moving after a disturbance.
    Running,
    /// A pointer holds a vertex, or its proxy is still catching up after the release.
    Dragging,
    /// Stopped by the user or because the page is hidden.
    Paused,
    /// The watchdog ran out without the system coming to rest. Never to be reported as convergence.
    Unsettled,
}

impl Status {
    /// The word the panel writes for this state.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Sleeping => "sleeping",
            Status::Running => "running",
            Status::Dragging => "dragging",
            Status::Paused => "paused",
            Status::Unsettled => "unsettled",
        }
    }
}

/// What one call to [`SpringSystem::step`](super::SpringSystem::step) did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepOutcome {
    /// Whether the system took a step. It does not when it is asleep, paused, unsettled or faulted.
    pub stepped: bool,
    /// How many quiet steps in a row there have now been.
    pub quiet_steps: u32,
    /// Whether this step put the system to sleep.
    pub slept: bool,
    /// Whether this step ran the watchdog out.
    pub unsettled: bool,
    /// Whether the step produced a number that is not finite. The state was put back as it was.
    pub faulted: bool,
}
