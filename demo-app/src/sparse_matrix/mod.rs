//! The sparse-matrix demo, in a window of its own:
//!
//! - `panel-sparse-matrix` / `#spmv-open` — [`build_sparse_matrix_demo`]: the gallery's panel, which keeps the
//!   description and a button that opens the demo in a window as large as the screen allows,
//!   `sparse-matrix-window.html`.
//! - [`build_sparse_matrix_window`], which that page runs: a sparse matrix times a vector, drawn as a bipartite
//!   dependency graph with two data nodes. The controls and the readout are `<foreignObject>`s in the window's one
//!   `<svg>`, and the graph, which pans and zooms, is between them.
//!
//! A small graph is two single columns whose edges are coloured by the sign of their value and sized by its size, and a
//! click on a node lights its edges and writes out the arithmetic it stands for. The controls also reach past the
//! library's documented resource limits on purpose: a size over the budget is refused with a reason before any `Scene`
//! call is made, and what each build, step, drag and rebuild costs is shown.
//!
//! The pieces are in two crates of their own. `spmv-core` generates the matrix, computes the exact integer product,
//! holds the budget and writes out the arithmetic; `spmv-scene` builds the scene and measures it. This module is the
//! controls and the readout, and `shell` is the `<svg>` that holds them. Their markup is in `controls.html` and
//! `readout.html`, beside it.

mod build;
mod controls;
mod frame_callback;
mod frames;
mod interaction;
mod live;
mod panel;
mod readout;

mod shell;

use crate::source_file;
use build::{build, teardown};
use controls::{body_elements, on, on_typed};
use demo_common::util::{document, required_element};
use frames::{stop_run, toggle_run};
use interaction::{
    drag, edges_changed, goto_changed, order_changed, pick_changed, rebuild, reset_view, step, zoom_in, zoom_out,
};
use panel::Panel;
use readout::{n, text};
use shell::build_host;
use std::cell::RefCell;
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the window's `<svg>`, the matrix and the product, the budget, then the scene that draws
/// them.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("sparse_matrix/shell.rs", "shell.rs"),
    source_file!("sparse_matrix/build.rs", "build.rs"),
    source_file!("sparse_matrix/controls.rs", "controls.rs"),
    source_file!("sparse_matrix/frame_callback.rs", "frame_callback.rs"),
    source_file!("sparse_matrix/frames.rs", "frames.rs"),
    source_file!("sparse_matrix/interaction.rs", "interaction.rs"),
    source_file!("sparse_matrix/live.rs", "live.rs"),
    source_file!("sparse_matrix/panel.rs", "panel.rs"),
    source_file!("sparse_matrix/readout.rs", "readout.rs"),
    source_file!("spmv-core/src/matrix.rs", "../../../spmv-core/src/matrix.rs"),
    source_file!("spmv-core/src/run.rs", "../../../spmv-core/src/run.rs"),
    source_file!("spmv-core/src/explain.rs", "../../../spmv-core/src/explain.rs"),
    source_file!("spmv-core/src/budget.rs", "../../../spmv-core/src/budget.rs"),
    source_file!("spmv-scene/src/layout/mod.rs", "../../../spmv-scene/src/layout/mod.rs"),
    source_file!("spmv-scene/src/scene/mod.rs", "../../../spmv-scene/src/scene/mod.rs"),
    source_file!("spmv-scene/src/scene/built.rs", "../../../spmv-scene/src/scene/built.rs"),
    source_file!("spmv-scene/src/scene/host.rs", "../../../spmv-scene/src/scene/host.rs"),
    source_file!(
        "spmv-scene/src/scene/selection.rs",
        "../../../spmv-scene/src/scene/selection.rs"
    ),
    source_file!(
        "spmv-scene/src/scene/build_options.rs",
        "../../../spmv-scene/src/scene/build_options.rs"
    ),
    source_file!(
        "spmv-scene/src/scene/build_timings.rs",
        "../../../spmv-scene/src/scene/build_timings.rs"
    ),
    source_file!("spmv-scene/src/scene/marks.rs", "../../../spmv-scene/src/scene/marks.rs"),
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
/// A scene is made readable (its edges signed and its nodes tagged, so that a node can be selected) up to this many
/// nonzeros.
const READABLE_UP_TO: usize = 30_000;
/// A scene says the value on each edge up to this many nonzeros, and shows only a selected node's edges above it.
const LABELS_UP_TO: usize = 60;
/// The nodes of each side that the list of nodes to highlight offers.
const PICKABLE: usize = 100;
/// The steps whose latency the readout summarises.
const WINDOW: usize = 120;

thread_local! {
    static PANEL: RefCell<Panel> = RefCell::new(Panel::default());
    /// How many frame callbacks are alive, which a test reads from the window's `data-live-frame-callbacks` to see that
    /// none is left behind.
    static LIVE_CALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How much one press of Zoom in or Zoom out changes the scale, as the library's own wheel notch does.
const ZOOM_STEP: f64 = 1.25;
/// The scales that the library's view accepts.
const SCALES: (f64, f64) = (0.25, 4.0);

/// The gallery's panel: the description is in the page, and this wires the button that opens the demo in a window of
/// its own, as large as the screen allows.
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

/// Builds the demo in its window: the `<svg>` with the controls and the readout, the wiring of every control, and the
/// first, small scenario.
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
    // A browser stops the animation frames of a hidden page, so a frame cannot be the one to notice that the page is
    // hidden: the run would be left running, and would go on when the page came back. The page says when it is hidden,
    // and the run stops then.
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
