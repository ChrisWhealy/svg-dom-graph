//! A scene that is built and in the window.

use spmv_core::{Prepared, Run};
use spmv_scene::scene::{Built, Selection};
use wasm_bindgen::closure::Closure;

/// A scene that is built and in the window.
pub(super) struct Live {
    pub(super) scene_svg: web_sys::Element,
    /// The size of the scene's space, for zooming about its centre.
    pub(super) view_size: (f64, f64),
    pub(super) built: Built,
    pub(super) prepared: Prepared,
    pub(super) run: Run,
    pub(super) verified: bool,
    pub(super) steps: Vec<f64>,
    /// The step of the page's clock in milliseconds, measured once.
    pub(super) clock_step: f64,
    pub(super) selection: Option<Selection>,
    /// The listener that turns a click on a node into a selection. It goes when the scene's `<svg>` does.
    pub(super) click: Option<Closure<dyn FnMut(web_sys::Event)>>,
}
