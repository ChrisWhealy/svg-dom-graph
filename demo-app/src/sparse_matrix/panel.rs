//! What the panel holds between events: the shell, the live scene and the run in progress.

use super::shell::Shell;
use super::{frame_callback::FrameCallback, live::Live};

#[derive(Default)]
pub(super) struct Panel {
    pub(super) shell: Option<Shell>,
    pub(super) live: Option<Live>,
    /// Counted up by every build and teardown. A frame that was asked for under an older number does nothing.
    pub(super) generation: u32,
    pub(super) running: bool,
    /// The frame that is asked for, and the callback that it will call. The panel owns the callback, so that a frame
    /// that is cancelled and never called does not leave it behind: cancelling drops it.
    pub(super) frame: Option<i32>,
    pub(super) callback: Option<FrameCallback>,
    /// The callback that is running, or has run. A callback cannot be dropped while it runs, so it is dropped when the
    /// next one starts or when the run is stopped from outside a frame.
    pub(super) finished: Option<FrameCallback>,
    pub(super) in_frame: bool,
    /// Counted up for each `<svg>` made, so that no two have the same id.
    pub(super) svgs: u32,
    /// When a run next takes a step, which the Run speed list sets, so that a step can be followed.
    pub(super) next_step_at: f64,
}
