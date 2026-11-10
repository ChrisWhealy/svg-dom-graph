//! What the service-simulation target of the recovery workload breaks: a discrete-event simulator's controller, on its
//! surface, in a page.
//!
//! The graph executor in [`crate::recovery`] builds a twin scene and checks it against a reference model of a graph,
//! and knows nothing of a controller. This one builds a [`service_sim`] controller on a real surface, with the page's
//! own timers and frames, and gives the sweeps of `perf/runner/page/service-recovery.js` what they need to break it: an
//! act for every operation a person can ask for, a checkpoint of the logical state and of the page, and the hooks that
//! a fault needs and a clean build does not have.
//!
//! The model checkpoint that a sweep compares is the engine's own hash of its state at the committed slice, taken here
//! by replaying the run. The sweep compares it with the hash that an independent engine, in JavaScript, made for the
//! same slice, which the runner supplies. So an engine that went wrong cannot agree with itself, and a controller that
//! committed the wrong slice is seen.

use serde::Deserialize;
use service_sim::controller::{Controller, Outcome};
use service_sim::host::{BrowserScheduler, GalleryHint, PageHost, PageLifecycle};
use service_sim::projection::View;
use service_sim::surface::Surface;
use service_sim_engine::hash::sha256_hex;
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Engine, Error as EngineError, Limits};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use svg_dom::SvgRoot;
use svg_dom::root::utils::Size;

const FIXTURES: &str = include_str!("../../dess/s1/fixtures.json");
const STYLE_ID: &str = "perf-service-style";

type Factory = Rc<dyn Fn() -> Result<Engine, EngineError>>;
type Page = Controller<PageHost, BrowserScheduler>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that one slot of the sweep holds: the controller, what it listens to, and where its page is.
struct Fixture {
    controller: Page,
    factory: Factory,
    scenario: String,
    svg: web_sys::Element,
    container: web_sys::Element,
    stage: web_sys::Element,
    status: web_sys::Element,
    button: web_sys::Element,
    /// How many times the fault hook of a draw was called since it was set, which says how many places a host can leave
    /// in one draw.
    hook_calls: Rc<RefCell<usize>>,
    /// Kept, so that what they listen to goes when the fixture does.
    _gallery: GalleryHint,
    _lifecycle: PageLifecycle,
}

thread_local! {
    static FIXTURES_BY_SLOT: RefCell<BTreeMap<u32, Fixture>> = const { RefCell::new(BTreeMap::new()) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn error(message: impl std::fmt::Display) -> String {
    serde_json::json!({ "ok": false, "error": message.to_string() }).to_string()
}

fn ok(value: serde_json::Value) -> String {
    let mut value = value;
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

/// The id of the `<svg>` on `slot`, and of the things around it.
#[must_use]
pub fn host_id(slot: u32) -> String {
    format!("perf-service-svg-{slot}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scenarios that can be broken: a fixture by its name, or the trace of a seed of one of the panels.
fn factory_for(scenario: &str) -> Result<(Factory, usize), String> {
    let (name, seed) = scenario
        .split_once(':')
        .map_or((scenario, None), |(n, s)| (n, s.parse::<u64>().ok()));
    let s = match (name, seed) {
        ("panel", Some(seed)) => Scenario::panel(seed),
        ("panel2", Some(seed)) => Scenario::panel2(seed),
        ("mixed_outage_7", Some(seed)) => Scenario::mixed_outage_7(seed),
        (fixture, None) => {
            let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).map_err(|e| e.to_string())?;
            let found = fixtures
                .get(fixture)
                .ok_or_else(|| format!("there is no scenario called {fixture}"))?;
            Scenario::fixture(fixture, found)
        },
        _ => return Err(format!("there is no scenario called {scenario}")),
    }
    .map_err(|e| e.message())?;
    let servers = s.config.servers;
    let factory: Factory =
        Rc::new(move || Engine::new(&s.config_id, s.seed, s.config, &s.script, s.params.as_ref(), Limits::default()));
    Ok((factory, servers))
}

/// The hash that the engine makes of its whole state at slice `slice`, by replaying the run from its first slice.
fn model_checkpoint(factory: &Factory, slice: u64) -> Result<String, String> {
    let mut engine = factory().map_err(|e| e.message())?;
    let mut last = engine.checkpoint(0, 0);
    for _ in 0..slice {
        last = engine
            .step()
            .map_err(|e| e.message())?
            .ok_or("the run is shorter than the committed slice")?
            .checkpoint;
    }
    Ok(last)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn ensure_style() -> Result<(), String> {
    let document = document()?;
    if document.get_element_by_id(STYLE_ID).is_some() {
        return Ok(());
    }
    let style = document.create_element("style").map_err(js("style"))?;
    style.set_id(STYLE_ID);
    style.set_text_content(Some(
        ".perf-service-stage .section { display: none } .perf-service-stage .section.active { display: block }",
    ));
    document.body().ok_or("no body")?.append_child(&style).map_err(js("style"))?;
    Ok(())
}

/// The page that a fixture lives on, which has the gallery's markup and rules: a stage of two panels, of which the
/// first is shown and holds the host. A hidden panel is then hidden as the gallery hides one, by a class, and the host
/// is still in the document.
fn make_page(
    slot: u32,
) -> Result<
    (
        web_sys::Element,
        web_sys::Element,
        web_sys::Element,
        web_sys::Element,
        web_sys::Element,
    ),
    String,
> {
    ensure_style()?;
    let document = document()?;
    let make = |tag: &str, id: &str| -> Result<web_sys::Element, String> {
        let e = document.create_element(tag).map_err(js(tag))?;
        e.set_id(id);
        Ok(e)
    };
    let root = make("div", &format!("perf-service-{slot}"))?;
    let stage = make("div", &format!("perf-service-stage-{slot}"))?;
    stage.set_class_name("perf-service-stage");
    let first = make("div", &format!("perf-service-panel-{slot}-a"))?;
    first.set_class_name("section active");
    let second = make("div", &format!("perf-service-panel-{slot}-b"))?;
    second.set_class_name("section");
    let container = make("div", &format!("perf-service-container-{slot}"))?;
    let status = make("p", &format!("perf-service-status-{slot}"))?;
    let button = make("button", &format!("perf-service-play-{slot}"))?;
    button.set_attribute("aria-pressed", "false").map_err(js("button"))?;
    button.set_text_content(Some("Play"));
    first.append_child(&container).map_err(js("container"))?;
    first.append_child(&status).map_err(js("status"))?;
    first.append_child(&button).map_err(js("button"))?;
    stage.append_child(&first).map_err(js("panel"))?;
    stage.append_child(&second).map_err(js("panel"))?;
    root.append_child(&stage).map_err(js("stage"))?;
    document.body().ok_or("no body")?.append_child(&root).map_err(js("root"))?;
    Ok((root, stage, container, status, button))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Deserialize)]
struct Config {
    scenario: String,
}

/// Builds a controller on a surface on `slot`, at the first slice. A failure leaves nothing built.
#[must_use]
pub fn build(config_json: &str, slot: u32) -> String {
    let _ = teardown(slot);
    let config: Config = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => return error(e),
    };
    match build_fixture(&config.scenario, slot) {
        Ok(fixture) => {
            let summary = summary_of(&fixture);
            FIXTURES_BY_SLOT.with_borrow_mut(|f| f.insert(slot, fixture));
            ok(summary)
        },
        Err(message) => {
            remove_page(slot);
            error(message)
        },
    }
}

fn remove_page(slot: u32) {
    if let Some(root) = document()
        .ok()
        .and_then(|d| d.get_element_by_id(&format!("perf-service-{slot}")))
    {
        root.remove();
    }
    // The rules of the stage are the harness's, and go with the last page, so that the document is as it was before the
    // first.
    if FIXTURES_BY_SLOT.with_borrow(|f| f.is_empty()) {
        if let Some(style) = document().ok().and_then(|d| d.get_element_by_id(STYLE_ID)) {
            style.remove();
        }
    }
}

fn summary_of(fixture: &Fixture) -> serde_json::Value {
    serde_json::json!({
        "scenario": fixture.scenario,
        "elements": fixture.svg.query_selector_all("*").map_or(0, |l| l.length()),
    })
}

fn build_fixture(scenario: &str, slot: u32) -> Result<Fixture, String> {
    let (factory, servers) = factory_for(scenario)?;
    let (_root, stage, container, status, button) = make_page(slot)?;
    let id = host_id(slot);
    let (w, h) = Surface::view_box();
    let svg =
        SvgRoot::create_in(&format!("perf-service-container-{slot}"), Size::new(w, h)).map_err(|e| e.to_string())?;
    svg.root.set_id(&id);
    svg.set_view_box(0.0, 0.0, w, h).map_err(|e| e.to_string())?;
    let host = document()?.get_element_by_id(&id).ok_or("the host was not made")?;
    // The surface is built with the page's own DOM calls, which a sweep may fail, so a failure here is a result and is
    // not caught.
    let surface = Surface::build(&id, servers)?;
    let page_host = PageHost::new(surface, host.clone()).with_voices(
        Some(status.clone()),
        Some(status.clone()),
        Some(button.clone()),
    );
    let controller = Controller::new(page_host, BrowserScheduler::new(), Rc::clone(&factory), View::default())
        .map_err(|e| e.message())?;
    let (hint, lifecycle, left) = (controller.clone(), controller.clone(), controller.clone());
    let gallery = GalleryHint::install(&stage, move || hint.presence_changed())?;
    let lifecycle = PageLifecycle::install(move || lifecycle.presence_changed(), move || left.suspend())?;
    Ok(Fixture {
        controller,
        factory,
        scenario: scenario.to_owned(),
        svg: host,
        container,
        stage,
        status,
        button,
        hook_calls: Rc::new(RefCell::new(0)),
        _gallery: gallery,
        _lifecycle: lifecycle,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn with_fixture<T>(slot: u32, work: impl FnOnce(&mut Fixture) -> Result<T, String>) -> Result<T, String> {
    FIXTURES_BY_SLOT
        .with_borrow_mut(|f| work(f.get_mut(&slot).ok_or_else(|| format!("nothing is built on slot {slot}"))?))
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

#[derive(Deserialize)]
struct Op {
    op: String,
    #[serde(default)]
    target: Option<u64>,
    #[serde(default)]
    ms: Option<u32>,
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    on: Option<bool>,
    #[serde(default)]
    scenario: Option<String>,
}

/// Does one thing that a person, the page or a test can do to the controller, and says what came of it. An outcome that
/// is not `done` is a result and not an error: the sweeps are made of operations that fail.
#[must_use]
pub fn act(slot: u32, op_json: &str) -> String {
    let op: Op = match serde_json::from_str(op_json) {
        Ok(op) => op,
        Err(e) => return error(e),
    };
    with_fixture(slot, |f| {
        let c = &f.controller;
        let outcome = match op.op.as_str() {
            "next" => Some(c.next()),
            "previous" => Some(c.previous()),
            "restart" => Some(c.restart()),
            "seek" => Some(c.seek(op.target.ok_or("a seek needs a target")?)),
            "retry" => Some(c.retry()),
            "play" => Some(c.play()),
            "repair" => Some(c.repair()),
            "select_job" => Some(c.select_job(op.id)),
            "set_speed" => Some(c.set_speed(op.ms.ok_or("a speed needs ms")?)),
            "switch" => {
                let (factory, _) = factory_for(op.scenario.as_deref().ok_or("a switch needs a scenario")?)?;
                let outcome = c.switch_scenario(Rc::clone(&factory)).map_err(|e| e.message())?;
                if outcome == Outcome::Done {
                    f.factory = factory;
                }
                Some(outcome)
            },
            "pause" => {
                c.pause();
                None
            },
            "presence" => {
                c.presence_changed();
                None
            },
            "suspend" => {
                c.suspend();
                None
            },
            "dispose" => {
                c.dispose();
                None
            },
            "keep_cancelled" => {
                c.scheduler().keep_cancelled(op.on.unwrap_or(true));
                None
            },
            other => return Err(format!("there is no operation called {other}")),
        };
        Ok(serde_json::json!({ "outcome": outcome.map(outcome_name) }))
    })
    .map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The logical state and the page of `slot`, and what the page holds that is alive: all that a sweep compares, as one
/// object.
#[must_use]
pub fn checkpoint(slot: u32) -> String {
    with_fixture(slot, |f| {
        let c = &f.controller;
        let committed = c.committed();
        let page = c.with_host(|h| h.surface().snapshot())?;
        let (data, trusted) = c.with_host(|h| (h.surface().data_texts().unwrap_or_default(), h.surface().is_trusted()));
        let slice = c.committed_slice();
        Ok(serde_json::json!({
            "committed_slice": slice,
            "model_sha256": model_checkpoint(&f.factory, slice)?,
            "projection_sha256": committed.sha256(),
            "page_sha256": sha256_hex(page.as_bytes()),
            "data_cells": data,
            "surface_trusted": trusted,
            "dirty": c.is_dirty(),
            "playing": c.is_playing(),
            "epoch": c.epoch(),
            "seek_token": c.seek_token(),
            "generation": c.generation(),
            "selected": c.selected(),
            "last_slice": c.last_slice(),
            "pending_retry": c.pending_retry(),
            "waiting_tasks": c.scheduler().waiting(),
            "live_closures": c.scheduler().live_closures(),
            "controls_stale": c.with_host(|h| h.controls_stale()),
            "control_pressed": f.button.get_attribute("aria-pressed"),
            "control_label": f.button.text_content(),
            "announcement": f.status.text_content(),
            "connected": f.svg.is_connected(),
            "elements": f.svg.query_selector_all("*").map_or(0, |l| l.length()),
        }))
    })
    .map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sets the hook that is called after each field of the overlay and after the data node's setter. With `detach_after`
/// it removes the host from the document after that many calls, as a host that leaves in the middle of a draw does.
/// Without, it counts. With `null`, it is cleared.
#[must_use]
pub fn hook(slot: u32, detach_after: Option<usize>, clear: bool) -> String {
    with_fixture(slot, |f| {
        *f.hook_calls.borrow_mut() = 0;
        let (calls, svg) = (Rc::clone(&f.hook_calls), f.svg.clone());
        let hook: Option<Box<dyn FnMut(service_sim::surface::FaultPoint)>> = if clear {
            None
        } else {
            Some(Box::new(move |_| {
                let mut n = calls.borrow_mut();
                if detach_after == Some(*n) {
                    svg.remove();
                }
                *n += 1;
            }))
        };
        f.controller.with_host(|h| h.surface().set_fault_hook(hook));
        Ok(serde_json::json!({}))
    })
    .map_or_else(error, ok)
}

/// How many times the hook was called since it was set.
#[must_use]
pub fn hook_calls(slot: u32) -> String {
    with_fixture(slot, |f| Ok(serde_json::json!({ "calls": *f.hook_calls.borrow() }))).map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts the host `<svg>` back in its container, where a sweep removed it, and does not tell the controller.
#[must_use]
pub fn attach(slot: u32) -> String {
    with_fixture(slot, |f| {
        f.container.append_child(&f.svg).map_err(js("attach"))?;
        Ok(serde_json::json!({}))
    })
    .map_or_else(error, ok)
}

/// Shows the first panel of the stage and hides the second, or the other way, as the gallery's route does.
#[must_use]
pub fn show_panel(slot: u32, shown: bool) -> String {
    with_fixture(slot, |f| {
        let first = f
            .stage
            .query_selector(".section:first-child")
            .map_err(js("panel"))?
            .ok_or("no first panel")?;
        let second = f
            .stage
            .query_selector(".section:last-child")
            .map_err(js("panel"))?
            .ok_or("no second panel")?;
        let (on, off) = if shown { (&first, &second) } else { (&second, &first) };
        on.class_list().add_1("active").map_err(js("class"))?;
        off.class_list().remove_1("active").map_err(js("class"))?;
        Ok(serde_json::json!({}))
    })
    .map_or_else(error, ok)
}

/// The data node's cells as the page shows them, and the host svg's element count, which a sweep compares before and
/// after.
#[must_use]
pub fn counts(slot: u32) -> String {
    with_fixture(slot, |f| {
        Ok(serde_json::json!({
            "elements": f.svg.query_selector_all("*").map_or(0, |l| l.length()),
            "live_closures": f.controller.scheduler().live_closures(),
            "waiting_tasks": f.controller.scheduler().waiting(),
        }))
    })
    .map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Disposes the controller on `slot`, drops it, and removes its page. Safe to call with nothing built.
#[must_use]
pub fn teardown(slot: u32) -> String {
    let fixture = FIXTURES_BY_SLOT.with_borrow_mut(|f| f.remove(&slot));
    if let Some(f) = fixture {
        f.controller.dispose();
        drop(f);
    }
    remove_page(slot);
    ok(serde_json::json!({}))
}

/// Drops every fixture.
pub fn forget() {
    let slots: Vec<u32> = FIXTURES_BY_SLOT.with_borrow(|f| f.keys().copied().collect());
    for slot in slots {
        let _ = teardown(slot);
    }
    // The rules of the stage are the harness's, and go with the last fixture, so that the document is as it was.
    if let Some(style) = document().ok().and_then(|d| d.get_element_by_id(STYLE_ID)) {
        style.remove();
    }
}
