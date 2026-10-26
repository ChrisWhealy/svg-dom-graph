//! The route through the tree that is marked as each canonical code is assigned.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// As each canonical code is assigned, the route from the root to that byte's leaf is marked with the library's
/// secondary selection: the boxes on it, and the label of each of its links. Nothing is marked at any other step.
#[test]
fn the_route_to_each_byte_is_marked_as_its_canonical_code_is_assigned() -> Result<(), String> {
    let tab = open()?;
    for position in 0..=5 {
        goto(&tab, position)?;
        check_eq(marked(&tab)?, Vec::<String>::new())?;
    }

    // 'a' is the root's left child: the root, `a`, and the one link, labelled 0.
    goto(&tab, 6)?;
    check_eq(marked(&tab)?, ["0", "11", "a x5"].map(str::to_owned).to_vec())?;

    // 'b' is 110: right, right, left. The root, 6 and 4 are on the route, and so is `b`, with links 1, 1 and 0.
    goto(&tab, 7)?;
    check_eq(
        marked(&tab)?,
        ["0", "1", "1", "11", "4", "6", "b x2"].map(str::to_owned).to_vec(),
    )?;

    // 'c' is 100: right, left, left. The root, 6 and 2, and `c`, with links 1, 0 and 0.
    goto(&tab, 8)?;
    check_eq(
        marked(&tab)?,
        ["0", "0", "1", "11", "2", "6", "c x1"].map(str::to_owned).to_vec(),
    )?;

    // Going back clears the old route and marks the new one: it is never added to.
    goto(&tab, 6)?;
    check_eq(marked(&tab)?, ["0", "11", "a x5"].map(str::to_owned).to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The route's boxes are the right number deep for the byte's code length, in the long sentence too: each step marks a
/// box for every level down to the leaf, and a label for every link.
#[test]
fn a_marked_route_has_a_box_for_every_level_and_a_label_for_every_link() -> Result<(), String> {
    let tab = open_default()?;
    goto(&tab, 27)?;
    // Position 27 gives the first canonical code, to the byte with the shortest code. Its row is ringed and its length
    // is the number of links on the route.
    let rows = table(&tab)?;
    let length: usize = rows[0][3].parse().map_err(|_| "no length")?;
    let marked = marked(&tab)?;
    // The route has `length + 1` boxes and `length` labels, which read 0 or 1.
    let labels = marked.iter().filter(|t| *t == "0" || *t == "1").count();
    check_eq(labels, length)?;
    check_eq(marked.len(), 2 * length + 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every link has room for an arrowhead on each side of its `0` or `1` label.
///
/// The arrowhead is 10 pixels long and needs 6 more of clearance, and the link has to be long enough to show a slant
/// clearly. So the gap above a label, and the gap below it, must each be at least twice that: 32 pixels. The rectangles
/// are read from the rendered boxes, so this checks what is drawn, not the constants that drew it.
#[test]
fn every_link_has_room_for_an_arrowhead() -> Result<(), String> {
    const MINIMUM_GAP: f64 = 32.0;
    let tab = open_default()?;
    goto(&tab, 25)?;

    // Every box in the forest, as [text, horizontal middle, top, height], in the scene's own units.
    let json = text(
        &tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} text')].map(t => {{ \
                 const r = t.closest('g')?.querySelector('rect'); \
                 if (!r) return null; \
                 const b = r.getBBox(); \
                 return [t.textContent, b.x + b.width / 2, b.y, b.height]; \
             }}).filter(Boolean))"
        ),
    )?;
    let boxes: Vec<(String, f64, f64, f64)> =
        serde_json::from_str(&json).map_err(|e| format!("could not read the boxes: {e}"))?;
    let end = boxes.iter().position(|b| b.0 == "Byte").ok_or("the table has no heading")?;
    let forest = &boxes[..end];

    // A link's label is a box of one bit. A branch's weight is at least 2, so only a label reads 0 or 1.
    let labels: Vec<_> = forest.iter().filter(|b| b.0 == "0" || b.0 == "1").collect();
    check_eq(labels.len(), 50)?;
    for (_, x, top, height) in labels {
        let bottom = top + height;
        // The box straight below the label is the child it points at.
        let below = forest
            .iter()
            .filter(|b| (b.1 - x).abs() < 0.5 && b.2 >= bottom - 0.01)
            .map(|b| b.2 - bottom)
            .fold(f64::INFINITY, f64::min);
        // The lowest box wholly above the label is on the parent's level.
        let above = forest
            .iter()
            .filter(|b| b.2 + b.3 <= *top + 0.01)
            .map(|b| top - (b.2 + b.3))
            .fold(f64::INFINITY, f64::min);
        check(
            below >= MINIMUM_GAP - 0.5,
            &format!("only {below} pixels below the label at x={x}"),
        )?;
        check(
            above >= MINIMUM_GAP - 0.5,
            &format!("only {above} pixels above the label at x={x}"),
        )?;
    }
    Ok(())
}
