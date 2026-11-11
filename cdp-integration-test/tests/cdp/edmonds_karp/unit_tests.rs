//! The tests of [`super`], which holds the helpers they use.

use super::*;

// The scene.

#[test]
fn the_controls_are_in_the_scene_and_the_graph_is_built_in_layers() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SVG} [data-action]"))?, 10)?;
    check_eq(count(&tab, &format!("{SVG} foreignObject select"))?, 2)?;
    check_eq(count(&tab, &format!("{SVG} foreignObject input[type=\"range\"]"))?, 1)?;
    check_eq(
        text(
            &tab,
            "String(Array.from(document.querySelectorAll('#panel-edmonds-karp button, #panel-edmonds-karp select, \
             #panel-edmonds-karp input')).filter(c => !c.closest('#ek-stage-svg')).length)",
        )?,
        "0".to_owned(),
    )?;
    check_eq(
        attribute(&tab, &format!("{SVG} .ek-root"), "data-ek-fixture")?,
        "canonical".to_owned(),
    )?;
    check_eq(count(&tab, &format!("{SVG} g.ek-node[data-ek-vertex]"))?, 6)?;
    check_eq(count(&tab, &format!("{SVG} g.ek-edge[data-ek-edge]"))?, 10)?;
    check_eq(count(&tab, &format!("{SVG} g.ek-res[data-ek-edge]"))?, 20)?;
    let layers = text(
        &tab,
        &format!(
            "Array.from(document.querySelector('{SVG} .ek-root').children).map(c => c.getAttribute('class') || c.tagName).join(' ')"
        ),
    )?;
    check_eq(layers, "defs ek-lanes ek-edges ek-labels ek-nodes ek-badges".to_owned())?;
    check_eq(attribute(&tab, SVG, "width")?, "740".to_owned())?;
    check_eq(attribute(&tab, SVG, "height")?, "568".to_owned())
}

#[test]
fn every_id_carries_the_instance_the_generation_and_the_graph_and_no_bare_edge_id_is_in_the_document()
-> Result<(), String> {
    let tab = open()?;
    check_eq(root_id(&tab)?, "ek-1-g0-canonical-root".to_owned())?;
    let ids = text(
        &tab,
        &format!("Array.from(document.querySelectorAll('{SVG} .ek-root [id]')).map(e => e.id).join(' ')"),
    )?;
    for id in ids.split(' ') {
        check(
            id.starts_with("ek-1-g0-canonical-"),
            &format!("{id} carries the instance, generation and graph"),
        )?;
    }
    check_eq(count(&tab, "#e0, #v1, [id=\"e0\"]")?, 0)?;
    let unique = text(
        &tab,
        "String(new Set(Array.from(document.querySelectorAll('[id]')).map(e => e.id)).size === \
         document.querySelectorAll('[id]').length)",
    )?;
    check_eq(unique, "true".to_owned())
}

#[test]
fn the_page_opens_at_step_zero_paused_with_the_pinned_default_speed() -> Result<(), String> {
    let tab = open()?;
    check_eq((position(&tab)?, last_step(&tab)?), (0, 86))?;
    check_eq(field(&tab, "phase")?, "Initial".to_owned())?;
    check_eq(field(&tab, "total")?, "0".to_owned())?;
    check_eq(play_label(&tab)?, "Play".to_owned())?;
    check_eq(text(&tab, "document.getElementById('ek-speed').value")?, "2".to_owned())?;
    check_eq(
        text(&tab, "document.getElementById('ek-fixture').value")?,
        "canonical".to_owned(),
    )?;
    check_eq(attribute(&tab, "#ek-seek", "max")?, "86".to_owned())?;
    check_eq(edge_label(&tab, 0)?, "0/16".to_owned())
}

#[test]
fn the_original_edges_and_both_residual_lanes_have_names_and_the_residual_lanes_say_what_they_belong_to()
-> Result<(), String> {
    let tab = open()?;
    check_eq(
        attribute(&tab, &edge(0), "aria-label")?,
        "e0 s to v1; flow 0 of 16; forward residual 16; reverse residual 0".to_owned(),
    )?;
    check_eq(attribute(&tab, &vertex("v1"), "aria-label")?, "Vertex v1".to_owned())?;
    check_eq(
        attribute(&tab, &lane(0, "f"), "aria-label")?,
        "forward of e0 s to v1, capacity 16".to_owned(),
    )?;
    // A lane with no room is hidden from the accessibility tree.
    check_eq(attribute(&tab, &lane(0, "r"), "aria-hidden")?, "true".to_owned())?;
    check(has_class(&tab, &lane(0, "r"), "ek-off")?, "and from the picture")
}

// The runs.

#[test]
fn the_canonical_run_ends_at_a_flow_of_23_and_a_cut_of_23() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "ek-jump-to-cut")?;
    check_eq(position(&tab)?, 86)?;
    check_eq(field(&tab, "total")?, "23".to_owned())?;
    check_eq(
        field(&tab, "cut")?,
        "S = s, v1, v2, v4; T = v3, t; crossing e4, e7, e9; max flow = min cut = 23".to_owned(),
    )?;
    for v in ["s", "v1", "v2", "v4"] {
        check(has_class(&tab, &vertex(v), "ek-s")?, &format!("{v} is in S"))?;
    }
    for v in ["v3", "t"] {
        check(has_class(&tab, &vertex(v), "ek-t")?, &format!("{v} is in T"))?;
    }
    for e in 0..10 {
        let crossing = [4, 7, 9].contains(&e);
        check_eq(has_class(&tab, &edge(e), "ek-cut")?, crossing)?;
    }
    check(!has_class(&tab, &edge(5), "ek-cut")?, "the edge from T to S is not marked")?;
    check(
        attribute(&tab, &vertex("v3"), "aria-label")?.ends_with("in T"),
        "the name says it",
    )
}

#[test]
fn the_augmentation_buttons_visit_the_three_paths_and_each_edge_label_says_before_and_after() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "ek-next-augmentation")?;
    check_eq(field(&tab, "phase")?, "FlowAugmented".to_owned())?;
    check_eq(field(&tab, "total")?, "12".to_owned())?;
    check_eq(edge_label(&tab, 0)?, "0\u{2192}12/16".to_owned())?;
    check_eq(edge_label(&tab, 4)?, "0\u{2192}12/12".to_owned())?;
    check_eq(edge_label(&tab, 1)?, "0/13".to_owned())?;
    check(
        field(&tab, "path")?.starts_with("s \u{2192} v1 \u{2192} v3 \u{2192} t"),
        "the path is in text",
    )?;
    click(&tab, "ek-next-augmentation")?;
    check_eq(field(&tab, "total")?, "16".to_owned())?;
    click(&tab, "ek-next-augmentation")?;
    check_eq(field(&tab, "total")?, "23".to_owned())?;
    check(
        field(&tab, "path")?.starts_with("s \u{2192} v2 \u{2192} v4 \u{2192} v3 \u{2192} t"),
        "the third path",
    )?;
    click(&tab, "ek-previous-augmentation")?;
    check_eq(field(&tab, "total")?, "16".to_owned())
}

#[test]
fn the_reverse_graph_takes_flow_back_along_e0_and_says_so_in_the_graph_and_in_text() -> Result<(), String> {
    let tab = open()?;
    select(&tab, "ek-fixture", "reverse")?;
    check_eq(
        attribute(&tab, &format!("{SVG} .ek-root"), "data-ek-fixture")?,
        "reverse".to_owned(),
    )?;
    check_eq((position(&tab)?, last_step(&tab)?), (0, 43))?;
    click(&tab, "ek-next-augmentation")?;
    click(&tab, "ek-next-augmentation")?;
    check_eq(field(&tab, "total")?, "3".to_owned())?;
    check_eq(edge_label(&tab, 0)?, "2\u{2192}1/2".to_owned())?;
    check(
        field(&tab, "augmentation")?.contains("e0 R takes back 1: 2 to 1"),
        &field(&tab, "augmentation")?,
    )?;
    check(field(&tab, "augmentation")?.contains("total 2 to 3"), "and the total")?;
    check(
        has_class(&tab, &lane(0, "r"), "ek-path")?,
        "the reverse lane of e0 is on the path",
    )?;
    check(has_class(&tab, &edge(0), "ek-aug")?, "and e0 is marked as changed")?;
    check_eq(
        attribute(&tab, &lane(0, "r"), "aria-label")?,
        "reverse of e0 v3 to v1, capacity 1".to_owned(),
    )
}

// The keyboard and the pointer.

#[test]
fn the_graph_is_one_tab_stop_and_the_arrow_keys_go_through_the_vertices_and_then_the_edges() -> Result<(), String> {
    let tab = open()?;
    let stops = |tab: &Tab| {
        count(
            tab,
            &format!("{SVG} g.ek-node[tabindex=\"0\"], {SVG} g.ek-edge[tabindex=\"0\"]"),
        )
    };
    check_eq(stops(&tab)?, 1)?;
    check_eq(
        count(&tab, &format!("{SVG} g.ek-node[tabindex], {SVG} g.ek-edge[tabindex]"))?,
        16,
    )?;
    text(&tab, &format!("document.querySelector('{}').focus(), 'ok'", vertex("s")))?;
    let on = |tab: &Tab| {
        text(
            tab,
            "document.activeElement.getAttribute('data-ek-vertex') || document.activeElement.getAttribute('data-ek-edge') || ''",
        )
    };
    check_eq(on(&tab)?, "s".to_owned())?;
    for (key, expected) in [
        ("ArrowRight", "v1"),
        ("ArrowDown", "v2"),
        ("ArrowLeft", "v1"),
        ("ArrowUp", "s"),
        ("ArrowUp", "e9"),
        ("ArrowRight", "s"),
        ("End", "e9"),
        ("Home", "s"),
    ] {
        tab.press_key(key).map_err(|e| e.to_string())?;
        check_eq(on(&tab)?, expected.to_owned())?;
        check_eq(stops(&tab)?, 1)?;
    }
    check_eq(position(&tab)?, 0)?;
    // Vertices come first, and then the edges by their IDs.
    for _ in 0..6 {
        tab.press_key("ArrowRight").map_err(|e| e.to_string())?;
    }
    check_eq(on(&tab)?, "e0".to_owned())
}

#[test]
fn a_focus_never_moves_the_step_and_a_page_key_or_enter_is_left_alone() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 20)?;
    text(&tab, &format!("document.querySelector('{}').focus(), 'ok'", edge(4)))?;
    for key in ["PageDown", "Enter", " "] {
        tab.press_key(key).map_err(|e| e.to_string())?;
    }
    check_eq(position(&tab)?, 20)?;
    check(has_class(&tab, &edge(4), "ek-focused")?, "the focus is marked")
}

#[test]
fn a_click_on_a_dashed_transition_focuses_the_edge_it_belongs_to_and_not_the_step() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 40)?;
    text(
        &tab,
        &format!(
            "(() => {{ document.querySelector('{}').dispatchEvent(new PointerEvent('pointerdown', {{ bubbles: true }})); return 'ok'; }})()",
            lane(4, "f")
        ),
    )?;
    check(has_class(&tab, &edge(4), "ek-focused")?, "the parent edge is focused")?;
    check_eq(attribute(&tab, &edge(4), "tabindex")?, "0".to_owned())?;
    check_eq(position(&tab)?, 40)
}

// The toggles.

#[test]
fn the_residual_toggle_hides_every_lane_and_changes_no_text_or_step() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 40)?;
    let before = (position(&tab)?, field(&tab, "event")?, field(&tab, "queue")?);
    check_eq(attribute(&tab, "#ek-toggle-residual", "aria-pressed")?, "true".to_owned())?;
    click(&tab, "ek-toggle-residual")?;
    check_eq(attribute(&tab, "#ek-toggle-residual", "aria-pressed")?, "false".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} g.ek-res:not(.ek-off)"))?, 0)?;
    check_eq((position(&tab)?, field(&tab, "event")?, field(&tab, "queue")?), before)?;
    click(&tab, "ek-toggle-residual")?;
    check(count(&tab, &format!("{SVG} g.ek-res:not(.ek-off)"))? > 0, "they come back")
}

#[test]
fn the_queue_toggle_hides_the_marks_and_the_text_of_the_queue_stays() -> Result<(), String> {
    let tab = open()?;
    let at = {
        // The first step with two vertices in the queue.
        let mut at = 0;
        for step in 1..=86 {
            seek(&tab, step)?;
            if field(&tab, "queue")?.contains(',') {
                at = step;
                break;
            }
        }
        at
    };
    check(at > 0, "a step with a queue of two")?;
    check(count(&tab, &format!("{SVG} .ek-badge:not(.ek-off)"))? >= 2, "marks are shown")?;
    let queue = field(&tab, "queue")?;
    click(&tab, "ek-toggle-queue")?;
    check_eq(count(&tab, &format!("{SVG} .ek-badge:not(.ek-off)"))?, 0)?;
    check_eq(field(&tab, "queue")?, queue)
}

#[test]
fn the_detail_toggle_marks_the_examined_transition_but_the_event_line_always_changes() -> Result<(), String> {
    let tab = open()?;
    let mut examined = 0;
    for step in 1..=86 {
        seek(&tab, step)?;
        if field(&tab, "phase")? == "ArcExamined" {
            examined = step;
            break;
        }
    }
    check(examined > 0, "a step that examines an arc")?;
    check(count(&tab, &format!("{SVG} g.ek-edge.ek-examined"))? == 1, "the arc is marked")?;
    let line = field(&tab, "event")?;
    click(&tab, "ek-toggle-detail")?;
    check_eq(count(&tab, &format!("{SVG} g.ek-edge.ek-examined"))?, 0)?;
    check_eq(field(&tab, "event")?, line.clone())?;
    check_eq(field(&tab, "history")?, "\u{2014}".to_owned())?;
    // The next step has a new line, whether or not the picture changed.
    click(&tab, "ek-next")?;
    check(field(&tab, "event")? != line, "the line changes at every step")
}

// The buttons at an end.

#[test]
fn a_button_at_an_end_is_dimmed_keeps_the_focus_and_says_the_pinned_words() -> Result<(), String> {
    let tab = open()?;
    for id in ["restart", "previous", "previous-augmentation"] {
        check_eq(attribute(&tab, &format!("#ek-{id}"), "aria-disabled")?, "true".to_owned())?;
        check_eq(attribute(&tab, &format!("#ek-{id}"), "tabindex")?, "0".to_owned())?;
    }
    for id in ["next", "play", "next-augmentation", "jump-to-cut"] {
        check_eq(attribute(&tab, &format!("#ek-{id}"), "aria-disabled")?, "false".to_owned())?;
    }
    text(&tab, "document.getElementById('ek-previous').focus(), 'ok'")?;
    click(&tab, "ek-previous")?;
    check_eq(position(&tab)?, 0)?;
    check_eq(focused(&tab)?, "ek-previous".to_owned())?;
    check_eq(live(&tab)?, "Already at the first step".to_owned())?;
    click(&tab, "ek-previous-augmentation")?;
    check_eq(live(&tab)?, "No earlier augmentation".to_owned())?;
    click(&tab, "ek-jump-to-cut")?;
    check_eq(attribute(&tab, "#ek-next", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#ek-play", "aria-disabled")?, "true".to_owned())?;
    click(&tab, "ek-next")?;
    check_eq(live(&tab)?, "Already at the last step".to_owned())?;
    click(&tab, "ek-next-augmentation")?;
    check_eq(live(&tab)?, "No later augmentation".to_owned())?;
    click(&tab, "ek-play")?;
    check_eq(live(&tab)?, "Already at the last step".to_owned())?;
    check_eq(play_label(&tab)?, "Play".to_owned())
}

#[test]
fn a_dimmed_button_works_from_the_keyboard() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('ek-next').focus(), 'ok'")?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check_eq(position(&tab)?, 1)?;
    tab.press_key(" ").map_err(|e| e.to_string())?;
    check_eq(position(&tab)?, 2)?;
    check_eq(focused(&tab)?, "ek-next".to_owned())
}

// The range.

#[test]
fn a_burst_of_moves_draws_once_at_the_next_frame_and_resting_says_where_it_is_once() -> Result<(), String> {
    let tab = open_manual()?;
    text(
        &tab,
        "(() => { const r = document.getElementById('ek-seek'); \
         for (const v of ['10', '40', '25']) { r.value = v; r.dispatchEvent(new Event('input', { bubbles: true })); } \
         return 'ok'; })()",
    )?;
    check_eq(waiting_frames(&tab)?, 1)?;
    check_eq(position(&tab)?, 0)?;
    check_eq(advance_frames(&tab, 1)?, 1)?;
    check_eq(position(&tab)?, 25)?;
    check_eq(live(&tab)?, String::new())?;
    text(
        &tab,
        "(() => { document.getElementById('ek-seek').dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check(live(&tab)?.starts_with("Step 25 of 86"), &live(&tab)?)?;
    check_eq(text(&tab, "document.getElementById('ek-seek').value")?, "25".to_owned())
}

// Playing.

#[test]
fn play_runs_one_tick_at_a_time_and_pause_takes_the_tick_back() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "ek-play")?;
    check_eq(play_label(&tab)?, "Pause".to_owned())?;
    check_eq(waiting(&tab)?, 1)?;
    check_eq(advance(&tab, 5)?, 5)?;
    check_eq((position(&tab)?, waiting(&tab)?), (5, 1))?;
    check_eq(live(&tab)?, String::new())?;
    click(&tab, "ek-play")?;
    check_eq((play_label(&tab)?, waiting(&tab)?), ("Play".to_owned(), 0))?;
    check(live(&tab)?.starts_with("Paused at step 5 of 86"), &live(&tab)?)
}

#[test]
fn playing_the_whole_run_reaches_the_end_and_stops_there() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "ek-play")?;
    check_eq(advance(&tab, 1_000)?, 86)?;
    check_eq((position(&tab)?, field(&tab, "phase")?), (86, "MinCutShown".to_owned()))?;
    check_eq((waiting(&tab)?, play_label(&tab)?), (0, "Play".to_owned()))?;
    check_eq(attribute(&tab, "#ek-play", "aria-disabled")?, "true".to_owned())?;
    check(live(&tab)?.starts_with("Step 86 of 86: MinCutShown"), "the cut is announced")
}

#[test]
fn every_other_control_stops_the_playback() -> Result<(), String> {
    let tab = open_manual()?;
    for id in ["ek-next", "ek-previous", "ek-restart", "ek-next-augmentation", "ek-jump-to-cut"] {
        click(&tab, "ek-play")?;
        check_eq(waiting(&tab)?, 1)?;
        click(&tab, id)?;
        check_eq((waiting(&tab)?, play_label(&tab)?), (0, "Play".to_owned()))?;
        click(&tab, "ek-restart")?;
    }
    click(&tab, "ek-play")?;
    text(
        &tab,
        "(() => { const r = document.getElementById('ek-seek'); r.value = '30'; r.dispatchEvent(new Event('input', { bubbles: true })); return 'ok'; })()",
    )?;
    check_eq((waiting(&tab)?, play_label(&tab)?), (0, "Play".to_owned()))
}

#[test]
fn a_new_speed_while_playing_keeps_one_tick_waiting() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "ek-play")?;
    select(&tab, "ek-speed", "4")?;
    check_eq((waiting(&tab)?, play_label(&tab)?), (1, "Pause".to_owned()))?;
    check_eq(advance(&tab, 3)?, 3)?;
    check_eq(position(&tab)?, 3)
}

#[test]
fn a_timer_that_cannot_be_set_leaves_play_paused_and_says_so() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { if (!window.__st) { window.__st = window.setTimeout; } \
         window.setTimeout = function (f, d) { if (d === 200) { throw new Error('injected'); } return window.__st.apply(this, arguments); }; \
         return 'ok'; })()",
    )?;
    click(&tab, "ek-play")?;
    check_eq(play_label(&tab)?, "Play".to_owned())?;
    check(!banner_hidden(&tab)?, "reported")?;
    check(content(&tab, "#ek-error")?.contains("Play could not carry on"), "and says what")?;
    check_eq(position(&tab)?, 0)?;
    text(&tab, "(() => { window.setTimeout = window.__st; return 'ok'; })()")?;
    click(&tab, "ek-play")?;
    check_eq(play_label(&tab)?, "Pause".to_owned())?;
    check(banner_hidden(&tab)? || true, "the message may go")?;
    wait_until(&tab, 10, |tab| Ok(position(tab)? > 2))?;
    click(&tab, "ek-play")
}

// A write that fails.

#[test]
fn a_write_that_fails_is_reported_and_the_next_button_draws_the_page_again() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 20)?;
    fail_writes_of(&tab, "class", true)?;
    click(&tab, "ek-next")?;
    fail_writes_of(&tab, "class", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check(content(&tab, "#ek-error")?.contains("may be out of date"), "and says why")?;
    check_eq(position(&tab)?, 20)?;
    click(&tab, "ek-next")?;
    check_eq(position(&tab)?, 21)?;
    check(banner_hidden(&tab)?, "a draw that works hides it")
}

#[test]
fn a_button_with_nowhere_to_go_still_draws_a_page_that_is_out_of_date() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "ek-jump-to-cut")?;
    fail_writes_of(&tab, "class", true)?;
    click(&tab, "ek-previous")?;
    fail_writes_of(&tab, "class", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(position(&tab)?, 86)?;
    click(&tab, "ek-next")?;
    check_eq(position(&tab)?, 86)?;
    check(banner_hidden(&tab)?, "repaired by a button that had nowhere to go")?;
    check_eq(live(&tab)?, "Already at the last step".to_owned())
}

#[test]
fn a_failed_toggle_keeps_the_pressed_state_that_was_committed_and_is_repaired() -> Result<(), String> {
    // The first write of a toggle is aria-pressed, and the fill and the stroke come after it. A failure at any of them
    // leaves the button as it was, and not half changed.
    for name in ["aria-pressed", "fill", "stroke"] {
        let tab = open()?;
        let look = |tab: &Tab| -> Result<(String, String, String), String> {
            Ok((
                attribute(tab, "#ek-toggle-queue", "aria-pressed")?,
                attribute(tab, "#ek-toggle-queue rect", "fill")?,
                attribute(tab, "#ek-toggle-queue rect", "stroke")?,
            ))
        };
        let committed = look(&tab)?;
        fail_writes_of(&tab, name, true)?;
        click(&tab, "ek-toggle-queue")?;
        fail_writes_of(&tab, name, false)?;
        check(!banner_hidden(&tab)?, &format!("{name}: reported"))?;
        check_eq(look(&tab)?, committed.clone())
            .map_err(|e| format!("{name}: the button shows what was committed: {e}"))?;
        check_eq(attribute(&tab, "#ek-toggle-queue", "aria-pressed")?, "true".to_owned())?;
        click(&tab, "ek-next")?;
        check(banner_hidden(&tab)?, &format!("{name}: repaired"))?;
        check_eq(look(&tab)?, committed).map_err(|e| format!("{name}: after the repair: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_play_button_whose_label_could_not_be_written_is_repaired_by_a_press_with_nowhere_to_go() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "ek-play")?;
    check_eq(play_label(&tab)?, "Pause".to_owned())?;
    fail_writes_of(&tab, "aria-label", true)?;
    click(&tab, "ek-play")?;
    fail_writes_of(&tab, "aria-label", false)?;
    check_eq(waiting(&tab)?, 0)?;
    check(!banner_hidden(&tab)?, "reported")?;
    // The run is at its first step, so Previous has nowhere to go. It still draws a page that is out of date.
    click(&tab, "ek-previous")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check_eq(play_label(&tab)?, "Play".to_owned())?;
    check_eq(attribute(&tab, "#ek-play", "aria-label")?, "Play".to_owned())?;
    check_eq(position(&tab)?, 0)
}

#[test]
fn a_failed_removal_of_an_attribute_is_reported_and_the_next_action_repairs_it() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 40)?;
    let lanes = format!("{SVG} g.ek-res");
    let both = format!("{lanes}[aria-label][aria-hidden]");
    check(
        count(&tab, &format!("{lanes}[aria-label]"))? > 0,
        "the lanes are named while they show",
    )?;
    // Hiding the lanes takes their name off them.
    fail_removals_of(&tab, "aria-label", true)?;
    click(&tab, "ek-toggle-residual")?;
    fail_removals_of(&tab, "aria-label", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(attribute(&tab, "#ek-toggle-residual", "aria-pressed")?, "true".to_owned())?;
    click(&tab, "ek-next")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check_eq(position(&tab)?, 41)?;
    check_eq(count(&tab, &both)?, 0)?;
    check(
        count(&tab, &format!("{lanes}[aria-label]"))? > 0,
        "the toggle did not take, so they show",
    )?;
    // With nothing failing, the same toggle hides them and leaves no name on any.
    click(&tab, "ek-toggle-residual")?;
    check(banner_hidden(&tab)?, "no failure")?;
    check_eq(count(&tab, &format!("{lanes}:not(.ek-off)"))?, 0)?;
    check_eq(count(&tab, &format!("{lanes}[aria-label]"))?, 0)?;
    check_eq(count(&tab, &both)?, 0)
}

// A switch of graph that fails.

#[test]
fn a_switch_that_fails_at_a_control_leaves_the_old_graph_whole_and_a_later_switch_works() -> Result<(), String> {
    for name in ["max", "aria-valuetext", "aria-disabled"] {
        let tab = open()?;
        seek(&tab, 30)?;
        fail_writes_of(&tab, name, true)?;
        select(&tab, "ek-fixture", "reverse")?;
        check_eq(root_id(&tab)?, "ek-1-g0-canonical-root".to_owned()).map_err(|e| format!("{name}: {e}"))?;
        check_eq(position(&tab)?, 30).map_err(|e| format!("{name}: {e}"))?;
        check_eq(last_step(&tab)?, 86).map_err(|e| format!("{name}: {e}"))?;
        check_eq(
            text(&tab, "document.getElementById('ek-fixture').value")?,
            "canonical".to_owned(),
        )
        .map_err(|e| format!("{name}: {e}"))?;
        check(!banner_hidden(&tab)?, &format!("{name}: reported"))?;
        fail_writes_of(&tab, name, false)?;
        click(&tab, "ek-next")?;
        check_eq(position(&tab)?, 31).map_err(|e| format!("{name}: repaired: {e}"))?;
        check_eq(attribute(&tab, "#ek-seek", "max")?, "86".to_owned())?;
        select(&tab, "ek-fixture", "reverse")?;
        check_eq(root_id(&tab)?, "ek-1-g1-reverse-root".to_owned()).map_err(|e| format!("{name}: then: {e}"))?;
        check_eq((position(&tab)?, last_step(&tab)?), (0, 43))?;
        check_eq(attribute(&tab, "#ek-seek", "max")?, "43".to_owned())?;
    }
    Ok(())
}

#[test]
fn a_switch_whose_replacement_fails_leaves_the_old_graph_and_its_controls() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 12)?;
    fail_replacement(&tab, true, false)?;
    select(&tab, "ek-fixture", "reverse")?;
    fail_replacement(&tab, false, false)?;
    check_eq(root_id(&tab)?, "ek-1-g0-canonical-root".to_owned())?;
    check_eq((position(&tab)?, last_step(&tab)?), (12, 86))?;
    check_eq(attribute(&tab, "#ek-seek", "max")?, "86".to_owned())?;
    check_eq(
        text(&tab, "document.getElementById('ek-fixture').value")?,
        "canonical".to_owned(),
    )?;
    check(!banner_hidden(&tab)?, "reported")?;
    click(&tab, "ek-next")?;
    check_eq(position(&tab)?, 13)?;
    select(&tab, "ek-fixture", "reverse")?;
    check_eq(root_id(&tab)?, "ek-1-g1-reverse-root".to_owned())
}

#[test]
fn a_replacement_that_throws_after_it_changed_the_host_puts_the_old_graph_back() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 12)?;
    fail_replacement(&tab, true, true)?;
    select(&tab, "ek-fixture", "reverse")?;
    fail_replacement(&tab, false, true)?;
    check_eq(root_id(&tab)?, "ek-1-g0-canonical-root".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} .ek-root"))?, 1)?;
    check_eq((position(&tab)?, last_step(&tab)?), (12, 86))?;
    check_eq(
        text(&tab, "document.getElementById('ek-fixture').value")?,
        "canonical".to_owned(),
    )?;
    check(!banner_hidden(&tab)?, "reported")?;
    click(&tab, "ek-next")?;
    check_eq(position(&tab)?, 13)?;
    check(banner_hidden(&tab)?, "repaired")?;
    select(&tab, "ek-fixture", "reverse")?;
    check_eq(root_id(&tab)?, "ek-1-g1-reverse-root".to_owned())
}

#[test]
fn a_failed_switch_pauses_the_playback_and_a_successful_one_does_too() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "ek-play")?;
    fail_replacement(&tab, true, false)?;
    select(&tab, "ek-fixture", "reverse")?;
    fail_replacement(&tab, false, false)?;
    check_eq((waiting(&tab)?, play_label(&tab)?), (0, "Play".to_owned()))?;
    click(&tab, "ek-play")?;
    select(&tab, "ek-fixture", "reverse")?;
    check_eq((waiting(&tab)?, play_label(&tab)?), (0, "Play".to_owned()))?;
    check_eq(
        attribute(&tab, &format!("{SVG} .ek-root"), "data-ek-fixture")?,
        "reverse".to_owned(),
    )
}

#[test]
fn a_switch_keeps_the_toggles_and_the_speed_clears_the_focus_and_moves_a_focused_graph_to_s() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "ek-toggle-residual")?;
    select(&tab, "ek-speed", "4")?;
    text(&tab, &format!("document.querySelector('{}').focus(), 'ok'", edge(4)))?;
    select(&tab, "ek-fixture", "reverse")?;
    check_eq(attribute(&tab, "#ek-toggle-residual", "aria-pressed")?, "false".to_owned())?;
    check_eq(text(&tab, "document.getElementById('ek-speed').value")?, "4".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} g.ek-res:not(.ek-off)"))?, 0)?;
    // The focus was in the old graph, so it goes to s in the new one, which is then the only thing marked as focused.
    check_eq(count(&tab, &format!("{SVG} g.ek-edge.ek-focused"))?, 0)?;
    check_eq(count(&tab, &format!("{SVG} g.ek-node.ek-focused"))?, 1)?;
    check_eq(attribute(&tab, &vertex("s"), "tabindex")?, "0".to_owned())?;
    check_eq(
        text(&tab, "document.activeElement.getAttribute('data-ek-vertex') || ''")?,
        "s".to_owned(),
    )?;
    check_eq(count(&tab, &format!("{SVG} g.ek-edge"))?, 7)
}

#[test]
fn a_page_that_was_diffed_step_by_step_is_the_same_as_one_drawn_in_full_at_every_step_and_after_any_seeks()
-> Result<(), String> {
    // The reference: every draw is a full one, and each step is reached by a seek.
    let fresh = {
        let tab = open_at("?always-full")?;
        let mut pages = Vec::with_capacity(87);
        for step in 0..=86 {
            if step > 0 {
                seek(&tab, step)?;
            }
            pages.push(page_html(&tab)?);
        }
        pages
    };
    let tab = open()?;
    check_eq(page_html(&tab)?, fresh[0].clone())?;
    for (step, _) in fresh.iter().enumerate().take(86 + 1).skip(1) {
        click(&tab, "ek-next")?;
        check(
            page_html(&tab)? == fresh[step],
            &format!("step {step}: a diff is not a full redraw"),
        )?;
    }
    // Then in any order, reached by seeks that jump far and back.
    let mut x = 987_654_321u64;
    for _ in 0..120 {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let step = (x >> 33) as usize % 87;
        seek(&tab, step)?;
        check(
            page_html(&tab)? == fresh[step],
            &format!("after a seek to {step}: a diff is not a full redraw"),
        )?;
    }
    Ok(())
}

#[test]
fn the_same_holds_for_the_reverse_graph_and_for_a_graph_that_was_switched_to() -> Result<(), String> {
    let fresh = {
        let tab = open_at("?always-full")?;
        select(&tab, "ek-fixture", "reverse")?;
        let mut pages = Vec::new();
        for step in 0..=43 {
            if step > 0 {
                seek(&tab, step)?;
            }
            pages.push(page_html(&tab)?);
        }
        pages
    };
    let tab = open()?;
    select(&tab, "ek-fixture", "reverse")?;
    for (step, _) in fresh.iter().enumerate().take(43 + 1) {
        if step > 0 {
            click(&tab, "ek-next")?;
        }
        let page = page_html(&tab)?;
        check(
            page == fresh[step],
            &format!("reverse step {step}: {}", first_difference(&page, &fresh[step])),
        )?;
    }
    Ok(())
}

// The end.

#[test]
fn the_manual_clocks_hooks_are_on_the_window_while_the_page_is_and_come_off_when_it_is_left() -> Result<(), String> {
    let tab = open_manual()?;
    let types = |tab: &Tab| {
        text(
            tab,
            "['__ekAdvance', '__ekAdvanceFrame', '__ekPending', '__ekPendingFrames'].map(n => typeof window[n]).join(',')",
        )
    };
    check_eq(types(&tab)?, "function,function,function,function".to_owned())?;
    click(&tab, "ek-play")?;
    check_eq(advance(&tab, 2)?, 2)?;
    text(&tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()")?;
    check_eq(types(&tab)?, "undefined,undefined,undefined,undefined".to_owned())?;
    let at = position(&tab)?;
    click(&tab, "ek-next")?;
    click(&tab, "ek-play")?;
    check_eq(position(&tab)?, at)
}

#[test]
fn a_page_with_the_real_clock_puts_no_hooks_on_the_window() -> Result<(), String> {
    let tab = open()?;
    check_eq(
        text(&tab, "typeof window.__ekAdvance + ',' + typeof window.__ekPending")?,
        "undefined,undefined".to_owned(),
    )
}
