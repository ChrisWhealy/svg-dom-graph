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

use cdp_integration_test::{build_demo, build_fixture, fixture_dir, launch_browser, serve};
use headless_chrome::{Browser, Element, Tab, protocol::cdp::Input};
use std::{
    ops::Deref,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
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
/// Opens a fresh tab on the shared fixture page and waits for the last node (`labelled`, the ninth and final one
/// added by the fixture) before returning it. Since the fixture builds every node synchronously, that node's presence
/// proves the whole scene finished building.
pub(crate) fn new_tab() -> Result<Arc<Tab>, String> {
    let shared = shared()?;
    let tab = shared.browser.new_tab().map_err(|e| format!("failed to open a new tab: {e}"))?;
    tab.navigate_to(&shared.base_url)
        .map_err(|e| format!("failed to navigate to fixture page: {e}"))?;
    tab.bring_to_front().map_err(|e| format!("failed to bring tab to front: {e}"))?;
    tab.activate().map_err(|e| format!("failed to activate tab: {e}"))?;
    tab.wait_for_element_with_custom_timeout(
        "#diagram > g.svg-dom-graph-content > g:nth-of-type(9)",
        Duration::from_secs(10),
    )
    .map_err(|e| format!("fixture did not finish building in time: {e}"))?;
    Ok(tab)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The demo gallery, built and served once per test run, and the browser that opens its pages.
///
/// Kept apart from [`SHARED`], so a run that selects only fixture tests never pays for building the demo.
struct DemoShared {
    browser: Browser,
    base_url: String,
}

static DEMO: OnceLock<Result<DemoShared, String>> = OnceLock::new();

fn demo() -> Result<&'static DemoShared, String> {
    DEMO.get_or_init(|| {
        let port = serve(build_demo());
        let browser =
            launch_browser().map_err(|e| format!("failed to launch Chrome — is it installed locally? {e}"))?;
        Ok(DemoShared {
            browser,
            base_url: format!("http://127.0.0.1:{port}"),
        })
    })
    .as_ref()
    .map_err(String::clone)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A demo tab that holds the lock every demo test shares, until it is dropped.
///
/// A click through the toolbar needs its tab in front, and Chrome has one foreground tab at a time. Demo tests that ran
/// side by side would each take the front from the others and see their clicks fail. The lock makes them take turns.
pub(crate) struct DemoTab {
    tab: Arc<Tab>,
    _turn: MutexGuard<'static, ()>,
}

impl Deref for DemoTab {
    type Target = Tab;

    fn deref(&self) -> &Tab {
        &self.tab
    }
}

static DEMO_TURN: Mutex<()> = Mutex::new(());

/// Waits for its turn, then opens a fresh tab on the demo's window page `page` (for example `chacha20-window.html`) and waits until `ready`
/// matches an element. The viewport is made `size` large, so a diagram taller than Chrome's headless window is wholly
/// on screen and every part of it can be hit by a real mouse.
pub(crate) fn new_demo_tab(page: &str, ready: &str, size: (u32, u32)) -> Result<DemoTab, String> {
    // A test that failed while holding the lock poisons it, which says nothing about the next test's tab.
    let turn = DEMO_TURN.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let demo = demo()?;
    let tab = demo.browser.new_tab().map_err(|e| format!("failed to open a new tab: {e}"))?;
    tab.call_method(headless_chrome::protocol::cdp::Emulation::SetDeviceMetricsOverride {
        width: size.0,
        height: size.1,
        device_scale_factor: 1.0,
        mobile: false,
        scale: None,
        screen_width: None,
        screen_height: None,
        position_x: None,
        position_y: None,
        dont_set_visible_size: None,
        screen_orientation: None,
        viewport: None,
        display_feature: None,
        device_posture: None,
    })
    .map_err(|e| format!("failed to size the viewport: {e}"))?;
    tab.navigate_to(&format!("{}/{page}", demo.base_url))
        .map_err(|e| format!("failed to navigate to {page}: {e}"))?;
    tab.bring_to_front().map_err(|e| format!("failed to bring tab to front: {e}"))?;
    tab.activate().map_err(|e| format!("failed to activate tab: {e}"))?;
    tab.wait_for_element_with_custom_timeout(ready, Duration::from_secs(30))
        .map_err(|e| format!("{page} did not finish building in time: {e}"))?;
    Ok(DemoTab { tab, _turn: turn })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Drives a real mouse press-move-release sequence over CDP's `Input.dispatchMouseEvent`, the same primitive a real
/// OS-level mouse drag produces. Unlike `EventTarget::dispatchEvent(new PointerEvent(...))`, this goes through the
/// browser's actual hit-testing, pointer capture, and default-action machinery.
///
/// A `waypoint` must have at least two points. The first is where the button goes down, and the last is where it comes
/// back up. Any in between are intermediate `mousemove`s while the button is held. Real drags rarely jump straight from
/// their start location to the end location in one move. Some of what this suite exists to catch (pointer capture,
/// default-action suppression etc) only manifests itself once the pointer actually leaves its starting element.
///
/// A short pause follows each dispatched event — see [`SETTLE`]'s own doc comment for why.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A short pause inserted after every individual `Input.dispatchMouseEvent` call this suite makes.
///
/// `Tab::call_method` returns once Chrome's browser process has accepted the CDP command. It does not wait for the
/// corresponding JS event to finish running in the page's renderer process. Those two processes talk over IPC. Dispatch
/// is not guaranteed to keep up with a tight, back-to-back loop of commands under load. On a slower or busier machine
/// (for example, a GitHub Actions runner), a `pointermove` or `pointerup` fired by this suite can be coalesced. It can
/// also arrive before this crate's own listener has processed the one before it. For `pointerup`, dropped-overlap
/// correction depends on all previous `move_node` calls from `pointermove` already having been applied. So losing even
/// one mid-drag step can leave the model in an implausible state, i.e. one the pointerup handler never expected. This
/// pause is cheap, costing a handful of milliseconds per waypoint. That is negligible in comparison to launching Chrome
/// and building the wasm fixture. It is far cheaper than trying to track down an intermittent CI failure.
const SETTLE: Duration = Duration::from_millis(50);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Parses a node's own `<g>` `transform="translate(x, y)"` attribute. This is the world-space origin every node box
/// carries there, instead of on its individual children — its rect, label, or grid cells. Every one of those is drawn
/// once, in local coordinates relative to `(0, 0)`, and never rewritten as the node moves.
pub(crate) fn group_translate(group: &Element<'_>) -> Result<(f64, f64), String> {
    let value = group
        .get_attribute_value("transform")
        .map_err(|e| format!("{e}"))?
        .ok_or("node's <g> has no transform attribute")?;
    let inner = value
        .strip_prefix("translate(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| format!("transform {value:?} is not a translate(...) expression"))?;
    let mut parts = inner.split(',').map(str::trim);
    let x = parts
        .next()
        .ok_or_else(|| format!("transform {value:?} has no x component"))?
        .parse::<f64>()
        .map_err(|e| format!("transform {value:?}: x did not parse as f64: {e}"))?;
    let y = parts
        .next()
        .ok_or_else(|| format!("transform {value:?} has no y component"))?
        .parse::<f64>()
        .map_err(|e| format!("transform {value:?}: y did not parse as f64: {e}"))?;
    Ok((x, y))
}

/// Turns the real mouse wheel one notch at `(x, y)` with Ctrl held, over CDP's `Input.dispatchMouseEvent`.
///
/// `delta_y` is in pixels, and negative scrolls up, which zooms in: one notch is `-100.0`. This is a genuine `wheel`
/// event, hit-tested and dispatched by the browser like one from a real device, not one built by a test and sent
/// straight at an element.
pub(crate) fn ctrl_wheel(tab: &Tab, (x, y): (f64, f64), delta_y: f64) -> Result<(), String> {
    /// The `Input.dispatchMouseEvent` modifier bit for Ctrl.
    const CTRL: u32 = 2;
    tab.call_method(Input::DispatchMouseEvent {
        Type: Input::DispatchMouseEventTypeOption::MouseWheel,
        x,
        y,
        modifiers: Some(CTRL),
        timestamp: None,
        // The left button is held down: this is a wheel turned in the middle of a drag.
        button: Some(Input::MouseButton::Left),
        buttons: Some(1),
        click_count: None,
        force: None,
        tangential_pressure: None,
        tilt_x: None,
        tilt_y: None,
        twist: None,
        delta_x: Some(0.0),
        delta_y: Some(delta_y),
        pointer_Type: None,
    })
    .map_err(|e| format!("mouse wheel at ({x}, {y}) failed: {e}"))?;
    std::thread::sleep(SETTLE);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) fn mouse_event(
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
    std::thread::sleep(SETTLE);
    Ok(())
}
