//! The page side of workload W6: a controller on a surface, built once and driven for measurement.
//!
//! It gives the page a few primitives, and the page's JavaScript orchestrates them: which slices are priming, which are
//! warm-up and which are measured, what the independent engine says each slice's hash is, and what a result carries.
//! What has to be fast and exact is here. A timed batch is one call, `sim_batch`, in which the clock is read before the
//! first draw and after the fourth commit and nothing else is: the slices are prepared before the first read, and every
//! check is made after the second.
//!
//! Two kinds of fixture are built. The measurement one (`product: false`) asks whether the host may be drawn on by a
//! cached activation flag and `isConnected`, and reads no layout, as design section 8.3 requires of a timed draw. The
//! product one (`product: true`) is the controller as a person's page has it, with its gallery hint, its lifecycle
//! listeners and the full test of visibility, and two real controls: a Next button and a Play and Pause button. Its
//! scheduler writes a user-timing mark as every timer callback is entered, outside the product's code, so that Layer C
//! can measure from the callback's entry to the slice's presentation without an instrumentation hook in a release
//! build.

use serde::Deserialize;
use service_sim::controller::{Controller, Outcome};
use service_sim::host::{BrowserScheduler, GalleryHint, PageHost, PageLifecycle};
use service_sim::projection::View;
use service_sim::scheduler::{Scheduler, Task};
use service_sim::surface::Surface;
use service_sim_engine::hash::sha256_hex;
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Engine, Error as EngineError, Limits};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use svg_dom::SvgRoot;
use svg_dom::root::utils::Size;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

type Factory = Rc<dyn Fn() -> Result<Engine, EngineError>>;

const ROOT_ID: &str = "perf-sim";
const SVG_ID: &str = "perf-sim-svg";
const NEXT_ID: &str = "perf-sim-next";
const PLAY_ID: &str = "perf-sim-play";
const SEEK_ID: &str = "perf-sim-seek";
const STATUS_ID: &str = "perf-sim-status";
const STAGE_ID: &str = "perf-sim-stage";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scheduler that stamps the entry of each timer callback, and otherwise is the browser's. It belongs to the harness,
/// and the product's code is unaware of it: the stamp is taken immediately before the product's task is invoked.
#[derive(Clone)]
struct Stamping {
    inner: BrowserScheduler,
    /// The clock, in milliseconds, when the last timer callback was entered.
    entry_ms: Rc<Cell<f64>>,
    /// How many timer callbacks have been entered.
    ticks: Rc<Cell<u64>>,
}

fn now() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or(0.0, |p| p.now())
}

impl Scheduler for Stamping {
    fn schedule_once(&self, delay_ms: u32, task: Task) -> Result<u64, String> {
        let (entry, ticks) = (Rc::clone(&self.entry_ms), Rc::clone(&self.ticks));
        self.inner.schedule_once(
            delay_ms,
            Box::new(move || {
                let n = ticks.get() + 1;
                ticks.set(n);
                // The user-timing mark of the entry is written first, since it is what Layer C measures from, and the
                // product's task is invoked straight after it. The product's code holds neither the mark nor the call.
                if let Some(performance) = web_sys::window().and_then(|w| w.performance()) {
                    let _ = performance.mark(&format!("sim-tick-{n}"));
                }
                entry.set(now());
                task();
            }),
        )
    }

    fn cancel(&self, handle: u64) {
        self.inner.cancel(handle);
    }

    fn request_frame(&self, task: Task) -> Result<u64, String> {
        self.inner.request_frame(task)
    }

    fn cancel_frame(&self, handle: u64) {
        self.inner.cancel_frame(handle);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct Listener {
    target: web_sys::Element,
    event: &'static str,
    closure: Closure<dyn FnMut()>,
}

struct Sim {
    controller: Controller<PageHost, Stamping>,
    scheduler: Stamping,
    root: web_sys::Element,
    /// Kept so that what they listen to goes when the fixture does.
    listeners: Vec<Listener>,
    _gallery: Option<GalleryHint>,
    _lifecycle: Option<PageLifecycle>,
}

thread_local! {
    static SIM: RefCell<Option<Sim>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn error(message: impl std::fmt::Display) -> String {
    serde_json::json!({ "ok": false, "error": message.to_string() }).to_string()
}

fn ok(mut value: serde_json::Value) -> String {
    if let Some(object) = value.as_object_mut() {
        object.insert("ok".to_string(), serde_json::json!(true));
    }
    value.to_string()
}

fn document() -> Result<web_sys::Document, String> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| "there is no document".to_owned())
}

fn js<E: std::fmt::Debug>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e:?}")
}

fn outcome_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Done => "done",
        Outcome::AtBoundary => "at_boundary",
        Outcome::Pending => "pending",
        Outcome::Unavailable => "unavailable",
        Outcome::Failed => "failed",
    }
}

fn with_sim<T>(work: impl FnOnce(&mut Sim) -> Result<T, String>) -> Result<T, String> {
    SIM.with_borrow_mut(|s| work(s.as_mut().ok_or("no simulator is built")?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Deserialize)]
struct Config {
    /// A string, since a number from JavaScript rounds a seed over 2**53. A number is accepted for the small ones.
    #[serde(deserialize_with = "seed_from_text_or_number")]
    seed: u64,
    /// The arrivals of a scaled run of the frozen revision, when the trace is one.
    #[serde(default)]
    arrivals: Option<u64>,
    /// The controller as a person's page has it, with real controls, when true. The measurement fixture, when false.
    #[serde(default)]
    product: bool,
    /// How the container of the scene is staged while the surface is built into it, for the gallery's staging probe
    /// (dess/demo-panel/p0). `opacity_inert` is a connected, laid-out container with `opacity:0`, `pointer-events:none`
    /// and `inert`, in the flow of the page. `opacity_inert_absolute` is the same taken out of the flow.
    /// `offscreen_inert` is a connected container moved far off the screen with `inert`. `opacity_aria_hidden` is the
    /// probe's negative control. Absent means a container that is shown and live.
    #[serde(default)]
    staging: Option<String>,
}

struct Page {
    root: web_sys::Element,
    stage: web_sys::Element,
    status: web_sys::Element,
    next: web_sys::Element,
    play: web_sys::Element,
    seek: web_sys::Element,
}

fn seed_from_text_or_number<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Seed {
        Text(String),
        Number(u64),
    }
    match Seed::deserialize(deserializer)? {
        Seed::Text(text) => text.parse().map_err(serde::de::Error::custom),
        Seed::Number(number) => Ok(number),
    }
}

fn make_page(document: &web_sys::Document, staging: Option<&str>) -> Result<Page, String> {
    let make = |tag: &str, id: &str| -> Result<web_sys::Element, String> {
        let e = document.create_element(tag).map_err(js(tag))?;
        e.set_id(id);
        Ok(e)
    };
    let root = make("div", ROOT_ID)?;
    let stage = make("div", STAGE_ID)?;
    let container = make("div", "perf-sim-container")?;
    // The staging is applied before anything is built into the container, since the scene has to be constructed under
    // it. Neither candidate uses `display:none` or `visibility:hidden`, which would make the host unmeasurable.
    match staging {
        None => {},
        Some("opacity_inert") => {
            container
                .set_attribute("style", "opacity:0;pointer-events:none")
                .map_err(js("staging"))?;
            container.set_attribute("inert", "").map_err(js("staging"))?;
        },
        // The same, taken out of the flow of the page, so that a staged host does not move the live scene or what
        // follows it.
        Some("opacity_inert_absolute") => {
            container
                .set_attribute(
                    "style",
                    "position:absolute;left:0;top:0;width:1000px;opacity:0;pointer-events:none",
                )
                .map_err(js("staging"))?;
            container.set_attribute("inert", "").map_err(js("staging"))?;
        },
        // Not a candidate: the negative control of the probe, which hides the scene from the accessibility tree and the
        // eye, and does not isolate it from the keyboard. The probe has to find that it leaks.
        Some("opacity_aria_hidden") => {
            container
                .set_attribute("style", "opacity:0;pointer-events:none")
                .map_err(js("staging"))?;
            container.set_attribute("aria-hidden", "true").map_err(js("staging"))?;
        },
        Some("offscreen_inert") => {
            container
                .set_attribute("style", "position:absolute;left:-20000px;top:0;width:1000px")
                .map_err(js("staging"))?;
            container.set_attribute("inert", "").map_err(js("staging"))?;
        },
        Some(other) => return Err(format!("there is no staging called {other}")),
    }
    let status = make("p", STATUS_ID)?;
    status.set_attribute("role", "status").map_err(js("status"))?;
    status.set_attribute("aria-live", "polite").map_err(js("status"))?;
    let toolbar = make("div", "perf-sim-controls")?;
    toolbar.set_attribute("role", "toolbar").map_err(js("toolbar"))?;
    toolbar
        .set_attribute("aria-label", "Simulation controls")
        .map_err(js("toolbar"))?;
    let next = make("button", NEXT_ID)?;
    next.set_attribute("aria-label", "Next slice").map_err(js("next"))?;
    next.set_text_content(Some("Next"));
    let play = make("button", PLAY_ID)?;
    play.set_attribute("aria-pressed", "false").map_err(js("play"))?;
    play.set_text_content(Some("Play"));
    // The range of the seek control is the fixed windows' end, which is the farthest that any series goes.
    let seek = make("input", SEEK_ID)?;
    seek.set_attribute("type", "range").map_err(js("seek"))?;
    seek.set_attribute("min", "0").map_err(js("seek"))?;
    seek.set_attribute("max", "536").map_err(js("seek"))?;
    seek.set_attribute("step", "1").map_err(js("seek"))?;
    seek.set_attribute("value", "0").map_err(js("seek"))?;
    seek.set_attribute("aria-label", "Seek to slice").map_err(js("seek"))?;
    toolbar.append_child(&next).map_err(js("toolbar"))?;
    toolbar.append_child(&play).map_err(js("toolbar"))?;
    toolbar.append_child(&seek).map_err(js("toolbar"))?;
    // The controls come first, so that they are in view in a small window and a real click reaches them without
    // scrolling.
    stage.append_child(&toolbar).map_err(js("toolbar"))?;
    stage.append_child(&status).map_err(js("status"))?;
    stage.append_child(&container).map_err(js("container"))?;
    root.append_child(&stage).map_err(js("stage"))?;
    document.body().ok_or("no body")?.append_child(&root).map_err(js("root"))?;
    Ok(Page {
        root,
        stage,
        status,
        next,
        play,
        seek,
    })
}

fn build_sim(config: &Config) -> Result<Sim, String> {
    let document = document()?;
    let mut scenario = Scenario::mixed_outage_7(config.seed).map_err(|e| e.message())?;
    if let (Some(n), Some(params)) = (config.arrivals, scenario.params.as_mut()) {
        params.arrivals = n;
    }
    let servers = scenario.config.servers;
    let factory: Factory = Rc::new(move || {
        Engine::new(
            &scenario.config_id,
            scenario.seed,
            scenario.config,
            &scenario.script,
            scenario.params.as_ref(),
            Limits::default(),
        )
    });
    let Page {
        root,
        stage,
        status,
        next,
        play,
        seek,
    } = make_page(&document, config.staging.as_deref())?;
    let built = (|| -> Result<Sim, String> {
        let (w, h) = Surface::view_box();
        let svg = SvgRoot::create_in("perf-sim-container", Size::new(w, h)).map_err(|e| e.to_string())?;
        svg.root.set_id(SVG_ID);
        svg.set_view_box(0.0, 0.0, w, h).map_err(|e| e.to_string())?;
        let host = document.get_element_by_id(SVG_ID).ok_or("the host was not made")?;
        let surface = Surface::build(SVG_ID, servers)?;
        let mut page_host =
            PageHost::new(surface, host).with_voices(Some(status.clone()), Some(status.clone()), Some(play.clone()));
        if !config.product {
            // The activation flag stands for a visible document and a rendered host. Nothing hides the page while a
            // batch is timed, and whoever does keeps the flag up to date outside it.
            page_host = page_host.with_cached_activation(Rc::new(Cell::new(true)));
        }
        let scheduler = Stamping {
            inner: BrowserScheduler::new(),
            entry_ms: Rc::new(Cell::new(0.0)),
            ticks: Rc::new(Cell::new(0)),
        };
        let controller = Controller::new(page_host, scheduler.clone(), Rc::clone(&factory), View::default())
            .map_err(|e| e.message())?;
        let mut listeners = Vec::new();
        let mut gallery = None;
        let mut lifecycle = None;
        if config.product {
            let (hint, shown, left) = (controller.clone(), controller.clone(), controller.clone());
            gallery = Some(GalleryHint::install(&stage, move || hint.presence_changed())?);
            lifecycle = Some(PageLifecycle::install(
                move || shown.presence_changed(),
                move || left.suspend(),
            )?);
            for (control, name) in [(&next, "next"), (&play, "play"), (&seek, "seek")] {
                let c = controller.clone();
                let slider = control.clone();
                let closure: Closure<dyn FnMut()> = match name {
                    "next" => Closure::new(move || {
                        c.next();
                    }),
                    "play" => Closure::new(move || {
                        if c.is_playing() {
                            c.pause();
                        } else {
                            c.play();
                        }
                    }),
                    _ => Closure::new(move || {
                        let value = js_sys::Reflect::get(&slider, &"value".into())
                            .ok()
                            .and_then(|v| v.as_string())
                            .and_then(|v| v.parse::<u64>().ok());
                        if let Some(target) = value {
                            c.seek(target);
                        }
                    }),
                };
                let event = if name == "seek" { "change" } else { "click" };
                control
                    .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
                    .map_err(js("listener"))?;
                listeners.push(Listener {
                    target: control.clone(),
                    event,
                    closure,
                });
            }
        }
        Ok(Sim {
            controller,
            scheduler,
            root: root.clone(),
            listeners,
            _gallery: gallery,
            _lifecycle: lifecycle,
        })
    })();
    if built.is_err() {
        root.remove();
    }
    built
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds a controller on a surface at the first slice. Any built before it is removed first.
#[must_use]
pub fn build(config_json: &str) -> String {
    let _ = teardown();
    let config: Config = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => return error(e),
    };
    match build_sim(&config) {
        Ok(sim) => {
            let elements = sim.root.query_selector_all("*").map_or(0, |l| l.length());
            SIM.with_borrow_mut(|s| *s = Some(sim));
            ok(serde_json::json!({ "elements": elements, "product": config.product }))
        },
        Err(message) => error(message),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One timed batch: the next `count` slices are worked out first, and then the clock is read, `count` calls of `next`
/// are made, and the clock is read again. Nothing else is inside the interval, and the outcomes are looked at
/// afterwards.
///
/// `elapsed_ms` is the interval. It is the synchronous cost of drawing and committing `count` slices, including the
/// data node's setter, the overlay writes, the connectedness guards before and after each write and the commit's own
/// bookkeeping. It is not a frame time.
#[must_use]
pub fn batch(count: u32) -> String {
    let result = with_sim(|sim| {
        let checkpoints = sim.controller.prepare_ahead_with_checkpoints(u64::from(count))?;
        let made = checkpoints.len();
        if made < count as usize {
            return Err(format!("the run ended after {made} of the {count} slices that a batch needs"));
        }
        let performance = web_sys::window()
            .and_then(|w| w.performance())
            .ok_or("there is no performance object")?;
        let mut outcomes = Vec::with_capacity(count as usize);
        // Where the engine is, read outside the interval before and after it. If the interval stepped it, the slices
        // were not ready when the clock started, and the figure holds engine work that Layer B excludes.
        let engine_before = sim.controller.live_engine().slice;
        let start = performance.now();
        for _ in 0..count {
            outcomes.push(sim.controller.next());
        }
        let end = performance.now();
        let engine_after = sim.controller.live_engine().slice;
        let names: Vec<&str> = outcomes.iter().map(|o| outcome_name(*o)).collect();
        Ok(serde_json::json!({
            "elapsed_ms": end - start,
            "outcomes": names,
            // The engine's own hash after each slice of the batch, taken as it stepped and outside the interval, so
            // that every slice is checked and not only the last.
            "checkpoints": checkpoints,
            "engine_stepped_in_interval": engine_after != engine_before,
        }))
    });
    result.map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `count` calls of `next`, untimed, stopping early at the first that does not commit.
#[must_use]
pub fn step(count: u32) -> String {
    with_sim(|sim| {
        let mut done = 0;
        let mut last = Outcome::Done;
        for _ in 0..count {
            last = sim.controller.next();
            if last != Outcome::Done {
                break;
            }
            done += 1;
        }
        Ok(serde_json::json!({ "done": done, "last": outcome_name(last) }))
    })
    .map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A full redraw of the committed slice, which is what a repair is. After a delta draw it has to leave the page as it
/// was.
#[must_use]
pub fn repair() -> String {
    with_sim(|sim| Ok(serde_json::json!({ "outcome": outcome_name(sim.controller.repair()) }))).map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The speed of playback, in milliseconds a slice.
#[must_use]
pub fn set_speed(ms: u32) -> String {
    with_sim(|sim| Ok(serde_json::json!({ "outcome": outcome_name(sim.controller.set_speed(ms)) })))
        .map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a check outside a timed interval reads: the cursor, the controller's own engine and the page it drew.
#[must_use]
pub fn state() -> String {
    with_sim(|sim| {
        let c = &sim.controller;
        let live = c.live_engine();
        let (page, cells, trusted, setter_calls) = c.with_host(|h| {
            let surface = h.surface();
            (
                surface.snapshot().unwrap_or_default(),
                surface.data_texts().unwrap_or_default(),
                surface.is_trusted(),
                surface.setter_calls(),
            )
        });
        Ok(serde_json::json!({
            "committed_slice": c.committed_slice(),
            "engine_slice": live.slice,
            "engine_recorded_at": live.recorded_at,
            "engine_sha256": live.sha256,
            "page_sha256": sha256_hex(page.as_bytes()),
            // What the page holds, read back from its elements, against what the projection says it should: the one
            // check that a write that is skipped on a full redraw as well as on a delta cannot pass.
            "page_is_projection": page == c.committed().canonical(),
            "projection_sha256": c.committed().sha256(),
            "data_cells": cells,
            "surface_trusted": trusted,
            "setter_calls": setter_calls,
            "dirty": c.is_dirty(),
            // The host's own test of whether it may be drawn on, as the controller asks it.
            "presence": format!("{:?}", c.with_host(|h| h.presence_for_test())),
            "reads_layout": c.with_host(|h| h.reads_layout()),
            "playing": c.is_playing(),
            "epoch": c.epoch(),
            "waiting_tasks": c.scheduler().inner.waiting(),
            "live_closures": c.scheduler().inner.live_closures(),
            "timer_entries": sim.scheduler.ticks.get(),
            "timer_entry_ms": sim.scheduler.entry_ms.get(),
            "elements": sim.root.query_selector_all("*").map_or(0, |l| l.length()),
        }))
    })
    .map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Ends the controller, takes off what listens to the page, and removes the page. It is safe to call with nothing
/// built.
#[must_use]
pub fn teardown() -> String {
    let sim = SIM.with_borrow_mut(Option::take);
    if let Some(sim) = sim {
        sim.controller.dispose();
        for listener in &sim.listeners {
            let _ = listener
                .target
                .remove_event_listener_with_callback(listener.event, listener.closure.as_ref().unchecked_ref());
        }
        sim.root.remove();
    }
    ok(serde_json::json!({}))
}
