//! What runs the playback's ticks: the browser's timers, or a clock that a test advances by hand.

use super::controller::{ManualScheduler, Scheduler};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// The names of the page's own flag and hooks, which a test can set before the panel is built.
const MANUAL_CLOCK: &str = "__rbtManualClock";
const MANUAL_CLOCK_QUERY: &str = "manual-clock";
const ADVANCE_HOOK: &str = "__rbtAdvance";
const PENDING_HOOK: &str = "__rbtPending";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs tasks with the browser's own timers. Each closure is kept until its timer has run or been cleared, and then
/// dropped.
pub(super) struct BrowserScheduler {
    window: web_sys::Window,
    timers: RefCell<HashMap<i32, Closure<dyn FnMut()>>>,
    finished: Rc<RefCell<Vec<i32>>>,
}

impl BrowserScheduler {
    pub(super) fn new() -> Result<Self, String> {
        Ok(Self {
            window: web_sys::window().ok_or_else(|| "no window to set timers on".to_owned())?,
            timers: RefCell::new(HashMap::new()),
            finished: Rc::new(RefCell::new(Vec::new())),
        })
    }

    /// Drops the closures of the timers that have run.
    fn reap(&self) {
        for id in self.finished.borrow_mut().drain(..).collect::<Vec<_>>() {
            self.timers.borrow_mut().remove(&id);
        }
    }
}

impl Scheduler for BrowserScheduler {
    fn schedule_once(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> Result<u64, String> {
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
        let handle = self
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), delay)
            .map_err(|e| format!("could not set a timer: {e:?}"))?;
        id.set(handle);
        self.timers.borrow_mut().insert(handle, closure);
        Ok(handle as u64)
    }

    fn cancel(&self, handle: u64) {
        self.window.clear_timeout_with_handle(handle as i32);
        self.timers.borrow_mut().remove(&(handle as i32));
        self.reap();
    }

    fn teardown(&self) {
        let timers: Vec<i32> = self.timers.borrow().keys().copied().collect();
        for handle in timers {
            self.window.clear_timeout_with_handle(handle);
        }
        self.timers.borrow_mut().clear();
        self.finished.borrow_mut().clear();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The functions that a test calls on the window to drive a manual clock.
struct Hooks {
    advance: Closure<dyn Fn(u32) -> u32>,
    pending: Closure<dyn Fn() -> u32>,
}

impl Hooks {
    fn named(&self) -> [(&'static str, &JsValue); 2] {
        [(ADVANCE_HOOK, self.advance.as_ref()), (PENDING_HOOK, self.pending.as_ref())]
    }
}

/// A manual clock, and the hooks that a test drives it with. It owns the hooks, so it can take them off the window and
/// free them, which happens when the page is torn down, and again if the clock is dropped without that.
struct ManualClock {
    clock: Rc<ManualScheduler>,
    window: web_sys::Window,
    hooks: RefCell<Option<Hooks>>,
}

impl ManualClock {
    /// Puts `window.__rbtAdvance(n)`, which runs up to `n` ticks, and `window.__rbtPending()`, which says how many
    /// wait, on the window.
    ///
    /// # Errors
    ///
    /// Returns `Err` if there is no window, or a hook could not be put on it. A hook that was put on is taken off
    /// again.
    fn install() -> Result<Self, String> {
        let window = web_sys::window().ok_or_else(|| "no window to put the clock hooks on".to_owned())?;
        let clock = Rc::new(ManualScheduler::default());
        let runner = Rc::clone(&clock);
        let counter = Rc::clone(&clock);
        let hooks = Hooks {
            advance: Closure::<dyn Fn(u32) -> u32>::new(move |n: u32| {
                let mut ran = 0;
                while ran < n && runner.advance() {
                    ran += 1;
                }
                ran
            }),
            pending: Closure::<dyn Fn() -> u32>::new(move || counter.pending() as u32),
        };
        let this = Self {
            clock,
            window,
            hooks: RefCell::new(None),
        };
        for (at, (name, hook)) in hooks.named().into_iter().enumerate() {
            if let Err(e) = js_sys::Reflect::set(&this.window, &JsValue::from_str(name), hook) {
                for (earlier, hook) in hooks.named().into_iter().take(at) {
                    remove_hook(&this.window, earlier, hook);
                }
                return Err(format!("could not put {name} on the window: {e:?}"));
            }
        }
        *this.hooks.borrow_mut() = Some(hooks);
        Ok(this)
    }

    /// Takes the hooks off the window and frees them. A hook that a later clock has put in its place is left alone.
    fn release(&self) {
        let Some(hooks) = self.hooks.borrow_mut().take() else {
            return;
        };
        for (name, hook) in hooks.named() {
            remove_hook(&self.window, name, hook);
        }
    }
}

impl Scheduler for ManualClock {
    fn schedule_once(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> Result<u64, String> {
        self.clock.schedule_once(delay_ms, task)
    }

    fn cancel(&self, handle: u64) {
        self.clock.cancel(handle);
    }

    fn teardown(&self) {
        self.clock.teardown();
        self.release();
    }
}

impl Drop for ManualClock {
    fn drop(&mut self) {
        self.release();
    }
}

/// Removes the property `name` of `window`, if it is `hook`. Another clock may have replaced it, and that is not for
/// this one to remove.
fn remove_hook(window: &web_sys::Window, name: &str, hook: &JsValue) {
    let key = JsValue::from_str(name);
    let ours = js_sys::Reflect::get(window, &key).is_ok_and(|current| js_sys::Object::is(&current, hook));
    if ours {
        let _ = js_sys::Reflect::delete_property(window, &key);
    }
}

/// Whether the page that is being built asked, with a flag on the window or `?manual-clock` in its address, to be
/// played by hand and not by the clock. A browser test sets one of them before the panel is built.
fn manual_clock_requested() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let flag = js_sys::Reflect::get(&window, &JsValue::from_str(MANUAL_CLOCK)).is_ok_and(|flag| flag.is_truthy());
    flag || window.location().search().is_ok_and(|query| query.contains(MANUAL_CLOCK_QUERY))
}

/// Makes the scheduler for the page: the browser's, or when a test asked for it a scheduler that runs a tick only when
/// the test calls its hook. The hooks are taken off the window by [`Scheduler::teardown`].
///
/// # Errors
///
/// Returns `Err` if there is no window, or the hooks could not be put on it.
pub(super) fn make_scheduler() -> Result<Rc<dyn Scheduler>, String> {
    if manual_clock_requested() {
        Ok(Rc::new(ManualClock::install()?))
    } else {
        Ok(Rc::new(BrowserScheduler::new()?))
    }
}
