//! The walk itself: what each position shows, and that stepping back restores it.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The walk starts by counting: five trees of one node, the counts in the table, and nothing else filled in.
#[test]
fn the_walk_starts_by_counting_with_five_single_node_trees() -> Result<(), String> {
    let tab = open()?;
    check_eq(position(&tab)?, Some(0))?;
    check(
        explanation(&tab)?.starts_with("Counted 11 bytes, 5 of them distinct"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(forest(&tab)?, (5, 0))?;
    check_eq(status(&tab)?, "11 bytes, 5 distinct, 4 merges.".to_owned())?;
    // The commonest byte first, and bytes that occur as often as each other in byte order.
    check_eq(
        table(&tab)?,
        vec![
            row("a", "5", "–", "–", "–"),
            row("b", "2", "–", "–", "–"),
            row("r", "2", "–", "–", "–"),
            row("c", "1", "–", "–", "–"),
            row("d", "1", "–", "–", "–"),
        ],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The first merge joins `c` and `d`, the two lightest, and draws the new branch with its two labelled links.
#[test]
fn the_first_merge_joins_the_two_lightest_trees() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "Next selection")?;
    check_eq(position(&tab)?, Some(1))?;
    check(
        explanation(&tab)?
            .starts_with("Merge 1 of 4: the two lightest trees are 'c' (1) and 'd' (1). Joined, they weigh 2."),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(forest(&tab)?, (5, 1))?;
    // Both links of the one branch are labelled, left 0 and right 1.
    let all = texts(&tab)?;
    let forest_boxes = &all[..all.iter().position(|t| t == "Byte").ok_or("the table has no heading")?];
    check_eq(forest_boxes.iter().filter(|t| *t == "0").count(), 1)?;
    check_eq(forest_boxes.iter().filter(|t| *t == "1").count(), 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// After the last merge there is one tree of nine boxes, and the table still has no lengths.
#[test]
fn the_last_merge_leaves_one_tree() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 4)?;
    check(
        explanation(&tab)?.starts_with(
            "Merge 4 of 4: the two lightest trees are 'a' (5) and the tree of weight 6. Joined, they weigh 11.",
        ),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    // Five leaves and four branches, with eight links each labelled.
    check_eq(forest(&tab)?, (5, 4))?;
    check(
        table(&tab)?.iter().all(|r| r[2..] == ["–", "–", "–"]),
        "the table was filled in too soon",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reading the lengths fills in the tree paths and lengths, and no canonical code yet. Then each step adds one code.
#[test]
fn the_lengths_come_before_the_canonical_codes_which_come_one_at_a_time() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 5)?;
    check(
        explanation(&tab)?.starts_with("Each code's length is the depth of its leaf"),
        "lengths explanation",
    )?;
    check_eq(
        table(&tab)?,
        vec![
            row("a", "5", "0", "1", "–"),
            row("b", "2", "110", "3", "–"),
            row("r", "2", "111", "3", "–"),
            row("c", "1", "100", "3", "–"),
            row("d", "1", "101", "3", "–"),
        ],
    )?;

    goto(&tab, 6)?;
    check(
        explanation(&tab)?.starts_with("Canonical codes start at all zeros: 'a' has length 1 and code 0."),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(table(&tab)?[0][4].as_str(), "0")?;
    check_eq(table(&tab)?[1][4].as_str(), "–")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The last position has every canonical code. Where the tree's path and the canonical code differ, both are shown.
#[test]
fn the_last_step_has_every_canonical_code_and_shows_where_they_differ_from_the_tree() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST)?;
    check_eq(
        table(&tab)?,
        vec![
            row("a", "5", "0", "1", "0"),
            row("b", "2", "110", "3", "100"),
            row("r", "2", "111", "3", "111"),
            row("c", "1", "100", "3", "101"),
            row("d", "1", "101", "3", "110"),
        ],
    )?;
    // Pass one ends here, but the walk goes on into pass two.
    click(&tab, "Next selection")?;
    check_eq(position(&tab)?, Some(LAST + 1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A step back from the end draws exactly what a step forward to the same position did.
#[test]
fn stepping_back_restores_what_stepping_forward_drew() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 3)?;
    let forward = (explanation(&tab)?, forest(&tab)?, table(&tab)?, marked(&tab)?);

    goto(&tab, LAST)?;
    goto(&tab, 3)?;
    check_eq((explanation(&tab)?, forest(&tab)?, table(&tab)?, marked(&tab)?), forward)?;

    goto(&tab, 0)?;
    check(explanation(&tab)?.starts_with("Counted 11"), "back at the start")?;
    check_eq(forest(&tab)?, (5, 0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The demo opens on the first sentence of *Pride and Prejudice*, with its punctuation: 26 distinct bytes in 116.
#[test]
fn the_demo_opens_on_the_first_sentence_of_pride_and_prejudice() -> Result<(), String> {
    let tab = open_default()?;
    check_eq(status(&tab)?, "116 bytes, 26 distinct, 25 merges.".to_owned())?;
    check_eq(position(&tab)?, Some(0))?;
    check_eq(forest(&tab)?, (26, 0))?;
    let rows = table(&tab)?;
    check_eq(rows.len(), 26)?;
    // The space is 22 of the 116 bytes, so it is first. The comma, the full stop and the capital letter are bytes like
    // any other, and occur once each.
    check_eq(rows[0].clone(), row("space", "22", "–", "–", "–"))?;
    let counts: Vec<(&str, &str)> = rows.iter().map(|r| (r[0].as_str(), r[1].as_str())).collect();
    for expected in [(",", "1"), (".", "1"), ("I", "1"), ("a", "9"), ("n", "9")] {
        check(counts.contains(&expected), &format!("{expected:?} is not in {counts:?}"))?;
    }
    // Most often first, so the counts never go up.
    let numbers: Vec<u32> = rows.iter().filter_map(|r| r[1].parse().ok()).collect();
    check(
        numbers.windows(2).all(|w| w[0] >= w[1]),
        &format!("not in frequency order: {numbers:?}"),
    )?;
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.len(),
        116,
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every merge of the long sentence works, ends in one tree of 26 leaves and 25 branches, and then every byte has a
/// code.
#[test]
fn the_long_sentence_walks_from_26_trees_to_one_and_then_to_26_codes() -> Result<(), String> {
    let tab = open_default()?;
    goto(&tab, 25)?;
    check_eq(forest(&tab)?, (26, 25))?;
    check(
        explanation(&tab)?.starts_with("Merge 25 of 25"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;

    goto(&tab, 52)?;
    check_eq(position(&tab)?, Some(52))?;
    let rows = table(&tab)?;
    check(
        rows.iter().all(|r| r[2..].iter().all(|c| c != "–")),
        "a byte has no code at the last position",
    )?;
    // The space is by far the commonest byte, so it has the shortest code and its canonical code is all zeros.
    check(
        rows[0][4].chars().all(|c| c == '0'),
        &format!("the space's code is {:?}", rows[0][4]),
    )
}
