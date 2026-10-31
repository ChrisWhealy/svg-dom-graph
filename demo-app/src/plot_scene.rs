//! A scene that holds one plot node, which the demos with plots draw on, and the listener that they attach.

use demo_common::util::{document, ensure_svg_in, stringify};
use svg_dom::{SvgRoot, root::utils::Point};
use svg_dom_graph::{
    NodeId,
    scene::{PlotConfig, PlotItem, Scene},
};
use wasm_bindgen::{JsCast, closure::Closure, convert::FromWasmAbi};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One plot: the scene that holds it and its node, which a new frame draws on.
pub(crate) struct Plot {
    pub(crate) scene: Scene,
    pub(crate) node: NodeId,
}

impl Plot {
    /// Makes a scene in the empty `#{stage}`, and puts a plot node in it that fills the scene.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page has no `#{stage}`, or if the scene or the node could not be made.
    pub(crate) fn new(stage: &str, config: PlotConfig) -> Result<Self, String> {
        let svg_id = format!("{stage}-svg");
        ensure_svg_in(&document()?, stage, &svg_id, config.size)?;
        let scene = Scene::new(SvgRoot::attach(&svg_id).map_err(stringify)?).map_err(stringify)?;
        let node = scene.add_plot_node(Point::new(0.0, 0.0), config).map_err(stringify)?;
        Ok(Self { scene, node })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws `items` in place of what the plot held.
    pub(crate) fn draw(&self, items: &[PlotItem]) -> Result<(), String> {
        self.scene.set_plot_items(self.node, items).map_err(stringify)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Calls `handler` each time `target` gets `event`, which is an event of type `E`. The listener is never dropped:
/// `Closure::forget` leaks it deliberately, for the whole life of the page.
///
/// # Errors
///
/// Returns `Err` if the listener could not be attached.
pub(crate) fn listen<E: FromWasmAbi + 'static>(
    target: &web_sys::EventTarget,
    event: &str,
    handler: impl FnMut(E) + 'static,
) -> Result<(), String> {
    let closure = Closure::<dyn FnMut(E)>::new(handler);
    target
        .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the {event} listener: {e:?}"))?;
    closure.forget();
    Ok(())
}
