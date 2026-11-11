//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging empty background pans the whole scene by the pointer's own movement. The surface holds pointer capture for
/// as long as the button is down, and lets go the moment it comes up.
#[test]
fn dragging_empty_background_pans_the_scene_and_holds_then_releases_pointer_capture() -> Result<(), String> {
    let tab = new_tab()?;
    check(
        content_translate(&tab)?.is_none(),
        "the scene was already panned before any drag",
    )?;

    mouse_event(&tab, MOVE, (450.0, 200.0), None)?;
    mouse_event(&tab, PRESS, (450.0, 200.0), Some(1))?;
    mouse_event(&tab, MOVE, (420.0, 190.0), Some(1))?;
    mouse_event(&tab, MOVE, (400.0, 180.0), Some(1))?;

    check(has_capture(&tab, SURFACE)?, "the surface does not hold pointer capture mid-pan")?;

    mouse_event(&tab, RELEASE, (400.0, 180.0), Some(0))?;

    check(
        !has_capture(&tab, SURFACE)?,
        "the surface still holds pointer capture after the button came up",
    )?;
    let (x, y) = content_translate(&tab)?.ok_or("the scene did not pan")?;
    check(
        close(x, -50.0) && close(y, -20.0),
        &format!("expected a pan of (-50, -20), got ({x}, {y})"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pan that starts on background and then sweeps across a node is still a pan. The node neither moves nor gets the
/// events, because the surface has captured the pointer.
#[test]
fn a_pan_that_crosses_over_a_node_stays_a_pan() -> Result<(), String> {
    let tab = new_tab()?;
    let solo = tab.find_element(SOLO).map_err(|e| format!("could not find solo: {e}"))?;
    let (solo_x, solo_y) = group_translate(&solo)?;

    // Starts on clear background, sweeps left across `solo` at (20..100, 20..60), and finishes on background below it.
    mouse_event(&tab, MOVE, (150.0, 90.0), None)?;
    mouse_event(&tab, PRESS, (150.0, 90.0), Some(1))?;
    mouse_event(&tab, MOVE, (100.0, 60.0), Some(1))?;
    mouse_event(&tab, MOVE, (60.0, 40.0), Some(1))?; // directly over solo
    mouse_event(&tab, MOVE, (30.0, 100.0), Some(1))?;
    check(
        has_capture(&tab, SURFACE)?,
        "the surface lost pointer capture while over a node",
    )?;
    mouse_event(&tab, RELEASE, (30.0, 100.0), Some(0))?;

    let (x, y) = content_translate(&tab)?.ok_or("the scene did not pan")?;
    check(
        close(x, -120.0) && close(y, 10.0),
        &format!("expected a pan of (-120, 10), got ({x}, {y})"),
    )?;

    // The node's own transform is untouched: it moved only because its parent layer did.
    let solo = tab.find_element(SOLO).map_err(|e| format!("could not re-find solo: {e}"))?;
    let (after_x, after_y) = group_translate(&solo)?;
    check(
        close(after_x, solo_x) && close(after_y, solo_y),
        &format!("solo's own position changed from ({solo_x}, {solo_y}) to ({after_x}, {after_y})"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node drag that starts on a node and then travels across empty background is still a node drag: the scene never
/// pans. The node holds pointer capture during the drag, and the surface never does.
#[test]
fn a_node_drag_that_crosses_over_background_stays_a_node_drag() -> Result<(), String> {
    let tab = new_tab()?;
    let solo = tab.find_element(SOLO).map_err(|e| format!("could not find solo: {e}"))?;
    let (before_x, before_y) = group_translate(&solo)?;

    // Presses on `solo`'s centre, then travels well beyond its own edges, over background.
    mouse_event(&tab, MOVE, (60.0, 40.0), None)?;
    mouse_event(&tab, PRESS, (60.0, 40.0), Some(1))?;
    mouse_event(&tab, MOVE, (120.0, 60.0), Some(1))?;
    mouse_event(&tab, MOVE, (180.0, 80.0), Some(1))?;
    check(has_capture(&tab, SOLO)?, "the node does not hold pointer capture mid-drag")?;
    check(
        !has_capture(&tab, SURFACE)?,
        "the pan surface captured the pointer during a node drag",
    )?;
    mouse_event(&tab, RELEASE, (180.0, 80.0), Some(0))?;

    check(
        !has_capture(&tab, SOLO)?,
        "the node still holds pointer capture after the button came up",
    )?;
    check(content_translate(&tab)?.is_none(), "a node drag panned the scene")?;

    let solo = tab.find_element(SOLO).map_err(|e| format!("could not re-find solo: {e}"))?;
    let (after_x, after_y) = group_translate(&solo)?;
    check(
        close(after_x, before_x + 120.0) && close(after_y, before_y + 40.0),
        &format!(
            "expected solo at ({}, {}), got ({after_x}, {after_y})",
            before_x + 120.0,
            before_y + 40.0
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pressing on a node never starts a pan, even though the pan surface lies behind it.
#[test]
fn pressing_on_a_node_never_starts_a_pan() -> Result<(), String> {
    let tab = new_tab()?;

    mouse_event(&tab, MOVE, (60.0, 40.0), None)?;
    mouse_event(&tab, PRESS, (60.0, 40.0), Some(1))?;
    check(
        !has_capture(&tab, SURFACE)?,
        "the pan surface captured a press that landed on a node",
    )?;
    mouse_event(&tab, RELEASE, (60.0, 40.0), Some(0))?;
    check(content_translate(&tab)?.is_none(), "a press on a node panned the scene")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A keyboard-only user can reach the scene with the Tab key, and the arrow keys then pan it. This goes through the
/// browser's own focus order and key handling, which a synthetic `keydown` dispatched at the element does not.
#[test]
fn tab_reaches_the_scene_and_the_arrow_keys_then_pan_it() -> Result<(), String> {
    let tab = new_tab()?;
    check(
        focused_role(&tab)?.is_empty(),
        "something already has focus before any key is pressed",
    )?;
    check(content_translate(&tab)?.is_none(), "the scene was already panned")?;

    tab.press_key("Tab").map_err(|e| format!("could not press Tab: {e}"))?;
    check(
        focused_role(&tab)? == "application",
        "Tab did not put focus on the scene's keyboard target",
    )?;

    for _ in 0..2 {
        tab.press_key("ArrowRight")
            .map_err(|e| format!("could not press ArrowRight: {e}"))?;
    }
    tab.press_key("ArrowDown")
        .map_err(|e| format!("could not press ArrowDown: {e}"))?;

    // The view moved right and down, so the content moved left and up: 40 units per press.
    let (x, y) = content_translate(&tab)?.ok_or("the arrow keys did not pan the scene")?;
    check(
        close(x, -80.0) && close(y, -40.0),
        &format!("expected a pan of (-80, -40), got ({x}, {y})"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Both real-wheel scenarios, one after the other.
///
/// **Ignored by default**, and run on its own with `cargo test -p cdp-integration-test -- --ignored`. Passing alone, it
/// fails when it shares the browser with the rest of this binary. Every other test opens its own tab in the one shared
/// Chrome and runs at the same time. A real mouse wheel, though, only reaches a tab the browser treats as active. With
/// several tabs in play the `Input.dispatchMouseEvent` call for the wheel times out ("The event waited for never came")
/// or loses its connection. Nothing in the scene is at fault: the same scenarios pass alone, in either order.
///
/// What it adds over the synthetic tests in `tests/drag/toolbar.rs` is a genuine wheel event in the middle of a
/// genuine, pointer-captured drag or pan. The composition itself — that the gesture carries on tracking the pointer
/// after the view changes — is proved there, without needing a real device.
#[test]
#[ignore = "a real mouse wheel is only delivered to the active tab, so this cannot share the browser with the other tests; run alone with --ignored"]
fn a_gesture_carries_on_correctly_after_a_real_wheel_zoom() -> Result<(), String> {
    a_node_drag_carries_on_correctly_after_a_real_wheel_zoom()?;
    a_pan_carries_on_correctly_after_a_real_wheel_zoom()
}
