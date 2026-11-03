//! The golden fixture, found by meaning and not by index, the rules of a red-black tree at every settled frame, the
//! links at every frame including the half-repaired ones, and the cases that the fixture does not reach.

use super::*;

const FIXTURE: [Op; 7] = [
    Op::Insert(10),
    Op::Insert(5),
    Op::Insert(1),
    Op::Insert(15),
    Op::Insert(20),
    Op::Insert(12),
    Op::Delete(15),
];

fn fixture() -> Trace {
    Trace::build(&FIXTURE).expect("the fixture is valid")
}

/// The snapshot that completes `op`.
fn complete(trace: &Trace, op: Op) -> &Snapshot {
    let at = trace
        .snapshots()
        .iter()
        .position(|s| {
            matches!((&s.event, op), (Event::CompleteInsert(k), Op::Insert(o)) | (Event::CompleteDelete(k), Op::Delete(o)) if *k == o)
        })
        .expect("the operation completes");
    &trace.snapshots()[at]
}

fn pairs(s: &Snapshot) -> Vec<(Key, Key)> {
    let mut edges = s.edges.clone();
    edges.sort_unstable();
    edges
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_trace_starts_empty() {
    let t = fixture();
    let first = &t.snapshots()[0];
    assert_eq!(
        (first.event.clone(), first.phase, first.root),
        (Event::Start, Phase::Empty, None)
    );
    assert!(first.nodes.is_empty() && first.settled);
}

#[test]
fn the_ordinals_count_from_zero_without_gaps() {
    for (at, s) in fixture().snapshots().iter().enumerate() {
        assert_eq!(s.ordinal, at);
    }
}

#[test]
fn the_settled_checkpoints_match_the_golden_table() {
    let t = fixture();
    let colour = |s: &Snapshot, k: Key| s.node(k).map(|n| n.colour.letter());
    let at = |op| complete(&t, op);
    let s = at(Op::Insert(10));
    assert_eq!((s.root, colour(s, 10)), (Some(10), Some('B')));
    let s = at(Op::Insert(5));
    assert_eq!(
        (colour(s, 10), colour(s, 5), s.node(10).and_then(|n| n.left)),
        (Some('B'), Some('R'), Some(5))
    );
    let s = at(Op::Insert(1));
    assert_eq!(s.root, Some(5));
    assert_eq!((colour(s, 5), colour(s, 1), colour(s, 10)), (Some('B'), Some('R'), Some('R')));
    assert_eq!(pairs(s), vec![(5, 1), (5, 10)]);
    let s = at(Op::Insert(15));
    assert_eq!(
        (colour(s, 5), colour(s, 1), colour(s, 10), colour(s, 15)),
        (Some('B'), Some('B'), Some('B'), Some('R'))
    );
    assert_eq!(s.node(10).and_then(|n| n.right), Some(15));
    let s = at(Op::Insert(20));
    assert_eq!(pairs(s), vec![(5, 1), (5, 15), (15, 10), (15, 20)]);
    assert_eq!((colour(s, 15), colour(s, 10), colour(s, 20)), (Some('B'), Some('R'), Some('R')));
    let s = at(Op::Insert(12));
    assert_eq!(
        (colour(s, 15), colour(s, 10), colour(s, 12), colour(s, 20)),
        (Some('R'), Some('B'), Some('R'), Some('B'))
    );
    assert_eq!(s.node(10).and_then(|n| n.right), Some(12));
    let s = at(Op::Delete(15));
    assert_eq!(
        (s.root, colour(s, 1), colour(s, 12), colour(s, 10), colour(s, 20)),
        (Some(5), Some('B'), Some('R'), Some('B'), Some('B'))
    );
    assert_eq!(pairs(s), vec![(5, 1), (5, 12), (12, 10), (12, 20)]);
    assert!(!s.has_key(15));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_four_rotations_come_in_order_in_their_phases() {
    let t = fixture();
    let rotations: Vec<(Phase, &str, Key)> = t
        .snapshots()
        .iter()
        .filter_map(|s| match s.event {
            Event::RotateLeft(k) => Some((s.phase, "left", k)),
            Event::RotateRight(k) => Some((s.phase, "right", k)),
            _ => None,
        })
        .collect();
    assert_eq!(
        rotations,
        vec![
            (Phase::Insert, "right", 10),
            (Phase::Insert, "left", 10),
            (Phase::Delete, "left", 10),
            (Phase::Delete, "right", 20),
        ]
    );
}

#[test]
fn every_rotation_changes_the_links() {
    let t = fixture();
    for (at, s) in t.snapshots().iter().enumerate() {
        if matches!(s.event, Event::RotateLeft(_) | Event::RotateRight(_)) {
            assert_ne!(
                pairs(s),
                pairs(&t.snapshots()[at - 1]),
                "a rotation at {at} left the edges as they were"
            );
        }
    }
}

#[test]
fn the_deletion_runs_choose_then_transplant_then_two_rotations() {
    let t = fixture();
    let choose = t
        .find(Phase::Delete, |e| {
            matches!(e, Event::ChooseSuccessor { deleted: 15, successor: 20 })
        })
        .expect("chosen");
    let transplant = t
        .find(Phase::Delete, |e| matches!(e, Event::Transplant { deleted: 15, successor: 20 }))
        .expect("transplanted");
    let left = t.find(Phase::Delete, |e| matches!(e, Event::RotateLeft(10))).expect("left");
    let right = t.find(Phase::Delete, |e| matches!(e, Event::RotateRight(20))).expect("right");
    assert!(choose < transplant && transplant < left && left < right);
    let s = &t.snapshots()[choose];
    let n15 = s.node(15).expect("15 is still there when the successor is chosen");
    assert_eq!(
        (n15.left, n15.right, s.node(20).map(|n| n.colour)),
        (Some(10), Some(20), Some(Colour::Black))
    );
    assert_eq!(s.node(20).and_then(|n| n.left), None, "the successor has no left child");
}

#[test]
fn the_transplant_moves_the_original_node_and_never_copies_a_key() {
    let t = fixture();
    let at = t
        .find(Phase::Delete, |e| matches!(e, Event::Transplant { .. }))
        .expect("transplant");
    let (before, after) = (&t.snapshots()[at - 1], &t.snapshots()[at]);
    assert!(before.has_key(15) && !after.has_key(15));
    assert_eq!(
        after.node(20).map(|n| (n.parent, n.colour, n.left)),
        Some((Some(5), Colour::Red, Some(10)))
    );
    assert_eq!(after.nodes.len(), before.nodes.len() - 1, "one node fewer, and none renamed");
    assert_eq!(after.replaced_by.get(&15), Some(&20));
    assert!(before.replaced_by.is_empty(), "the mapping exists only from the transplant on");
    let keys = |s: &Snapshot| s.nodes.iter().map(|n| n.key).collect::<Vec<_>>();
    let mut expected = keys(before);
    expected.retain(|&k| k != 15);
    assert_eq!(keys(after), expected);
}

#[test]
fn the_replacement_mapping_is_kept_to_the_end_of_the_run() {
    let t = fixture();
    let at = t
        .find(Phase::Delete, |e| matches!(e, Event::Transplant { .. }))
        .expect("transplant");
    for s in &t.snapshots()[at..] {
        assert_eq!(s.replaced_by.get(&15), Some(&20));
    }
}

#[test]
fn the_fixup_starts_at_the_missing_right_child_of_twenty() {
    let t = fixture();
    let at = t
        .find(Phase::Delete, |e| matches!(e, Event::Transplant { .. }))
        .expect("transplant");
    assert_eq!(
        t.snapshots()[at].cursor,
        Some(FixupCursor {
            x: None,
            x_parent: Some(20),
            x_side: ChildSide::Right
        })
    );
    let cases: Vec<(u8, bool)> = t
        .snapshots()
        .iter()
        .filter_map(|s| match s.event {
            Event::DeleteFixupCase { case, mirrored, .. } => Some((case, mirrored)),
            _ => None,
        })
        .collect();
    assert_eq!(cases, vec![(3, true), (4, true)]);
    let first = t
        .find(Phase::Delete, |e| matches!(e, Event::DeleteFixupCase { .. }))
        .expect("a case");
    assert!(
        t.snapshots()[first].description.contains("missing child"),
        "{}",
        t.snapshots()[first].description
    );
}

#[test]
fn the_frame_after_the_transplant_is_a_repair_in_progress() {
    let t = fixture();
    let at = t
        .find(Phase::Delete, |e| matches!(e, Event::Transplant { .. }))
        .expect("transplant");
    assert!(!t.snapshots()[at].settled, "the black heights differ until the fix-up is done");
    assert!(t.snapshots().iter().any(|s| !s.settled), "some frame is transitional");
}

#[test]
fn every_description_names_its_pivot() {
    for s in fixture().snapshots() {
        assert!(!s.description.is_empty());
        match s.event {
            Event::RotateLeft(k) | Event::RotateRight(k) => {
                assert!(s.description.contains(&k.to_string()), "{}", s.description);
                assert!(
                    s.description.contains("subtree") || s.description.contains("no "),
                    "{}",
                    s.description
                );
            },
            _ => {},
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_frame_has_whole_links_and_every_settled_frame_follows_the_rules() {
    for s in fixture().snapshots() {
        check_structure(s.root, &s.nodes).unwrap_or_else(|e| panic!("frame {}: {e}", s.ordinal));
        assert_eq!(s.settled, check_rules(s.root, &s.nodes).is_ok(), "frame {}", s.ordinal);
        if matches!(s.event, Event::CompleteInsert(_) | Event::CompleteDelete(_) | Event::Start) {
            assert!(s.settled, "frame {} completes an operation", s.ordinal);
        }
        assert_eq!(s.edges.len(), s.nodes.iter().filter(|n| n.parent.is_some()).count());
    }
}

#[test]
fn the_edges_are_the_parent_links_and_nothing_else() {
    for s in fixture().snapshots() {
        let from_links: BTreeSet<(Key, Key)> = s
            .nodes
            .iter()
            .flat_map(|n| [n.left, n.right].into_iter().flatten().map(move |c| (n.key, c)))
            .collect();
        assert_eq!(s.edges.iter().copied().collect::<BTreeSet<_>>(), from_links);
    }
}

#[test]
fn a_broken_tree_is_caught_by_the_checkers() {
    let node = |key, parent, left, right, colour| NodeSnapshot {
        key,
        colour,
        parent,
        left,
        right,
    };
    let ok = [
        node(5, None, Some(1), Some(9), Colour::Black),
        node(1, Some(5), None, None, Colour::Red),
        node(9, Some(5), None, None, Colour::Red),
    ];
    assert!(check_rules(Some(5), &ok).is_ok());
    let mut red_root = ok;
    red_root[0].colour = Colour::Red;
    assert!(check_rules(Some(5), &red_root).is_err());
    assert!(check_structure(Some(5), &red_root).is_ok(), "a red root is a rule, not a link");
    let mut red_red = [
        node(5, None, Some(1), None, Colour::Black),
        node(1, Some(5), Some(0), None, Colour::Red),
        node(0, Some(1), None, None, Colour::Red),
    ];
    assert!(check_rules(Some(5), &red_red).unwrap_err().contains("red child"));
    red_red[2].colour = Colour::Black;
    assert!(check_rules(Some(5), &red_red).is_err(), "unequal black heights");
    let mut cycle = ok;
    cycle[1].left = Some(5);
    assert!(check_structure(Some(5), &cycle).is_err());
    let mut unordered = ok;
    unordered[1].key = 7;
    unordered[0].left = Some(7);
    assert!(check_structure(Some(5), &unordered).is_err());
    let mut wrong_parent = ok;
    wrong_parent[2].parent = Some(1);
    assert!(check_structure(Some(5), &wrong_parent).is_err());
    assert!(check_structure(Some(5), &ok[..2]).is_err(), "a missing child");
    assert!(check_structure(None, &ok).is_err());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_duplicate_key_is_refused() {
    assert_eq!(Trace::build(&[Op::Insert(3), Op::Insert(3)]), Err(SolverError::DuplicateKey(3)));
}

#[test]
fn a_key_that_is_not_there_cannot_be_deleted() {
    assert_eq!(Trace::build(&[Op::Delete(3)]), Err(SolverError::MissingKey(3)));
    assert_eq!(Trace::build(&[Op::Insert(1), Op::Delete(2)]), Err(SolverError::MissingKey(2)));
}

#[test]
fn deleting_the_only_key_leaves_an_empty_tree() {
    let t = Trace::build(&[Op::Insert(7), Op::Delete(7)]).expect("valid");
    let last = &t.snapshots()[t.last()];
    assert!(last.nodes.is_empty() && last.root.is_none() && last.settled);
    assert!(last.description.contains("empty"));
}

#[test]
fn the_empty_run_is_one_snapshot() {
    assert_eq!(Trace::build(&[]).expect("valid").last(), 0);
}

#[test]
fn deleting_the_root_with_two_children_works() {
    let t = Trace::build(&[Op::Insert(2), Op::Insert(1), Op::Insert(3), Op::Delete(2)]).expect("valid");
    let s = &t.snapshots()[t.last()];
    assert_eq!(s.nodes.iter().map(|n| n.key).collect::<Vec<_>>(), vec![1, 3]);
    assert!(s.settled);
}

#[test]
fn a_successor_deeper_than_the_right_child_is_moved_with_its_right_subtree() {
    let t = Trace::build(&[
        Op::Insert(50),
        Op::Insert(30),
        Op::Insert(70),
        Op::Insert(60),
        Op::Insert(80),
        Op::Insert(65),
        Op::Delete(50),
    ])
    .expect("valid");
    let at = t
        .find(Phase::Delete, |e| matches!(e, Event::Transplant { deleted: 50, .. }))
        .expect("transplant");
    let Event::Transplant { successor, .. } = t.snapshots()[at].event else {
        unreachable!()
    };
    assert_eq!(successor, 60, "the smallest key of the right subtree");
    assert_ne!(
        t.snapshots()[at - 1].node(60).and_then(|n| n.parent),
        Some(50),
        "it was not the right child"
    );
    assert!(t.snapshots()[t.last()].settled);
    assert_eq!(t.snapshots()[t.last()].nodes.len(), 5);
}

#[test]
fn a_missing_left_child_starts_the_fixup_on_the_left_side() {
    // 10 black with children 5 and 15, both black after the recolouring of 20 below. Deleting 5 leaves a NIL on the
    // left of 10.
    let t =
        Trace::build(&[Op::Insert(10), Op::Insert(5), Op::Insert(15), Op::Insert(20), Op::Delete(5)]).expect("valid");
    let at = t.find(Phase::Delete, |e| matches!(e, Event::Splice { .. })).expect("a splice");
    let cursor = t.snapshots()[at].cursor;
    assert_eq!(
        cursor.map(|c| (c.x, c.x_parent, c.x_side)),
        Some((None, Some(10), ChildSide::Left))
    );
    assert!(t.snapshots()[t.last()].settled);
}

#[test]
fn a_missing_right_child_starts_the_fixup_on_the_right_side() {
    let t =
        Trace::build(&[Op::Insert(10), Op::Insert(5), Op::Insert(15), Op::Insert(1), Op::Delete(15)]).expect("valid");
    let at = t.find(Phase::Delete, |e| matches!(e, Event::Splice { .. })).expect("a splice");
    let cursor = t.snapshots()[at].cursor;
    assert_eq!(
        cursor.map(|c| (c.x, c.x_parent, c.x_side)),
        Some((None, Some(10), ChildSide::Right))
    );
    assert!(t.snapshots()[t.last()].settled);
}

#[test]
fn deleting_a_red_leaf_needs_no_fixup() {
    let t = Trace::build(&[Op::Insert(10), Op::Insert(5), Op::Delete(5)]).expect("valid");
    assert!(t.find(Phase::Delete, |e| matches!(e, Event::DeleteFixupCase { .. })).is_none());
    assert!(t.snapshots()[t.last()].settled);
}

#[test]
fn too_many_keys_are_refused() {
    let ops: Vec<Op> = (0..=MAX_KEYS as Key).map(Op::Insert).collect();
    assert!(matches!(
        Trace::build(&ops),
        Err(SolverError::TooManyKeys { .. }) | Err(SolverError::BudgetExceeded { .. })
    ));
}

#[test]
fn every_error_says_what_it_is_about() {
    for e in [
        SolverError::DuplicateKey(1),
        SolverError::MissingKey(2),
        SolverError::TooManyKeys { count: 9, max: 8 },
        SolverError::BudgetExceeded { max: 3 },
    ] {
        assert!(!e.to_string().is_empty());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A small deterministic generator, so that the broad test is the same on every run.
struct Splitmix(u64);

impl Splitmix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[test]
fn many_random_runs_keep_every_rule_and_match_a_set() {
    let mut rng = Splitmix(0x0BAD_C0DE_2026_1103);
    for run in 0..300 {
        let mut present = BTreeSet::new();
        let mut ops = Vec::new();
        for _ in 0..(5 + rng.next() % 30) {
            let key = (rng.next() % 40) as Key;
            if present.contains(&key) {
                present.remove(&key);
                ops.push(Op::Delete(key));
            } else {
                present.insert(key);
                ops.push(Op::Insert(key));
            }
        }
        let t = Trace::build(&ops).unwrap_or_else(|e| panic!("run {run}: {e}"));
        let mut expected = BTreeSet::new();
        for s in t.snapshots() {
            check_structure(s.root, &s.nodes).unwrap_or_else(|e| panic!("run {run} frame {}: {e}", s.ordinal));
            match s.event {
                Event::CompleteInsert(k) => {
                    expected.insert(k);
                },
                Event::CompleteDelete(k) => {
                    expected.remove(&k);
                },
                _ => {},
            }
            if matches!(s.event, Event::CompleteInsert(_) | Event::CompleteDelete(_)) {
                check_rules(s.root, &s.nodes).unwrap_or_else(|e| panic!("run {run} frame {}: {e}", s.ordinal));
                assert_eq!(s.nodes.iter().map(|n| n.key).collect::<BTreeSet<_>>(), expected, "run {run}");
            }
        }
    }
}

#[test]
fn a_fixup_case_names_what_it_will_do_and_changes_nothing_itself() {
    // Every case of the fix-up that any of these runs reaches, on the fixture and on trees that reach the other cases.
    let runs: [&[Op]; 3] = [
        &FIXTURE,
        &[
            Op::Insert(10),
            Op::Insert(5),
            Op::Insert(15),
            Op::Insert(1),
            Op::Insert(7),
            Op::Insert(12),
            Op::Insert(20),
            Op::Delete(1),
            Op::Delete(7),
        ],
        &[
            Op::Insert(1),
            Op::Insert(2),
            Op::Insert(3),
            Op::Insert(4),
            Op::Insert(5),
            Op::Insert(6),
            Op::Insert(7),
            Op::Delete(1),
            Op::Delete(2),
        ],
    ];
    let mut cases = BTreeSet::new();
    for ops in runs {
        let t = Trace::build(ops).expect("valid");
        for (at, s) in t.snapshots().iter().enumerate() {
            let Event::DeleteFixupCase { case, .. } = s.event else { continue };
            cases.insert(case);
            let before = &t.snapshots()[at - 1];
            assert_eq!((&s.nodes, s.root), (&before.nodes, before.root), "case {case} changed the tree");
            assert!(s.description.contains("will "), "{}", s.description);
            assert!(
                !s.description.contains(" turns ") && !s.description.contains(" rotates "),
                "{}",
                s.description
            );
        }
    }
    assert!(cases.contains(&3) && cases.contains(&4), "{cases:?}");
}
