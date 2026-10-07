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
/// The fixture's `named_value` node (`#diagram > g.svg-dom-graph-content > g:nth-of-type(8)`) is a named, single-value
/// `u64` data node. Its own `aria-label` reads `"B: u64 = AB CD EF 01 23 45 67 89"`. This test proves that exact
/// `role`/`name` pair reaches Chrome's own computed accessibility tree. It also proves the rendered value text still
/// reaches it somewhere in the node's own descendant subtree.
///
/// The second half is the point — `draw_content_box` chose `role="group"` over `role="img"` specifically so a
/// descendant value stays independently reachable. Otherwise it would be collapsed into one atomic image instead — see
/// that function's own doc comment. A DOM-level check (`get_attribute("role")`) cannot tell the two apart; only the
/// browser's own accessibility-tree computation can.
#[test]
fn a_named_data_nodes_own_accessibility_tree_exposes_role_name_and_descendant_text() -> Result<(), String> {
    let tab = new_tab()?;
    let group = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(8)")
        .map_err(|e| format!("could not find named_value's own <g>: {e}"))?;

    tab.call_method(Accessibility::Enable(None))
        .map_err(|e| format!("Accessibility.enable failed: {e}"))?;

    // `fetch_relatives: true` also pulls in every ancestor up to the page root. Any one of those can carry an
    // `ignoredReasons` entry this crate's own CDP dependency does not know every variant of. In practice,
    // `"uninteresting"` fails the whole response's own deserialization, not just that one node. `false` here avoids
    // that; `descendant_name_contains` below walks descendants through a separate, narrower call instead.
    let own = tab
        .call_method(Accessibility::GetPartialAXTree {
            node_id: None,
            backend_node_id: Some(group.backend_node_id),
            object_id: None,
            fetch_relatives: Some(false),
        })
        .map_err(|e| format!("Accessibility.getPartialAXTree failed: {e}"))?;
    let own_node = own
        .nodes
        .first()
        .ok_or("named_value's own <g> was not present in its own returned accessibility tree")?;

    let role = ax_string(&own_node.role);
    if role.as_deref() != Some("group") {
        return Err(format!("expected named_value's own AX role \"group\", got {role:?}"));
    }
    let expected_name = "B: u64 = AB CD EF 01 23 45 67 89";
    let name = ax_string(&own_node.name);
    if name.as_deref() != Some(expected_name) {
        return Err(format!("expected named_value's own AX name {expected_name:?}, got {name:?}"));
    }

    if !descendant_name_contains(&tab, &own_node.node_id, "AB CD EF 01 23 45 67 89")? {
        return Err(
            "expected some descendant AXNode to still expose the rendered value text, found none — the group's \
             own name may have swallowed it"
                .to_owned(),
        );
    }

    Ok(())
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The fixture's selection toolbar is bound to `named_value`, with no prior `Scene::set_selection` call (see the
/// fixture's own doc comment). It proves Chrome's own computed accessibility tree exposes three things.
///
/// 1. The bar itself appears as `toolbar "Selection controls"`
/// 2. Each of its three buttons appears under its own accessible name, not the bare "Prev", "Next" or "Restart" text a
///    sighted reader sees
/// 3. `aria-disabled="true"` on Previous and Restart, and its absence on Next, becomes a real `disabled` AX property
///    with the right boolean value
///
/// `named_value` holds exactly one value, so `Next` is enabled and `Previous`/`Restart` are disabled from the moment
/// the toolbar is shown. That is `named_value`'s own default, unstarted state, reached with no interaction at all. See
/// `show_selection_toolbar`'s own doc comment ("Disabled buttons") for why an empty/single- value node's own buttons
/// start, and stay, in exactly this shape.
#[test]
fn the_selection_toolbars_own_accessibility_tree_exposes_its_role_name_buttons_and_disabled_state() -> Result<(), String>
{
    let tab = new_tab()?;
    let toolbar = tab
        .find_element("#diagram > [role=\"toolbar\"][aria-label=\"Selection controls\"]")
        .map_err(|e| format!("could not find the selection toolbar's own <g>: {e}"))?;

    tab.call_method(Accessibility::Enable(None))
        .map_err(|e| format!("Accessibility.enable failed: {e}"))?;

    // Same reasoning as the named-data-node test above for `fetch_relatives: Some(false)`.
    let own = tab
        .call_method(Accessibility::GetPartialAXTree {
            node_id: None,
            backend_node_id: Some(toolbar.backend_node_id),
            object_id: None,
            fetch_relatives: Some(false),
        })
        .map_err(|e| format!("Accessibility.getPartialAXTree failed: {e}"))?;
    let own_node = own
        .nodes
        .first()
        .ok_or("the selection toolbar's own <g> was not present in its own returned accessibility tree")?;

    let role = ax_string(&own_node.role);
    if role.as_deref() != Some("toolbar") {
        return Err(format!(
            "expected the selection toolbar's own AX role \"toolbar\", got {role:?}"
        ));
    }
    let name = ax_string(&own_node.name);
    if name.as_deref() != Some("Selection controls") {
        return Err(format!(
            "expected the selection toolbar's own AX name \"Selection controls\", got {name:?}"
        ));
    }

    let buttons = descendant_buttons(&tab, &own_node.node_id)?;
    let found: Vec<(String, bool)> = buttons
        .iter()
        .map(|button| (ax_string(&button.name).unwrap_or_default(), ax_disabled(button)))
        .collect();

    for (expected_name, expected_disabled) in [
        ("Previous selection", true),
        ("Next selection", false),
        ("Restart selection", true),
    ] {
        match found.iter().find(|(name, _)| name == expected_name) {
            None => {
                return Err(format!(
                    "no AX button named {expected_name:?} found among the toolbar's own descendants; found {found:?}"
                ));
            },
            Some((_, disabled)) if *disabled != expected_disabled => {
                return Err(format!(
                    "expected {expected_name:?}'s own AX disabled property to be {expected_disabled}, got {disabled}"
                ));
            },
            Some(_) => {},
        }
    }

    Ok(())
}
