//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens on the reference graph with the walk at its start. Every edge has a label with its weight, in the
/// order of the edges, and named for assistive technology. The distances are `0` and `∞`, drawn as text, the predecessors
/// are dashes, nothing is settled, and the queue holds the source, with the rest of its rows empty and read out as such.
#[test]
fn the_panel_opens_on_the_graph_with_labelled_edges_and_arrays_of_text() -> Result<(), String> {
    let tab = open()?;
    check_eq(vertex_boxes(&tab)?.len(), 6)?;
    let labels = labels(&tab)?;
    check_eq(labels.iter().map(|l| l.index).collect::<Vec<_>>(), (0..WEIGHTS.len()).collect())?;
    check_eq(labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), WEIGHTS.to_vec())?;
    check(
        labels
            .iter()
            .all(|l| l.name.starts_with("Edge from ") && l.name.ends_with(&format!(": {}", l.text))),
        &format!(
            "each label is named for its edge: {:?}",
            labels.iter().map(|l| &l.name).collect::<Vec<_>>()
        ),
    )?;
    check_eq(labels[0].name.as_str(), "Edge from A to B: 4")?;

    check_eq(
        cells(&tab, "Distance")?,
        ["0", "\u{221E}", "\u{221E}", "\u{221E}", "\u{221E}", "\u{221E}"]
            .map(String::from)
            .to_vec(),
    )?;
    check_eq(cells(&tab, "Previous")?, vec!["\u{2014}".to_owned(); 6])?;
    check_eq(cells(&tab, "Settled")?, vec!["false".to_owned(); 6])?;
    let queue = cells(&tab, "Queue, nearest first")?;
    check_eq(queue.len(), 9)?;
    check_eq(queue[0].as_str(), "(0, A)")?;
    check(
        queue[1..].iter().all(String::is_empty),
        &format!("the rest of the queue is empty: {queue:?}"),
    )?;
    let names = cell_names(&tab, "Queue, nearest first")?;
    check(
        names[1..].iter().all(|n| n.ends_with(": empty")),
        &format!("empty cells are read out as empty: {names:?}"),
    )?;
    check(
        cell_names(&tab, "Distance")?[1].ends_with(": \u{221E}"),
        "the unreached mark is in the name",
    )?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(said(&tab)?.starts_with("Step 1 of "), "the first step is announced")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every label is clear of the boxes, of the other labels and of its own line, at rest. Then C is dragged with a real
/// mouse through several places, and C to D and C to E, whose routes leave C together, keep their labels apart and clear
/// of the lines, which have followed.
#[test]
fn labels_stay_beside_their_connectors_and_apart_at_rest_and_while_a_vertex_is_dragged() -> Result<(), String> {
    let tab = open()?;
    let check_all = |when: &str| -> Result<(), String> {
        let (boxes, labels) = (vertex_boxes(&tab)?, labels(&tab)?);
        for (i, label) in labels.iter().enumerate() {
            let line = line(&tab, i)?;
            check(
                !line.iter().any(|&(x, y)| {
                    x > label.area.left
                        && x < label.area.left + label.area.width
                        && y > label.area.top
                        && y < label.area.top + label.area.height
                }),
                &format!("{when}: the line of edge {i} passes through its label {:?}", label.area),
            )?;
            for (v, &vertex) in boxes.iter().enumerate() {
                check(
                    !label.area.overlaps(vertex),
                    &format!("{when}: the label of edge {i} is on {}", VERTICES[v]),
                )?;
            }
            for (j, other) in labels.iter().enumerate().skip(i + 1) {
                check(!label.area.overlaps(other.area), &format!("{when}: labels {i} and {j} overlap"))?;
            }
        }
        Ok(())
    };
    check_all("at rest")?;

    for (to, name) in [
        ((0.0, -90.0), "C moved up"),
        ((60.0, -40.0), "C moved up and right"),
        ((0.0, 0.0), "C back"),
    ] {
        let c = vertex_boxes(&tab)?[2].centre();
        drag(&tab, &[c, (c.0 + to.0 / 2.0, c.1 + to.1 / 2.0), (c.0 + to.0, c.1 + to.1)])?;
        std::thread::sleep(Duration::from_millis(150));
        check_all(name)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A step that examines an edge emphasises that edge and no other, and rings the two vertices it joins. The arrays have
/// no focus ring, and the vertices no selection: no node has both. At the last step every edge of the route to F is
/// emphasised at once, and no other, and the vertices are all settled.
#[test]
fn the_examined_edge_and_then_the_whole_route_are_emphasised() -> Result<(), String> {
    let tab = open()?;
    check(
        stroke_widths(&tab)?.iter().all(|w| w == "1.5"),
        "no edge is emphasised at the start",
    )?;
    check(ringed(&tab)?.is_empty(), "no vertex is ringed at the start")?;

    goto_note(&tab, "Edge A to B")?;
    let thick: Vec<usize> = stroke_widths(&tab)?
        .iter()
        .enumerate()
        .filter(|(_, w)| *w == "3.5")
        .map(|(i, _)| i)
        .collect();
    check_eq(thick, vec![0])?;
    check_eq(ringed(&tab)?, vec!["A".to_owned(), "B".to_owned()])?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelectorAll('{DIAGRAM} path[aria-current=\"true\"]').length.toString()"),
        )?
        .as_str(),
        "1",
    )?;
    // A data node takes a selection and a plain one a ring, and never both: no array has a ring.
    let ring_on_array = text(
        &tab,
        &format!(
            "[...document.querySelectorAll('{DIAGRAM} g[aria-label*=\"data grid\"] rect')].some(r => r.getAttribute('stroke-width') === '3').toString()"
        ),
    )?;
    check_eq(ring_on_array.as_str(), "false")?;

    let last = step_count(&tab)? - 1;
    goto(&tab, last)?;
    let thick: Vec<usize> = stroke_widths(&tab)?
        .iter()
        .enumerate()
        .filter(|(_, w)| *w == "3.5")
        .map(|(i, _)| i)
        .collect();
    check_eq(thick, ROUTE.to_vec())?;
    check(
        said(&tab)?.contains("A to C to B to D to E to F"),
        &format!("the route is said: {:?}", said(&tab)?),
    )?;
    check(said(&tab)?.contains("length of 13"), "and its length")?;
    check_eq(
        cells(&tab, "Distance")?,
        ["0", "3", "2", "8", "10", "13"].map(String::from).to_vec(),
    )?;
    check_eq(
        cells(&tab, "Previous")?,
        ["\u{2014}", "C", "A", "B", "D", "E"].map(String::from).to_vec(),
    )?;
    check_eq(filled_settled(&tab)?.len(), 6)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A step draws the scene again, and keeps the zoom, where a vertex was dragged to, and the keyboard focus on the
/// toolbar button that had it, and does the same going back.
#[test]
fn a_redraw_keeps_the_zoom_the_dragged_vertex_and_the_keyboard_focus() -> Result<(), String> {
    let tab = open()?;
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"Zoom in\"]"))
        .map_err(|e| format!("could not find Zoom in: {e}"))?
        .click()
        .map_err(|e| format!("could not zoom in: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in changes the view")?;

    let b = vertex_boxes(&tab)?[1].centre();
    drag(&tab, &[b, (b.0 + 30.0, b.1 + 20.0), (b.0 + 60.0, b.1 + 40.0)])?;
    std::thread::sleep(Duration::from_millis(150));
    let moved = vertex_boxes(&tab)?[1];
    check(
        (moved.centre().0 - (b.0 + 60.0)).abs() < 1.5,
        &format!("B was dragged: {moved:?}"),
    )?;

    tab.evaluate(
        &format!("document.querySelector('{DIAGRAM} g[aria-label=\"Next selection\"]').focus()"),
        false,
    )
    .map_err(|e| format!("could not focus Next: {e}"))?;
    let from = step_position(&tab)?;
    tab.press_key("Enter").map_err(|e| format!("could not press Enter: {e}"))?;
    wait_for_move(&tab, from)?;

    check_eq(view(&tab)?, zoomed.clone())?;
    let after = vertex_boxes(&tab)?[1];
    check(
        (after.left - moved.left).abs() < 0.5 && (after.top - moved.top).abs() < 0.5,
        &format!("B went back to where it was dragged: {moved:?} then {after:?}"),
    )?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('aria-label') ?? 'none'")?.as_str(),
        "Next selection",
    )?;
    goto(&tab, 0)?;
    check_eq(view(&tab)?, zoomed)?;
    let back = vertex_boxes(&tab)?[1];
    check(
        (back.left - moved.left).abs() < 0.5 && (back.top - moved.top).abs() < 0.5,
        "and again going back",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Nothing but the note moves or changes size from one step to the next: the three arrays, the queue and every vertex
/// are exactly where they were at the start in every step, however much the text in them changes.
#[test]
fn the_arrays_the_queue_and_the_vertices_never_move_or_change_size() -> Result<(), String> {
    let tab = open()?;
    let before = placed(&tab)?;
    check_eq(before.len(), 4 + VERTICES.len())?;
    let count = step_count(&tab)?;
    for position in 1..count {
        click(&tab, "Next selection")?;
        check(
            same_places(&before, &placed(&tab)?),
            &format!("an array, the queue or a vertex moved or resized at step {position}"),
        )?;
    }
    check_eq(step_position(&tab)?, Some(count - 1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Restart" gives the graph new random weights from 1 to 10 and goes back to the first step, with the run worked out on
/// them. The panel opens on the reference weights, so every visit starts the same, and stepping back does not change
/// them.
#[test]
fn restart_gives_the_graph_new_random_weights() -> Result<(), String> {
    let tab = open()?;
    let weights = |tab: &Tab| -> Result<Vec<String>, String> { Ok(labels(tab)?.into_iter().map(|l| l.text).collect()) };
    check_eq(weights(&tab)?, WEIGHTS.map(String::from).to_vec())?;

    // Stepping forwards and back keeps the weights.
    click(&tab, "Next selection")?;
    click(&tab, "Next selection")?;
    click(&tab, "Previous selection")?;
    check_eq(weights(&tab)?, WEIGHTS.map(String::from).to_vec())?;

    // Restart changes them, so far from the reference that the chance of the same nine is one in a billion.
    let moved = vertex_boxes(&tab)?;
    click(&tab, "Restart selection")?;
    check_eq(step_position(&tab)?, Some(0))?;
    let after = weights(&tab)?;
    check_eq(after.len(), WEIGHTS.len())?;
    check(
        after.iter().all(|w| w.parse::<u32>().is_ok_and(|w| (1..=10).contains(&w))),
        &format!("every new weight is from 1 to 10: {after:?}"),
    )?;
    check(
        after != WEIGHTS.map(String::from).to_vec(),
        "restart gave the reference weights again",
    )?;
    check(
        said(&tab)?.starts_with("The edge weights are new."),
        "the new weights are announced",
    )?;

    // The run starts again from A, and the graph is still where it was.
    check_eq(cells(&tab, "Distance")?[0].as_str(), "0")?;
    let again = vertex_boxes(&tab)?;
    check(
        again
            .iter()
            .zip(&moved)
            .all(|(a, b)| (a.left - b.left).abs() < 0.5 && (a.top - b.top).abs() < 0.5),
        "the vertices stayed where they were",
    )?;

    // Walking the whole run works on the new weights, and ends at F.
    let count = step_count(&tab)?;
    goto(&tab, count - 1)?;
    check(
        said(&tab)?.contains("shortest route"),
        "the run on the new weights finds its route",
    )?;
    check(
        !ringed(&tab)?.is_empty() || !filled_settled(&tab)?.is_empty(),
        "the end of the run is drawn",
    )
}
