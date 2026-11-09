//! The browser's side of the controller: a scheduler on the page's timers and frames, a host that is the surface and
//! the predicate that says whether it may be drawn on, and the hint that a gallery panel was hidden.
//!
//! The scheduler owns the closure of every task it hands to the browser, so that a task that is cancelled and never
//! called does not leave its Rust state behind, and counts them, so that a test can see that none is left. A closure
//! cannot be dropped while it runs, so one that has run is released by a microtask, after the browser's call of it has
//! returned.

use crate::controller::{DrawFailure, Host, Presence};
use crate::projection::Projection;
use crate::scheduler::{Scheduler, Task};
use crate::surface::{DrawError, Surface};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Timer,
    Frame,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A closure that counts itself while it lives, so that the count is of closures and not of the entries that were meant
/// to hold them.
struct Tracked {
    _closure: Closure<dyn FnMut()>,
    alive: Rc<Cell<usize>>,
}

impl Tracked {
    fn new(closure: Closure<dyn FnMut()>, alive: &Rc<Cell<usize>>) -> Self {
        alive.set(alive.get() + 1);
        Self {
            _closure: closure,
            alive: Rc::clone(alive),
        }
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.alive.set(self.alive.get().saturating_sub(1));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct Entry {
    kind: Kind,
    browser_id: i32,
    /// Held so that the function the browser calls stays alive, and dropped when the entry is.
    closure: Tracked,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Default)]
struct Tasks {
    next: u64,
    live: HashMap<u64, Entry>,
    /// Closures that have run, and wait to be dropped after the browser's call of them has returned.
    finished: Vec<Tracked>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scheduler on the page's `setTimeout` and `requestAnimationFrame`.
#[derive(Clone, Default)]
pub struct BrowserScheduler {
    tasks: Rc<RefCell<Tasks>>,
    /// How many closures are alive, which a test reads.
    alive: Rc<Cell<usize>>,
}

impl BrowserScheduler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many closures the scheduler holds alive: those that wait, and those that have run and are not yet released.
    #[must_use]
    pub fn live_closures(&self) -> usize {
        self.alive.get()
    }

    /// How many tasks are waiting for the browser to call them.
    #[must_use]
    pub fn waiting(&self) -> usize {
        self.tasks.borrow().live.len()
    }

    fn release_finished_later(&self) {
        let tasks = Rc::clone(&self.tasks);
        let release = Closure::once_into_js(move || {
            tasks.borrow_mut().finished.clear();
        });
        if let Some(window) = web_sys::window() {
            window.queue_microtask(release.unchecked_ref());
        }
    }

    fn add(&self, kind: Kind, delay_ms: u32, task: Task) -> Result<u64, String> {
        let window = web_sys::window().ok_or("there is no window")?;
        let handle = {
            let mut t = self.tasks.borrow_mut();
            t.next += 1;
            t.next
        };
        let me = self.clone();
        let task = RefCell::new(Some(task));
        let closure = Closure::<dyn FnMut()>::new(move || {
            // It has run, so it is no longer waiting. Its closure is not dropped here, since it is running.
            let entry = me.tasks.borrow_mut().live.remove(&handle);
            if let Some(entry) = entry {
                me.tasks.borrow_mut().finished.push(entry.closure);
                me.release_finished_later();
            }
            if let Some(task) = task.borrow_mut().take() {
                task();
            }
        });
        let browser_id = match kind {
            Kind::Timer => window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                i32::try_from(delay_ms).unwrap_or(i32::MAX),
            ),
            Kind::Frame => window.request_animation_frame(closure.as_ref().unchecked_ref()),
        }
        .map_err(|e| format!("the browser refused a task: {e:?}"))?;
        let mut t = self.tasks.borrow_mut();
        t.live.insert(
            handle,
            Entry {
                kind,
                browser_id,
                closure: Tracked::new(closure, &self.alive),
            },
        );
        Ok(handle)
    }

    fn take_back(&self, handle: u64) {
        let entry = self.tasks.borrow_mut().live.remove(&handle);
        if let (Some(entry), Some(window)) = (entry, web_sys::window()) {
            match entry.kind {
                Kind::Timer => window.clear_timeout_with_handle(entry.browser_id),
                Kind::Frame => window.cancel_animation_frame(entry.browser_id).ok().unwrap_or(()),
            }
            // A task that was cancelled is never called, so its closure is dropped here, and not by the browser.
            drop(entry.closure);
        }
    }
}

impl Scheduler for BrowserScheduler {
    fn schedule_once(&self, delay_ms: u32, task: Task) -> Result<u64, String> {
        self.add(Kind::Timer, delay_ms, task)
    }

    fn cancel(&self, handle: u64) {
        self.take_back(handle);
    }

    fn request_frame(&self, task: Task) -> Result<u64, String> {
        self.add(Kind::Frame, 0, task)
    }

    fn cancel_frame(&self, handle: u64) {
        self.take_back(handle);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The surface, and the test of whether it may be drawn on.
///
/// The test is synchronous, and checks three things: that the host `<svg>` is in the document, that the document is
/// visible, and that the host is actually rendered, which `checkVisibility()` says and which is false when an ancestor
/// panel is hidden. A mere check of attachment is not enough, since a hidden gallery panel is still in the document.
pub struct PageHost {
    surface: Surface,
    host: web_sys::Element,
    disposed: bool,
    status: Option<web_sys::Element>,
    problems: Option<web_sys::Element>,
    play_button: Option<web_sys::Element>,
}

impl PageHost {
    /// A host for `surface`, whose `<svg>` is `host`.
    #[must_use]
    pub fn new(surface: Surface, host: web_sys::Element) -> Self {
        Self {
            surface,
            host,
            disposed: false,
            status: None,
            problems: None,
            play_button: None,
        }
    }

    /// Names the elements that say things to a person: the live region, the place a failure is reported, and the Play
    /// button.
    pub fn with_voices(
        mut self,
        status: Option<web_sys::Element>,
        problems: Option<web_sys::Element>,
        play_button: Option<web_sys::Element>,
    ) -> Self {
        self.status = status;
        self.problems = problems;
        self.play_button = play_button;
        self
    }

    /// Marks the host as gone for good.
    pub fn dispose(&mut self) {
        self.disposed = true;
    }

    /// The surface, for a test that reads the page or sets a fault hook.
    pub fn surface(&mut self) -> &mut Surface {
        &mut self.surface
    }

    /// The same as [`Host::presence`], for a caller that does not have the trait in scope.
    #[must_use]
    pub fn presence_for_test(&self) -> Presence {
        self.presence()
    }

    /// Whether the host element is rendered, by the browser's own account.
    fn rendered(&self) -> bool {
        let check = js_sys::Reflect::get(&self.host, &"checkVisibility".into()).ok();
        match check.and_then(|f| f.dyn_into::<js_sys::Function>().ok()) {
            Some(f) => f.call0(&self.host).ok().and_then(|v| v.as_bool()).unwrap_or(false),
            // A browser without the method is asked whether the element has a box.
            None => self.host.get_bounding_client_rect().width() > 0.0,
        }
    }
}

impl Host for PageHost {
    fn presence(&self) -> Presence {
        if self.disposed {
            Presence::Disposed
        } else if !self.host.is_connected() {
            Presence::Detached
        } else if web_sys::window().and_then(|w| w.document()).is_none_or(|d| d.hidden()) || !self.rendered() {
            Presence::Hidden
        } else {
            Presence::Live
        }
    }

    fn draw(&mut self, projection: &Projection, full: bool) -> Result<(), DrawFailure> {
        let result = if full {
            self.surface.redraw(projection)
        } else {
            self.surface.draw(projection)
        };
        match result {
            Ok(_) => Ok(()),
            Err(DrawError::Detached) => Err(DrawFailure::Detached),
            Err(other) => Err(DrawFailure::Failed(other.to_string())),
        }
    }

    fn announce(&mut self, text: &str) {
        if let Some(e) = &self.status {
            e.set_text_content(Some(text));
        }
    }

    fn report(&mut self, text: &str) {
        if let Some(e) = &self.problems {
            e.set_text_content(Some(text));
            e.remove_attribute("hidden").ok();
        }
        web_sys::console::error_1(&text.into());
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn playing_changed(&mut self, playing: bool) {
        if let Some(button) = &self.play_button {
            button
                .set_attribute("aria-pressed", if playing { "true" } else { "false" })
                .ok();
            button.set_text_content(Some(if playing { "Pause" } else { "Play" }));
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The hint that a gallery panel was shown or hidden. The gallery toggles a class on a panel and tells no one, and a
/// `MutationObserver` on the stage is delivered as a microtask at the end of that task, so it reaches the controller
/// before any timer that was already queued can run. It is a hint, and the test at the entry of every callback is still
/// made.
pub struct GalleryHint {
    observer: web_sys::MutationObserver,
    _callback: Closure<dyn FnMut()>,
}

impl GalleryHint {
    /// Watches the `class` attribute of everything under `stage`, and calls `changed` when one changes.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the observer could not be made or started.
    pub fn install(stage: &web_sys::Element, changed: impl Fn() + 'static) -> Result<Self, String> {
        let callback = Closure::<dyn FnMut()>::new(changed);
        let observer = web_sys::MutationObserver::new(callback.as_ref().unchecked_ref())
            .map_err(|e| format!("could not make an observer: {e:?}"))?;
        let options = web_sys::MutationObserverInit::new();
        options.set_attributes(true);
        options.set_subtree(true);
        options.set_attribute_filter(&js_sys::Array::of1(&"class".into()));
        observer
            .observe_with_options(stage, &options)
            .map_err(|e| format!("could not observe the stage: {e:?}"))?;
        Ok(Self { observer, _callback: callback })
    }
}

impl Drop for GalleryHint {
    fn drop(&mut self) {
        self.observer.disconnect();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The page's own lifecycle: it is hidden or shown, and it is left or returned to.
///
/// `presence()` asks `document.hidden` at every callback, but a page that is hidden and then shown again before the
/// next callback is never seen to be hidden by that test, so playback would go on as if nothing had happened. The page
/// says when it is hidden, and the controller stops then, by the same path as for a hidden gallery panel.
///
/// - `visibilitychange` calls `changed`, which stops playback if the page is hidden and repairs a dirty page if it is
///   shown.
/// - `pagehide` calls `left`, which stops unconditionally, since a page that is being left may still read as visible.
/// - `pageshow` calls `changed`, for a page that comes back from the browser's cache. It never resumes playback.
pub struct PageLifecycle {
    document: web_sys::Document,
    window: web_sys::Window,
    changed: Closure<dyn FnMut()>,
    left: Closure<dyn FnMut()>,
}

impl PageLifecycle {
    /// Listens for the page's lifecycle events until it is dropped.
    ///
    /// # Errors
    ///
    /// Returns `Err` if there is no window or a listener could not be added.
    pub fn install(changed: impl Fn() + 'static, left: impl Fn() + 'static) -> Result<Self, String> {
        let window = web_sys::window().ok_or("there is no window")?;
        let document = window.document().ok_or("there is no document")?;
        let changed = Closure::<dyn FnMut()>::new(changed);
        let left = Closure::<dyn FnMut()>::new(left);
        let add = |target: &web_sys::EventTarget, event: &str, f: &Closure<dyn FnMut()>| {
            target
                .add_event_listener_with_callback(event, f.as_ref().unchecked_ref())
                .map_err(|e| format!("could not listen for {event}: {e:?}"))
        };
        add(&document, "visibilitychange", &changed)?;
        add(&window, "pageshow", &changed)?;
        add(&window, "pagehide", &left)?;
        Ok(Self {
            document,
            window,
            changed,
            left,
        })
    }
}

impl Drop for PageLifecycle {
    fn drop(&mut self) {
        let remove = |target: &web_sys::EventTarget, event: &str, f: &Closure<dyn FnMut()>| {
            target
                .remove_event_listener_with_callback(event, f.as_ref().unchecked_ref())
                .ok();
        };
        remove(&self.document, "visibilitychange", &self.changed);
        remove(&self.window, "pageshow", &self.changed);
        remove(&self.window, "pagehide", &self.left);
    }
}
