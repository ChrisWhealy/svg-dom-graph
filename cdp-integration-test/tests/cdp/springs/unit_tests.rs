//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens with six circles and nine labelled edges, asleep, with no frame pending.
#[test]
fn the_panel_opens_asleep_with_six_circles_nine_labels_and_no_frame_pending() -> Result<(), String> {
    let tab = open()?;
    check(physics(&tab)? == "sleeping", &format!("{:?}", physics(&tab)?))?;
    check(pending(&tab)? == "0", "a frame was pending at rest")?;
    check(
        text(&tab, "document.querySelectorAll('#springs-diagram ellipse').length.toString()")? == "6",
        "six circles",
    )?;
    check(weights(&tab)? == "4,2,1,5,8,10,2,6,3", &weights(&tab)?)?;
    check(
        text(&tab, "document.getElementById('springs-status').textContent")?.starts_with("Sleeping"),
        "the status line",
    )?;
    for name in VERTICES {
        centre(&tab, name)?;
    }
    check(
        text(
            &tab,
            "(document.querySelector('[data-action=reset]').getAttribute('aria-disabled') === 'true').toString()",
        )? == "false",
        "Reset is enabled at rest",
    )
}

/// A real drag of B pulls its neighbours, and after the release the graph settles, the panel says it is sleeping, and
/// the physics has no frame pending.
#[test]
fn a_real_drag_moves_the_neighbours_and_the_graph_settles_and_stops_asking_for_frames() -> Result<(), String> {
    let tab = open()?;
    let before = positions(&tab)?;
    let (x, y) = centre(&tab, "B")?;
    drag(&tab, &[(x, y), (x + 20.0, y + 8.0), (x + 40.0, y + 16.0), (x + 60.0, y + 25.0)])?;
    // Something is moving for a while. It is never asleep before the neighbours have reacted.
    let moved = Instant::now();
    let mut neighbours_moved = false;
    while Instant::now() - moved < Duration::from_secs(3) {
        let now = positions(&tab)?;
        if now != before && physics(&tab)? != "sleeping" {
            neighbours_moved = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    check(neighbours_moved, "the neighbours never moved")?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(pending(&tab)? == "0", "a frame was pending after the graph went to sleep")?;
    check(positions(&tab)? != before, "the graph ended where it began")?;
    check(
        text(&tab, "document.getElementById('springs-announcement').textContent")?.contains("settled"),
        "the settling was not announced",
    )
}

/// While a pointer holds a circle, Reset is unavailable. After the release it comes back by itself, and then it puts
/// every circle back where it started, and nothing else changes: the weights are kept.
#[test]
fn reset_is_unavailable_while_a_circle_is_held_and_after_it_puts_the_layout_back() -> Result<(), String> {
    let tab = open()?;
    let start = positions(&tab)?;
    let labels = weights(&tab)?;
    let held = run(
        &tab,
        &format!(
            "{} fire('pointerdown', cx, cy); fire('pointermove', cx + 40, cy + 20); await sleep(300); \
             const during = document.querySelector('[data-action=reset]').getAttribute('aria-disabled') === 'true'; \
             const state = document.querySelector('{PANEL}').dataset.physics; \
             fire('pointerup', cx + 40, cy + 20); \
             return JSON.stringify([during, state]);",
            pointer_helpers("B", 1)
        ),
    )?;
    check(held == "[true,\"dragging\"]", &held)?;
    // Not run for the user: nothing was queued, and the circle is not put back by itself.
    wait_for_state(&tab, "sleeping", 25)?;
    check(positions(&tab)? != start, "Reset ran although it was unavailable")?;
    check(
        text(
            &tab,
            "(document.querySelector('[data-action=reset]').getAttribute('aria-disabled') === 'true').toString()",
        )? == "false",
        "Reset did not come back",
    )?;
    press(&tab, "reset")?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(positions(&tab)? == start, "Reset did not put every circle back")?;
    check(weights(&tab)? == labels, "Reset changed the weights")?;
    check(pending(&tab)? == "0", "a frame was pending at rest")
}

/// Randomise weights on a network at rest gives the springs new weights and a tension, which starts it moving with
/// nothing dragged. Reset then puts the layout back, and it stays there.
#[test]
fn randomising_sets_a_network_at_rest_moving_by_itself_and_reset_puts_it_back() -> Result<(), String> {
    let tab = open()?;
    let (start, labels) = (positions(&tab)?, weights(&tab)?);
    check(physics(&tab)? == "sleeping", "not at rest to begin with")?;
    press(&tab, "randomise")?;
    let woke = run(
        &tab,
        &format!(
            "const panel = document.querySelector('{PANEL}'); \
             const sleep = ms => new Promise(r => setTimeout(r, ms)); \
             await sleep(150); \
             return panel.dataset.physics;"
        ),
    )?;
    check(woke == "running", &format!("the network did not start moving: {woke}"))?;
    wait_for_state(&tab, "sleeping", 25)?;
    let now = weights(&tab)?;
    check(now != labels, "the weights did not change")?;
    check(
        now.split(',').all(|w| w.parse::<u32>().is_ok_and(|w| (1..=10).contains(&w))),
        &format!("every new weight is from 1 to 10: {now}"),
    )?;
    check(positions(&tab)? != start, "the network came to rest where it started")?;
    check(pending(&tab)? == "0", "a frame was pending at rest")?;
    press(&tab, "reset")?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(positions(&tab)? == start, "Reset did not put the layout back")?;
    check(weights(&tab)? == now, "Reset changed the weights")
}

/// A circle can be dragged right up to the margin at the top of the scene, as near as it can go to the left, and no
/// further. No room is kept there for a toolbar, because this scene has none.
#[test]
fn a_circle_can_be_dragged_to_the_top_margin_of_the_scene_and_no_further() -> Result<(), String> {
    let tab = open()?;
    let result = run(
        &tab,
        &format!(
            "{} const svg = document.getElementById('springs-diagram').getBoundingClientRect(); \
             const box = () => g.querySelector('ellipse').getBoundingClientRect(); \
             fire('pointerdown', cx, cy); \
             for (let i = 1; i <= 10; i++) {{ fire('pointermove', cx, cy - i * 40); await sleep(30); }} \
             await sleep(400); \
             const top = box().top - svg.top, left = box().left - svg.left; \
             fire('pointerup', cx, cy - 400); \
             return JSON.stringify([Math.round(top), Math.round(left)]);",
            pointer_helpers("B", 1)
        ),
    )?;
    // The scene's margin is 20 units, and the view is not zoomed.
    check(
        result.starts_with("[20,"),
        &format!("B stopped at {result}, not at the 20 unit margin"),
    )?;
    wait_for_state(&tab, "sleeping", 25)
}

/// The three buttons are in the scene, so they zoom and pan with it, and none is an HTML button.
#[test]
fn the_buttons_are_drawn_in_the_scene() -> Result<(), String> {
    let tab = open()?;
    let found = text(
        &tab,
        "JSON.stringify(['reset', 'pause', 'randomise'].map(a => \
            document.querySelectorAll('#springs-diagram > g.svg-dom-graph-content > g[role=button][data-action=' + a + ']').length) \
            .concat([document.querySelectorAll('#panel-springs button').length]))",
    )?;
    check(found == "[1,1,1,0]", &found)
}

/// A held circle that has not moved goes quiet: the panel keeps saying it is dragging, but no frame is pending, and it
/// is never reported as sleeping. A new position wakes it.
#[test]
fn a_held_circle_that_goes_quiet_stops_requesting_frames_without_sleeping_and_a_move_wakes_it() -> Result<(), String> {
    let tab = open()?;
    let result = run(
        &tab,
        &format!(
            "{} fire('pointerdown', cx, cy); fire('pointermove', cx + 30, cy + 10); \
             const panel = document.querySelector('{PANEL}'); \
             let quiet = false; \
             for (let i = 0; i < 200 && !quiet; i++) {{ await sleep(50); quiet = panel.dataset.physicsPending === '0'; }} \
             const idle = [panel.dataset.physics, panel.dataset.physicsPending]; \
             fire('pointermove', cx + 50, cy + 25); await sleep(150); \
             const woken = [panel.dataset.physics, panel.dataset.physicsPending]; \
             fire('pointerup', cx + 50, cy + 25); \
             return JSON.stringify([quiet, idle, woken]);",
            pointer_helpers("B", 1)
        ),
    )?;
    check(result.starts_with("[true,[\"dragging\",\"0\"],[\"dragging\",\"1\"]"), &result)?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(pending(&tab)? == "0", "a frame was pending at rest")
}

/// Two circles held by two pointers at once each keep their own proxy, and both settle after the release.
#[test]
fn two_circles_dragged_at_once_both_settle_after_their_pointers_are_released() -> Result<(), String> {
    let tab = open()?;
    let start = positions(&tab)?;
    let result = run(
        &tab,
        "const find = name => [...document.querySelectorAll('#springs-diagram > g.svg-dom-graph-content > g')] \
            .find(g => g.querySelector(':scope > ellipse') && g.querySelector('text').textContent === name); \
         const b = find('B'), d = find('D'); \
         const centre = g => { const r = g.querySelector('ellipse').getBoundingClientRect(); \
             return [r.left + r.width / 2, r.top + r.height / 2]; }; \
         const fire = (g, id, type, x, y) => g.dispatchEvent(new PointerEvent(type, \
             { bubbles: true, clientX: x, clientY: y, button: 0, pointerId: id, pointerType: 'mouse' })); \
         const sleep = ms => new Promise(r => setTimeout(r, ms)); \
         const [bx, by] = centre(b), [dx, dy] = centre(d); \
         fire(b, 1, 'pointerdown', bx, by); fire(d, 2, 'pointerdown', dx, dy); \
         fire(b, 1, 'pointermove', bx + 30, by + 10); fire(d, 2, 'pointermove', dx - 20, dy + 15); \
         await sleep(300); \
         const reset = document.querySelector('[data-action=reset]').getAttribute('aria-disabled') === 'true'; \
         fire(b, 1, 'pointerup', bx + 30, by + 10); await sleep(100); \
         const stillDisabled = document.querySelector('[data-action=reset]').getAttribute('aria-disabled') === 'true'; \
         fire(d, 2, 'pointerup', dx - 20, dy + 15); \
         return JSON.stringify([reset, stillDisabled]);",
    )?;
    check(result == "[true,true]", &result)?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(positions(&tab)? != start, "nothing moved")?;
    check(pending(&tab)? == "0", "a frame was pending at rest")
}

/// A press on a circle that is still catching up after a release takes it over without a jump and without an error.
#[test]
fn a_second_press_during_the_catch_up_takes_over_and_the_graph_still_settles() -> Result<(), String> {
    let tab = open()?;
    let result = run(
        &tab,
        &format!(
            "{} fire('pointerdown', cx, cy); fire('pointermove', cx + 120, cy); \
             fire('pointerup', cx + 120, cy); \
             const panel = document.querySelector('{PANEL}'); \
             const draining = panel.dataset.physics; \
             const r2 = g.querySelector('ellipse').getBoundingClientRect(); \
             const nx = r2.left + r2.width / 2, ny = r2.top + r2.height / 2; \
             fire('pointerdown', nx, ny); await sleep(100); \
             const again = panel.dataset.physics; \
             fire('pointerup', nx, ny); \
             return JSON.stringify([draining, again, document.getElementById('springs-error').hidden]);",
            pointer_helpers("B", 1)
        ),
    )?;
    check(result == "[\"dragging\",\"dragging\",true]", &result)?;
    wait_for_state(&tab, "sleeping", 25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pause stops the physics and its frames, and Resume starts it again from a fresh clock.
#[test]
fn pause_stops_the_physics_and_its_frames_and_resume_carries_on() -> Result<(), String> {
    let tab = open()?;
    let (x, y) = centre(&tab, "B")?;
    drag(&tab, &[(x, y), (x + 30.0, y + 10.0), (x + 60.0, y + 25.0)])?;
    press(&tab, "pause")?;
    wait_for_state(&tab, "paused", 5)?;
    check(pending(&tab)? == "0", "a frame was pending while paused")?;
    check(
        text(&tab, "document.querySelector('[data-action=pause] text').textContent")? == "Resume",
        "the button does not say Resume",
    )?;
    let frozen = positions(&tab)?;
    std::thread::sleep(Duration::from_millis(400));
    check(positions(&tab)? == frozen, "the circles moved while paused")?;
    press(&tab, "pause")?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(pending(&tab)? == "0", "a frame was pending at rest")
}

/// A page that is hidden pauses the physics and cancels its frame, and shown again it resumes. A hidden page that the
/// user had also paused is not resumed by showing it.
#[test]
fn a_hidden_page_pauses_the_physics_and_showing_it_again_resumes_it() -> Result<(), String> {
    let tab = open()?;
    let (x, y) = centre(&tab, "B")?;
    drag(&tab, &[(x, y), (x + 30.0, y + 10.0), (x + 60.0, y + 25.0)])?;
    run(
        &tab,
        "Object.defineProperty(document, 'hidden', { value: true, configurable: true }); \
         document.dispatchEvent(new Event('visibilitychange')); return 'hidden';",
    )?;
    wait_for_state(&tab, "paused", 5)?;
    check(pending(&tab)? == "0", "a frame was pending while the page was hidden")?;
    check(
        text(&tab, "document.getElementById('springs-status').textContent")?.contains("hidden"),
        "the status does not say why",
    )?;
    run(
        &tab,
        "Object.defineProperty(document, 'hidden', { value: false, configurable: true }); \
         document.dispatchEvent(new Event('visibilitychange')); return 'shown';",
    )?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(pending(&tab)? == "0", "a frame was pending at rest")
}

/// Leaving the page tears the demo down: no frame is left pending, and a drag afterwards neither fails nor moves
/// anything's neighbours.
#[test]
fn leaving_the_page_tears_the_demo_down_and_a_late_event_reaches_nothing() -> Result<(), String> {
    let tab = open()?;
    let result = run(
        &tab,
        &format!(
            "{} const errors = []; window.addEventListener('error', e => errors.push(e.message)); \
             fire('pointerdown', cx, cy); fire('pointermove', cx + 60, cy + 25); await sleep(100); \
             window.dispatchEvent(new PageTransitionEvent('pagehide', {{ persisted: false }})); await sleep(100); \
             const panel = document.querySelector('{PANEL}'); \
             const pending = panel.dataset.physicsPending; \
             const before = {POSITIONS}; \
             fire('pointermove', cx + 90, cy + 40); fire('pointerup', cx + 90, cy + 40); await sleep(300); \
             return JSON.stringify([pending, document.getElementById('springs-error').hidden, errors]);",
            pointer_helpers("B", 1)
        ),
    )?;
    // No uncaught error either: a listener that drops itself while it runs is reported by the browser as one.
    check(result == "[\"0\",true,[]]", &result)
}

/// A page the browser puts in its back-forward cache is suspended and not torn down. Restored, its panel is still
/// marked as built and is never built again, so the demo that was already running has to be the one that carries on.
#[test]
fn a_page_suspended_in_the_back_forward_cache_resumes_and_still_responds_to_a_drag() -> Result<(), String> {
    let tab = open()?;
    run(
        &tab,
        "window.dispatchEvent(new PageTransitionEvent('pagehide', { persisted: true })); return 'suspended';",
    )?;
    wait_for_state(&tab, "paused", 5)?;
    check(pending(&tab)? == "0", "a frame was pending while the page was suspended")?;
    run(
        &tab,
        "window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })); return 'restored';",
    )?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(
        a_drag_of_b_moves_a(&tab)?,
        "a drag after the restore did not move B's neighbour",
    )?;
    check(
        text(&tab, "document.getElementById('springs-error').hidden.toString()")? == "true",
        "an error was shown",
    )?;
    // The buttons work too.
    let labels = weights(&tab)?;
    press(&tab, "randomise")?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(weights(&tab)? != labels, "a button did nothing after the restore")
}

/// A real round trip away from the panel and back. Whether the browser restores the page from its cache or loads it
/// again, a drag afterwards must still move B's neighbour.
#[test]
fn navigating_away_and_back_leaves_a_panel_that_still_responds_to_a_drag() -> Result<(), String> {
    let tab = open()?;
    tab.navigate_to("about:blank")
        .map_err(|e| format!("could not navigate away: {e}"))?;
    tab.wait_until_navigated()
        .map_err(|e| format!("did not finish navigating away: {e}"))?;
    text(&tab, "history.back(), 'back'")?;
    tab.wait_for_element_with_custom_timeout("#springs-diagram g[aria-label^=\"Zoom in\"]", Duration::from_secs(30))
        .map_err(|e| format!("the panel did not come back: {e}"))?;
    wait_for_state(&tab, "sleeping", 25)?;
    check(
        a_drag_of_b_moves_a(&tab)?,
        "a drag after coming back did not move B's neighbour",
    )
}

/// The Dijkstra demo is untouched by the springs: its own panel still builds and still finds the route of length 13.
#[test]
fn the_dijkstra_panel_is_unchanged_by_the_springs_panel_existing() -> Result<(), String> {
    let tab = new_demo_tab(
        "index.html#panel-dijkstra",
        "#dijkstra-diagram g[aria-label^=\"Distance:\"]",
        VIEWPORT,
    )?;
    check(
        text(&tab, "document.querySelectorAll('#dijkstra-diagram ellipse').length.toString()")? == "6",
        "six vertices",
    )?;
    check(
        text(
            &tab,
            "[...document.querySelectorAll('#dijkstra-diagram [data-svg-dom-graph-edge-label] text')].map(t => t.textContent).join(',')",
        )? == "4,2,1,5,8,10,2,6,3",
        "the reference weights",
    )?;
    // The springs panel's own elements are not part of this page's physics: nothing is pending here.
    check(
        text(&tab, "document.querySelector('#panel-springs')?.dataset.physicsPending ?? '0'")? == "0",
        "the springs panel has a frame pending on the Dijkstra page",
    )
}
