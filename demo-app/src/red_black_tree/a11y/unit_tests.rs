//! The resolution of a choice, the keyboard, and the names, on the golden fixture.

use super::*;
use crate::red_black_tree::solver::{Op, Phase, Trace};

fn fixture() -> Trace {
    Trace::build(&[
        Op::Insert(10),
        Op::Insert(5),
        Op::Insert(1),
        Op::Insert(15),
        Op::Insert(20),
        Op::Insert(12),
        Op::Delete(15),
    ])
    .expect("valid")
}

fn complete_insert(t: &Trace, key: Key) -> &Snapshot {
    t.snapshots()
        .iter()
        .find(|s| s.event == Event::CompleteInsert(key))
        .expect("complete")
}

fn transplant(t: &Trace) -> usize {
    t.find(Phase::Delete, |e| matches!(e, Event::Transplant { .. }))
        .expect("transplant")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_key_in_the_tree_is_itself() {
    let t = fixture();
    assert_eq!(resolve(Some(10), complete_insert(&t, 12)), Some(10));
}

#[test]
fn no_choice_is_no_selection() {
    let t = fixture();
    assert_eq!(resolve(None, &t.snapshots()[t.last()]), None);
}

#[test]
fn a_deleted_key_resolves_to_its_successor_and_comes_back_on_reversing() {
    let t = fixture();
    let at = transplant(&t);
    assert_eq!(
        resolve(Some(15), &t.snapshots()[at - 1]),
        Some(15),
        "before the transplant 15 is still there"
    );
    assert_eq!(resolve(Some(15), &t.snapshots()[at]), Some(20));
    assert_eq!(resolve(Some(15), &t.snapshots()[t.last()]), Some(20));
    // Going back past the deletion, with the intent unchanged, selects 15 again.
    assert_eq!(resolve(Some(15), complete_insert(&t, 12)), Some(15));
}

#[test]
fn choosing_the_successor_after_the_deletion_does_not_come_back_to_15() {
    let t = fixture();
    assert_eq!(resolve(Some(20), complete_insert(&t, 12)), Some(20));
}

#[test]
fn a_key_not_yet_inserted_falls_back_to_the_root_and_then_the_smallest_key() {
    let t = fixture();
    let s = complete_insert(&t, 5);
    assert_eq!(resolve(Some(12), s), s.root);
    assert_eq!(resolve(Some(12), &t.snapshots()[0]), None, "an empty tree has nothing");
}

#[test]
fn a_direct_seek_and_a_walk_give_the_same_answer() {
    let t = fixture();
    let walked: Vec<_> = t.snapshots().iter().map(|s| resolve(Some(15), s)).collect();
    let direct: Vec<_> = (0..=t.last())
        .rev()
        .rev()
        .map(|i| resolve(Some(15), &t.snapshots()[i]))
        .collect();
    assert_eq!(walked, direct);
}

#[test]
fn a_chain_of_replacements_is_followed_and_a_cycle_ends() {
    let t = fixture();
    let mut s = t.snapshots()[t.last()].clone();
    s.replaced_by.clear();
    s.replaced_by.insert(99, 98);
    s.replaced_by.insert(98, 12);
    assert_eq!(resolve(Some(99), &s), Some(12));
    s.replaced_by.insert(98, 99);
    assert_eq!(resolve(Some(99), &s), s.root, "a cycle falls back to the root");
}

#[test]
fn the_roving_key_starts_at_the_root_and_follows_the_same_rule() {
    let t = fixture();
    let s = complete_insert(&t, 5);
    assert_eq!(resolve_roving(None, s), s.root);
    assert_eq!(resolve_roving(Some(15), &t.snapshots()[t.last()]), Some(20));
    assert_eq!(resolve_roving(None, &t.snapshots()[0]), None);
}

#[test]
fn a_replacement_is_announced_and_an_ordinary_choice_is_not() {
    let t = fixture();
    let last = &t.snapshots()[t.last()];
    assert!(
        selection_note(Some(15), last)
            .expect("a note")
            .contains("took the place of your selected 15")
    );
    assert_eq!(selection_note(Some(20), last), None);
    assert_eq!(selection_note(None, last), None);
    assert!(
        selection_note(Some(12), complete_insert(&t, 5))
            .expect("a note")
            .contains("root")
    );
    assert!(selection_note(Some(12), &t.snapshots()[0]).expect("a note").contains("empty"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_keys_go_through_the_tree() {
    let t = fixture();
    let s = complete_insert(&t, 12);
    // 5 is the root, with 1 on its left and 15 on its right. 15 has 10 on its left and 20 on its right.
    assert_eq!(navigate(s, 5, Nav::Left), None, "the root has no parent");
    assert_eq!(navigate(s, 5, Nav::Right), Some(1), "left before right");
    assert_eq!(navigate(s, 1, Nav::Right), None, "a leaf goes nowhere");
    assert_eq!(navigate(s, 10, Nav::Left), Some(15));
    assert_eq!(
        navigate(s, 15, Nav::Right),
        Some(10),
        "a node with two children goes to its left"
    );
    assert_eq!(navigate(s, 15, Nav::Down), Some(20), "and reaches its right child with Down");
    assert_eq!(navigate(s, 10, Nav::Right), Some(12), "only a right child");
    assert_eq!(navigate(s, 5, Nav::Down), Some(10));
    assert_eq!(navigate(s, 5, Nav::Up), Some(1));
    assert_eq!(navigate(s, 1, Nav::Up), None);
    assert_eq!(navigate(s, 20, Nav::Down), None);
    assert_eq!(navigate(s, 20, Nav::Home), Some(1));
    assert_eq!(navigate(s, 1, Nav::Home), None, "already there");
    assert_eq!(navigate(s, 1, Nav::End), Some(20));
    assert_eq!(navigate(s, 99, Nav::Down), None, "a key that is not there");
}

#[test]
fn only_the_navigation_keys_are_navigation() {
    assert_eq!(Nav::from_key("ArrowUp"), Some(Nav::Up));
    assert_eq!(Nav::from_key("End"), Some(Nav::End));
    assert_eq!(Nav::from_key("Enter"), None);
    assert_eq!(Nav::from_key("a"), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_items_have_truthful_levels_positions_and_names() {
    let t = fixture();
    let s = complete_insert(&t, 12);
    let items = items(s, Some(12));
    assert_eq!(items.len(), 6);
    let root = &items[&5];
    assert_eq!(
        (root.level, root.pos_in_set, root.set_size, root.expanded),
        (1, 1, 1, Some(true))
    );
    assert_eq!(root.name, "5, black, root");
    let twelve = &items[&12];
    assert_eq!(
        (twelve.level, twelve.pos_in_set, twelve.set_size, twelve.expanded),
        (4, 1, 1, None)
    );
    assert_eq!(twelve.name, "12, red, right child of 10");
    assert!(twelve.selected && !items[&5].selected);
    let (left, right) = (&items[&10], &items[&20]);
    assert_eq!((left.pos_in_set, left.set_size, right.pos_in_set, right.set_size), (1, 2, 2, 2));
    assert_eq!(items[&15].children, vec![10, 20]);
}

#[test]
fn an_empty_tree_has_no_items_and_a_name() {
    let t = fixture();
    assert!(items(&t.snapshots()[0], None).is_empty());
    assert_eq!(tree_name(&t.snapshots()[0]), "Red-black tree, empty");
    assert_eq!(tree_name(complete_insert(&t, 10)), "Red-black tree, 1 key, root 10");
    assert!(tree_name(complete_insert(&t, 5)).contains("2 keys"));
}

#[test]
fn every_name_says_the_colour_in_words() {
    let t = fixture();
    for s in t.snapshots() {
        for n in &s.nodes {
            assert!(item_name(s, n.key).contains(n.colour.word()));
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_repair_is_not_described_as_a_valid_tree() {
    let t = fixture();
    let at = transplant(&t);
    assert!(summary(&t.snapshots()[at], t.last()).contains("Repair in progress"));
    let done = &t.snapshots()[t.last()];
    assert!(!summary(done, t.last()).contains("Repair in progress"));
    assert!(summary(done, t.last()).starts_with(&format!("Step {} of {}", t.last(), t.last())));
}

#[test]
fn only_finished_operations_and_the_big_moves_are_announced_while_playing() {
    assert!(announced_while_playing(&Event::RotateLeft(1)));
    assert!(announced_while_playing(&Event::CompleteInsert(1)));
    assert!(announced_while_playing(&Event::Transplant { deleted: 1, successor: 2 }));
    assert!(!announced_while_playing(&Event::Insert(1)));
    assert!(!announced_while_playing(&Event::Recolour(vec![])));
    assert!(!announced_while_playing(&Event::Start));
}
