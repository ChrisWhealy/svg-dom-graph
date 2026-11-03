//! A red-black tree as a list of events, and a standalone snapshot of the whole tree after each. Pure: nothing here
//! touches the DOM.
//!
//! The tree is the classic bottom-up one. Children are `Option`s, and an absent child is a black NIL leaf that is never
//! allocated, so a deletion fix-up carries its own position (see [`FixupCursor`]) and never asks a NIL for its parent.
//! A key is the identity of its node for the whole trace. A deletion moves the original successor into the place of the
//! deleted node and never copies a key.

use std::collections::{BTreeMap, BTreeSet};

/// The identity of a node, which is its key. Keys are unique, and a node keeps its key for as long as it exists.
pub(crate) type Key = u16;

/// The most keys that a tree can hold, and the most snapshots that a trace can hold.
pub(crate) const MAX_KEYS: usize = 64;
pub(crate) const SNAPSHOT_BUDGET: usize = 4_000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Colour {
    Red,
    Black,
}

impl Colour {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Black => "black",
        }
    }

    pub(crate) fn letter(self) -> char {
        match self {
            Self::Red => 'R',
            Self::Black => 'B',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChildSide {
    Left,
    Right,
}

/// Where a deletion fix-up is. `x` is the node that carries the extra black, which is absent when it is a NIL leaf, and
/// then `x_parent` and `x_side` say which child slot it is the absence of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FixupCursor {
    pub(crate) x: Option<Key>,
    pub(crate) x_parent: Option<Key>,
    pub(crate) x_side: ChildSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Empty,
    Insert,
    Delete,
}

/// Why an operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SolverError {
    DuplicateKey(Key),
    MissingKey(Key),
    TooManyKeys { count: usize, max: usize },
    BudgetExceeded { max: usize },
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateKey(k) => write!(f, "the key {k} is already in the tree"),
            Self::MissingKey(k) => write!(f, "the key {k} is not in the tree"),
            Self::TooManyKeys { count, max } => write!(f, "{count} keys is more than the {max} that fit"),
            Self::BudgetExceeded { max } => write!(f, "the trace needs more than {max} snapshots"),
        }
    }
}

impl std::error::Error for SolverError {}

/// One operation of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    Insert(Key),
    Delete(Key),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What happened to make a snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    /// The empty tree, before anything.
    Start,
    Insert(Key),
    Recolour(Vec<(Key, Colour)>),
    RotateLeft(Key),
    RotateRight(Key),
    /// The node to delete has two children, and this is the smallest key of its right subtree.
    ChooseSuccessor {
        deleted: Key,
        successor: Key,
    },
    /// The successor takes the deleted node's place, with its colour, and the deleted node is gone.
    Transplant {
        deleted: Key,
        successor: Key,
    },
    /// The node to delete has at most one child, and that child (if any) takes its place.
    Splice {
        deleted: Key,
        child: Option<Key>,
    },
    /// One case of the deletion fix-up, at `cursor`.
    DeleteFixupCase {
        case: u8,
        mirrored: bool,
        cursor: FixupCursor,
    },
    CompleteInsert(Key),
    CompleteDelete(Key),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeSnapshot {
    pub(crate) key: Key,
    pub(crate) colour: Colour,
    pub(crate) parent: Option<Key>,
    pub(crate) left: Option<Key>,
    pub(crate) right: Option<Key>,
}

/// The whole state after one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) ordinal: usize,
    pub(crate) phase: Phase,
    /// The operation that this event belongs to.
    pub(crate) op: Option<Op>,
    pub(crate) event: Event,
    pub(crate) root: Option<Key>,
    /// Every node, in key order.
    pub(crate) nodes: Vec<NodeSnapshot>,
    /// One `(parent, child)` for each node that has a parent, in key order of the child.
    pub(crate) edges: Vec<(Key, Key)>,
    pub(crate) active: Vec<Key>,
    pub(crate) description: String,
    /// Whether the tree satisfies every red-black rule here. A transitional frame of a repair does not.
    pub(crate) settled: bool,
    /// Which node took the place of each deleted key, so far in the run.
    pub(crate) replaced_by: BTreeMap<Key, Key>,
    /// Where the deletion fix-up is, while it is running.
    pub(crate) cursor: Option<FixupCursor>,
}

impl Snapshot {
    pub(crate) fn node(&self, key: Key) -> Option<&NodeSnapshot> {
        self.nodes.binary_search_by_key(&key, |n| n.key).ok().map(|at| &self.nodes[at])
    }

    pub(crate) fn has_key(&self, key: Key) -> bool {
        self.node(key).is_some()
    }

    pub(crate) fn minimum_key(&self) -> Option<Key> {
        self.nodes.first().map(|n| n.key)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
struct Node {
    colour: Colour,
    parent: Option<Key>,
    left: Option<Key>,
    right: Option<Key>,
}

/// The tree that the algorithm works on.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tree {
    root: Option<Key>,
    nodes: BTreeMap<Key, Node>,
}

impl Tree {
    fn n(&self, key: Key) -> &Node {
        &self.nodes[&key]
    }

    fn nm(&mut self, key: Key) -> &mut Node {
        self.nodes.get_mut(&key).expect("a key that is in the tree")
    }

    fn colour_of(&self, key: Option<Key>) -> Colour {
        key.map_or(Colour::Black, |k| self.n(k).colour)
    }

    fn left_of(&self, key: Key) -> Option<Key> {
        self.n(key).left
    }

    fn right_of(&self, key: Key) -> Option<Key> {
        self.n(key).right
    }

    fn parent_of(&self, key: Key) -> Option<Key> {
        self.n(key).parent
    }

    fn set_child(&mut self, parent: Option<Key>, side: ChildSide, child: Option<Key>) {
        match parent {
            None => self.root = child,
            Some(p) => match side {
                ChildSide::Left => self.nm(p).left = child,
                ChildSide::Right => self.nm(p).right = child,
            },
        }
        if let Some(c) = child {
            self.nm(c).parent = parent;
        }
    }

    /// Which child slot of its parent `key` is in.
    fn side_of(&self, key: Key) -> ChildSide {
        match self.parent_of(key) {
            Some(p) if self.n(p).right == Some(key) => ChildSide::Right,
            _ => ChildSide::Left,
        }
    }

    fn minimum(&self, mut key: Key) -> Key {
        while let Some(l) = self.left_of(key) {
            key = l;
        }
        key
    }

    /// Puts `new` where `old` was under `old`'s parent. `old` keeps its own links.
    fn transplant(&mut self, old: Key, new: Option<Key>) {
        let (parent, side) = (self.parent_of(old), self.side_of(old));
        self.set_child(parent, side, new);
    }

    fn rotate_left(&mut self, x: Key) {
        let y = self.right_of(x).expect("a left rotation needs a right child");
        let (parent, side) = (self.parent_of(x), self.side_of(x));
        let moved = self.left_of(y);
        self.nm(x).right = moved;
        if let Some(m) = moved {
            self.nm(m).parent = Some(x);
        }
        self.set_child(parent, side, Some(y));
        self.nm(y).left = Some(x);
        self.nm(x).parent = Some(y);
    }

    fn rotate_right(&mut self, x: Key) {
        let y = self.left_of(x).expect("a right rotation needs a left child");
        let (parent, side) = (self.parent_of(x), self.side_of(x));
        let moved = self.right_of(y);
        self.nm(x).left = moved;
        if let Some(m) = moved {
            self.nm(m).parent = Some(x);
        }
        self.set_child(parent, side, Some(y));
        self.nm(y).right = Some(x);
        self.nm(x).parent = Some(y);
    }

    /// Every node as a snapshot, in key order.
    fn node_snapshots(&self) -> Vec<NodeSnapshot> {
        self.nodes
            .iter()
            .map(|(&key, n)| NodeSnapshot {
                key,
                colour: n.colour,
                parent: n.parent,
                left: n.left,
                right: n.right,
            })
            .collect()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checks the links of a tree given as snapshots of its nodes.
pub(crate) fn check_structure(root: Option<Key>, nodes: &[NodeSnapshot]) -> Result<(), String> {
    let by_key: BTreeMap<Key, &NodeSnapshot> = nodes.iter().map(|n| (n.key, n)).collect();
    if by_key.len() != nodes.len() {
        return Err("two nodes have one key".to_owned());
    }
    let Some(root) = root else {
        return if nodes.is_empty() {
            Ok(())
        } else {
            Err("there are nodes but no root".to_owned())
        };
    };
    let Some(root_node) = by_key.get(&root) else {
        return Err(format!("the root {root} is not a node"));
    };
    if root_node.parent.is_some() {
        return Err(format!("the root {root} has a parent"));
    }
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    // An in-order walk with an explicit stack, so that a cycle cannot recurse for ever.
    let mut stack: Vec<(Key, bool)> = vec![(root, false)];
    while let Some((key, visited)) = stack.pop() {
        if visited {
            order.push(key);
            continue;
        }
        if !seen.insert(key) {
            return Err(format!("{key} is reached twice, so there is a cycle"));
        }
        let Some(n) = by_key.get(&key) else {
            return Err(format!("{key} is linked but is not a node"));
        };
        if let Some(r) = n.right {
            stack.push((r, false));
        }
        stack.push((key, true));
        if let Some(l) = n.left {
            stack.push((l, false));
        }
        for (child, side) in [(n.left, "left"), (n.right, "right")] {
            if let Some(c) = child {
                match by_key.get(&c) {
                    Some(cn) if cn.parent == Some(key) => {},
                    Some(_) => return Err(format!("the {side} child {c} of {key} does not name it as its parent")),
                    None => return Err(format!("the {side} child {c} of {key} is not a node")),
                }
            }
        }
    }
    if seen.len() != nodes.len() {
        return Err("some node cannot be reached from the root".to_owned());
    }
    if order.windows(2).any(|w| w[0] >= w[1]) {
        return Err("the keys are not in order".to_owned());
    }
    Ok(())
}

/// Checks the red-black rules of a tree given as snapshots of its nodes, on top of the links.
pub(crate) fn check_rules(root: Option<Key>, nodes: &[NodeSnapshot]) -> Result<(), String> {
    check_structure(root, nodes)?;
    let Some(root) = root else { return Ok(()) };
    let by_key: BTreeMap<Key, &NodeSnapshot> = nodes.iter().map(|n| (n.key, n)).collect();
    if by_key[&root].colour != Colour::Black {
        return Err("the root is red".to_owned());
    }
    fn height(by_key: &BTreeMap<Key, &NodeSnapshot>, key: Option<Key>) -> Result<usize, String> {
        let Some(key) = key else { return Ok(1) };
        let n = by_key[&key];
        if n.colour == Colour::Red {
            for child in [n.left, n.right].into_iter().flatten() {
                if by_key[&child].colour == Colour::Red {
                    return Err(format!("red {key} has a red child {child}"));
                }
            }
        }
        let (l, r) = (height(by_key, n.left)?, height(by_key, n.right)?);
        if l != r {
            return Err(format!("the black heights under {key} differ: {l} and {r}"));
        }
        Ok(l + usize::from(n.colour == Colour::Black))
    }
    height(&by_key, Some(root)).map(|_| ())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The tree and the snapshots made of it so far.
struct Builder {
    tree: Tree,
    snapshots: Vec<Snapshot>,
    op: Option<Op>,
    phase: Phase,
    replaced_by: BTreeMap<Key, Key>,
}

impl Builder {
    fn emit(
        &mut self,
        event: Event,
        active: Vec<Key>,
        description: String,
        cursor: Option<FixupCursor>,
    ) -> Result<(), SolverError> {
        if self.snapshots.len() >= SNAPSHOT_BUDGET {
            return Err(SolverError::BudgetExceeded { max: SNAPSHOT_BUDGET });
        }
        let nodes = self.tree.node_snapshots();
        let edges = nodes.iter().filter_map(|n| n.parent.map(|p| (p, n.key))).collect();
        let settled = check_rules(self.tree.root, &nodes).is_ok();
        self.snapshots.push(Snapshot {
            ordinal: self.snapshots.len(),
            phase: self.phase,
            op: self.op,
            event,
            root: self.tree.root,
            nodes,
            edges,
            active,
            description,
            settled,
            replaced_by: self.replaced_by.clone(),
            cursor,
        });
        Ok(())
    }

    fn colour(&mut self, changes: &[(Key, Colour)], why: &str) -> Result<(), SolverError> {
        for &(k, c) in changes {
            self.tree.nm(k).colour = c;
        }
        let list: Vec<String> = changes.iter().map(|&(k, c)| format!("{k} {}", c.word())).collect();
        let active = changes.iter().map(|&(k, _)| k).collect();
        self.emit(
            Event::Recolour(changes.to_vec()),
            active,
            format!("Recolour {}. {why}", list.join(", ")),
            None,
        )
    }

    fn rotate_left(&mut self, x: Key) -> Result<(), SolverError> {
        let y = self.tree.right_of(x).expect("a left rotation needs a right child");
        let moved = self.tree.left_of(y);
        self.tree.rotate_left(x);
        let carried = moved.map_or_else(
            || format!("{y} had no left subtree to hand over, so {x} now has no right child"),
            |m| format!("the subtree rooted at {m} moves from {y} to be the right child of {x}"),
        );
        self.emit(
            Event::RotateLeft(x),
            vec![x, y],
            format!("Left-rotate at {x}: its right child {y} moves up to take its place, and {x} becomes the left child of {y}; {carried}."),
            None,
        )
    }

    fn rotate_right(&mut self, x: Key) -> Result<(), SolverError> {
        let y = self.tree.left_of(x).expect("a right rotation needs a left child");
        let moved = self.tree.right_of(y);
        self.tree.rotate_right(x);
        let carried = moved.map_or_else(
            || format!("{y} had no right subtree to hand over, so {x} now has no left child"),
            |m| format!("the subtree rooted at {m} moves from {y} to be the left child of {x}"),
        );
        self.emit(
            Event::RotateRight(x),
            vec![x, y],
            format!("Right-rotate at {x}: its left child {y} moves up to take its place, and {x} becomes the right child of {y}; {carried}."),
            None,
        )
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn insert(&mut self, key: Key) -> Result<(), SolverError> {
        if self.tree.nodes.contains_key(&key) {
            return Err(SolverError::DuplicateKey(key));
        }
        if self.tree.nodes.len() >= MAX_KEYS {
            return Err(SolverError::TooManyKeys {
                count: self.tree.nodes.len() + 1,
                max: MAX_KEYS,
            });
        }
        self.op = Some(Op::Insert(key));
        self.phase = Phase::Insert;
        let mut parent = None;
        let mut at = self.tree.root;
        let mut side = ChildSide::Left;
        while let Some(k) = at {
            parent = Some(k);
            side = if key < k { ChildSide::Left } else { ChildSide::Right };
            at = if key < k { self.tree.left_of(k) } else { self.tree.right_of(k) };
        }
        self.tree.nodes.insert(
            key,
            Node {
                colour: Colour::Red,
                parent: None,
                left: None,
                right: None,
            },
        );
        self.tree.set_child(parent, side, Some(key));
        let where_ = match parent {
            None => "as the root of an empty tree".to_owned(),
            Some(p) => format!("as the {} child of {p}", if side == ChildSide::Left { "left" } else { "right" }),
        };
        self.emit(Event::Insert(key), vec![key], format!("Insert {key}, red, {where_}."), None)?;
        self.insert_fixup(key)?;
        let n = self.tree.nodes.len();
        let root = self.tree.root.expect("a tree that was just inserted into has a root");
        self.emit(
            Event::CompleteInsert(key),
            vec![key],
            format!(
                "Insert {key} complete: a valid red-black tree of {n} {}, root {root}.",
                plural(n)
            ),
            None,
        )
    }

    fn insert_fixup(&mut self, mut z: Key) -> Result<(), SolverError> {
        while let Some(p) = self.tree.parent_of(z).filter(|&p| self.tree.n(p).colour == Colour::Red) {
            // A red parent is never the root, so it has a parent.
            let g = self.tree.parent_of(p).expect("a red node is not the root");
            let parent_is_left = self.tree.left_of(g) == Some(p);
            let uncle = if parent_is_left { self.tree.right_of(g) } else { self.tree.left_of(g) };
            if self.tree.colour_of(uncle) == Colour::Red {
                let u = uncle.expect("a red uncle exists");
                self.colour(
                    &[(p, Colour::Black), (u, Colour::Black), (g, Colour::Red)],
                    &format!("Parent {p} and uncle {u} are both red, so they turn black and grandparent {g} turns red. The conflict moves up to {g}."),
                )?;
                z = g;
                continue;
            }
            let inner = if parent_is_left {
                self.tree.right_of(p) == Some(z)
            } else {
                self.tree.left_of(p) == Some(z)
            };
            let mut top = p;
            if inner {
                if parent_is_left {
                    self.rotate_left(p)?;
                } else {
                    self.rotate_right(p)?;
                }
                top = z;
            }
            self.colour(
                &[(top, Colour::Black), (g, Colour::Red)],
                &format!("The uncle is black or absent, so {top} turns black and grandparent {g} turns red before {g} rotates."),
            )?;
            if parent_is_left {
                self.rotate_right(g)?;
            } else {
                self.rotate_left(g)?;
            }
            break;
        }
        let root = self.tree.root.expect("a tree that was just inserted into has a root");
        if self.tree.n(root).colour == Colour::Red {
            self.colour(&[(root, Colour::Black)], "The root is always black.")?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn delete(&mut self, z: Key) -> Result<(), SolverError> {
        if !self.tree.nodes.contains_key(&z) {
            return Err(SolverError::MissingKey(z));
        }
        self.op = Some(Op::Delete(z));
        self.phase = Phase::Delete;
        let (zl, zr) = (self.tree.left_of(z), self.tree.right_of(z));
        let z_colour = self.tree.n(z).colour;
        let z_parent = self.tree.parent_of(z);
        let z_side = self.tree.side_of(z);
        let mut removed_colour = z_colour;
        let cursor;
        if let (Some(l), Some(r)) = (zl, zr) {
            let y = self.tree.minimum(r);
            removed_colour = self.tree.n(y).colour;
            let y_colour_word = removed_colour.word();
            self.emit(
                Event::ChooseSuccessor { deleted: z, successor: y },
                vec![z, y],
                format!(
                    "Delete {z}, which has two children, {l} and {r}. Its successor is {y}, the smallest key of its right subtree, and {y} is {y_colour_word}."
                ),
                None,
            )?;
            let x = self.tree.right_of(y);
            let moved_up;
            let x_parent;
            let x_side;
            if self.tree.parent_of(y) == Some(z) {
                x_parent = Some(y);
                x_side = ChildSide::Right;
                moved_up = format!("{y} is already {z}'s child, so it keeps its right side");
            } else {
                x_parent = self.tree.parent_of(y);
                x_side = ChildSide::Left;
                self.tree.transplant(y, x);
                self.tree.set_child(Some(y), ChildSide::Right, Some(r));
                moved_up = format!(
                    "{y}'s right subtree first takes {y}'s old place, and {y} takes over {z}'s right subtree {r}"
                );
            }
            self.tree.transplant(z, Some(y));
            self.tree.set_child(Some(y), ChildSide::Left, Some(l));
            self.tree.nm(y).colour = z_colour;
            self.tree.nodes.remove(&z);
            self.replaced_by.insert(z, y);
            cursor = FixupCursor { x, x_parent, x_side };
            let debt = if removed_colour == Colour::Black {
                format!(
                    " The black that {y} used to supply is missing from the {} side of {}: that missing child is the extra black.",
                    if x_side == ChildSide::Left { "left" } else { "right" },
                    x_parent.map_or_else(|| "the tree".to_owned(), |p| p.to_string())
                )
            } else {
                String::new()
            };
            self.emit(
                Event::Transplant { deleted: z, successor: y },
                vec![y],
                format!(
                    "Transplant: the original node {y} takes the place of {z}, {moved_up}, and copies {z}'s colour, {}. Node {z} is removed.{debt}",
                    z_colour.word()
                ),
                (removed_colour == Colour::Black).then_some(cursor),
            )?;
        } else {
            let child = zl.or(zr);
            self.tree.transplant(z, child);
            self.tree.nodes.remove(&z);
            cursor = FixupCursor {
                x: child,
                x_parent: z_parent,
                x_side: z_side,
            };
            let note = match child {
                Some(c) => format!("its only child {c} takes its place"),
                None => "it has no children, so its place is simply empty".to_owned(),
            };
            let debt = if removed_colour == Colour::Black {
                " The black that it supplied is missing, so a repair follows."
            } else {
                ""
            };
            self.emit(
                Event::Splice { deleted: z, child },
                child.into_iter().collect(),
                format!("Delete {z}: {note}.{debt}"),
                (removed_colour == Colour::Black).then_some(cursor),
            )?;
        }
        if removed_colour == Colour::Black {
            self.delete_fixup(cursor)?;
        }
        let n = self.tree.nodes.len();
        let description = match self.tree.root {
            Some(root) => format!("Delete {z} complete: a valid red-black tree of {n} {}, root {root}.", plural(n)),
            None => format!("Delete {z} complete: the tree is empty."),
        };
        self.emit(Event::CompleteDelete(z), vec![], description, None)
    }

    fn delete_fixup(&mut self, mut c: FixupCursor) -> Result<(), SolverError> {
        while c.x != self.tree.root && self.tree.colour_of(c.x) == Colour::Black {
            let Some(p) = c.x_parent else { break };
            let mirrored = c.x_side == ChildSide::Right;
            // `near` is the side of `p` that `x` is on, and `far` is the other.
            let sibling_of = |t: &Tree| if mirrored { t.left_of(p) } else { t.right_of(p) };
            let mut w = sibling_of(&self.tree).expect("the sibling of a node that carries an extra black exists");
            let describe = |case: u8, text: &str, cursor: FixupCursor| {
                let x = cursor.x.map_or_else(|| "the missing child".to_owned(), |k| k.to_string());
                format!(
                    "Delete fix-up case {case}{}: x is {x} (parent {}, {} side). {text}",
                    if mirrored { " (mirrored)" } else { "" },
                    cursor.x_parent.map_or_else(|| "none".to_owned(), |p| p.to_string()),
                    if cursor.x_side == ChildSide::Left { "left" } else { "right" },
                )
            };
            if self.tree.colour_of(Some(w)) == Colour::Red {
                self.emit(
                    Event::DeleteFixupCase { case: 1, mirrored, cursor: c },
                    vec![p, w],
                    describe(1, &format!("Sibling {w} is red, so {w} turns black, {p} turns red, and {p} rotates to make the sibling black."), c),
                    Some(c),
                )?;
                self.colour(&[(w, Colour::Black), (p, Colour::Red)], "Preparing the rotation of case 1.")?;
                if mirrored {
                    self.rotate_right(p)?;
                } else {
                    self.rotate_left(p)?;
                }
                w = sibling_of(&self.tree).expect("the new sibling exists");
            }
            let (near_child, far_child) = if mirrored {
                (self.tree.right_of(w), self.tree.left_of(w))
            } else {
                (self.tree.left_of(w), self.tree.right_of(w))
            };
            if self.tree.colour_of(near_child) == Colour::Black && self.tree.colour_of(far_child) == Colour::Black {
                self.emit(
                    Event::DeleteFixupCase { case: 2, mirrored, cursor: c },
                    vec![p, w],
                    describe(2, &format!("Sibling {w} and both its children are black, so {w} turns red and the extra black moves up to {p}."), c),
                    Some(c),
                )?;
                self.colour(&[(w, Colour::Red)], "The extra black moves up one level.")?;
                let up = c.x_parent.expect("checked above");
                c = FixupCursor {
                    x: Some(up),
                    x_parent: self.tree.parent_of(up),
                    x_side: self.tree.side_of(up),
                };
                continue;
            }
            if self.tree.colour_of(far_child) == Colour::Black {
                let n = near_child.expect("a red near child exists");
                self.emit(
                    Event::DeleteFixupCase { case: 3, mirrored, cursor: c },
                    vec![p, w, n],
                    describe(3, &format!("Sibling {w} is black, its far child is black and its near child {n} is red, so {n} turns black, {w} turns red, and {w} rotates to put a red child on the far side."), c),
                    Some(c),
                )?;
                self.colour(&[(n, Colour::Black), (w, Colour::Red)], "Preparing the rotation of case 3.")?;
                if mirrored {
                    self.rotate_left(w)?;
                } else {
                    self.rotate_right(w)?;
                }
                w = sibling_of(&self.tree).expect("the new sibling exists");
            }
            let far =
                if mirrored { self.tree.left_of(w) } else { self.tree.right_of(w) }.expect("a red far child exists");
            let p_colour = self.tree.n(p).colour;
            self.emit(
                Event::DeleteFixupCase { case: 4, mirrored, cursor: c },
                vec![p, w, far],
                describe(4, &format!("Sibling {w} is black with a red far child {far}, so {w} takes {p}'s colour, {p} and {far} turn black, and {p} rotates to settle the extra black."), c),
                Some(c),
            )?;
            self.colour(
                &[(w, p_colour), (p, Colour::Black), (far, Colour::Black)],
                "Preparing the rotation of case 4.",
            )?;
            if mirrored {
                self.rotate_right(p)?;
            } else {
                self.rotate_left(p)?;
            }
            c = FixupCursor {
                x: self.tree.root,
                x_parent: None,
                x_side: ChildSide::Left,
            };
            break;
        }
        if let Some(x) = c.x.filter(|&x| self.tree.n(x).colour == Colour::Red) {
            self.colour(
                &[(x, Colour::Black)],
                "The node that carries the extra black is red, so it turns black and absorbs it.",
            )?;
        }
        Ok(())
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "key" } else { "keys" }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every snapshot of a run, worked out before anything is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Trace {
    snapshots: Vec<Snapshot>,
}

impl Trace {
    /// Runs `ops` on an empty tree.
    ///
    /// # Errors
    ///
    /// Returns `Err` for a duplicate key, a key to delete that is not there, or a run that is too long.
    pub(crate) fn build(ops: &[Op]) -> Result<Self, SolverError> {
        let mut b = Builder {
            tree: Tree::default(),
            snapshots: Vec::new(),
            op: None,
            phase: Phase::Empty,
            replaced_by: BTreeMap::new(),
        };
        b.emit(Event::Start, vec![], "The tree is empty.".to_owned(), None)?;
        for &op in ops {
            match op {
                Op::Insert(k) => b.insert(k)?,
                Op::Delete(k) => b.delete(k)?,
            }
        }
        Ok(Self { snapshots: b.snapshots })
    }

    /// The index of the last snapshot.
    pub(crate) fn last(&self) -> usize {
        self.snapshots.len() - 1
    }

    pub(crate) fn snapshot(&self, index: usize) -> Option<&Snapshot> {
        self.snapshots.get(index)
    }

    #[cfg(test)]
    pub(crate) fn snapshots(&self) -> &[Snapshot] {
        &self.snapshots
    }

    /// The index of the first snapshot in `phase` whose event satisfies `wanted`, searched by meaning and not by
    /// number.
    pub(crate) fn find(&self, phase: Phase, wanted: impl Fn(&Event) -> bool) -> Option<usize> {
        self.snapshots.iter().position(|s| s.phase == phase && wanted(&s.event))
    }
}

#[cfg(test)]
mod unit_tests;
