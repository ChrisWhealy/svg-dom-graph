//! The walk against "abracadabra", worked by hand: the merges leave leaves `a`=0 `b`=1 `c`=2 `d`=3 `r`=4, then `c`+`d` =
//! node 5, `b`+`r` = node 6, node 5 + node 6 = node 7, and `a` + node 7 = the root, node 8.

use super::{
    super::algorithm::{Node, trace},
    *,
};

fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn abracadabra() -> Result<Trace, String> {
    trace(b"abracadabra").ok_or_else(|| "the input is encodable".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Positions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_walk_has_one_step_for_counting_each_merge_the_lengths_and_each_code() -> Result<(), String> {
    // 1 counting step, 4 merges, 1 lengths step, 5 codes.
    check_eq(steps(&abracadabra()?), 11)
}

#[test]
fn a_position_past_the_end_is_the_last() -> Result<(), String> {
    let t = abracadabra()?;
    check_eq(shown(&t, 500).caption, shown(&t, 10).caption)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The forest
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn counting_starts_with_every_byte_as_its_own_tree_lightest_first() -> Result<(), String> {
    // Weights: a=5 b=2 c=1 d=1 r=2. Lightest first, ties to the lower index: c, d, b, r, a.
    let s = shown(&abracadabra()?, 0);
    check_eq(s.roots, vec![2, 3, 1, 4, 0])?;
    check_eq(s.merged, None)
}

#[test]
fn each_merge_replaces_two_roots_with_their_parent() -> Result<(), String> {
    let t = abracadabra()?;
    let roots: Vec<Vec<usize>> = (0..=4).map(|p| shown(&t, p).roots).collect();
    check_eq(
        roots,
        vec![
            vec![2, 3, 1, 4, 0],
            // c+d = node 5 (weight 2) goes after b and r, which are older.
            vec![1, 4, 5, 0],
            // b+r = node 6 (weight 4). Node 5 (weight 2) is now lightest.
            vec![5, 6, 0],
            // 5+6 = node 7 (weight 6), heavier than a (5).
            vec![0, 7],
            vec![8],
        ],
    )
}

#[test]
fn the_merge_step_names_the_node_it_made() -> Result<(), String> {
    let t = abracadabra()?;
    check_eq(shown(&t, 1).merged.map(|m| (m.left, m.right, m.parent)), Some((2, 3, 5)))?;
    check_eq(shown(&t, 4).merged.map(|m| m.parent), Some(8))?;
    check_eq(shown(&t, 5).merged, None)
}

#[test]
fn the_forest_stays_one_tree_after_the_last_merge() -> Result<(), String> {
    let t = abracadabra()?;
    for position in 4..steps(&t) {
        check_eq(shown(&t, position).roots, vec![8])?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The table
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_table_holds_counts_only_until_the_lengths_are_read_and_the_commonest_byte_comes_first() -> Result<(), String> {
    let t = abracadabra()?;
    for position in 0..=4 {
        for row in shown(&t, position).rows {
            check_eq((row.path, row.length, row.canonical), (None, None, None))?;
        }
    }
    // Most often first, and bytes that occur as often as each other in byte order: a=5, then b=2 and r=2, then c=1 and d=1.
    let counts: Vec<(u8, u32)> = shown(&t, 0).rows.iter().map(|r| (r.symbol, r.count)).collect();
    check_eq(counts, vec![(b'a', 5), (b'b', 2), (b'r', 2), (b'c', 1), (b'd', 1)])
}

#[test]
fn the_table_stays_in_frequency_order_at_every_step() -> Result<(), String> {
    for text in [&b"abracadabra"[..], b"the quick brown fox jumps over the lazy dog", b"aaaa"] {
        let t = trace(text).ok_or("encodable")?;
        for position in 0..steps(&t) {
            let rows = shown(&t, position).rows;
            let key = |r: &Row| (std::cmp::Reverse(r.count), r.symbol);
            check(
                rows.windows(2).all(|w| key(&w[0]) < key(&w[1])),
                &format!("rows out of order at position {position}"),
            )?;
        }
    }
    Ok(())
}

#[test]
fn reading_the_lengths_shows_the_tree_paths_and_no_canonical_codes() -> Result<(), String> {
    let s = shown(&abracadabra()?, 5);
    let paths: Vec<(u8, Option<String>, Option<u8>)> =
        s.rows.iter().map(|r| (r.symbol, r.path.clone(), r.length)).collect();
    // From the tree built above: a=0, c=100, d=101, b=110, r=111. The rows are in frequency order: a, b, r, c, d.
    let expected = [
        (b'a', "0", 1),
        (b'b', "110", 3),
        (b'r', "111", 3),
        (b'c', "100", 3),
        (b'd', "101", 3),
    ]
    .map(|(s, p, l)| (s, Some(p.to_owned()), Some(l)))
    .to_vec();
    check_eq(paths, expected)?;
    check(s.rows.iter().all(|r| r.canonical.is_none()), "no canonical code yet")?;
    check_eq(s.focus_row, None)?;
    check_eq(s.route, Vec::<usize>::new())
}

#[test]
fn canonical_codes_appear_one_at_a_time_in_canonical_order() -> Result<(), String> {
    let t = abracadabra()?;
    // Canonical order is a, b, c, d, r. In the table, which is in frequency order, those rows are 0, 1, 3, 4 and 2.
    let table_rows = [0, 1, 3, 4, 2];
    for position in 6..=10 {
        let s = shown(&t, position);
        let assigned = position - 5;
        check_eq(s.rows.iter().filter(|r| r.canonical.is_some()).count(), assigned)?;
        check_eq(s.focus_row, Some(table_rows[assigned - 1]))?;
    }
    let codes: Vec<(u8, Option<String>)> = shown(&t, 10).rows.iter().map(|r| (r.symbol, r.canonical.clone())).collect();
    check_eq(
        codes,
        [(b'a', "0"), (b'b', "100"), (b'r', "111"), (b'c', "101"), (b'd', "110")]
            .map(|(s, c)| (s, Some(c.to_owned())))
            .to_vec(),
    )
}

#[test]
fn the_route_to_the_byte_just_given_a_code_runs_from_the_root_to_its_leaf() -> Result<(), String> {
    let t = abracadabra()?;
    // Leaves a=0 b=1 c=2 d=3 r=4, then c+d = 5, b+r = 6, 5+6 = 7, and a+7 = the root, 8.
    check_eq(shown(&t, 6).route, vec![8, 0])?;
    check_eq(shown(&t, 7).route, vec![8, 7, 6, 1])?;
    check_eq(shown(&t, 8).route, vec![8, 7, 5, 2])?;
    check_eq(shown(&t, 10).route, vec![8, 7, 6, 4])?;
    // No route before the first code, and none while the tree is being built.
    for position in 0..=5 {
        check(shown(&t, position).route.is_empty(), &format!("a route at position {position}"))?;
    }
    Ok(())
}

#[test]
fn the_canonical_codes_differ_from_the_tree_paths_where_the_tree_put_b_before_c() -> Result<(), String> {
    let rows = shown(&abracadabra()?, 10).rows;
    let differing: Vec<u8> = rows.iter().filter(|r| r.path != r.canonical).map(|r| r.symbol).collect();
    check_eq(differing, vec![b'b', b'c', b'd'])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Captions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_counting_caption_says_what_was_counted_and_what_comes_next() -> Result<(), String> {
    let caption = shown(&abracadabra()?, 0).caption;
    check_eq(caption.len(), 3)?;
    check_eq(
        caption[0].as_str(),
        "Counted 11 bytes, 5 of them distinct. Each distinct byte is now a tree of one node.",
    )?;
    check(
        caption[2].contains("rare bytes end up deepest"),
        "the caption says why the lightest merge first",
    )
}

#[test]
fn a_merge_caption_numbers_the_merge_and_names_the_trees_it_joined() -> Result<(), String> {
    let t = abracadabra()?;
    check_eq(
        shown(&t, 1).caption[0].as_str(),
        "**Merge 1 of 4**: the two lightest trees are 'c' (1) and 'd' (1). Joined, they weigh 2.",
    )?;
    check_eq(
        shown(&t, 3).caption[0].as_str(),
        "**Merge 3 of 4**: the two lightest trees are the tree of weight 2 and the tree of weight 4. Joined, they weigh 6.",
    )?;
    // Every merge says which side is 0 and which is 1.
    for position in 1..=4 {
        check(
            shown(&t, position).caption[1].starts_with("The lighter goes left (0)"),
            "the left and right rule",
        )?;
    }
    Ok(())
}

#[test]
fn a_merge_caption_counts_the_trees_left_and_ends_with_the_complete_tree() -> Result<(), String> {
    let t = abracadabra()?;
    check_eq(shown(&t, 1).caption[2].as_str(), "4 trees remain.")?;
    check_eq(shown(&t, 3).caption[2].as_str(), "Two trees remain.")?;
    check_eq(shown(&t, 4).caption[2].as_str(), "One tree remains: the code tree is complete.")
}

#[test]
fn a_merge_caption_mentions_the_tie_rule_only_when_a_third_tree_ties_with_the_second() -> Result<(), String> {
    let t = abracadabra()?;
    // Before merge 2 the trees weigh 2, 2, 2 and 5: 'b' and 'r' are taken, and the tree made last also weighs 2.
    check(
        shown(&t, 2).caption[2].starts_with("Another tree also weighs 2. Ties go to the tree made first"),
        "tie",
    )?;
    // Before merge 1 the trees weigh 1, 1, 2, 2 and 5. The third is heavier than the second, so nothing was tied.
    check(!shown(&t, 1).caption[2].contains("Ties"), "no tie at merge 1")
}

#[test]
fn the_lengths_caption_defines_a_length_and_ends_with_what_comes_next() -> Result<(), String> {
    let caption = shown(&abracadabra()?, 5).caption;
    check_eq(caption.len(), 3)?;
    check(
        caption[0].starts_with("Each code's length is the depth of its leaf"),
        "the definition",
    )?;
    check(
        caption[1].contains("0 for each left link and 1 for each right link"),
        "the path",
    )?;
    check(caption[2].contains("Only these lengths are needed"), "what comes next")
}

#[test]
fn a_canonical_caption_works_the_code_out_and_compares_it_with_the_tree() -> Result<(), String> {
    let t = abracadabra()?;
    // 'a' is first: all zeros. Its tree path is also 0.
    let first = shown(&t, 6).caption;
    check_eq(
        first[0].as_str(),
        "Canonical codes start at all zeros: 'a' has length 1 and code 0.",
    )?;
    check_eq(first[1].as_str(), "The tree's own path for 'a' happens to be the same.")?;
    check(first[2].starts_with("Codes are handed out shortest first"), "the ordering rule")?;
    // 'b' follows 'a': the previous code 0, plus 1, shifted left by the 2 bits it is longer by, is 100. The tree said 110.
    let second = shown(&t, 7).caption;
    check_eq(
        second[0].as_str(),
        "'b' has length 3, so its code is the previous code 0 plus 1, shifted left 2: 100.",
    )?;
    check_eq(
        second[1].as_str(),
        "The tree's own path for 'b' was 110. Both are valid, and both are 3 bits long.",
    )?;
    check_eq(second.len(), 2)?;
    check_eq(
        shown(&t, 8).caption[0].as_str(),
        "'c' has length 3, so its code is the previous code 100 plus 1: 101.",
    )?;
    // The last code says why only the lengths matter.
    check(
        shown(&t, 10).caption[2].contains("a decoder never needs the tree"),
        "the last step",
    )
}

#[test]
fn bytes_are_labelled_as_text_when_they_are_printable_and_as_hexadecimal_when_not() -> Result<(), String> {
    check_eq(symbol_label(b'a'), "a".to_owned())?;
    check_eq(symbol_label(b' '), "space".to_owned())?;
    check_eq(symbol_label(b'~'), "~".to_owned())?;
    check_eq(symbol_label(0x00), "0x00".to_owned())?;
    check_eq(symbol_label(0x7f), "0x7F".to_owned())?;
    check_eq(symbol_label(0xc3), "0xC3".to_owned())
}

#[test]
fn one_distinct_byte_has_nothing_to_merge_and_a_length_of_one() -> Result<(), String> {
    let t = trace(b"aaaa").ok_or("encodable")?;
    // Counted, lengths, one code.
    check_eq(steps(&t), 3)?;
    check_eq(shown(&t, 0).roots, vec![0])?;
    let lengths = shown(&t, 1);
    check_eq(
        (lengths.rows[0].path.clone(), lengths.rows[0].length),
        (Some(String::new()), Some(1)),
    )?;
    check(
        lengths.caption[0].contains("It is given length 1"),
        "the caption explains the forced length",
    )?;
    check_eq(shown(&t, 2).rows[0].canonical.clone(), Some("0".to_owned()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Layout
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_branch_is_centred_over_its_children_and_sits_one_level_higher() -> Result<(), String> {
    let t = abracadabra()?;
    // The first merge: leaves c (2) and d (3) under node 5.
    let (placed, _) = layout(&t.tree.nodes, &[5]);
    let at = |node: usize| placed.iter().find(|p| p.node == node).copied().ok_or("placed");
    let (c, d, parent) = (at(2)?, at(3)?, at(5)?);
    check_eq(parent.x, (c.x + d.x) / 2.0)?;
    check_eq((c.height, d.height, parent.height), (0, 0, 1))
}

#[test]
fn leaves_keep_the_order_a_left_to_right_walk_meets_them() -> Result<(), String> {
    let t = abracadabra()?;
    let (placed, _) = layout(&t.tree.nodes, &[8]);
    let mut leaves: Vec<_> = placed.iter().filter(|p| p.height == 0).collect();
    leaves.sort_by(|a, b| a.x.total_cmp(&b.x));
    // The root's left child is `a`, then node 7 holds node 5 (c, d) and node 6 (b, r).
    check_eq(leaves.iter().map(|p| p.node).collect::<Vec<_>>(), vec![0, 2, 3, 1, 4])?;
    check_eq(placed.iter().map(|p| p.height).max(), Some(3))
}

#[test]
fn separate_trees_are_further_apart_than_neighbouring_leaves_and_never_overlap() -> Result<(), String> {
    let t = abracadabra()?;
    let (placed, width) = layout(&t.tree.nodes, &[2, 3, 1, 4, 0]);
    // All five are single leaves, in the order given. The gap between the edges of neighbours is the tree gap.
    for pair in placed.windows(2) {
        let gap = (pair[1].x - pair[1].width / 2.0) - (pair[0].x + pair[0].width / 2.0);
        check(gap >= 22.0 - 1e-9, &format!("trees {pair:?} are only {gap} apart"))?;
    }
    check(width > 0.0, "the forest has width")?;
    check(
        placed.iter().all(|p| p.x + p.width / 2.0 <= width + 1e-9),
        "everything is within the width",
    )
}

#[test]
fn a_leaf_is_as_wide_as_its_label_needs() -> Result<(), String> {
    let t = trace(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa    b").ok_or("encodable")?;
    // The leaf for 'a' reads "a ×38" and the one for 'b' reads "b ×1". The text is wider in the first.
    let (placed, _) = layout(&t.tree.nodes, &[0, 2]);
    let width = |node: usize| placed.iter().find(|p| p.node == node).map(|p| p.width);
    check(width(0) > width(2), "a longer label needs a wider box")?;
    check(placed.iter().all(|p| p.width >= 48.0), "no box is narrower than the minimum")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Spacing between levels
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn each_connector_between_a_parent_and_its_child_has_room_for_an_elbow_and_an_arrowhead() -> Result<(), String> {
    // Whatever the measured heights are, the gap each side of the bit label is twice the arrowhead and its clearance.
    for (box_height, bit_height) in [(30.2, 30.2), (34.0, 20.0), (50.0, 24.0)] {
        let spacing = Spacing { box_height, bit_height };
        let gap_each_side = (spacing.level() - box_height - bit_height) / 2.0;
        // An elbow bends midway, so the last stretch is half the gap. It holds the 10-pixel arrowhead, plus clearance.
        check(
            gap_each_side / 2.0 >= 10.0 + 6.0,
            &format!("the last stretch is only {} pixels", gap_each_side / 2.0),
        )?;
    }
    Ok(())
}

#[test]
fn a_child_sits_a_whole_level_below_its_parent() -> Result<(), String> {
    let t = abracadabra()?;
    let spacing = Spacing {
        box_height: 30.2,
        bit_height: 30.2,
    };
    let (placed, _) = layout(&t.tree.nodes, &[8]);
    let leaf_y = 500.0;
    for p in &placed {
        let Node::Branch { left, right, .. } = t.tree.nodes[p.node] else { continue };
        for child in [left, right] {
            let c = placed.iter().find(|c| c.node == child).ok_or("child placed")?;
            // The space between the parent's bottom edge and the child's top edge holds both connectors and the label.
            let between = c.top(leaf_y, spacing.level()) - (p.top(leaf_y, spacing.level()) + spacing.box_height);
            check(
                between >= spacing.bit_height + 4.0 * 16.0 - 1e-9,
                &format!("only {between} pixels between a parent and child"),
            )?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Markdown
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_byte_that_markdown_reads_as_formatting_is_escaped() -> Result<(), String> {
    for (byte, escaped) in [
        (b'*', "\\*"),
        (b'_', "\\_"),
        (b'`', "\\`"),
        (b'\\', "\\\\"),
        (b'a', "a"),
        (b' ', "space"),
        (0xc3, "0xC3"),
    ] {
        check_eq(symbol_markdown(byte), escaped.to_owned())?;
    }
    Ok(())
}

#[test]
fn a_caption_names_a_formatting_byte_escaped_and_never_bare() -> Result<(), String> {
    let t = trace(b"a*b*c_d_e").ok_or("encodable")?;
    let all: String = (0..steps(&t)).flat_map(|p| shown(&t, p).caption).collect::<Vec<_>>().join("\n");
    check(all.contains("'\\*'"), "the star is named escaped")?;
    check(all.contains("'\\_'"), "the underscore is named escaped")?;
    check(
        !all.contains("'*'") && !all.contains("'_'"),
        "a bare star or underscore would be read as emphasis",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   A real sentence
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const AUSTEN: &[u8] =
    b"It is a truth universally acknowledged, that a single man in possession of a good fortune must be in want of a wife.";

#[test]
fn the_austen_sentence_has_fifty_three_positions_and_ends_as_one_tree() -> Result<(), String> {
    let t = trace(AUSTEN).ok_or("encodable")?;
    // 26 distinct bytes: 1 counting step, 25 merges, 1 lengths step and 26 codes.
    check_eq(steps(&t), 53)?;
    check_eq(shown(&t, 0).roots.len(), 26)?;
    check_eq(shown(&t, 25).roots.len(), 1)?;
    let last = shown(&t, 52);
    check(last.rows.iter().all(|r| r.canonical.is_some()), "every byte has a code")?;
    check(last.rows.iter().all(|r| r.length.is_some()), "every byte has a length")
}

#[test]
fn a_sentence_with_many_equal_weights_reports_ties() -> Result<(), String> {
    let t = trace(AUSTEN).ok_or("encodable")?;
    // Nine bytes occur once, so the early merges are all ties.
    let ties = (1..=25)
        .filter(|&p| shown(&t, p).caption[2].starts_with("Another tree also weighs"))
        .count();
    check(ties > 3, &format!("only {ties} merges reported a tie"))
}

#[test]
fn every_step_explains_itself_in_one_to_three_paragraphs_none_of_them_empty() -> Result<(), String> {
    for text in [AUSTEN, b"abracadabra", b"aaaa", b"the quick brown fox jumps over the lazy dog"] {
        let t = trace(text).ok_or("encodable")?;
        for position in 0..steps(&t) {
            let caption = shown(&t, position).caption;
            check(
                (1..=3).contains(&caption.len()),
                &format!("{} paragraphs at {position}", caption.len()),
            )?;
            check(caption.iter().all(|p| !p.trim().is_empty()), "an empty paragraph")?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Pass two
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// "abracadabra" has the canonical codes a=0 b=100 c=101 d=110 r=111, so it packs to 0100 1110 1010 1100 1001 110 and a
// single bit of padding: the bytes 0x4E, 0xAC and 0x9C.

#[test]
fn pass_two_has_one_step_for_each_input_byte_and_a_flush() -> Result<(), String> {
    check_eq(pack_steps(&abracadabra()?), 12)
}

#[test]
fn a_step_appends_the_code_after_the_pending_bits_and_marks_only_the_code() -> Result<(), String> {
    // Step 0 packs 'a' = 0, and step 1 packs 'b' = 100 after the one pending bit.
    let t = abracadabra()?;
    let first = packing(&t, 0);
    check_eq(first.appended, vec![false])?;
    check_eq(first.marked, vec![0])?;
    check_eq(first.pending, vec![false])?;
    let second = packing(&t, 1);
    check_eq(second.appended, vec![false, true, false, false])?;
    check_eq(second.marked, vec![1, 2, 3])?;
    check_eq(second.symbol, Some(b'b'))
}

#[test]
fn a_step_writes_a_byte_as_soon_as_eight_bits_are_ready() -> Result<(), String> {
    // After 'a' 'b' 'r' there are 7 bits, and the next 'a' makes the eighth.
    let t = abracadabra()?;
    check_eq(packing(&t, 2).output, vec![])?;
    let fourth = packing(&t, 3);
    check_eq(fourth.output, vec![0x4E])?;
    check_eq(fourth.new_bytes, vec![0])?;
    check_eq(fourth.pending, vec![])
}

#[test]
fn the_flush_pads_the_last_byte_with_zeros_and_ends_on_the_packed_output() -> Result<(), String> {
    let t = abracadabra()?;
    let flush = packing(&t, 11);
    check_eq(flush.symbol, None)?;
    check_eq(flush.output.clone(), vec![0x4E, 0xAC, 0x9C])?;
    check_eq(flush.output, t.packed.bytes.clone())?;
    // The last 7 bits are pending, and the eighth is padding.
    check_eq(flush.appended.len(), 8)?;
    check_eq(flush.marked, vec![7])?;
    check_eq(flush.new_bytes, vec![2])?;
    check_eq(flush.pending, vec![])
}

#[test]
fn a_position_past_the_flush_is_the_flush() -> Result<(), String> {
    let t = abracadabra()?;
    check_eq(packing(&t, 99), packing(&t, 11))
}

#[test]
fn an_exact_multiple_of_eight_bits_needs_no_padding() -> Result<(), String> {
    // One distinct byte has a code of 1 bit, so 8 of them fill a byte.
    let t = trace(b"aaaaaaaa").ok_or("encodable")?;
    let flush = packing(&t, 8);
    check_eq(flush.appended, vec![])?;
    check_eq(flush.new_bytes, vec![])?;
    check_eq(flush.output, vec![0])?;
    check(
        flush.caption[0].contains("no padding is needed"),
        "the flush says no padding is needed",
    )
}

#[test]
fn every_pass_two_step_explains_itself_in_three_lines_none_of_them_empty() -> Result<(), String> {
    let t = trace(b"It is a truth universally acknowledged").ok_or("encodable")?;
    for index in 0..pack_steps(&t) {
        let caption = packing(&t, index).caption;
        check_eq(caption.len(), 3)?;
        check(caption.iter().all(|l| !l.is_empty()), "no line is empty")?;
    }
    Ok(())
}

#[test]
fn the_output_after_every_step_is_a_prefix_of_the_packed_bytes() -> Result<(), String> {
    let t = trace(b"It is a truth universally acknowledged, that a single man").ok_or("encodable")?;
    for index in 0..pack_steps(&t) {
        let output = packing(&t, index).output;
        check(t.packed.bytes.starts_with(&output), &format!("step {index} output is a prefix"))?;
    }
    Ok(())
}

#[test]
fn only_the_flush_has_a_summary_and_it_compares_the_sizes() -> Result<(), String> {
    // 11 input bytes become 3, so the ratio is 11 to 3 = 3.67 and the output is 27.3% of the original.
    let t = abracadabra()?;
    check((0..11).all(|i| packing(&t, i).summary.is_none()), "no summary before the flush")?;
    let summary = packing(&t, 11).summary.ok_or("the flush has a summary")?;
    check(summary.contains("**Original**: 11 bytes (88 bits)"), &summary)?;
    check(
        summary.contains("**Encoded**: 3 bytes, which is 23 bits of code and 1 of padding"),
        &summary,
    )?;
    check(summary.contains("3.67 to 1"), &summary)?;
    check(summary.contains("27.3% of the original size, a saving of 72.7%"), &summary)
}

#[test]
fn a_summary_of_output_as_large_as_the_input_saves_nothing() -> Result<(), String> {
    // One byte of input has a 1-bit code and is padded to one byte, so nothing is saved.
    let t = trace(&[7]).ok_or("encodable")?;
    let summary = packing(&t, 1).summary.ok_or("the flush has a summary")?;
    check(summary.contains("100.0% of the original size, a saving of 0.0%"), &summary)
}
