//! `Accessibility.getPartialAXTree`/`getChildAXNodes` regression coverage — what the browser's own computed
//! accessibility tree actually exposes, not just what `wasm-pack test`'s DOM-attribute checks can see.
//! `tests/drag/data_node.rs` already proves the right `role`/`aria-label` attributes land on the rendered DOM. This
//! proves Chrome's own accessibility-tree computation actually turns them into an exposed `role`/`name`. It also proves
//! the descendant value text survives alongside it, rather than being swallowed into one atomic image.

use crate::common::new_tab;
use headless_chrome::{
    Tab,
    protocol::cdp::Accessibility::{self, AXNodeId},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An `AXValue`'s own string content, if it has one — `role`/`name` are always this shape in practice, whatever
/// `AXValueType` they formally carry.
fn ax_string(value: &Option<Accessibility::AXValue>) -> Option<String> {
    value.as_ref()?.value.as_ref()?.as_str().map(str::to_owned)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether any of `root`'s own descendants, walked breadth-first via repeated `Accessibility.getChildAXNodes` calls,
/// has a name containing `needle`.
///
/// Chrome does not map `root`'s own DOM children onto AX nodes 1:1. A `<text>` element with no `<tspan>` still becomes
/// two AX levels: a wrapping `"generic"` node, then a `"StaticText"` leaf carrying the actual rendered string as its
/// own name. A fixed-depth check would silently break if a future Chrome version wraps text differently. This walks
/// however deep the real tree goes instead, bounded only to rule out a malformed response looping forever.
fn descendant_name_contains(tab: &Tab, root: &AXNodeId, needle: &str) -> Result<bool, String> {
    let mut frontier = vec![root.clone()];
    for _ in 0..8 {
        if frontier.is_empty() {
            return Ok(false);
        }
        let mut next = Vec::new();
        for id in frontier {
            let children = tab
                .call_method(Accessibility::GetChildAXNodes { id, frame_id: None })
                .map_err(|e| format!("Accessibility.getChildAXNodes failed: {e}"))?;
            for child in children.nodes {
                if ax_string(&child.name).is_some_and(|name| name.contains(needle)) {
                    return Ok(true);
                }
                next.push(child.node_id);
            }
        }
        frontier = next;
    }
    Ok(false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether `node`'s own `AXProperty` list carries `disabled: true` — the computed accessibility-tree counterpart to the
/// rendered DOM's own `aria-disabled="true"`, which `tests/drag/selection_toolbar.rs` already checks at the attribute
/// level. Only the browser's own accessibility-tree computation can say whether that attribute actually reaches a real
/// AX property, under this exact name, with this exact boolean value.
fn ax_disabled(node: &Accessibility::AXNode) -> bool {
    node.properties.iter().flatten().any(|property| {
        matches!(property.name, Accessibility::AXPropertyName::Disabled)
            && property.value.value.as_ref().and_then(serde_json::Value::as_bool) == Some(true)
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every descendant of `root` whose own AX role is `"button"`, walked breadth-first via repeated
/// `Accessibility.getChildAXNodes` calls. See [`descendant_name_contains`]'s own doc comment for why this walks however
/// deep the real tree goes. It does not assume the toolbar's own three buttons are its direct AX children.
fn descendant_buttons(tab: &Tab, root: &AXNodeId) -> Result<Vec<Accessibility::AXNode>, String> {
    let mut frontier = vec![root.clone()];
    let mut buttons = Vec::new();
    for _ in 0..8 {
        if frontier.is_empty() {
            break;
        }
        let mut next = Vec::new();
        for id in frontier {
            let children = tab
                .call_method(Accessibility::GetChildAXNodes { id, frame_id: None })
                .map_err(|e| format!("Accessibility.getChildAXNodes failed: {e}"))?;
            for child in children.nodes {
                if ax_string(&child.role).as_deref() == Some("button") {
                    buttons.push(child.clone());
                }
                next.push(child.node_id.clone());
            }
        }
        frontier = next;
    }
    Ok(buttons)
}

mod unit_tests;
