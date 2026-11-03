//! What a person who cannot see the colours is told, and how the keyboard moves through the tree. Pure: nothing here
//! touches the DOM.
//!
//! A person's choice of a key is kept as an intent (`chosen`), and what is shown as selected is worked out again from
//! the snapshot each time (see [`resolve`]). The key that holds the single tab stop works the same way (`roving`).

use super::solver::{Colour, Event, Key, Snapshot};
use std::collections::BTreeMap;

pub(crate) const AT_FIRST: &str = "Already at the first step.";
pub(crate) const AT_LAST: &str = "Already at the last step.";
pub(crate) const KEYBOARD_HELP: &str = "Tab to enter the tree. Up/Down move in sorted key order; Home/End go to the first/last key; Left goes to the parent; Right goes to the first available child (left before right); use Up/Down to reach the other child; Enter/Space selects.";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The key that `chosen` stands for in `s`.
///
/// A key that is in the tree is itself. A key that a deletion replaced is the key that took its place, following the
/// replacements until one is in the tree. Any other key, such as one that is not yet inserted, gives the root, and then
/// the smallest key. An empty tree, or no choice, gives nothing.
pub(crate) fn resolve(chosen: Option<Key>, s: &Snapshot) -> Option<Key> {
    let mut key = chosen?;
    // A mapping chain cannot be longer than the mapping, so a cycle ends the walk.
    for _ in 0..=s.replaced_by.len() {
        if s.has_key(key) {
            return Some(key);
        }
        match s.replaced_by.get(&key) {
            Some(&next) => key = next,
            None => break,
        }
    }
    s.root.or_else(|| s.minimum_key())
}

/// The key that holds the single tab stop. With no intent it is the root.
pub(crate) fn resolve_roving(roving: Option<Key>, s: &Snapshot) -> Option<Key> {
    match roving {
        None => s.root.or_else(|| s.minimum_key()),
        some => resolve(some, s),
    }
}

/// What to say when the key that is shown as selected is not the key that was chosen.
pub(crate) fn selection_note(chosen: Option<Key>, s: &Snapshot) -> Option<String> {
    let chosen = chosen?;
    let shown = resolve(Some(chosen), s);
    if shown == Some(chosen) {
        return None;
    }
    Some(match shown {
        Some(k) if s.replaced_by.contains_key(&chosen) => {
            format!("{k} is shown as selected: it took the place of your selected {chosen}.")
        },
        Some(k) => {
            format!("Your selected {chosen} is not in the tree at this step, so {k}, the root, is shown instead.")
        },
        None => format!("Your selected {chosen} is not in the tree at this step, and the tree is empty."),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A key of the keyboard that moves through the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Nav {
    Up,
    Down,
    Home,
    End,
    Left,
    Right,
}

impl Nav {
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "ArrowUp" => Some(Self::Up),
            "ArrowDown" => Some(Self::Down),
            "Home" => Some(Self::Home),
            "End" => Some(Self::End),
            "ArrowLeft" => Some(Self::Left),
            "ArrowRight" => Some(Self::Right),
            _ => None,
        }
    }
}

/// The key that `nav` goes to from `from`, or `None` if it goes nowhere.
///
/// Up and Down follow the sorted order. Left goes to the parent. Right goes to the left child, and if there is none, to
/// the right child, so a node with two children reaches its right child with Down.
pub(crate) fn navigate(s: &Snapshot, from: Key, nav: Nav) -> Option<Key> {
    let at = s.nodes.iter().position(|n| n.key == from)?;
    let target = match nav {
        Nav::Up => at.checked_sub(1).map(|i| s.nodes[i].key),
        Nav::Down => s.nodes.get(at + 1).map(|n| n.key),
        Nav::Home => s.nodes.first().map(|n| n.key),
        Nav::End => s.nodes.last().map(|n| n.key),
        Nav::Left => s.nodes[at].parent,
        Nav::Right => s.nodes[at].left.or(s.nodes[at].right),
    };
    target.filter(|&k| k != from)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One key as the tree of the accessibility tree has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Item {
    pub(crate) key: Key,
    pub(crate) colour: Colour,
    pub(crate) level: usize,
    pub(crate) pos_in_set: usize,
    pub(crate) set_size: usize,
    pub(crate) selected: bool,
    /// `Some(true)` for a key that shows children, and `None` for one that has none: there is no collapsing.
    pub(crate) expanded: Option<bool>,
    pub(crate) name: String,
    /// The children, left before right.
    pub(crate) children: Vec<Key>,
}

/// Every key of `s`, with the place it has in the tree, and which one is shown as selected.
pub(crate) fn items(s: &Snapshot, selected: Option<Key>) -> BTreeMap<Key, Item> {
    let mut found = BTreeMap::new();
    let Some(root) = s.root else { return found };
    let mut work = vec![(root, 1_usize, 1_usize, 1_usize)];
    while let Some((key, level, pos_in_set, set_size)) = work.pop() {
        let Some(n) = s.node(key) else { continue };
        let children: Vec<Key> = [n.left, n.right].into_iter().flatten().collect();
        let size = children.len();
        for (at, &child) in children.iter().enumerate() {
            work.push((child, level + 1, at + 1, size));
        }
        found.insert(
            key,
            Item {
                key,
                colour: n.colour,
                level,
                pos_in_set,
                set_size,
                selected: selected == Some(key),
                expanded: (!children.is_empty()).then_some(true),
                name: item_name(s, key),
                children,
            },
        );
    }
    found
}

/// A name that does not rely on colour, such as "12, red, right child of 10".
pub(crate) fn item_name(s: &Snapshot, key: Key) -> String {
    let Some(n) = s.node(key) else { return key.to_string() };
    let place = match n.parent {
        None => "root".to_owned(),
        Some(p) => {
            let side = if s.node(p).is_some_and(|parent| parent.left == Some(key)) {
                "left"
            } else {
                "right"
            };
            format!("{side} child of {p}")
        },
    };
    format!("{key}, {}, {place}", n.colour.word())
}

/// The name of the whole tree.
pub(crate) fn tree_name(s: &Snapshot) -> String {
    match s.root {
        None => "Red-black tree, empty".to_owned(),
        Some(root) => {
            let n = s.nodes.len();
            format!("Red-black tree, {n} {}, root {root}", if n == 1 { "key" } else { "keys" })
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a step says, which warns when the tree is in the middle of a repair.
pub(crate) fn summary(s: &Snapshot, last: usize) -> String {
    let state = if s.settled {
        ""
    } else {
        "Repair in progress, not yet a valid red-black tree. "
    };
    format!("Step {} of {last}. {state}{}", s.ordinal, s.description)
}

/// Whether a step is announced while the run plays: a finished operation, a rotation, a transplant or a case of the
/// deletion fix-up, and not the small moves between them.
pub(crate) fn announced_while_playing(event: &Event) -> bool {
    matches!(
        event,
        Event::RotateLeft(_)
            | Event::RotateRight(_)
            | Event::Transplant { .. }
            | Event::Splice { .. }
            | Event::DeleteFixupCase { .. }
            | Event::CompleteInsert(_)
            | Event::CompleteDelete(_)
    )
}

#[cfg(test)]
mod unit_tests;
