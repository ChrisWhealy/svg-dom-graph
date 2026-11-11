//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The first step is the first quarter round's first `ADD`: before it, every operator is unreached.
#[test]
fn the_unstarted_walk_moves_to_the_first_step_of_the_first_quarter_round() -> Result<(), String> {
    let tab = open()?;
    check(position(&tab)?.is_none(), "the walk did not start unstarted")?;
    check(
        heading(&tab)?.contains("round 1 of 20: column quarter round 1 of 4"),
        &format!("unexpected heading {:?}", heading(&tab)?),
    )?;
    let first_add = operator(&tab, "ADD", 0)?;
    check(
        first_add.contains("not yet computed"),
        &format!("the first ADD was reached: {first_add}"),
    )?;

    click(&tab, "Next selection")?;
    check(position(&tab)? == Some(0), "Next did not move to position 0")?;
    let first_add = operator(&tab, "ADD", 0)?;
    // 0x61707865 + 0x03020100, the RFC 8439 block's constant and first key word.
    check(
        first_add.contains("64 72 79 65") && !first_add.contains("not yet computed"),
        &format!("the first ADD does not hold its result: {first_add}"),
    )?;
    check(
        operator(&tab, "XOR", 0)?.contains("not yet computed"),
        "the first XOR was reached too soon",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Position 11 is the first quarter round's last step, and 12 begins the second one, on the second column.
#[test]
fn position_12_begins_the_next_quarter_round() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 11)?;
    check(
        heading(&tab)?.ends_with("column quarter round 1 of 4"),
        &format!("unexpected heading at 11: {:?}", heading(&tab)?),
    )?;
    check(
        !operator(&tab, "XOR", 0)?.contains("not yet computed"),
        "the first quarter round was not complete at position 11",
    )?;
    let before = label(&tab, &format!("{DIAGRAM} g[aria-label^=\"state before\"]"))?;
    check(
        before.contains("highlighted: cells 0, 4, 8, 12"),
        &format!("column 0 at 11: {before}"),
    )?;

    click(&tab, "Next selection")?;
    check(position(&tab)? == Some(12), "did not move to 12")?;
    check(
        heading(&tab)?.ends_with("column quarter round 2 of 4"),
        &format!("unexpected heading at 12: {:?}", heading(&tab)?),
    )?;
    check(
        operator(&tab, "XOR", 0)?.contains("not yet computed"),
        "the second quarter round started with its first XOR already reached",
    )?;
    let before = label(&tab, &format!("{DIAGRAM} g[aria-label^=\"state before\"]"))?;
    check(
        before.contains("highlighted: cells 1, 5, 9, 13"),
        &format!("column 1 at 12: {before}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Position 47 ends the first round, which mixes columns, and 48 begins the second, which mixes diagonals.
#[test]
fn position_48_changes_from_a_column_round_to_a_diagonal_round() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 47)?;
    check(
        heading(&tab)?.contains("round 1 of 20: column quarter round 4 of 4"),
        &format!("unexpected heading at 47: {:?}", heading(&tab)?),
    )?;

    click(&tab, "Next selection")?;
    check(position(&tab)? == Some(48), "did not move to 48")?;
    check(
        heading(&tab)?.contains("round 2 of 20: diagonal quarter round 1 of 4"),
        &format!("unexpected heading at 48: {:?}", heading(&tab)?),
    )?;
    let before = label(&tab, &format!("{DIAGRAM} g[aria-label^=\"state before\"]"))?;
    check(
        before.contains("highlighted: cells 0, 5, 10, 15"),
        &format!("diagonal 0 at 48: {before}"),
    )
}

/// Crossing from the last quarter round to the keystream block redraws the whole scene. The view, the toolbar and the
/// keyboard focus must come through it, and through the redraw back again.
#[test]
fn the_keystream_block_redraw_keeps_the_view_the_toolbar_and_the_focus() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST_STEP)?;
    check(!is_keystream(&tab)?, "the keystream block was shown before its position")?;
    check(
        heading(&tab)?.contains("round 20 of 20"),
        &format!("unexpected heading at 959: {:?}", heading(&tab)?),
    )?;
    zoom_and_pan(&tab)?;
    let view_before = view(&tab)?;

    press(&tab, "Next selection")?;
    check(position(&tab)? == Some(KEYSTREAM), "did not move to 960")?;
    check(is_keystream(&tab)?, "the keystream block is not on show at 960")?;
    check(
        heading(&tab)?.contains("keystream block"),
        &format!("unexpected heading at 960: {:?}", heading(&tab)?),
    )?;
    check(
        view(&tab)? == view_before,
        &format!("the view changed: {view_before:?} -> {:?}", view(&tab)?),
    )?;
    check(
        focused(&tab)? == "Next selection",
        &format!("focus was lost, and is on {:?}", focused(&tab)?),
    )?;
    // The toolbar is rebuilt with the scene, so every button must be there and the walk must still be at 960.
    for name in [
        "Previous round",
        "Previous selection",
        "Next selection",
        "Next round",
        "Restart selection",
    ] {
        button(&tab, name)?;
    }

    press(&tab, "Previous selection")?;
    check(position(&tab)? == Some(LAST_STEP), "did not move back to 959")?;
    check(!is_keystream(&tab)?, "the keystream block is still on show at 959")?;
    check(
        heading(&tab)?.contains("round 20 of 20"),
        &format!("unexpected heading back at 959: {:?}", heading(&tab)?),
    )?;
    check(
        view(&tab)? == view_before,
        &format!("the view changed on the way back: {:?}", view(&tab)?),
    )?;
    check(
        focused(&tab)? == "Previous selection",
        &format!("focus was lost on the way back, and is on {:?}", focused(&tab)?),
    )?;
    // The quarter round must be whole again, with every value shown.
    check(
        !operator(&tab, "XOR", 0)?.contains("not yet computed"),
        "the redrawn quarter round lost its values",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Restart goes back to the unstarted walk from anywhere: a quarter round, a later round, or the keystream block.
#[test]
fn restart_returns_to_the_unstarted_diagram_from_any_position() -> Result<(), String> {
    let tab = open()?;
    for from in [30, KEYSTREAM] {
        goto(&tab, from)?;
        click(&tab, "Restart selection")?;
        check(
            position(&tab)?.is_none(),
            &format!("restart from {from} left the walk at {:?}", position(&tab)?),
        )?;
        check(
            !is_keystream(&tab)?,
            &format!("restart from {from} left the keystream block on show"),
        )?;
        check(
            heading(&tab)?.contains("round 1 of 20: column quarter round 1 of 4"),
            &format!("restart from {from} gave the heading {:?}", heading(&tab)?),
        )?;
        for index in 0..4 {
            let add = operator(&tab, "ADD", index)?;
            check(
                add.contains("u32 = 00 00 00 00") && add.contains("not yet computed"),
                &format!("restart from {from} left ADD {index} with a value: {add}"),
            )?;
        }
    }
    Ok(())
}

/// Dragging stays inside the diagram's own edges in both layouts, and after changing between them: the `<svg>` is
/// resized to fit each, so each must get its own limits.
#[test]
fn nodes_stay_inside_the_view_box_in_both_layouts_and_after_changing_between_them() -> Result<(), String> {
    let tab = open()?;
    drag_to_each_corner(&tab, "state before")?;

    goto(&tab, KEYSTREAM)?;
    drag_to_each_corner(&tab, "keystream block")?;

    goto(&tab, LAST_STEP)?;
    drag_to_each_corner(&tab, "state before")
}
