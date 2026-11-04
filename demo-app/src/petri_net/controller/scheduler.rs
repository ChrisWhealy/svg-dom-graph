//! What runs a delayed task or a frame: the trait the controller schedules with, and a clock that a test advances by
//! hand.

use std::cell::{Cell, RefCell};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs a task after a delay or at the next frame, and can take it back. The page uses the browser's, and a test uses
/// [`ManualScheduler`], which runs a task only when it is told to.
pub(crate) trait Scheduler {
    /// Runs `task` after `delay_ms` milliseconds, and gives a handle that [`Scheduler::cancel`] takes.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the task could not be scheduled, in which case it will never run.
    fn schedule_once(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> Result<u64, String>;

    /// Takes back the task with `handle`, if it has not run.
    fn cancel(&self, handle: u64);

    /// Runs `task` at the next animation frame.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the frame could not be asked for, in which case the task will never run.
    fn request_frame(&self, task: Box<dyn FnOnce()>) -> Result<u64, String>;

    /// Takes back the frame with `handle`, if it has not come.
    fn cancel_frame(&self, handle: u64);

    /// Gives up everything that the scheduler holds in the browser: the tasks that are waiting, and any hook that it
    /// put on the page. Nothing is scheduled after this.
    fn teardown(&self) {}
}

/// A task that is waiting: its handle, its delay, and what it runs.
type Waiting = (u64, u32, Box<dyn FnOnce()>);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scheduler that runs a task only when [`ManualScheduler::advance`] or [`ManualScheduler::advance_frame`] is called,
/// and so needs no clock.
#[derive(Default)]
pub(crate) struct ManualScheduler {
    next: Cell<u64>,
    timers: RefCell<Vec<Waiting>>,
    frames: RefCell<Vec<Waiting>>,
    /// Whether a cancelled task stays, as a callback that was already on its way to the browser's queue would.
    keep_cancelled: Cell<bool>,
    timer_failures: Cell<u32>,
    frame_failures: Cell<u32>,
    #[cfg(test)]
    teardowns: Cell<u32>,
}

impl ManualScheduler {
    /// How many timers are waiting, and how many frames.
    pub(crate) fn pending(&self) -> usize {
        self.timers.borrow().len()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) fn pending_frames(&self) -> usize {
        self.frames.borrow().len()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The delay of the timer that is waiting, if there is one.
    #[cfg(test)]
    pub(crate) fn delay(&self) -> Option<u32> {
        self.timers.borrow().first().map(|&(_, delay, _)| delay)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn run_first(list: &RefCell<Vec<Waiting>>) -> bool {
        let task = {
            let mut tasks = list.borrow_mut();
            if tasks.is_empty() { None } else { Some(tasks.remove(0)) }
        };
        match task {
            Some((_, _, task)) => {
                task();
                true
            },
            None => false,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs the oldest waiting timer, and says whether there was one. The task may schedule another.
    pub(crate) fn advance(&self) -> bool {
        Self::run_first(&self.timers)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs the oldest waiting frame, and says whether there was one.
    pub(crate) fn advance_frame(&self) -> bool {
        Self::run_first(&self.frames)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[cfg(test)]
    pub(crate) fn teardowns(&self) -> u32 {
        self.teardowns.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the next `count` timers fail to be scheduled.
    #[cfg(test)]
    pub(crate) fn fail_next_timers(&self, count: u32) {
        self.timer_failures.set(count);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the next `count` frames fail to be asked for.
    #[cfg(test)]
    pub(crate) fn fail_next_frames(&self, count: u32) {
        self.frame_failures.set(count);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes cancelling leave the task waiting, to test that a stale callback does nothing.
    #[cfg(test)]
    pub(crate) fn keep_cancelled(&self) {
        self.keep_cancelled.set(true);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn push(
        &self,
        list: &RefCell<Vec<Waiting>>,
        failures: &Cell<u32>,
        delay: u32,
        task: Box<dyn FnOnce()>,
    ) -> Result<u64, String> {
        if failures.get() > 0 {
            failures.set(failures.get() - 1);
            return Err("the browser would not schedule it".to_owned());
        }
        let handle = self.next.get() + 1;
        self.next.set(handle);
        list.borrow_mut().push((handle, delay, task));
        Ok(handle)
    }
}

impl Scheduler for ManualScheduler {
    fn schedule_once(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> Result<u64, String> {
        self.push(&self.timers, &self.timer_failures, delay_ms, task)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn cancel(&self, handle: u64) {
        if !self.keep_cancelled.get() {
            self.timers.borrow_mut().retain(|&(at, _, _)| at != handle);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn request_frame(&self, task: Box<dyn FnOnce()>) -> Result<u64, String> {
        self.push(&self.frames, &self.frame_failures, 0, task)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn cancel_frame(&self, handle: u64) {
        if !self.keep_cancelled.get() {
            self.frames.borrow_mut().retain(|&(at, _, _)| at != handle);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn teardown(&self) {
        #[cfg(test)]
        self.teardowns.set(self.teardowns.get() + 1);
        self.timers.borrow_mut().clear();
        self.frames.borrow_mut().clear();
    }
}
