//! `make_draggable`'s `pointerdown` and `pointermove` handlers must call `prevent_default()`, so a mouse drag across a
//! node's label does not fall through to the browser's native text-selection gesture.
//!
//! This is checked by reading back `Event::defaultPrevented`, not by checking `window.getSelection()` after the drag. A
//! first attempt did check the selection directly, driven by real, CDP-dispatched mouse input — the same technique used
//! by `small_drag` and `overlap_resolution`. However, a control experiment (defined as a bare `<rect>` painted
//! immediately before a `<text>` in one `<g>`, with no listeners and no `svg-dom-graph` involved) showed headless
//! Chrome never starts a text selection over that exact shape to begin with.
//!
//! `svg-dom-graph`'s own box+label markup is exactly that shape, so the selection outcome can't distinguish a working
//! `prevent_default()` from a missing one here since it would pass either way. Reading `defaultPrevented` instead
//! checks the mechanism `svg-dom-graph`'s code is actually responsible for, independent of that browser/headless-mode
//! quirk.

use crate::common::{drag, new_tab};
use std::time::Duration;

#[test]
fn dragging_a_node_prevents_the_pointerdowns_default_action() -> Result<(), String> {
    let tab = new_tab()?;

    // Added after `make_draggable`'s own pointerdown listener already exists on this same `<g>` (asserted by
    // `new_tab` waiting for the fixture's last node before returning), so it runs after it in bubble-phase
    // registration order — by the time it runs, `defaultPrevented` reflects whatever `svg-dom-graph`'s own
    // handler already did to this event.
    tab.evaluate(
        "window.__defaultPrevented = null; \
         document.querySelector('#diagram > g:nth-of-type(1)') \
             .addEventListener('pointerdown', (e) => { window.__defaultPrevented = e.defaultPrevented; });",
        false,
    )
    .map_err(|e| format!("could not install the pointerdown probe listener: {e}"))?;

    let rect = tab
        .find_element("#diagram > g:nth-of-type(1) rect")
        .map_err(|e| format!("could not find solo's <rect>: {e}"))?;
    let midpoint = rect.get_midpoint().map_err(|e| format!("could not get solo's midpoint: {e}"))?;

    drag(
        &tab,
        &[
            (midpoint.x, midpoint.y),
            (midpoint.x + 20.0, midpoint.y + 15.0),
            (midpoint.x + 40.0, midpoint.y + 30.0),
        ],
    )?;

    std::thread::sleep(Duration::from_millis(100));

    let default_prevented = tab
        .evaluate("window.__defaultPrevented", false)
        .map_err(|e| format!("could not read window.__defaultPrevented: {e}"))?
        .value
        .and_then(|value| value.as_bool());

    if default_prevented != Some(true) {
        return Err(format!(
            "expected solo's pointerdown to have defaultPrevented == true, got {default_prevented:?}"
        ));
    }

    Ok(())
}
