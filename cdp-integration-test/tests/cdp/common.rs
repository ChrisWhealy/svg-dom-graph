//! Shared browser lifecycle for every scenario module in this binary.
//!
//! Builds the `cdp-test-fixture` wasm package, starts its static server, and launches Chrome exactly once per test run,
//! via a lazily-initialised `OnceLock` — mirrors `svg-dom`'s own `cdp-integration-test/tests/cdp/common.rs`.
//!
//! Each module opens its own [`Tab`] via [`new_tab`], for test isolation: a fresh navigation reloads the wasm module,
//! so every tab gets an independent copy of the fixture's node positions.
//!
//! Every `#[test]` in this binary returns `Result<(), String>` — a failure prints its `String` message directly, with
//! no panic and no stack trace.

use cdp_integration_test::{build_fixture, fixture_dir, launch_browser, serve};
use headless_chrome::{Browser, Tab, protocol::cdp::Input};
use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct Shared {
    // This is never read after construction, but it must outlive every test. Dropping `Browser` closes the Chrome
    // process and with it, every `Tab` opened from it.
    browser: Browser,
    base_url: String,
}

static SHARED: OnceLock<Result<Shared, String>> = OnceLock::new();

fn shared() -> Result<&'static Shared, String> {
    SHARED
        .get_or_init(|| {
            let dir = fixture_dir();
            build_fixture(&dir);
            let port = serve(dir);
            let browser =
                launch_browser().map_err(|e| format!("failed to launch Chrome — is it installed locally? {e}"))?;
            Ok(Shared {
                browser,
                base_url: format!("http://127.0.0.1:{port}/index.html"),
            })
        })
        .as_ref()
        .map_err(String::clone)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Opens a fresh tab on the shared fixture page and waits for the last node (`mover`, the third and final one added by
/// the fixture) before returning it.  Since the fixture builds every node synchronously, that node's presence proves
/// the whole scene finished building.
pub(crate) fn new_tab() -> Result<Arc<Tab>, String> {
    let shared = shared()?;
    let tab = shared.browser.new_tab().map_err(|e| format!("failed to open a new tab: {e}"))?;
    tab.navigate_to(&shared.base_url)
        .map_err(|e| format!("failed to navigate to fixture page: {e}"))?;
    tab.bring_to_front().map_err(|e| format!("failed to bring tab to front: {e}"))?;
    tab.activate().map_err(|e| format!("failed to activate tab: {e}"))?;
    tab.wait_for_element_with_custom_timeout("#diagram > g:nth-of-type(3)", Duration::from_secs(10))
        .map_err(|e| format!("fixture did not finish building in time: {e}"))?;
    Ok(tab)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Drives a real mouse press-move-release sequence over CDP's `Input.dispatchMouseEvent`, the same primitive a real
/// OS-level mouse drag produces — unlike `EventTarget::dispatchEvent(new PointerEvent(...))`, this goes through the
/// browser's actual hit-testing, pointer capture, and default-action machinery.
///
/// `waypoints` must have at least two points: the first is where the button goes down, the last is where it comes back
/// up, and any in between are intermediate `mousemove`s while the button is held.  Real drags rarely jump straight from
/// their start location to the end location in one move, and some of what this suite exists to catch (pointer capture,
/// default-action suppression) only manifests itself once the pointer actually leaves its starting element.
pub(crate) fn drag(tab: &Tab, waypoints: &[(f64, f64)]) -> Result<(), String> {
    let (first, rest) = waypoints.split_first().ok_or("drag needs at least two waypoints")?;
    let (last, moves) = rest.split_last().ok_or("drag needs at least two waypoints")?;

    mouse_event(tab, Input::DispatchMouseEventTypeOption::MouseMoved, *first, None)?;
    mouse_event(tab, Input::DispatchMouseEventTypeOption::MousePressed, *first, Some(1))?;
    for point in moves {
        mouse_event(tab, Input::DispatchMouseEventTypeOption::MouseMoved, *point, Some(1))?;
    }
    mouse_event(tab, Input::DispatchMouseEventTypeOption::MouseMoved, *last, Some(1))?;
    mouse_event(tab, Input::DispatchMouseEventTypeOption::MouseReleased, *last, Some(0))?;
    Ok(())
}

fn mouse_event(
    tab: &Tab,
    kind: Input::DispatchMouseEventTypeOption,
    (x, y): (f64, f64),
    buttons: Option<u32>,
) -> Result<(), String> {
    let is_button_event = !matches!(kind, Input::DispatchMouseEventTypeOption::MouseMoved) || buttons.is_some();
    let kind_description = format!("{kind:?}");
    tab.call_method(Input::DispatchMouseEvent {
        Type: kind,
        x,
        y,
        modifiers: None,
        timestamp: None,
        button: if is_button_event { Some(Input::MouseButton::Left) } else { None },
        buttons,
        click_count: Some(1),
        force: None,
        tangential_pressure: None,
        tilt_x: None,
        tilt_y: None,
        twist: None,
        delta_x: None,
        delta_y: None,
        pointer_Type: None,
    })
    .map_err(|e| format!("{kind_description} at ({x}, {y}) failed: {e}"))?;
    Ok(())
}
