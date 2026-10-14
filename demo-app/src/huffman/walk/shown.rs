use super::{
    super::algorithm::{Code, Merge, Node, Trace},
    symbol_markdown,
};

/// One row of the code table, for one distinct byte. A column is `None` until the walk has reached it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::huffman) struct Row {
    pub(in crate::huffman) symbol: u8,
    pub(in crate::huffman) count: u32,
    /// The route from the root to the leaf, as the tree gives it.
    pub(in crate::huffman) path: Option<String>,
    /// The code length: the leaf's depth, or 1 for a tree of one leaf.
    pub(in crate::huffman) length: Option<u8>,
    /// The canonical code.
    pub(in crate::huffman) canonical: Option<String>,
}

/// Everything the scene shows at one position.
pub(in crate::huffman) struct Shown {
    /// What happened at this step, and why, in up to three sentences. The first says what happened.
    pub(in crate::huffman) caption: Vec<String>,
    /// The roots of the trees so far, lightest first.
    pub(in crate::huffman) roots: Vec<usize>,
    /// The last merge, if this step is a merge: the two trees it joined and the one it made.
    pub(in crate::huffman) merged: Option<Merge>,
    /// The code table, one row per distinct byte, the commonest first. Bytes that occur as often as each other are in
    /// byte order.
    pub(in crate::huffman) rows: Vec<Row>,
    /// The row the last step wrote to, if it wrote to exactly one.
    pub(in crate::huffman) focus_row: Option<usize>,
    /// The nodes on the route from the root down to the leaf of the byte the last step gave a canonical code, root first.
    /// Empty at every other step.
    pub(in crate::huffman) route: Vec<usize>,
}

/// What the scene shows at `position`. A position past the last is the last.
pub(in crate::huffman) fn shown(trace: &Trace, position: usize) -> Shown {
    let position = position.min(super::steps(trace) - 1);
    let merges = trace.tree.merges.len();
    let lengths_at = merges + 1;

    let merged = (1..=merges).contains(&position).then(|| trace.tree.merges[position - 1]);
    let lengths_known = position >= lengths_at;
    let assigned = position.saturating_sub(lengths_at);

    // The table shows the commonest byte first. `order` lists indices into `trace.frequencies`, which is in byte order.
    let mut order: Vec<usize> = (0..trace.frequencies.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(trace.frequencies[i].count), trace.frequencies[i].symbol));

    // `leaf_paths` is also in byte order, so it lines up with `trace.frequencies`.
    let paths = trace.tree.leaf_paths();
    let canonical = &trace.codebook.entries[..assigned];
    let rows: Vec<Row> = order
        .iter()
        .map(|&i| {
            let (frequency, path) = (&trace.frequencies[i], &paths[i].1);
            Row {
                symbol: frequency.symbol,
                count: frequency.count,
                length: lengths_known.then(|| path.len().max(1) as u8),
                path: lengths_known.then(|| path.clone()),
                canonical: canonical
                    .iter()
                    .find(|e| e.symbol == frequency.symbol)
                    .map(|e| code_text(e.code)),
            }
        })
        .collect();

    // The byte the last step gave a code to.
    let last = assigned.checked_sub(1).map(|i| trace.codebook.entries[i].symbol);
    let focus_row = last.and_then(|symbol| rows.iter().position(|r| r.symbol == symbol));
    let route = last.map(|symbol| trace.tree.route_to(symbol)).unwrap_or_default();

    Shown {
        caption: caption(trace, position),
        roots: roots_after(trace, position.min(merges)),
        merged,
        rows,
        focus_row,
        route,
    }
}

/// A code as text: its bits, padded with zeros on the left to its length.
pub(in crate::huffman) fn code_text(code: Code) -> String {
    format!("{:0width$b}", code.bits, width = usize::from(code.len))
}

/// The roots of the trees after the first `merges` merges, lightest first. Ties go to the node made first, as the
/// algorithm's own heap does.
fn roots_after(trace: &Trace, merges: usize) -> Vec<usize> {
    let leaves = trace.frequencies.len();
    let mut roots: Vec<usize> = (0..leaves).collect();

    for merge in &trace.tree.merges[..merges] {
        roots.retain(|&r| r != merge.left && r != merge.right);
        roots.push(merge.parent);
    }
    roots.sort_unstable_by_key(|&r| (trace.tree.nodes[r].weight(), r));
    roots
}

/// A node as a sentence names it: a byte with its count, or a tree by its weight.
fn describe(trace: &Trace, index: usize) -> String {
    match trace.tree.nodes[index] {
        Node::Leaf { symbol, weight } => format!("'{}' ({weight})", symbol_markdown(symbol)),
        Node::Branch { weight, .. } => format!("the tree of weight {weight}"),
    }
}

/// The sentences for `position`, which must be within the walk.
fn caption(trace: &Trace, position: usize) -> Vec<String> {
    let merges = trace.tree.merges.len();
    let lengths_at = merges + 1;
    let distinct = trace.frequencies.len();

    let lines: Vec<String> = if position == 0 {
        counting(trace)
    } else if position <= merges {
        merging(trace, position)
    } else if position == lengths_at {
        lengths(distinct)
    } else {
        assigning(trace, position - lengths_at - 1)
    };
    lines
}

/// The counting step.
fn counting(trace: &Trace) -> Vec<String> {
    let total: u32 = trace.frequencies.iter().map(|f| f.count).sum();
    let distinct = trace.frequencies.len();
    if distinct == 1 {
        return vec![
            format!("Counted {total} bytes, all the same."),
            "A single byte is one tree of one node, so there is nothing to merge.".to_owned(),
        ];
    }
    vec![
        format!("Counted {total} bytes, {distinct} of them distinct. Each distinct byte is now a tree of one node."),
        "A tree's weight is how often its bytes occur. The trees are kept in order of weight, lightest on the left."
            .to_owned(),
        "Next, the two lightest trees merge, again and again. So rare bytes end up deepest, with the longest codes."
            .to_owned(),
    ]
}

/// Merge number `position`, counting from 1.
fn merging(trace: &Trace, position: usize) -> Vec<String> {
    let merges = trace.tree.merges.len();
    let merge = trace.tree.merges[position - 1];
    let weight = trace.tree.nodes[merge.parent].weight();

    // The trees before this merge, lightest first. The merge took the first two, so a third of the same weight as the
    // second means the tie rule decided which of them went.
    let before = roots_after(trace, position - 1);
    let tie = before
        .get(2)
        .map(|&third| trace.tree.nodes[third].weight())
        .filter(|&w| w == trace.tree.nodes[before[1]].weight());
    let last_line = match (tie, merges - position) {
        (Some(w), _) => format!(
            "Another tree also weighs {w}. Ties go to the tree made first, so a text always gives the same tree."
        ),
        (None, 0) => "One tree remains: the code tree is complete.".to_owned(),
        (None, 1) => "Two trees remain.".to_owned(),
        (None, left) => format!("{} trees remain.", left + 1),
    };
    vec![
        format!(
            "**Merge {position} of {merges}**: the two lightest trees are {} and {}. Joined, they weigh {weight}.",
            describe(trace, merge.left),
            describe(trace, merge.right)
        ),
        "The lighter goes left (0) and the other right (1). Every byte inside now has a code one bit longer."
            .to_owned(),
        last_line,
    ]
}

/// The step that reads each code length from the tree.
fn lengths(distinct: usize) -> Vec<String> {
    if distinct == 1 {
        return vec![
            "A lone byte's tree has depth 0, which would be an empty code. It is given length 1.".to_owned(),
            "The tree has done its job. Only the length is needed next.".to_owned(),
        ];
    }
    vec![
        "Each code's length is the depth of its leaf: the number of links from the root down to it.".to_owned(),
        "Reading the links down to a leaf spells its tree path, with 0 for each left link and 1 for each right link."
            .to_owned(),
        "The tree has done its job. Only these lengths are needed to make the final codes.".to_owned(),
    ]
}

/// The step that assigns the canonical code of entry `index`, counting in canonical order.
fn assigning(trace: &Trace, index: usize) -> Vec<String> {
    let entries = &trace.codebook.entries;
    let entry = entries[index];
    let name = symbol_markdown(entry.symbol);
    let text = code_text(entry.code);

    let first = if index == 0 {
        format!(
            "Canonical codes start at all zeros: '{name}' has length {} and code {text}.",
            entry.code.len
        )
    } else {
        let previous = entries[index - 1].code;
        let shift = entry.code.len - previous.len;
        let rule = if shift == 0 {
            format!("the previous code {} plus 1", code_text(previous))
        } else {
            format!("the previous code {} plus 1, shifted left {shift}", code_text(previous))
        };
        format!("'{name}' has length {}, so its code is {rule}: {text}.", entry.code.len)
    };
    let path = trace
        .tree
        .leaf_paths()
        .into_iter()
        .find(|(symbol, _)| *symbol == entry.symbol)
        .map(|(_, path)| path)
        .unwrap_or_default();
    let versus_tree = if path == text {
        format!("The tree's own path for '{name}' happens to be the same.")
    } else if path.is_empty() {
        format!("The tree has no path for '{name}', since it is the only byte.")
    } else {
        format!(
            "The tree's own path for '{name}' was {path}. Both are valid, and both are {} bits long.",
            entry.code.len
        )
    };

    let mut lines = vec![first, versus_tree];
    // A lone byte has no order to explain, and the three-line caption has no room for both.
    if index == 0 && entries.len() > 1 {
        lines.push("Codes are handed out shortest first, and by byte value within one length.".to_owned());
    }
    if index + 1 == entries.len() {
        lines.push("Only the lengths are needed to rebuild these codes, so a decoder never needs the tree.".to_owned());
    }
    lines
}
