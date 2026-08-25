//! WASM fixture for `cdp-integration-test`'s Chrome-DevTools-Protocol integration tests.
//!
//! Builds a small `Scene` with known, fixed node positions, so the driving CDP tests can compute expected drag results
//! independently and compare them against real, browser-driven mouse input. This is something `wasm-bindgen-test`'s
//! synthetic `dispatchEvent` calls cannot exercise, since they go straight to a target element rather than through the
//! browser's own hit-testing, pointer capture, and default-action machinery.
//!
//! `index.html`'s `<svg id="diagram">` uses a `viewBox` matching its pixel size 1:1, so a CSS-pixel mouse delta is the
//! same size in this scene's user-space units. This removes the need for the tests to care about scaling.
//!
//! Nodes, in add order (`#diagram > g:nth-of-type(N)`):
//!
//! 1. `solo` — draggable, far from every other node. Used to prove an ordinary drag with no overlap involved.
//! 2. `blocker` — not draggable, fixed in place. The node `mover` is dragged onto.
//! 3. `mover` — draggable, starts far from `blocker`. Used to prove overlap resolution on drop.
//!
//! `Scene` is a cheap handle around an `Rc`-shared state, and its own listener closures deliberately hold only `Weak`
//! references back to it (so a dropped `Scene` cannot leak the whole page's DOM forever). That means a `Scene` built
//! and then simply let go out of scope, which is the natural shape of a `#[wasm_bindgen(start)]` function, drops before
//! the user ever gets a chance to click anything. This in turn, silently kills every listener with no panic and no
//! console output.
//!
//! `SCENE` below keeps this fixture's only `Scene` handle alive for the page's whole lifetime, the same pattern
//! `demo-app` now uses.

use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{Error, scene::Scene};
use wasm_bindgen::prelude::*;

thread_local! {
    // Keeps this fixture's `Scene` alive for as long as the page lives — see the module doc comment above.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    build().map_err(|e| JsValue::from_str(&e.to_string()))
}

fn build() -> Result<(), Error> {
    let svg = SvgRoot::attach("diagram")?;
    let scene = Scene::new(svg)?;
    let box_size = Size::new(80.0, 40.0);

    // Isolated: nothing else is anywhere near it, at any point in either test.
    let solo = scene.add_node(Point::new(20.0, 20.0), box_size, "solo")?;
    scene.make_draggable(solo)?;

    // Not draggable — `mover` is dragged onto this one.
    scene.add_node(Point::new(300.0, 150.0), box_size, "blocker")?;

    let mover = scene.add_node(Point::new(20.0, 150.0), box_size, "mover")?;
    scene.make_draggable(mover)?;

    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
