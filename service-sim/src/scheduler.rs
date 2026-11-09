//! What runs a delayed task or an animation frame: the trait that the controller schedules with, and a scheduler with a
//! clock that a test moves by hand.
//!
//! The host scheduler only chooses when the next slice is presented (design section 2). No simulated event is a timer.
//! A page uses the browser's timers and frames, and a test uses [`ManualScheduler`], which runs a task only when it is
//! told to, can keep a cancelled task as a callback that was already on its way would be, and can be made to fail.

use std::cell::{Cell, RefCell};

/// A task that a scheduler runs once.
pub type Task = Box<dyn FnOnce()>;

/// Runs a task after a delay or at the next frame, and can take it back.
pub trait Scheduler {
    /// Runs `task` after `delay_ms` milliseconds, and gives a handle that [`cancel`](Self::cancel) takes.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the task could not be scheduled, in which case it will never run.
    fn schedule_once(&self, delay_ms: u32, task: Task) -> Result<u64, String>;

    /// Takes back the task with `handle`, if it has not run.
    fn cancel(&self, handle: u64);

    /// Runs `task` at the next animation frame.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the frame could not be asked for, in which case the task will never run.
    fn request_frame(&self, task: Task) -> Result<u64, String>;

    /// Takes back the frame with `handle`, if it has not come.
    fn cancel_frame(&self, handle: u64);
}

/// A task that is waiting: its handle, when it is due, whether it was cancelled, and what it runs.
struct Waiting {
    handle: u64,
    due: u64,
    cancelled: bool,
    task: Option<Task>,
}

/// A scheduler with a virtual clock. Time passes only when [`advance_by`](Self::advance_by) is called, and a frame
/// comes only when [`advance_frame`](Self::advance_frame) is.
#[derive(Default)]
pub struct ManualScheduler {
    next: Cell<u64>,
    now: Cell<u64>,
    timers: RefCell<Vec<Waiting>>,
    frames: RefCell<Vec<Waiting>>,
    /// Whether a cancelled task stays and still runs, as a callback that was already on its way to the browser's queue
    /// would.
    keep_cancelled: Cell<bool>,
    timer_failures: Cell<u32>,
    frame_failures: Cell<u32>,
}

impl ManualScheduler {
    /// The virtual time, in milliseconds.
    #[must_use]
    pub fn now(&self) -> u64 {
        self.now.get()
    }

    /// How many timers are waiting that were not cancelled.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.timers.borrow().iter().filter(|t| !t.cancelled).count()
    }

    /// How many frames are waiting that were not cancelled.
    #[must_use]
    pub fn pending_frames(&self) -> usize {
        self.frames.borrow().iter().filter(|t| !t.cancelled).count()
    }

    /// How many tasks are held at all, which includes the cancelled ones that were kept.
    #[must_use]
    pub fn held(&self) -> usize {
        self.timers.borrow().len() + self.frames.borrow().len()
    }

    /// The delay, in milliseconds from now, of the timer that is waiting, if there is one.
    #[must_use]
    pub fn next_delay(&self) -> Option<u64> {
        self.timers
            .borrow()
            .iter()
            .filter(|t| !t.cancelled)
            .map(|t| t.due.saturating_sub(self.now.get()))
            .min()
    }

    /// Makes cancelling leave the task waiting, so that a test can run a callback that the controller believes it took
    /// back.
    pub fn keep_cancelled(&self, on: bool) {
        self.keep_cancelled.set(on);
    }

    /// Makes the next `count` timers fail to be scheduled.
    pub fn fail_next_timers(&self, count: u32) {
        self.timer_failures.set(count);
    }

    /// Makes the next `count` frames fail to be asked for.
    pub fn fail_next_frames(&self, count: u32) {
        self.frame_failures.set(count);
    }

    fn take_next(list: &RefCell<Vec<Waiting>>, limit: Option<u64>) -> Option<(u64, Task)> {
        let mut tasks = list.borrow_mut();
        let at = tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| limit.is_none_or(|l| t.due <= l))
            .min_by_key(|(_, t)| (t.due, t.handle))
            .map(|(i, _)| i)?;
        let mut waiting = tasks.remove(at);
        waiting.task.take().map(|task| (waiting.due, task))
    }

    /// Lets `ms` milliseconds pass, running every timer that falls due in the order of its time, and the ones that they
    /// schedule if those fall due too. Gives how many ran.
    pub fn advance_by(&self, ms: u64) -> usize {
        let limit = self.now.get() + ms;
        let mut ran = 0;
        while let Some((due, task)) = Self::take_next(&self.timers, Some(limit)) {
            self.now.set(self.now.get().max(due));
            task();
            ran += 1;
        }
        self.now.set(limit);
        ran
    }

    /// Runs the oldest waiting frame, and says whether there was one. The task may ask for another.
    pub fn advance_frame(&self) -> bool {
        match Self::take_next(&self.frames, None) {
            Some((_, task)) => {
                task();
                true
            },
            None => false,
        }
    }

    fn add(&self, list: &RefCell<Vec<Waiting>>, due: u64, task: Task) -> u64 {
        let handle = self.next.get() + 1;
        self.next.set(handle);
        list.borrow_mut().push(Waiting {
            handle,
            due,
            cancelled: false,
            task: Some(task),
        });
        handle
    }

    fn take_back(&self, list: &RefCell<Vec<Waiting>>, handle: u64) {
        let mut tasks = list.borrow_mut();
        if self.keep_cancelled.get() {
            if let Some(t) = tasks.iter_mut().find(|t| t.handle == handle) {
                t.cancelled = true;
            }
        } else {
            tasks.retain(|t| t.handle != handle);
        }
    }
}

impl Scheduler for ManualScheduler {
    fn schedule_once(&self, delay_ms: u32, task: Task) -> Result<u64, String> {
        if self.timer_failures.get() > 0 {
            self.timer_failures.set(self.timer_failures.get() - 1);
            return Err("the timer could not be scheduled".to_owned());
        }
        Ok(self.add(&self.timers, self.now.get() + u64::from(delay_ms), task))
    }

    fn cancel(&self, handle: u64) {
        self.take_back(&self.timers, handle);
    }

    fn request_frame(&self, task: Task) -> Result<u64, String> {
        if self.frame_failures.get() > 0 {
            self.frame_failures.set(self.frame_failures.get() - 1);
            return Err("the frame could not be asked for".to_owned());
        }
        Ok(self.add(&self.frames, self.now.get(), task))
    }

    fn cancel_frame(&self, handle: u64) {
        self.take_back(&self.frames, handle);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::rc::Rc;

    fn counter() -> (Rc<Cell<u32>>, impl Fn() -> Task) {
        let c = Rc::new(Cell::new(0));
        let c2 = Rc::clone(&c);
        (c, move || {
            let c = Rc::clone(&c2);
            Box::new(move || c.set(c.get() + 1))
        })
    }

    #[test]
    fn time_passes_only_when_it_is_moved_and_timers_run_in_the_order_of_their_time() {
        let s = ManualScheduler::default();
        let order = Rc::new(RefCell::new(Vec::new()));
        for (delay, name) in [(300, "c"), (100, "a"), (200, "b")] {
            let order = Rc::clone(&order);
            s.schedule_once(delay, Box::new(move || order.borrow_mut().push(name)))
                .expect("a timer");
        }
        assert_eq!(s.now(), 0);
        assert!(order.borrow().is_empty(), "nothing runs by itself");
        assert_eq!(s.advance_by(150), 1);
        assert_eq!(*order.borrow(), vec!["a"]);
        assert_eq!(s.advance_by(1_000), 2);
        assert_eq!(*order.borrow(), vec!["a", "b", "c"]);
        assert_eq!(s.now(), 1_150);
    }

    #[test]
    fn a_task_that_schedules_another_runs_it_in_the_same_advance_if_it_falls_due() {
        let s = Rc::new(ManualScheduler::default());
        let (ran, make) = counter();
        let (s2, again) = (Rc::clone(&s), make());
        s.schedule_once(
            10,
            Box::new(move || {
                again();
                s2.schedule_once(10, Box::new(|| {})).expect("another");
            }),
        )
        .expect("a timer");
        assert_eq!(s.advance_by(25), 2);
        assert_eq!(ran.get(), 1);
        assert_eq!(s.now(), 25);
    }

    #[test]
    fn a_cancelled_task_does_not_run_unless_it_is_kept_and_then_it_does() {
        let s = ManualScheduler::default();
        let (ran, make) = counter();
        let h = s.schedule_once(5, make()).expect("a timer");
        s.cancel(h);
        assert_eq!((s.pending(), s.held()), (0, 0));
        s.advance_by(10);
        assert_eq!(ran.get(), 0);

        s.keep_cancelled(true);
        let h = s.schedule_once(5, make()).expect("a timer");
        s.cancel(h);
        assert_eq!((s.pending(), s.held()), (0, 1), "a cancelled task is not pending but is held");
        s.advance_by(10);
        assert_eq!(ran.get(), 1, "a task that was already on its way runs");
        let f = s.request_frame(make()).expect("a frame");
        s.cancel_frame(f);
        assert!(s.advance_frame());
        assert_eq!(ran.get(), 2);
    }

    #[test]
    fn a_failure_can_be_injected_into_a_timer_or_a_frame_and_then_clears() {
        let s = ManualScheduler::default();
        s.fail_next_timers(1);
        assert!(s.schedule_once(1, Box::new(|| {})).is_err());
        assert!(s.schedule_once(1, Box::new(|| {})).is_ok());
        s.fail_next_frames(2);
        assert!(s.request_frame(Box::new(|| {})).is_err());
        assert!(s.request_frame(Box::new(|| {})).is_err());
        assert!(s.request_frame(Box::new(|| {})).is_ok());
        assert_eq!(s.next_delay(), Some(1));
    }
}
