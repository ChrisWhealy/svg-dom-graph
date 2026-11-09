//! The sparse-matrix demo, in a window of its own:
//!
//! - `panel-sparse-matrix` / `#spmv-open` — [`build_sparse_matrix_demo`]: the gallery's panel, which keeps the description and a button
//!   that opens the demo in a window as large as the screen allows, `sparse-matrix-window.html`.
//! - [`build_sparse_matrix_window`], which that page runs: a sparse matrix times a vector, drawn as a bipartite dependency graph with
//!   two data nodes. The controls and the readout are `<foreignObject>`s in the window's one `<svg>`, and the graph, which pans and
//!   zooms, is between them.
//!
//! A small graph is two single columns whose edges are coloured by the sign of their value and sized by its size, and a click on a
//! node lights its edges and writes out the arithmetic it stands for. The controls also reach past the library's documented resource
//! limits on purpose: a size over the budget is refused with a reason before any `Scene` call is made, and what each build, step, drag
//! and rebuild costs is shown.
//!
//! The pieces are in two crates of their own. `spmv-core` generates the matrix, computes the exact integer product, holds the budget and
//! writes out the arithmetic; `spmv-scene` builds the scene and measures it. This module is the controls and the readout, and `shell`
//! is the `<svg>` that holds them. Their markup is in `controls.html` and `readout.html`, beside it.

mod shell;

use crate::source_file;
use demo_common::util::{clear_error, document, report_error, required_element};
use shell::{SceneSvg, Shell, add_scene_svg, build_host};
use spmv_core::budget::Budget;
use spmv_core::explain;
use spmv_core::spec::{Error, Spec};
use spmv_core::stats::Latency;
use spmv_core::{Prepared, Run, UpdateOrder};
use spmv_scene::layout::Layout;
use spmv_scene::probe::timer_step;
use spmv_scene::scene::{BuildOptions, Built, Selection, element_count, heap_used, now};
use spmv_scene::worked;
use std::cell::RefCell;
use svg_dom_graph::scene::{InputMode, ViewTransform};
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the window's `<svg>`, the matrix and the product, the budget, then the scene that draws them.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("sparse_matrix/shell.rs", "shell.rs"),
    source_file!("spmv-core/src/matrix.rs", "../../../spmv-core/src/matrix.rs"),
    source_file!("spmv-core/src/run.rs", "../../../spmv-core/src/run.rs"),
    source_file!("spmv-core/src/explain.rs", "../../../spmv-core/src/explain.rs"),
    source_file!("spmv-core/src/budget.rs", "../../../spmv-core/src/budget.rs"),
    source_file!("spmv-scene/src/layout/mod.rs", "../../../spmv-scene/src/layout/mod.rs"),
    source_file!("spmv-scene/src/scene.rs", "../../../spmv-scene/src/scene.rs"),
    source_file!("spmv-scene/src/probe.rs", "../../../spmv-scene/src/probe.rs"),
];

/// The page the gallery's button opens, and the window's own name, so that a second click reuses it.
const WINDOW_PAGE: &str = "sparse-matrix-window.html";
const WINDOW_NAME: &str = "svg-dom-graph-sparse-matrix";
const ROOT: &str = "spmv-root";
const ERROR: &str = "spmv-error";
const SEED: u64 = 20_261_109;
/// A step is checked against a full recomputation of the product when the matrix has no more nonzeros than this.
const VERIFY_UP_TO: usize = 20_000;
/// A scene is made readable (its edges signed and its nodes tagged, so that a node can be selected) up to this many nonzeros.
const READABLE_UP_TO: usize = 30_000;
/// A scene says the value on each edge up to this many nonzeros, and shows only a selected node's edges above it.
const LABELS_UP_TO: usize = 60;
/// The nodes of each side that the list of nodes to highlight offers.
const PICKABLE: usize = 100;
/// The steps whose latency the readout summarises.
const WINDOW: usize = 120;

thread_local! {
    static PANEL: RefCell<Panel> = RefCell::new(Panel::default());
    /// How many frame callbacks are alive, which a test reads from the window's `data-live-frame-callbacks` to see that none is left behind.
    static LIVE_CALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// A callback for an animation frame, and the generation that it was asked for under. It counts itself while it lives.
struct FrameCallback {
    generation: u32,
    closure: Closure<dyn FnMut()>,
}

impl FrameCallback {
    fn new(generation: u32) -> Self {
        let closure = Closure::<dyn FnMut()>::new(move || frame(generation));
        Self::count(1);
        Self { generation, closure }
    }

    fn count(change: isize) {
        let live = LIVE_CALLBACKS.with(|c| {
            c.set(c.get().saturating_add_signed(change));
            c.get()
        });
        if let Some(host) = document().ok().and_then(|d| d.get_element_by_id(ROOT)) {
            host.set_attribute("data-live-frame-callbacks", &live.to_string()).ok();
        }
    }
}

impl Drop for FrameCallback {
    fn drop(&mut self) {
        Self::count(-1);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scene that is built and in the window.
struct Live {
    scene_svg: web_sys::Element,
    /// The size of the scene's space, for zooming about its centre.
    view_size: (f64, f64),
    built: Built,
    prepared: Prepared,
    run: Run,
    verified: bool,
    steps: Vec<f64>,
    /// The step of the page's clock in milliseconds, measured once.
    clock_step: f64,
    selection: Option<Selection>,
    /// The listener that turns a click on a node into a selection. It goes when the scene's `<svg>` does.
    click: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Default)]
struct Panel {
    shell: Option<Shell>,
    live: Option<Live>,
    /// Counted up by every build and teardown. A frame that was asked for under an older number does nothing.
    generation: u32,
    running: bool,
    /// The frame that is asked for, and the callback that it will call. The panel owns the callback, so that a frame that is cancelled
    /// and never called does not leave it behind: cancelling drops it.
    frame: Option<i32>,
    callback: Option<FrameCallback>,
    /// The callback that is running, or has run. A callback cannot be dropped while it runs, so it is dropped when the next one starts or
    /// when the run is stopped from outside a frame.
    finished: Option<FrameCallback>,
    in_frame: bool,
    /// Counted up for each `<svg>` made, so that no two have the same id.
    svgs: u32,
    /// When a run next takes a step, which the Run speed list sets, so that a step can be followed.
    next_step_at: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn text(id: &str, value: &str) {
    if let Some(e) = document().ok().and_then(|d| d.get_element_by_id(id)) {
        e.set_text_content(Some(value));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A count with thousands separators.
fn n(v: u64) -> String {
    let digits = v.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn ms(v: f64) -> String {
    if v >= 100.0 {
        format!("{v:.0} ms")
    } else if v >= 1.0 {
        format!("{v:.1} ms")
    } else {
        format!("{:.0} µs", v * 1000.0)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A time as the page's clock can tell it: one that is less than a step of the clock is not told as zero.
fn timed(v: f64, step: f64) -> String {
    if v < step { format!("under {}", ms(step)) } else { ms(v) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn megabytes(v: Option<f64>) -> String {
    v.map_or_else(
        || "not reported by this browser".to_owned(),
        |b| format!("{:.1} MB", b / 1_048_576.0),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn latency(samples: &[f64], step: f64) -> String {
    Latency::from_samples(samples).map_or_else(
        || "—".to_owned(),
        |l| {
            format!(
                "p50 {}, p95 {}, p99 {}, worst {} over {} samples",
                timed(l.p50, step),
                timed(l.p95, step),
                timed(l.p99, step),
                timed(l.max, step),
                l.n
            )
        },
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn selected(id: &str) -> Result<String, String> {
    let element = required_element(&document()?, id)?;
    element
        .dyn_ref::<web_sys::HtmlSelectElement>()
        .map(web_sys::HtmlSelectElement::value)
        .ok_or_else(|| format!("#{id} is not a <select>"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn number(id: &str) -> Result<u64, String> {
    selected(id)?.parse().map_err(|e| format!("#{id}: {e}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scenario that the controls describe.
fn spec_from_controls() -> Result<Spec, String> {
    let entities = number("spmv-size")?;
    let half = (entities / 2).max(1);
    let hub = selected("spmv-adjacency")? == "hub";
    // A data node cannot show more cells than the vector has entries, and a step cannot change more than it shows.
    let cells = number("spmv-cells")?.min(half);
    let updates = number("spmv-updates")?.min(cells);
    Ok(Spec {
        seed: SEED,
        rows: half,
        cols: half,
        per_row: 3,
        hub_cols: if hub { half.min(20) } else { 0 },
        hub_percent: if hub { 60 } else { 0 },
        label_len: number("spmv-labels")?,
        cells,
        updates,
        steps: 0,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn body_elements() -> Result<u32, String> {
    element_count(&document()?.body().ok_or("no body")?.into())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn set_class(id: &str, class: &str) {
    if let Some(e) = document().ok().and_then(|d| d.get_element_by_id(id)) {
        e.set_class_name(class);
    }
}

/// Stops a run in progress, cancelling the frame that is waiting, and makes any frame that already started do nothing.
fn stop_run() {
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.running = false;
        p.generation += 1;
        if let (Some(id), Some(window)) = (p.frame.take(), web_sys::window()) {
            window.cancel_animation_frame(id).ok();
        }
        // A frame that was cancelled is never called, so its callback is dropped here and not by the browser. One that is running cannot
        // be dropped from inside itself, and goes when the next starts.
        p.callback = None;
        if !p.in_frame {
            p.finished = None;
        }
    });
    if let Some(e) = document().ok().and_then(|d| d.get_element_by_id("spmv-run")) {
        e.set_text_content(Some("Run"));
        e.set_attribute("aria-pressed", "false").ok();
    }
}

/// Removes the scene and its `<svg>`, in that order, and its listener with it.
fn teardown() -> Result<(), String> {
    stop_run();
    let live = PANEL.with(|p| p.borrow_mut().live.take());
    let Some(live) = live else { return Ok(()) };
    let Live { scene_svg, built, click, .. } = live;
    drop(built);
    scene_svg.remove();
    drop(click);
    text("spmv-selected", "Click a node, or choose one above, to see what it computes.");
    set_pick_options(0, 0);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the scene is built with, for a scenario of `nonzeros` nonzeros that is to begin at `origin`.
fn options_for(prepared: &Prepared, origin: (f64, f64)) -> BuildOptions {
    let nonzeros = prepared.matrix().nnz();
    BuildOptions {
        readable: nonzeros <= READABLE_UP_TO,
        edge_labels: nonzeros <= LABELS_UP_TO,
        origin,
        worked: is_worked(prepared),
    }
}

/// Whether the multiplication is drawn as the operations that do it, which is for a matrix small enough to be read.
fn is_worked(prepared: &Prepared) -> bool {
    let spec = prepared.spec();
    spec.rows <= worked::MAX_SIDE && spec.cols <= worked::MAX_SIDE
}

/// Whether only the selected node's edges are shown, from the list of edges and the size of the scenario.
fn focus_wanted(nonzeros: usize) -> bool {
    match selected("spmv-edges").as_deref() {
        Ok("focus") => true,
        Ok("all") => false,
        _ => nonzeros > LABELS_UP_TO,
    }
}

/// Fills the list of nodes to highlight with the first of each side.
fn set_pick_options(columns: usize, rows: usize) {
    let Some(list) = document().ok().and_then(|d| d.get_element_by_id("spmv-pick")) else {
        return;
    };
    let mut html = String::from("<option value=\"\" selected>None</option>");
    for j in 0..columns.min(PICKABLE) {
        html.push_str(&format!("<option value=\"x{j}\">x{j}, a column</option>"));
    }
    for i in 0..rows.min(PICKABLE) {
        html.push_str(&format!("<option value=\"y{i}\">y{i}, a row</option>"));
    }
    list.set_inner_html(&html);
}

/// How much one press of Zoom in or Zoom out changes the scale, as the library's own wheel notch does.
const ZOOM_STEP: f64 = 1.25;
/// The scales that the library's view accepts.
const SCALES: (f64, f64) = (0.25, 4.0);

fn make_scene(document: &web_sys::Document, prepared: &Prepared, run: &Run) -> Result<(SceneSvg, Built), String> {
    let shell = PANEL
        .with(|p| p.borrow().shell.clone())
        .ok_or("the window has not been built")?;
    let layout = Layout::new(prepared.spec());
    let extent = if is_worked(prepared) {
        worked::extent(prepared.matrix())
    } else {
        (layout.width, layout.height)
    };
    let id = PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.svgs += 1;
        format!("spmv-svg-{}", p.svgs)
    });
    let svg = add_scene_svg(document, &shell, &id, extent, layout.single || is_worked(prepared))?;
    let made = Built::build_with(&id, prepared, run, options_for(prepared, svg.origin)).and_then(|built| {
        // The library's own toolbar is placed from the rendered size of a nested `<svg>`, which is the size of everything it draws, and
        // not of its room. So there is none, and the window's own buttons zoom. Dragging the background pans, and a wheel turned with
        // Ctrl or Cmd held zooms about the pointer.
        let scene = built.scene();
        scene
            .set_pan_mode(InputMode::On)
            .map_err(|e| format!("could not turn on panning: {e}"))?;
        scene
            .set_wheel_zoom_mode(InputMode::On)
            .map_err(|e| format!("could not turn on zooming by the wheel: {e}"))?;
        Ok(built)
    });
    match made {
        Ok(built) => Ok((svg, built)),
        Err(problem) => {
            svg.element.remove();
            Err(problem)
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the scenario the controls describe, or says why it was refused.
/// What a request to build came to. A request that is refused is not a failure, and it leaves the graph that was there, so a caller that
/// goes on to tear that graph down has to know that nothing new was built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildOutcome {
    Built,
    Refused,
}

fn build() -> Result<BuildOutcome, String> {
    clear_error(ERROR);
    let spec = spec_from_controls()?;
    let budget = Budget::default();
    // The budget is checked, and the matrix generated, before the scene that is there is touched and before a single scene call, so a
    // scenario that is refused leaves the graph that a person has exactly as it was.
    let prepared = match Prepared::new(&spec, &budget) {
        Ok(p) => p,
        Err(Error::Refused(r)) => {
            let had_one = PANEL.with(|p| p.borrow().live.is_some());
            text(
                "spmv-verdict",
                &format!(
                    "Refused before anything was built: {}. {}",
                    r.message,
                    if had_one { "The graph you had is unchanged." } else { "Nothing was built." }
                ),
            );
            set_class("spmv-verdict", "spmv-verdict spmv-refused");
            if !had_one {
                text(
                    "spmv-n-plan",
                    &format!("Refused ({}). No node, edge or data node was made.", r.code),
                );
            }
            return Ok(BuildOutcome::Refused);
        },
        Err(e) => return Err(e.to_string()),
    };
    teardown()?;
    for id in [
        "spmv-n-plan", "spmv-n-page", "spmv-n-build", "spmv-n-step", "spmv-n-check", "spmv-n-drag", "spmv-n-rebuild",
    ] {
        text(id, "—");
    }
    let mut run = prepared.run();
    // A matrix that is worked is for watching, so its steps go in order, and a measured one keeps the contract's random order.
    let order = if is_worked(&prepared) { "inorder" } else { "random" };
    set_list("spmv-order", order);
    set_list("spmv-speed", if is_worked(&prepared) { "1500" } else { "0" });
    run.set_order(order_of(order));
    let document = document()?;
    let before = body_elements()?;
    let heap_before = heap_used();
    let (svg, built) = make_scene(&document, &prepared, &run)?;
    let after = body_elements()?;
    let plan = *prepared.plan();
    let t = built.timings;
    let nonzeros = prepared.matrix().nnz();
    let verified = nonzeros <= VERIFY_UP_TO;
    let clock_step = timer_step();
    if built.is_readable() {
        built.set_focus(focus_wanted(nonzeros))?;
    }

    let click = scene_svg_click(&svg.element)?;
    set_pick_options(
        if is_worked(&prepared) { 0 } else { prepared.matrix().cols },
        prepared.matrix().rows,
    );
    text(
        "spmv-clock",
        &format!(
            "This page's clock steps in {}, so a time shorter than that is told as under one step, and not as zero.",
            ms(clock_step)
        ),
    );
    text(
        "spmv-verdict",
        &format!(
            "Accepted: {} nodes and {} edges, built in {}.",
            n(plan.nodes),
            n(plan.edges),
            ms(t.total_ms)
        ),
    );
    set_class("spmv-verdict", "spmv-verdict");
    text(
        "spmv-task",
        &task(
            prepared.spec().rows,
            prepared.spec().cols,
            prepared.matrix().nnz(),
            is_worked(&prepared),
        ),
    );
    text("spmv-story", story_intro(order_of(order)));
    text(
        "spmv-n-plan",
        &format!(
            "{} nodes, {} edges, {} data cells, the busiest node has {} edges. Estimated {} page elements, of a budget of {}.",
            n(plan.nodes),
            n(plan.edges),
            n(plan.cells),
            n(plan.max_degree),
            n(plan.est_dom),
            n(budget.max_dom_elements)
        ),
    );
    text(
        "spmv-n-page",
        &format!(
            "{} added to the page; the estimate was {}{}.",
            n(u64::from(after - before)),
            n(plan.est_dom),
            if nonzeros <= LABELS_UP_TO {
                ", which leaves out the labels on the edges"
            } else {
                ""
            }
        ),
    );
    text(
        "spmv-n-build",
        &format!(
            "{} in all: nodes {}, edges {}, data nodes {}, pattern square {}, readable styling {}.",
            ms(t.total_ms),
            ms(t.nodes_ms),
            ms(t.edges_ms),
            ms(t.data_ms),
            ms(t.spy_ms),
            ms(t.style_ms)
        ),
    );
    text(
        "spmv-n-heap",
        &format!("{} before, {} after.", megabytes(heap_before), megabytes(heap_used())),
    );
    text(
        "spmv-n-check",
        &if verified {
            "Every step is checked against a full recomputation of the product.".to_owned()
        } else {
            format!(
                "Not checked: over {} nonzeros the check is skipped, so that it does not time the step.",
                n(VERIFY_UP_TO as u64)
            )
        },
    );
    let readable = built.is_readable();
    let first_row = built.worked_row();
    PANEL.with(|p| {
        p.borrow_mut().live = Some(Live {
            scene_svg: svg.element,
            view_size: svg.view_size,
            built,
            prepared,
            run,
            verified,
            steps: Vec::new(),
            clock_step,
            selection: first_row.map(Selection::Row),
            click: Some(click),
        });
    });
    if let Some(row) = first_row {
        choose(Some(Selection::Row(row)))?;
    }
    if !readable {
        text(
            "spmv-selected",
            &format!(
                "Selecting a node is off above {} nonzeros, which the scene does not tag.",
                n(READABLE_UP_TO as u64)
            ),
        );
    }
    Ok(BuildOutcome::Built)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Turns a click on a node of the scene into a selection, and gives the listener, which the scene keeps.
fn scene_svg_click(scene_svg: &web_sys::Element) -> Result<Closure<dyn FnMut(web_sys::Event)>, String> {
    let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        let tag = event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|e| e.closest("[data-spmv-node]").ok().flatten())
            .and_then(|e| e.get_attribute("data-spmv-node"));
        if let Some(selection) = tag.as_deref().and_then(Selection::parse) {
            if let Err(message) = choose(Some(selection)) {
                report_error(ERROR, &message);
            }
        }
    });
    scene_svg
        .add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
        .map_err(|e| format!("could not listen for a click on a node: {e:?}"))?;
    Ok(listener)
}

/// What the selected node computes, written out from the matrix and the vector as they are now.
fn describe(live: &Live) -> String {
    let (matrix, x) = (live.prepared.matrix(), &live.run.x);
    match live.selection {
        Some(Selection::Row(i)) if i < matrix.rows => format!(
            "y{i} is made from row {i} of A: each number in the row is multiplied by the x of its column, and the products are \
             added.\n{}",
            explain::row(matrix, x, i)
        ),
        Some(Selection::Column(j)) if j < matrix.cols => explain::column_story(matrix, x, j),
        _ => "Click a node, or choose one above, to see what it computes.".to_owned(),
    }
}

/// Selects a node, or none: its edges are lit, and the arithmetic it stands for is written out.
fn choose(selection: Option<Selection>) -> Result<(), String> {
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        let live = p.live.as_mut().ok_or("Nothing is built yet.")?;
        if !live.built.is_readable() {
            return Ok(());
        }
        if live.built.worked_row().is_some() {
            // The multiplication shows one row, and a click on anything else leaves it as it is.
            if let Some(Selection::Row(i)) = selection {
                live.built.show_row(&live.prepared, &live.run, i)?;
                live.selection = selection;
            }
        } else {
            live.built.select(selection, live.prepared.matrix())?;
            live.selection = selection;
        }
        let selection = live.selection;
        text("spmv-selected", &describe(live));
        if let Some(list) = document()?.get_element_by_id("spmv-pick") {
            if let Some(list) = list.dyn_ref::<web_sys::HtmlSelectElement>() {
                let tag = selection.map(Selection::tag).unwrap_or_default();
                // A node beyond the first of each side is not in the list, and then the list says none.
                list.set_value(&tag);
                if list.value() != tag {
                    list.set_value("");
                }
            }
        }
        Ok(())
    })
}

/// Selects the node that a person typed, which can be any of them, and not only the first 100 that the list holds, so that a node
/// beyond those is reached from the keyboard too.
fn goto_changed() -> Result<(), String> {
    let input = document()?
        .get_element_by_id("spmv-goto")
        .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
        .ok_or("the box for a node is missing")?;
    let tag = input.value().trim().to_lowercase();
    if tag.is_empty() {
        return Ok(());
    }
    let selection =
        Selection::parse(&tag).ok_or_else(|| "Type a node as x or y and its number, such as x250 or y7.".to_owned())?;
    let (cols, rows, worked, readable) = PANEL
        .with(|p| {
            let p = p.borrow();
            p.live.as_ref().map(|l| {
                let m = l.prepared.matrix();
                (m.cols, m.rows, l.built.worked_row().is_some(), l.built.is_readable())
            })
        })
        .ok_or("Nothing is built yet.")?;
    let last = |n: usize| n.saturating_sub(1);
    match selection {
        Selection::Column(j) if j >= cols => {
            return Err(format!("There is no {tag}: the columns are x0 to x{}.", last(cols)));
        },
        Selection::Row(i) if i >= rows => {
            return Err(format!("There is no {tag}: the rows are y0 to y{}.", last(rows)));
        },
        Selection::Column(_) if worked => {
            return Err(format!(
                "This view shows one row at a time, so choose a row: y0 to y{}.",
                last(rows)
            ));
        },
        _ if !readable => {
            return Err(format!(
                "Selecting a node is off above {} nonzeros, which the scene does not tag.",
                n(READABLE_UP_TO as u64)
            ));
        },
        _ => {},
    }
    choose(Some(selection))
}

fn pick_changed() -> Result<(), String> {
    let tag = selected("spmv-pick")?;
    choose(Selection::parse(&tag))
}

fn edges_changed() -> Result<(), String> {
    PANEL.with(|p| {
        let p = p.borrow();
        let live = p.live.as_ref().ok_or("Nothing is built yet.")?;
        if live.built.is_readable() {
            live.built.set_focus(focus_wanted(live.prepared.matrix().nnz()))?;
        }
        Ok(())
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One step: the updates, the product kept up to date, and the two data nodes written.
fn step() -> Result<(), String> {
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        let live = p.live.as_mut().ok_or("Nothing is built yet.")?;
        if injected("__spmvFailNextStep") {
            return Err("A failure was injected into this step.".to_owned());
        }
        let start = now();
        let before = live.built.is_readable().then(|| (live.run.x.clone(), live.run.y.clone()));
        live.run.advance(live.verified).map_err(|e| e.to_string())?;
        live.built.apply(&live.run)?;
        if let Some((x, y)) = before {
            let mut said = story(live.run.step, &x, &y, &live.run.x, &live.run.y);
            if selected("spmv-order").as_deref() == Ok("inorder") {
                let window = (live.prepared.spec().cells as usize).min(live.run.x.len()).max(1);
                if let Some((last, _)) = live.run.last_updates.last() {
                    said.push_str(&format!(
                        " Inputs change in order, so the next step changes x{}.",
                        (last + 1) % window
                    ));
                }
            }
            text("spmv-story", &said);
        }
        let took = now() - start;
        if live.steps.len() == WINDOW {
            live.steps.remove(0);
        }
        live.steps.push(took);
        text(
            "spmv-n-step",
            &format!("Step {}: {}", live.run.step, latency(&live.steps, live.clock_step)),
        );
        // The vector has changed, so what a selected node computes has too.
        if live.selection.is_some() {
            text("spmv-selected", &describe(live));
        }
        Ok(())
    })
}

/// The task, in plain words, for the graph that was built.
fn task(rows: u64, cols: u64, nnz: usize, worked: bool) -> String {
    let what = "What this shows: a computer often has to multiply a table of numbers (the matrix A) by a list of numbers (x) to make a new \
                list (y). Search, graphics and machine learning all do it.";
    let step = "What a step does: A never changes. A step changes one number of x, and only the y that use that x are worked out again, \
                and they are outlined in green. Run keeps stepping, at the speed you choose.";
    if worked {
        format!(
            "{what}\nHow to read it: each y is made from one row of A. Multiply every number in the row by the x in its column, then add \
             the products. The boxes at the right do exactly that for the selected row, and the bars draw y. Click another row to see it.\n\
             {step} Steps go in order, x0 then x1 and so on, so you can tell what comes next. The Step order list can make it random.\n\
             Why it matters: most big tables are mostly zeros, so a computer keeps only the numbers that are not zero and skips the rest. \
             Choose a larger size to see how much a page can draw before it slows."
        )
    } else {
        format!(
            "{what}\nHow to read it: this matrix has {rows} rows, {cols} columns and {} numbers that are not zero. Each is a line from an \
             x on the left to a y on the right, and a y is the sum of each line's number times the x it comes from. Click a node to see \
             its sum.\n{step} The Step order list chooses in order or at random.\nWhy it matters: this size tests how big a drawing the \
             page can hold. The larger the matrix, the more lines the page must draw, and the readout under Measurements says what that \
             cost.",
            n(nnz as u64)
        )
    }
}

/// What the story line says before a step: how the steps will go.
fn story_intro(order: UpdateOrder) -> &'static str {
    if order == UpdateOrder::InOrder {
        "Press Step to change the next entry of x, starting with x0, and see which y change. Steps go in order: x0, x1, x2 and so on round."
    } else {
        "Press Step to change an entry of x chosen at random, and see which y change. The choice comes from a seeded random stream, so it has no pattern."
    }
}

fn order_of(value: &str) -> UpdateOrder {
    if value == "inorder" { UpdateOrder::InOrder } else { UpdateOrder::Random }
}

/// Sets what a list shows, if it has such a value.
fn set_list(id: &str, value: &str) {
    if let Some(list) = document()
        .ok()
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlSelectElement>().ok())
    {
        list.set_value(value);
    }
}

fn order_changed() -> Result<(), String> {
    let order = order_of(&selected("spmv-order")?);
    PANEL.with(|p| {
        if let Some(live) = p.borrow_mut().live.as_mut() {
            live.run.set_order(order);
        }
    });
    text("spmv-story", story_intro(order));
    Ok(())
}

/// Says in plain words what a step did: which inputs changed, and so which outputs.
fn story(step: u64, x0: &[i64], y0: &[i64], x1: &[i64], y1: &[i64]) -> String {
    let list = |before: &[i64], after: &[i64], name: &str, limit: usize| -> String {
        let changed: Vec<String> = (0..after.len().min(before.len()))
            .filter(|n| before[*n] != after[*n])
            .map(|n| format!("{name}{n} {} → {}", before[n], after[n]))
            .collect();
        let more = changed.len().saturating_sub(limit);
        let shown = changed.into_iter().take(limit).collect::<Vec<_>>().join(", ");
        if more > 0 { format!("{shown} and {more} more") } else { shown }
    };
    let (xs, ys) = (list(x0, x1, "x", 3), list(y0, y1, "y", 4));
    if xs.is_empty() {
        return format!("Step {step}: no input changed, so no output did.");
    }
    let outputs = if ys.is_empty() {
        "None of the outputs changed value.".to_owned()
    } else {
        format!("Outputs that use it changed: {ys}.")
    };
    format!("Step {step}: the input changed ({xs}). {outputs} A never changes, only x does.")
}

fn frame(generation: u32) {
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.in_frame = true;
        p.frame = None;
        // This is the callback that is being called. The one before it has finished, and goes.
        if p.callback.as_ref().is_some_and(|c| c.generation == generation) {
            p.finished = p.callback.take();
        }
    });
    frame_body(generation);
    let stopped = PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.in_frame = false;
        !p.running
    });
    if stopped {
        release_when_unwound();
    }
}

/// Lets go of the callback that has just run, after the browser's call of it has returned. A run that stops inside its own frame, for a
/// failed step or a failed request for the next frame or a hidden page, has no later frame to release it, and it cannot be dropped while
/// it is running. A microtask runs when the call has returned, and before anything else does.
fn release_when_unwound() {
    let release = Closure::once_into_js(|| {
        PANEL.with(|p| {
            let mut p = p.borrow_mut();
            if !p.in_frame && !p.running {
                p.finished = None;
            }
        });
    });
    if let Some(window) = web_sys::window() {
        window.queue_microtask(release.unchecked_ref());
    }
}

/// Whether a test has asked for a failure at `name`, which it does by setting that property of the window. It is read once and cleared, so
/// that a failure is injected once. Nothing sets one but a test.
fn injected(name: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let asked = js_sys::Reflect::get(&window, &name.into()).is_ok_and(|v| v.is_truthy());
    if asked {
        js_sys::Reflect::set(&window, &name.into(), &wasm_bindgen::JsValue::FALSE).ok();
    }
    asked
}

fn frame_body(generation: u32) {
    let current = PANEL.with(|p| {
        let p = p.borrow();
        p.running && p.generation == generation
    });
    if !current {
        return;
    }
    // A window that is hidden has no use for a step, and a run stops itself.
    if document().map_or(true, |d| d.hidden()) {
        stop_run();
        return;
    }
    let paced = PANEL.with(|p| {
        let p = p.borrow();
        now() < p.next_step_at
    });
    if paced {
        // Not yet time for the next step, so that a person can follow the last one.
        if let Err(message) = schedule(generation) {
            stop_run();
            report_error(ERROR, &message);
        }
        return;
    }
    let pace: f64 = selected("spmv-speed").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    PANEL.with(|p| p.borrow_mut().next_step_at = now() + pace);
    if let Err(message) = step() {
        stop_run();
        report_error(ERROR, &message);
        return;
    }
    if let Err(message) = schedule(generation) {
        stop_run();
        report_error(ERROR, &message);
    }
}

fn schedule(generation: u32) -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;
    if injected("__spmvFailNextSchedule") {
        return Err("A failure was injected into asking for the next frame.".to_owned());
    }
    let callback = FrameCallback::new(generation);
    let id = window
        .request_animation_frame(callback.closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not ask for a frame: {e:?}"))?;
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.frame = Some(id);
        p.callback = Some(callback);
    });
    Ok(())
}

fn toggle_run() -> Result<(), String> {
    let running = PANEL.with(|p| p.borrow().running);
    if running {
        stop_run();
        return Ok(());
    }
    if PANEL.with(|p| p.borrow().live.is_none()) {
        return Err("Nothing is built yet.".to_owned());
    }
    let generation = PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.generation += 1;
        p.running = true;
        p.next_step_at = 0.0;
        p.generation
    });
    if let Some(e) = document()?.get_element_by_id("spmv-run") {
        e.set_text_content(Some("Stop"));
        e.set_attribute("aria-pressed", "true").ok();
    }
    schedule(generation)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the node with the most edges back and forth, which redraws every connector on it each time.
fn drag() -> Result<(), String> {
    PANEL.with(|p| {
        let p = p.borrow();
        let live = p.live.as_ref().ok_or("Nothing is built yet.")?;
        if live.built.worked_row().is_some() {
            text(
                "spmv-n-drag",
                "Dragging a column is for the graph view of a larger matrix. Here you can drag any node yourself.",
            );
            return Ok(());
        }
        let matrix = live.prepared.matrix();
        let busiest = (0..matrix.cols).max_by_key(|j| matrix.col(*j).count()).unwrap_or(0);
        let degree = matrix.col(busiest).count();
        // The moves are timed together and divided, since one move can be shorter than a step of the page's clock.
        let moves = 30_u32;
        let start = now();
        for k in 0..moves {
            let d = if k % 2 == 0 { 7.0 } else { -7.0 };
            live.built.move_col(busiest, d, 3.0 * d)?;
        }
        let each = (now() - start) / f64::from(moves);
        text(
            "spmv-n-drag",
            &format!(
                "Column x{busiest} has {} edges: {moves} moves took {} each. A frame is 16.7 ms. You can also drag any node yourself.",
                n(degree as u64),
                timed(each, live.clock_step / f64::from(moves))
            ),
        );
        Ok(())
    })
}

/// Builds the scenario and tears it down ten times, and says whether the page ended where it began.
fn rebuild() -> Result<(), String> {
    // A request that is refused builds nothing and leaves the graph that is there, and there is nothing here to tear down and build again.
    if build()? == BuildOutcome::Refused {
        return Ok(());
    }
    let Some(prepared) = PANEL.with(|p| p.borrow().live.as_ref().map(|l| l.prepared.clone())) else {
        return Ok(());
    };
    teardown()?;
    let document = document()?;
    let before = body_elements()?;
    let heap_before = heap_used();
    let mut builds = Vec::new();
    for _ in 0..10 {
        let (svg, built) = make_scene(&document, &prepared, &prepared.run())?;
        builds.push(built.timings.total_ms);
        drop(built);
        svg.element.remove();
    }
    let after = body_elements()?;
    let message = format!(
        "10 builds and teardowns: the page had {} elements before and {} after{}; heap {} before, {} after (the collector decides when it falls). A build took {}.",
        n(u64::from(before)),
        n(u64::from(after)),
        if before == after {
            ", so nothing was left behind"
        } else {
            ", so something was left behind"
        },
        megabytes(heap_before),
        megabytes(heap_used()),
        latency(&builds, timer_step())
    );
    // The last build clears the readouts, so the result is written after it.
    build()?;
    text("spmv-n-rebuild", &message);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zooms by `factor` about the centre of the scene's room, so that what is in the middle stays there.
fn zoom(factor: f64) -> Result<(), String> {
    PANEL.with(|p| {
        let p = p.borrow();
        let live = p.live.as_ref().ok_or("Nothing is built yet.")?;
        let view = live.built.scene().view();
        let scale = (view.scale * factor).clamp(SCALES.0, SCALES.1);
        let ratio = scale / view.scale;
        let (cx, cy) = (live.view_size.0 / 2.0, live.view_size.1 / 2.0);
        live.built
            .scene()
            .set_view(ViewTransform {
                scale,
                tx: cx - ratio * (cx - view.tx),
                ty: cy - ratio * (cy - view.ty),
            })
            .map_err(|e| format!("could not zoom: {e}"))
    })
}

fn zoom_in() -> Result<(), String> {
    zoom(ZOOM_STEP)
}

fn zoom_out() -> Result<(), String> {
    zoom(1.0 / ZOOM_STEP)
}

/// Puts the view back to the whole picture, as it was built.
fn reset_view() -> Result<(), String> {
    PANEL.with(|p| {
        let p = p.borrow();
        let live = p.live.as_ref().ok_or("Nothing is built yet.")?;
        live.built
            .scene()
            .set_view(ViewTransform { scale: 1.0, tx: 0.0, ty: 0.0 })
            .map_err(|e| format!("could not reset the view: {e}"))
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn on(id: &str, event: &str, action: fn() -> Result<(), String>) -> Result<(), String> {
    let target = required_element(&document()?, id)?;
    let callback = Closure::<dyn FnMut()>::new(move || match action() {
        Ok(()) => clear_error(ERROR),
        Err(message) => report_error(ERROR, &message),
    });
    target
        .add_event_listener_with_callback(event, callback.as_ref().unchecked_ref())
        .map_err(|e| format!("could not wire #{id}: {e:?}"))?;
    callback.forget();
    Ok(())
}

/// As [`on`], for a box that a person types in. What they typed may not be a node that is there, which is the person's to correct and
/// not a fault of the page, so it is told in the banner and is not logged as an error.
fn on_typed(id: &str, event: &str, action: fn() -> Result<(), String>) -> Result<(), String> {
    let target = required_element(&document()?, id)?;
    let callback = Closure::<dyn FnMut()>::new(move || match action() {
        Ok(()) => clear_error(ERROR),
        Err(message) => {
            if let Some(banner) = document().ok().and_then(|d| d.get_element_by_id(ERROR)) {
                banner.set_text_content(Some(&message));
                banner.remove_attribute("hidden").ok();
            }
        },
    });
    target
        .add_event_listener_with_callback(event, callback.as_ref().unchecked_ref())
        .map_err(|e| format!("could not wire #{id}: {e:?}"))?;
    callback.forget();
    Ok(())
}

/// The gallery's panel: the description is in the page, and this wires the button that opens the demo in a window of its own, as
/// large as the screen allows.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#spmv-open`, or if a listener could not be attached to it.
pub(crate) fn build_sparse_matrix_demo() -> Result<(), String> {
    let button = required_element(&document()?, "spmv-open")?;
    let open = Closure::<dyn FnMut()>::new(move || {
        let Some(window) = web_sys::window() else { return };
        let (width, height) = window
            .screen()
            .ok()
            .map(|s| (s.avail_width().unwrap_or(1600), s.avail_height().unwrap_or(900)))
            .unwrap_or((1600, 900));
        let features = format!("popup,left=0,top=0,width={width},height={height}");
        // A blocked pop-up is the user's own choice, and there is nowhere to report it to. It is ignored.
        if let Ok(Some(popup)) = window.open_with_url_and_target_and_features(WINDOW_PAGE, WINDOW_NAME, &features) {
            let _ = popup.focus();
        }
    });
    button
        .add_event_listener_with_callback("click", open.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the open-window listener to #spmv-open: {e:?}"))?;
    open.forget();
    Ok(())
}

/// Builds the demo in its window: the `<svg>` with the controls and the readout, the wiring of every control, and the first, small
/// scenario.
///
/// # Errors
///
/// Returns `Err` if the window is missing its root, or the first scenario could not be built.
pub(crate) fn build_sparse_matrix_window() -> Result<(), String> {
    let shell = build_host(&document()?, ROOT)?;
    PANEL.with(|p| p.borrow_mut().shell = Some(shell));
    on("spmv-build", "click", || build().map(|_| ()))?;
    on("spmv-step", "click", step)?;
    on("spmv-run", "click", toggle_run)?;
    on("spmv-drag", "click", drag)?;
    on("spmv-rebuild", "click", rebuild)?;
    on("spmv-teardown", "click", || {
        teardown()?;
        let elements = body_elements()?;
        text(
            "spmv-verdict",
            &format!("Torn down. The page has {} elements.", n(u64::from(elements))),
        );
        Ok(())
    })?;
    on("spmv-zoom-in", "click", zoom_in)?;
    on("spmv-zoom-out", "click", zoom_out)?;
    on("spmv-zoom-reset", "click", reset_view)?;
    on("spmv-pick", "change", pick_changed)?;
    on("spmv-edges", "change", edges_changed)?;
    on("spmv-order", "change", order_changed)?;
    on_typed("spmv-goto", "change", goto_changed)?;
    // A run stops when the window goes, and nothing is stepped after it.
    let leaving = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
        let _ = teardown();
    });
    web_sys::window()
        .ok_or("no window")?
        .add_event_listener_with_callback("pagehide", leaving.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the pagehide listener: {e:?}"))?;
    leaving.forget();
    // A browser stops the animation frames of a hidden page, so a frame cannot be the one to notice that the page is hidden: the run
    // would be left running, and would go on when the page came back. The page says when it is hidden, and the run stops then.
    let hiding = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
        if document().is_ok_and(|d| d.hidden()) {
            stop_run();
        }
    });
    document()?
        .add_event_listener_with_callback("visibilitychange", hiding.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the visibilitychange listener: {e:?}"))?;
    hiding.forget();
    build().map(|_| ())
}
