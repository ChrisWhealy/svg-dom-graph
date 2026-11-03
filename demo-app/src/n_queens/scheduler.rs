//! What runs the playback's ticks: the browser's timers, or a clock that a test advances by hand.

use super::controller::{ManualScheduler, Scheduler};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// The names of the page's own flags and hooks, which a test can set before the panel is built.
const MANUAL_CLOCK: &str = "__nQueensManualClock";
const MANUAL_CLOCK_QUERY: &str = "manual-clock";
const ADVANCE_HOOK: &str = "__nQueensAdvance";
const PENDING_HOOK: &str = "__nQueensPending";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs a task after a delay with the browser's own timers. Each closure is kept until its timer has run or been
/// cleared, and then dropped.
pub(super) struct TimeoutScheduler {
    window: web_sys::Window,
    closures: RefCell<HashMap<i32, Closure<dyn FnMut()>>>,
    finished: Rc<RefCell<Vec<i32>>>,
}

impl TimeoutScheduler {
    pub(super) fn new() -> Result<Self, String> {
        Ok(Self {
            window: web_sys::window().ok_or_else(|| "no window to set timers on".to_owned())?,
            closures: RefCell::new(HashMap::new()),
            finished: Rc::new(RefCell::new(Vec::new())),
        })
    }

    /// Drops the closures of the timers that have run.
    pub(super) fn reap(&self) {
        let done: Vec<i32> = self.finished.borrow_mut().drain(..).collect();
        for id in done {
            self.closures.borrow_mut().remove(&id);
        }
    }
}

impl Scheduler for TimeoutScheduler {
    fn schedule(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> Result<u64, String> {
        self.reap();
        let id = Rc::new(Cell::new(0));
        let (finished, own) = (Rc::clone(&self.finished), Rc::clone(&id));
        let mut task = Some(task);
        let closure = Closure::<dyn FnMut()>::new(move || {
            finished.borrow_mut().push(own.get());
            if let Some(task) = task.take() {
                task();
            }
        });
        let delay = i32::try_from(delay_ms).unwrap_or(i32::MAX);
        match self
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), delay)
        {
            Ok(handle) => {
                id.set(handle);
                self.closures.borrow_mut().insert(handle, closure);
                Ok(handle as u64)
            },
            // The closure is dropped here, and the task with it, so nothing is left that could run.
            Err(e) => Err(format!("could not set a timer: {e:?}")),
        }
    }

    fn cancel(&self, handle: u64) {
        self.window.clear_timeout_with_handle(handle as i32);
        self.closures.borrow_mut().remove(&(handle as i32));
        self.reap();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the page that is being built asked, with a flag on the window or `?manual-clock` in its address, to be
/// played by hand and not by the clock. A browser test sets one of them before the panel is built.
pub(super) fn manual_clock_requested() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let flag = js_sys::Reflect::get(&window, &JsValue::from_str(MANUAL_CLOCK)).is_ok_and(|flag| flag.is_truthy());
    flag || window.location().search().is_ok_and(|query| query.contains(MANUAL_CLOCK_QUERY))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the scheduler for the page: the browser's timers, or when a test asked for it a scheduler that runs a tick
/// only when the test calls `window.__nQueensAdvance(n)`. `window.__nQueensPending()` says how many ticks are waiting.
///
/// # Errors
///
/// Returns `Err` if there is no window, or the hooks could not be put on it.
pub(super) fn make_scheduler() -> Result<Rc<dyn Scheduler>, String> {
    if !manual_clock_requested() {
        return Ok(Rc::new(TimeoutScheduler::new()?));
    }
    let window = web_sys::window().ok_or_else(|| "no window to put the clock hooks on".to_owned())?;
    let manual = Rc::new(ManualScheduler::default());
    let advance = Rc::clone(&manual);
    let advance_hook = Closure::<dyn Fn(u32) -> u32>::new(move |ticks: u32| {
        let mut ran = 0;
        while ran < ticks && advance.advance() {
            ran += 1;
        }
        ran
    });
    let pending = Rc::clone(&manual);
    let pending_hook = Closure::<dyn Fn() -> u32>::new(move || pending.pending() as u32);
    for (name, hook) in [(ADVANCE_HOOK, advance_hook.as_ref()), (PENDING_HOOK, pending_hook.as_ref())] {
        js_sys::Reflect::set(&window, &JsValue::from_str(name), hook)
            .map_err(|e| format!("could not put {name} on the window: {e:?}"))?;
    }
    advance_hook.forget();
    pending_hook.forget();
    Ok(manual)
}
