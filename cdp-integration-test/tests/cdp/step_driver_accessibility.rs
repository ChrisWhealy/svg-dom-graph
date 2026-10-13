//! The off-canvas "Step" driver node of each stepping demo stays out of Chrome's accessibility tree, and the selection
//! toolbar it drives stays in it.
//!
//! Each stepping demo walks its algorithm with a selection toolbar, which needs a data node to step along. The demos
//! give it a row of one numbered cell for each position and place it far off the canvas. Being off the canvas does not
//! take it out of the accessibility tree: without `util::hide_step_driver`, a screen reader would read out every one of
//! those numbers. `hide_step_driver` sets `aria-hidden` on the node, which a DOM check can see, but only Chrome's own
//! computed tree shows whether it took effect. Checked here for every demo that uses a driver, since one helper serves
//! them all.
//!
//! Like [`crate::chacha20_walk`], these run against the built demo gallery's window pages, not the fixture.

use crate::common::new_demo_tab;
use headless_chrome::{
    Element, Tab,
    protocol::cdp::{Accessibility, DOM, types::Method},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The demo window pages with a stepping driver.
const PAGES: [&str; 4] = [
    "sha1-window.html",
    "sha2-256-window.html",
    "sha3-sponge-window.html",
    "chacha20-window.html",
];
const DRIVER: &str = "g[aria-label^=\"Step:\"]";
const VIEWPORT: (u32, u32) = (1800, 1800);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Accessibility.getPartialAXTree` for one node, as a request and a reply this file defines itself.
///
/// The `headless_chrome` bindings have a typed version, but it cannot read a node that Chrome ignores because of
/// `aria-hidden`: the reason `ariaHiddenElement` is not one of the variants it knows, so the whole reply fails to
/// parse. That reason is the very thing this test exists to see, so the reply is kept as raw JSON.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PartialAxTree {
    backend_node_id: DOM::BackendNodeId,
    fetch_relatives: bool,
}

#[derive(Debug, Deserialize)]
struct PartialAxTreeReply {
    nodes: Vec<Value>,
}

impl Method for PartialAxTree {
    const NAME: &'static str = "Accessibility.getPartialAXTree";
    type ReturnObject = PartialAxTreeReply;
}

/// Chrome's accessibility node for `element` as raw JSON, or `None` if its tree holds no node for it at all.
///
/// `fetch_relatives` stays `false`: an ancestor can carry an ignore reason the typed bindings do not know.
fn ax_node(tab: &Tab, element: &Element<'_>) -> Result<Option<Value>, String> {
    let reply = tab
        .call_method(PartialAxTree {
            backend_node_id: element.backend_node_id,
            fetch_relatives: false,
        })
        .map_err(|e| format!("Accessibility.getPartialAXTree failed: {e}"))?;
    Ok(reply.nodes.into_iter().next())
}

/// A string at `node[field]["value"]`, such as an accessibility node's `role` or `name`.
fn ax_string<'a>(node: &'a Value, field: &str) -> Option<&'a str> {
    node.get(field)?.get("value")?.as_str()
}

/// Whether the browser leaves `element` out of the accessibility tree, either by having no node for it or by marking
/// the node ignored.
fn is_hidden(tab: &Tab, element: &Element<'_>) -> Result<bool, String> {
    Ok(ax_node(tab, element)?.is_none_or(|node| node["ignored"].as_bool() == Some(true)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_demos_step_driver_is_hidden_from_the_accessibility_tree_and_its_toolbar_is_not() -> Result<(), String> {
    for page in PAGES {
        let tab = new_demo_tab(page, DRIVER, VIEWPORT)?;
        tab.call_method(Accessibility::Enable(None))
            .map_err(|e| format!("{page}: Accessibility.enable failed: {e}"))?;

        // A nested scene has its own driver, so a page can have several. Each must be hidden, along with a number
        // cell inside it: hiding only the group's own node would not stop a screen reader reaching the cells.
        let drivers = tab
            .find_elements(DRIVER)
            .map_err(|e| format!("{page}: could not find the Step drivers: {e}"))?;
        if drivers.is_empty() {
            return Err(format!("{page}: found no Step driver"));
        }
        for (index, driver) in drivers.iter().enumerate() {
            if !is_hidden(&tab, driver)? {
                return Err(format!("{page}: Step driver {index} is in the accessibility tree"));
            }
        }
        let cells = tab
            .find_elements(&format!("{DRIVER} text"))
            .map_err(|e| format!("{page}: could not find the Step driver's cells: {e}"))?;
        if cells.is_empty() {
            return Err(format!("{page}: the Step driver has no cell text to check"));
        }
        for cell in cells.iter().take(3) {
            if !is_hidden(&tab, cell)? {
                return Err(format!("{page}: a Step driver cell is in the accessibility tree"));
            }
        }

        // The toolbar it drives is still there, as a named button. A nested scene's toolbar can be hidden when its
        // scene is not on show, so it is enough that one of them is exposed.
        let buttons = tab
            .find_elements("g[role=button][aria-label=\"Next selection\"]")
            .map_err(|e| format!("{page}: could not find the Next button: {e}"))?;
        let mut exposed = false;
        for button in &buttons {
            if let Some(node) = ax_node(&tab, button)?
                && node["ignored"].as_bool() != Some(true)
                && ax_string(&node, "role") == Some("button")
                && ax_string(&node, "name") == Some("Next selection")
            {
                exposed = true;
            }
        }
        if !exposed {
            return Err(format!(
                "{page}: no \"Next selection\" button is exposed as a button in the accessibility tree ({} found)",
                buttons.len()
            ));
        }
    }
    Ok(())
}
