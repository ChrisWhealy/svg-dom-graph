//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_controls_are_in_the_scene_and_the_net_has_20_places_18_gates_and_50_connectors() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SVG} [data-action]"))?, 7)?;
    check_eq(count(&tab, &format!("{SVG} foreignObject select"))?, 3)?;
    check_eq(count(&tab, &format!("{SVG} input[data-slider=\"seek\"]"))?, 1)?;
    check_eq(count(&tab, &format!("{SVG} .pn-place"))?, 20)?;
    check_eq(count(&tab, &format!("{SVG} .pn-gate"))?, 18)?;
    check_eq(count(&tab, &format!("{SVG} path[data-pn-arc]"))?, 50)?;
    check_eq(count(&tab, &format!("{SVG} .pn-cell"))?, 8)?;
    check_eq(count(&tab, &format!("{SVG} .pn-scene"))?, 1)?;
    check_eq(attribute(&tab, SVG, "data-pn-scenario")?, "walkthrough".to_owned())?;
    check_eq(attribute(&tab, SVG, "data-pn-history")?, "script".to_owned())?;
    shoot(&tab, "initial")
}

#[test]
fn ids_carry_the_instance_and_no_generation_and_the_library_nodes_are_hidden_from_assistive_technology()
-> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, &place("P0.Idle"), "id")?, "pn-1-p-0".to_owned())?;
    check_eq(attribute(&tab, &place("Used"), "id")?, "pn-1-p-19".to_owned())?;
    check_eq(attribute(&tab, &gate("Put[2]"), "id")?, "pn-1-t-8".to_owned())?;
    check_eq(attribute(&tab, "path[data-pn-arc=\"49\"]", "id")?, "pn-1-a-49".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} [id^=\"pn-\"][id*=\"-g\"]"))?, 0)?;
    let content = format!("{SVG} .svg-dom-graph-content");
    check_eq(count(&tab, &format!("{content} > g[aria-hidden=\"true\"]"))?, 38)?;
    check_eq(count(&tab, &format!("{content} > path[aria-hidden=\"true\"]"))?, 50)?;
    Ok(())
}

#[test]
fn every_place_and_gate_has_a_name_and_the_net_is_one_tab_stop() -> Result<(), String> {
    let tab = open()?;
    check_eq(
        count(&tab, &format!("{SVG} .pn-place[aria-label], {SVG} .pn-gate[aria-label]"))?,
        38,
    )?;
    check_eq(
        count(
            &tab,
            &format!("{SVG} .pn-place[tabindex=\"0\"], {SVG} .pn-gate[tabindex=\"0\"]"),
        )?,
        1,
    )?;
    check_eq(attribute(&tab, &place("Free"), "aria-label")?, "Free: 8 of 8 slots".to_owned())?;
    check_eq(
        attribute(&tab, &place("C1.Idle"), "aria-label")?,
        "C1.Idle: 1 token. Waiting: buffer empty".to_owned(),
    )?;
    let name = attribute(&tab, &gate("Put[0]"), "aria-label")?;
    check(
        name.starts_with("Put[0], disabled.") && name.contains("Missing P0.Ready."),
        &name,
    )?;
    check_eq(
        attribute(&tab, &gate("Start[3]"), "aria-label")?.starts_with("Start[3], enabled."),
        true,
    )
}

#[test]
fn the_page_opens_at_snapshot_zero_paused_with_the_four_starts_enabled() -> Result<(), String> {
    let tab = open()?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (0, 50))?;
    check_eq(field(&tab, "event")?, "Initial marking".to_owned())?;
    check_eq(field(&tab, "enabled")?, "Start[0], Start[1], Start[2] and Start[3]".to_owned())?;
    check_eq(play_label(&tab)?, "Play".to_owned())?;
    check_eq(content(&tab, ".pn-caption")?, "Used 0 of 8, Free 8".to_owned())?;
    for i in 0..4 {
        check_eq(status_of(&tab, &format!("Start[{i}]"))?, "enabled".to_owned())?;
        check_eq(status_of(&tab, &format!("Put[{i}]"))?, "disabled".to_owned())?;
    }
    check_eq(count(&tab, &format!("{SVG} .pn-waiting"))?, 3)?;
    check_eq(banner_hidden(&tab)?, true)?;
    check(
        disabled(&tab, "pn-restart")? && disabled(&tab, "pn-previous")? && !disabled(&tab, "pn-next")?,
        "buttons",
    )
}

#[test]
fn every_arc_ends_on_its_gate_and_the_free_arc_is_above_the_used_arc_at_every_put_and_take() -> Result<(), String> {
    let tab = open()?;
    let m = measured(&tab)?;
    let ends = arc_ends();
    check_eq(m.arcs.len(), 50)?;
    let mut anchors: std::collections::HashMap<String, Vec<(f64, f64)>> = std::collections::HashMap::new();
    for arc in &m.arcs {
        let (gate, place, input) = &ends[arc.i];
        let bar = &m.bars[gate];
        // An input arc ends at its gate, and an output arc starts there.
        let at = if *input {
            (arc.n[arc.n.len() - 2], arc.n[arc.n.len() - 1])
        } else {
            (arc.n[0], arc.n[1])
        };
        let (cx, cy) = (bar.x + bar.w / 2.0, bar.y + bar.h / 2.0);
        let on_edge = (at.0 - cx).abs() <= bar.w / 2.0 + 1.5 && (at.1 - cy).abs() <= bar.h / 2.0 + 1.5;
        check(
            on_edge,
            &format!("arc {} ({gate} and {place}) does not meet its gate at {at:?}", arc.i),
        )?;
        anchors.entry(gate.clone()).or_default().push(at);
    }
    for (gate, points) in &anchors {
        if gate.starts_with("Put") || gate.starts_with("Take") {
            check_eq(points.len(), 4)?;
            for (n, a) in points.iter().enumerate() {
                for b in &points[n + 1..] {
                    check(
                        (a.0 - b.0).abs() > 0.5 || (a.1 - b.1).abs() > 0.5,
                        &format!("{gate} has two arcs at {a:?}"),
                    )?;
                }
            }
        }
    }
    for gate in ["Put[0]", "Put[1]", "Put[2]", "Put[3]", "Take[0]", "Take[1]", "Take[2]"] {
        let y_at = |hub: &str| -> Result<f64, String> {
            let arc = ends
                .iter()
                .position(|(g, p, _)| g == gate && p == hub)
                .and_then(|at| m.arcs.get(at))
                .ok_or_else(|| format!("no arc for {gate} and {hub}"))?;
            let input = ends
                .iter()
                .find(|(g, p, _)| g == gate && p == hub)
                .map(|e| e.2)
                .unwrap_or(false);
            Ok(if input { arc.n[arc.n.len() - 1] } else { arc.n[1] })
        };
        check(y_at("Free")? < y_at("Used")?, &format!("{gate}: Free is not above Used"))?;
    }
    Ok(())
}

#[test]
fn every_spoke_is_clear_of_the_hub_that_it_does_not_belong_to() -> Result<(), String> {
    let tab = open()?;
    let m = measured(&tab)?;
    let ends = arc_ends();
    let mut smallest = f64::MAX;
    for arc in &m.arcs {
        let (_, place, _) = &ends[arc.i];
        for (name, hub) in &m.hubs {
            if place == name {
                continue;
            }
            let clearance = distance_to_segment(
                (hub.x, hub.y),
                (arc.n[0], arc.n[1]),
                (arc.n[arc.n.len() - 2], arc.n[arc.n.len() - 1]),
            ) - hub.r;
            smallest = smallest.min(clearance);
        }
    }
    println!("PHASE0 smallest clearance between a spoke and the opposite hub: {smallest:.1}");
    check(smallest >= 8.0, &format!("a spoke comes within {smallest:.1} of the other hub"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_walkthrough_blocks_a_producer_at_full_and_releases_it_with_a_take() -> Result<(), String> {
    let tab = open()?;
    checkpoint(&tab, 0)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 24))?;
    check_eq(content(&tab, ".pn-caption")?, "Used 8 of 8, Free 0".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} .pn-cell[data-pn-full=\"true\"]"))?, 8)?;
    check_eq(count(&tab, &format!("{SVG} .pn-waiting"))?, 0)?;
    checkpoint(&tab, 1)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 26))?;
    check_eq(field(&tab, "blocked")?, "P0 waits: buffer full.".to_owned())?;
    check(
        attribute(&tab, &place("P0.Ready"), "aria-label")?.ends_with("Waiting: buffer full"),
        "blocked label",
    )?;
    check_eq(status_of(&tab, "Put[0]")?, "disabled".to_owned())?;
    shoot(&tab, "blocked")?;
    checkpoint(&tab, 2)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 27))?;
    check_eq(status_of(&tab, "Put[0]")?, "new".to_owned())?;
    check_eq(status_of(&tab, "Take[0]")?, "fired".to_owned())?;
    check_eq(field(&tab, "event")?, "Take[0]".to_owned())?;
    check(
        field(&tab, "changes")?.starts_with("Newly enabled: Put[0]"),
        &field(&tab, "changes")?,
    )?;
    checkpoint(&tab, 3)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 45))?;
    check_eq(count(&tab, &format!("{SVG} .pn-waiting"))?, 3)?;
    check_eq(content(&tab, ".pn-caption")?, "Used 0 of 8, Free 8".to_owned())
}

#[test]
fn contention_has_two_enabled_puts_and_one_takes_the_last_slot() -> Result<(), String> {
    let tab = open()?;
    scenario(&tab, "contention")?;
    wait_until(&tab, 5, |t| Ok(attribute(t, SVG, "data-pn-scenario")? == "contention"))?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (0, 28))?;
    checkpoint(&tab, 0)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 25))?;
    // Put[0] was enabled two events ago, and Finish[1] has just enabled Put[1].
    check_eq(
        (status_of(&tab, "Put[0]")?, status_of(&tab, "Put[1]")?),
        ("enabled".to_owned(), "new".to_owned()),
    )?;
    checkpoint(&tab, 1)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 26))?;
    check_eq(status_of(&tab, "Put[1]")?, "disabled".to_owned())?;
    check_eq(field(&tab, "blocked")?, "P1 waits: buffer full.".to_owned())?;
    checkpoint(&tab, 2)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 27))?;
    check_eq(status_of(&tab, "Put[1]")?, "new".to_owned())
}

#[test]
fn the_interleaving_has_busy_ready_and_processing_workers_at_once() -> Result<(), String> {
    let tab = open()?;
    scenario(&tab, "interleaving")?;
    wait_until(&tab, 5, |t| Ok(last_snapshot(t)? == 30))?;
    checkpoint(&tab, 3)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 14))?;
    for j in 0..3 {
        check(
            attribute(&tab, &place(&format!("C{j}.Processing")), "class")?.contains("pn-tok-1"),
            "processing",
        )?;
    }
    check(attribute(&tab, &place("P3.Ready"), "class")?.contains("pn-tok-1"), "P3 ready")?;
    checkpoint(&tab, 4)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 30))?;
    check_eq(field(&tab, "enabled")?, "Start[0], Start[1], Start[2] and Start[3]".to_owned())?;
    shoot(&tab, "interleaving")
}

#[test]
fn changing_the_scenario_keeps_the_scene_the_ids_and_the_zoom() -> Result<(), String> {
    let tab = open()?;
    let before = text(
        &tab,
        &format!("[...document.querySelectorAll('{SVG} [id^=\"pn-\"]')].map(e => e.id).join()"),
    )?;
    text(&tab, "window.__scene = document.querySelector('#pn-stage-svg .pn-scene'); 'ok'")?;
    let zoom = format!("document.querySelector('{SVG} .svg-dom-graph-content').getAttribute('transform') || ''");
    let zoom_before = text(&tab, &zoom)?;
    scenario(&tab, "contention")?;
    wait_until(&tab, 5, |t| Ok(last_snapshot(t)? == 28))?;
    let after = text(
        &tab,
        &format!("[...document.querySelectorAll('{SVG} [id^=\"pn-\"]')].map(e => e.id).join()"),
    )?;
    check_eq(after, before)?;
    check_eq(
        text(
            &tab,
            "String(window.__scene === document.querySelector('#pn-stage-svg .pn-scene'))",
        )?,
        "true".to_owned(),
    )?;
    check_eq(text(&tab, &zoom)?, zoom_before)?;
    check_eq(count(&tab, &format!("{SVG} .pn-scene"))?, 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn selecting_by_click_or_focus_never_fires_and_never_moves_the_snapshot() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 5)?;
    tab.find_element(&format!("{} .pn-hit", gate("Put[0]")))
        .map_err(|e| e.to_string())?
        .click()
        .map_err(|e| e.to_string())?;
    check_eq(attribute(&tab, &gate("Put[0]"), "data-pn-selected")?, "true".to_owned())?;
    check(
        field(&tab, "inspector")?.starts_with("Put[0]: inputs P0.Ready and Free"),
        &field(&tab, "inspector")?,
    )?;
    check_eq(position(&tab)?, 5)?;
    focus(&tab, &place("Free"))?;
    check_eq(attribute(&tab, &place("Free"), "data-pn-selected")?, "true".to_owned())?;
    check_eq(attribute(&tab, &gate("Put[0]"), "data-pn-selected")?, "false".to_owned())?;
    check_eq(position(&tab)?, 5)?;
    check_eq(attribute(&tab, SVG, "data-pn-history")?, "script".to_owned())
}

#[test]
fn a_script_cannot_fire_by_a_key_a_double_click_or_the_button() -> Result<(), String> {
    let tab = open()?;
    focus(&tab, &gate("Start[0]"))?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    tab.press_key(" ").map_err(|e| e.to_string())?;
    double_click(&tab, &gate("Start[1]"))?;
    click(&tab, "pn-fire")?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (0, 50))?;
    check_eq(live(&tab)?, "Enter manual mode to fire a transition.".to_owned())?;
    check(disabled(&tab, "pn-fire")?, "fire is dimmed in a script")
}

#[test]
fn manual_mode_fires_by_the_button_the_key_and_the_double_click_once_each_and_a_disabled_one_changes_nothing()
-> Result<(), String> {
    let tab = open()?;
    checkpoint(&tab, 1)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 26))?;
    click(&tab, "pn-manual")?;
    wait_until(&tab, 5, |t| Ok(attribute(t, SVG, "data-pn-history")? == "manual"))?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (0, 0))?;
    check(
        field(&tab, "history")?.starts_with("Manual branch: forked from Walkthrough"),
        &field(&tab, "history")?,
    )?;
    check_eq(attribute(&tab, "#pn-manual", "aria-label")?, "Return to script".to_owned())?;
    check(
        disabled(&tab, "pn-play")? && disabled(&tab, "pn-checkpoint")?,
        "play and checkpoints are dimmed",
    )?;
    // A disabled transition says what it is missing and changes nothing.
    focus(&tab, &gate("Put[0]"))?;
    click(&tab, "pn-fire")?;
    check_eq(
        live(&tab)?,
        "Put[0] is not enabled: Free has no token. Nothing changed.".to_owned(),
    )?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (0, 0))?;
    // An enabled one fires once, by the key.
    focus(&tab, &gate("Take[0]"))?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (1, 1))?;
    check_eq(status_of(&tab, "Put[0]")?, "new".to_owned())?;
    check(live(&tab)?.starts_with("Take[0] consumed C0.Idle and Used"), &live(&tab)?)?;
    // And once by the double click, with the two clicks that come before it only selecting.
    double_click(&tab, &gate("Put[0]"))?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (2, 2))?;
    // And once by the button.
    focus(&tab, &gate("Take[1]"))?;
    click(&tab, "pn-fire")?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (3, 3))?;
    shoot(&tab, "manual")
}

#[test]
fn returning_restores_the_script_at_the_fork_and_the_branch_is_gone() -> Result<(), String> {
    let tab = open()?;
    checkpoint(&tab, 2)?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 27))?;
    click(&tab, "pn-manual")?;
    wait_until(&tab, 5, |t| Ok(attribute(t, SVG, "data-pn-history")? == "manual"))?;
    focus(&tab, &gate("Complete[0]"))?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check_eq(last_snapshot(&tab)?, 1)?;
    click(&tab, "pn-manual")?;
    wait_until(&tab, 5, |t| Ok(attribute(t, SVG, "data-pn-history")? == "script"))?;
    check_eq((position(&tab)?, last_snapshot(&tab)?), (27, 50))?;
    check_eq(attribute(&tab, "#pn-manual", "aria-label")?, "Enter manual".to_owned())?;
    check_eq(disabled(&tab, "pn-checkpoint")?, false)
}

#[test]
fn choosing_a_scenario_from_manual_discards_the_branch_and_says_so_until_the_next_action() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "pn-manual")?;
    wait_until(&tab, 5, |t| Ok(attribute(t, SVG, "data-pn-history")? == "manual"))?;
    scenario(&tab, "contention")?;
    wait_until(&tab, 5, |t| Ok(attribute(t, SVG, "data-pn-history")? == "script"))?;
    let notice = "Manual branch discarded. Contention: two producers, last free slot loaded at snapshot 0.";
    check_eq(content(&tab, "#pn-notice")?, notice.to_owned())?;
    check_eq(attribute(&tab, "#pn-notice", "hidden")?, "null".to_owned())?;
    check_eq(live(&tab)?, notice.to_owned())?;
    click(&tab, "pn-next")?;
    check_eq(attribute(&tab, "#pn-notice", "hidden")?, String::new())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_arrows_go_through_the_places_and_gates_in_ring_order_as_one_tab_stop() -> Result<(), String> {
    let tab = open()?;
    let stops = |tab: &Tab| count(tab, &format!("{SVG} .pn-place[tabindex=\"0\"], {SVG} .pn-gate[tabindex=\"0\"]"));
    focus(&tab, &place("P0.Idle"))?;
    for (key, expected) in [
        ("ArrowRight", "Start[0]"),
        ("ArrowRight", "P0.Busy"),
        ("ArrowDown", "Finish[0]"),
        ("ArrowLeft", "P0.Busy"),
        ("ArrowUp", "Start[0]"),
        ("End", "Complete[2]"),
        ("ArrowRight", "Complete[2]"),
        ("Home", "P0.Idle"),
        ("ArrowLeft", "P0.Idle"),
    ] {
        tab.press_key(key).map_err(|e| e.to_string())?;
        check_eq(focused(&tab)?, expected.to_owned())?;
        check_eq(stops(&tab)?, 1)?;
    }
    check_eq(position(&tab)?, 0)
}

#[test]
fn focused_wiring_puts_the_four_arcs_of_a_gate_forward_and_changes_nothing_else() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 7)?;
    let before = (field(&tab, "event")?, field(&tab, "enabled")?, position(&tab)?);
    focus(&tab, &gate("Put[2]"))?;
    click(&tab, "pn-toggle-focused")?;
    check_eq(count(&tab, &format!("{SVG} path[data-pn-emphasis=\"on\"]"))?, 4)?;
    check_eq(count(&tab, &format!("{SVG} path[data-pn-emphasis=\"dim\"]"))?, 46)?;
    check_eq(attribute(&tab, "#pn-toggle-focused", "aria-pressed")?, "true".to_owned())?;
    check_eq(
        (field(&tab, "event")?, field(&tab, "enabled")?, position(&tab)?),
        before.clone(),
    )?;
    shoot(&tab, "focused")?;
    click(&tab, "pn-toggle-focused")?;
    check_eq(count(&tab, &format!("{SVG} path[data-pn-emphasis=\"normal\"]"))?, 50)?;
    check_eq(count(&tab, &format!("{SVG} path[data-pn-arc]"))?, 50)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_buttons_step_and_a_button_at_an_end_is_dimmed_keeps_the_focus_and_says_so() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "pn-previous")?;
    check_eq(live(&tab)?, "Already at the first snapshot.".to_owned())?;
    for _ in 0..3 {
        click(&tab, "pn-next")?;
    }
    check_eq(position(&tab)?, 3)?;
    check_eq(field(&tab, "event")?, "Put[0]".to_owned())?;
    click(&tab, "pn-restart")?;
    check_eq(position(&tab)?, 0)?;
    seek(&tab, 50)?;
    check(disabled(&tab, "pn-next")?, "next is dimmed at the end")?;
    focus(&tab, "#pn-next")?;
    click(&tab, "pn-next")?;
    check_eq(live(&tab)?, "Already at the final snapshot.".to_owned())?;
    check_eq(focused(&tab)?, "pn-next".to_owned())
}

#[test]
fn a_burst_of_range_moves_draws_once_at_the_next_frame_and_resting_says_where_it_is() -> Result<(), String> {
    let tab = open_manual()?;
    text(
        &tab,
        "(() => { const r = document.getElementById('pn-seek'); for (const v of [3, 9, 14, 21]) { r.value = String(v); \
         r.dispatchEvent(new Event('input', { bubbles: true })); } return 'ok'; })()",
    )?;
    check_eq((waiting_frames(&tab)?, position(&tab)?), (1, 0))?;
    check_eq(advance_frames(&tab, 5)?, 1)?;
    check_eq(position(&tab)?, 21)
}

#[test]
fn play_runs_one_tick_at_a_time_and_pause_takes_the_tick_back() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "pn-play")?;
    check_eq((play_label(&tab)?, waiting(&tab)?), ("Pause".to_owned(), 1))?;
    check_eq(advance(&tab, 1)?, 1)?;
    check_eq(position(&tab)?, 1)?;
    click(&tab, "pn-play")?;
    check_eq((play_label(&tab)?, waiting(&tab)?), ("Play".to_owned(), 0))?;
    check(live(&tab)?.starts_with("Paused at snapshot 1 of 50."), &live(&tab)?)
}

#[test]
fn playing_a_script_reaches_the_end_stops_there_and_does_not_loop() -> Result<(), String> {
    let tab = open_manual()?;
    scenario(&tab, "contention")?;
    wait_until(&tab, 5, |t| Ok(last_snapshot(t)? == 28))?;
    click(&tab, "pn-play")?;
    check_eq(advance(&tab, 100)?, 28)?;
    check_eq((position(&tab)?, waiting(&tab)?, play_label(&tab)?), (28, 0, "Play".to_owned()))?;
    check(disabled(&tab, "pn-play")?, "play is dimmed at the end")?;
    click(&tab, "pn-play")?;
    check_eq(live(&tab)?, "Already at the final snapshot.".to_owned())?;
    check_eq(waiting(&tab)?, 0)
}

#[test]
fn every_other_control_stops_the_playback_and_manual_mode_cannot_play() -> Result<(), String> {
    let tab = open_manual()?;
    for stop in ["pn-next", "pn-restart", "pn-manual"] {
        click(&tab, "pn-play")?;
        check_eq(waiting(&tab)?, 1)?;
        click(&tab, stop)?;
        check_eq((waiting(&tab)?, play_label(&tab)?), (0, "Play".to_owned()))?;
        if stop == "pn-manual" {
            click(&tab, "pn-manual")?;
        }
    }
    click(&tab, "pn-manual")?;
    click(&tab, "pn-play")?;
    check_eq((waiting(&tab)?, live(&tab)?.starts_with("Play is for scripts")), (0, true))
}

#[test]
fn a_tick_that_was_waiting_when_the_scenario_changed_does_nothing() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "pn-play")?;
    scenario(&tab, "interleaving")?;
    wait_until(&tab, 5, |t| Ok(last_snapshot(t)? == 30))?;
    check_eq(advance(&tab, 5)?, 0)?;
    check_eq(position(&tab)?, 0)
}

#[test]
fn the_manual_clock_hooks_come_off_the_window_when_the_page_is_left() -> Result<(), String> {
    let tab = open_manual()?;
    check_eq(text(&tab, "typeof window.__pnAdvance")?, "function".to_owned())?;
    text(&tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()")?;
    check_eq(text(&tab, "typeof window.__pnAdvance")?, "undefined".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_write_that_fails_is_reported_and_the_next_button_draws_the_page_again() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 5)?;
    fail_writes_of(&tab, "data-pn-status", true)?;
    click(&tab, "pn-next")?;
    check_eq(banner_hidden(&tab)?, false)?;
    check_eq(position(&tab)?, 5)?;
    fail_writes_of(&tab, "data-pn-status", false)?;
    click(&tab, "pn-previous")?;
    check_eq(banner_hidden(&tab)?, true)?;
    check_eq(position(&tab)?, 4)?;
    // Start[1] is the event that made snapshot 4, so it has just enabled Finish[1].
    check_eq(status_of(&tab, "Finish[1]")?, "new".to_owned())?;
    check_eq(status_of(&tab, "Put[1]")?, "disabled".to_owned())
}

#[test]
fn a_button_with_nowhere_to_go_still_draws_a_page_that_is_out_of_date() -> Result<(), String> {
    let tab = open()?;
    fail_writes_of(&tab, "data-pn-selected", true)?;
    focus(&tab, &place("Free"))?;
    check_eq(banner_hidden(&tab)?, false)?;
    fail_writes_of(&tab, "data-pn-selected", false)?;
    click(&tab, "pn-previous")?;
    check_eq(banner_hidden(&tab)?, true)?;
    check_eq(live(&tab)?, "Already at the first snapshot.".to_owned())
}

#[test]
fn a_page_diffed_step_by_step_is_the_same_dom_as_one_drawn_in_full_every_time() -> Result<(), String> {
    const SIGNATURE: &str = "(() => { const o = document.querySelector('#pn-stage-svg .pn-overlay').outerHTML; \
        const p = [...document.querySelectorAll('[id^=\"pn-p-\"]')].map(e => e.textContent).join('|'); \
        const a = [...document.querySelectorAll('#pn-stage-svg path[data-pn-arc]')].map(e => e.getAttribute('data-pn-emphasis')).join(); \
        const c = ['pn-scenario','pn-speed','pn-seek'].map(i => document.getElementById(i).value).join(); \
        const b = [...document.querySelectorAll('#pn-stage-svg [data-action]')].map(e => e.getAttribute('aria-disabled') + e.getAttribute('aria-pressed') + e.getAttribute('aria-label')).join(); \
        return o + p + a + c + b; })()";
    let mut steps: Vec<Step> = Vec::new();
    for to in [3, 24, 26, 27, 12, 50, 0, 8, 49] {
        steps.push(Box::new(move |t| seek(t, to)));
    }
    steps.push(Box::new(|t| scenario(t, "contention")));
    for to in [25, 26, 27, 5] {
        steps.push(Box::new(move |t| seek(t, to)));
    }
    steps.push(Box::new(|t| click(t, "pn-manual")));
    steps.push(Box::new(|t| {
        focus(t, &gate("Start[0]"))?;
        click(t, "pn-fire")
    }));
    steps.push(Box::new(|t| click(t, "pn-toggle-focused")));
    steps.push(Box::new(|t| click(t, "pn-manual")));
    // The reference: every draw is a full one. Its tab is let go before the next is opened.
    let reference = {
        let whole = open_at("?always-full")?;
        let mut pages = vec![text(&whole, SIGNATURE)?];
        for step in &steps {
            step(&whole)?;
            pages.push(text(&whole, SIGNATURE)?);
        }
        pages
    };
    let diffed = open()?;
    check(text(&diffed, SIGNATURE)? == reference[0], "the first draw differs")?;
    for (n, step) in steps.iter().enumerate() {
        step(&diffed)?;
        let page = text(&diffed, SIGNATURE)?;
        check(
            page == reference[n + 1],
            &format!("after step {n}: {}", first_difference(&page, &reference[n + 1])),
        )?;
    }
    Ok(())
}

#[test]
fn the_text_of_every_option_of_every_selector_fits_across_its_selector() -> Result<(), String> {
    const WIDEST: &str = "(() => { const c = document.createElement('canvas').getContext('2d'); const out = [];
        for (const s of document.querySelectorAll('#pn-stage-svg select')) {
          const style = getComputedStyle(s);
          c.font = style.fontSize + ' ' + style.fontFamily;
          const room = s.getBoundingClientRect().width - 28;
          for (const o of s.options) { out.push([s.id, o.textContent, Math.round(c.measureText(o.textContent).width), Math.round(room)]); }
        }
        return JSON.stringify(out); })()";
    let tab = open()?;
    let mut rows: Vec<(String, String, u32, u32)> =
        serde_json::from_str(&text(&tab, WIDEST)?).map_err(|e| e.to_string())?;
    // The checkpoints change with the scenario, so each scenario's are measured too.
    for key in ["contention", "interleaving"] {
        scenario(&tab, key)?;
        wait_until(&tab, 5, |t| Ok(attribute(t, SVG, "data-pn-scenario")? == key))?;
        rows.extend(
            serde_json::from_str::<Vec<(String, String, u32, u32)>>(&text(&tab, WIDEST)?).map_err(|e| e.to_string())?,
        );
    }
    check(rows.len() > 20, "the options were not found")?;
    for (id, option, width, room) in rows {
        check(
            width <= room,
            &format!("{id}: {option:?} is {width}px wide, and there is room for {room}px"),
        )?;
    }
    shoot(&tab, "selectors")
}

#[test]
fn the_label_of_each_hub_is_above_its_count_and_both_are_inside_the_circle() -> Result<(), String> {
    let tab = open()?;
    for name in ["Free", "Used"] {
        // At the start Free is 8 and Used is 0, and the largest count is 8, so this is measured at both.
        let report = text(
            &tab,
            &format!(
                "(() => {{ const g = document.querySelector('{}'); const hit = g.querySelector('.pn-hit').getBoundingClientRect();
                   const count = g.querySelector('.pn-count').getBoundingClientRect();
                   const node = [...document.querySelectorAll('#pn-stage-svg .svg-dom-graph-content > g')].find(n => n.querySelector('text') && n.querySelector('text').textContent === '{name}');
                   const label = node.querySelector('text').getBoundingClientRect();
                   return JSON.stringify({{label: [label.top, label.bottom], count: [count.top, count.bottom], circle: [hit.top, hit.bottom]}}); }})()",
                place(name)
            ),
        )?;
        let v: serde_json::Value = serde_json::from_str(&report).map_err(|e| e.to_string())?;
        let f = |key: &str, at: usize| v[key][at].as_f64().unwrap_or(f64::NAN);
        check(
            f("label", 1) <= f("count", 0) + 1.0,
            &format!(
                "{name}: the label reaches down to {} and the count starts at {}",
                f("label", 1),
                f("count", 0)
            ),
        )?;
        check(
            f("label", 0) > f("circle", 0) && f("count", 1) < f("circle", 1),
            &format!("{name}: {report}"),
        )?;
        // The pair is balanced: the room above the label is about the room below the count.
        let (above, below) = (f("label", 0) - f("circle", 0), f("circle", 1) - f("count", 1));
        check(
            (above - below).abs() <= 6.0,
            &format!("{name}: {above:.1} above and {below:.1} below"),
        )?;
    }
    shoot(&tab, "hubs")
}

#[test]
fn the_label_of_the_manual_button_stays_inside_the_button_in_both_of_its_states() -> Result<(), String> {
    const FIT: &str = "(() => { const b = document.getElementById('pn-manual');
        const r = b.querySelector('rect').getBoundingClientRect(); const t = b.querySelector('text').getBoundingClientRect();
        return JSON.stringify([b.querySelector('text').textContent, t.left - r.left, r.right - t.right]); })()";
    let tab = open()?;
    for expected in ["Enter manual", "Return to script", "Enter manual"] {
        let (label, left, right): (String, f64, f64) =
            serde_json::from_str(&text(&tab, FIT)?).map_err(|e| e.to_string())?;
        check_eq(label.as_str(), expected)?;
        check(
            left >= 2.0 && right >= 2.0,
            &format!("{label:?} has {left:.1}px on the left and {right:.1}px on the right"),
        )?;
        click(&tab, "pn-manual")?;
        wait_until(&tab, 5, |t| Ok(attribute(t, "#pn-manual", "aria-label")? != expected))?;
    }
    Ok(())
}

#[test]
fn leaving_the_page_takes_off_every_listener_that_the_demo_added_and_only_those() -> Result<(), String> {
    // For each target, the types of listener that the demo adds to it, once each. The library's own listeners on the
    // same targets are not the demo's to take off, so each count is compared with itself from before.
    let targets: [(&str, &[&str]); 7] = [
        (
            "document.getElementById('pn-stage-svg')",
            &["click", "dblclick", "keydown", "focusin"],
        ),
        ("document.getElementById('pn-stage')", &["click", "keydown"]),
        ("document.getElementById('pn-seek')", &["input", "change"]),
        ("document.getElementById('pn-scenario')", &["change"]),
        ("document.getElementById('pn-checkpoint')", &["change"]),
        ("document.getElementById('pn-speed')", &["change"]),
        ("window", &["pagehide"]),
    ];
    let tab = open_manual()?;
    let before: Vec<_> = targets
        .iter()
        .map(|(t, _)| listener_counts(&tab, t))
        .collect::<Result<_, _>>()?;
    for ((target, types), counts) in targets.iter().zip(&before) {
        for kind in *types {
            check(
                counts.get(*kind).copied().unwrap_or(0) >= 1,
                &format!("{target} has no {kind} listener to begin with"),
            )?;
        }
    }
    text(&tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()")?;
    for (((target, types), was), _) in targets.iter().zip(&before).zip(0..) {
        let now = listener_counts(&tab, target)?;
        for kind in *types {
            let (was, now) = (was.get(*kind).copied().unwrap_or(0), now.get(*kind).copied().unwrap_or(0));
            if *target == "window" {
                // The one listener that is kept for the life of the page, which holds only a number.
                check_eq(now, was)?;
            } else {
                check_eq(
                    (target.to_owned(), kind.to_owned(), was - now),
                    (target.to_owned(), kind.to_owned(), 1),
                )?;
            }
        }
    }
    Ok(())
}

#[test]
fn a_page_that_has_been_left_does_nothing_and_leaving_twice_is_harmless() -> Result<(), String> {
    let tab = open_manual()?;
    seek(&tab, 4)?;
    text(
        &tab,
        "(() => { window.dispatchEvent(new Event('pagehide')); window.dispatchEvent(new Event('pagehide')); return 'ok'; })()",
    )?;
    click(&tab, "pn-next")?;
    focus(&tab, &gate("Put[0]"))?;
    check_eq(
        (position(&tab)?, attribute(&tab, &gate("Put[0]"), "data-pn-selected")?),
        (4, "false".to_owned()),
    )?;
    check_eq(text(&tab, "typeof window.__pnAdvance")?, "undefined".to_owned())
}
