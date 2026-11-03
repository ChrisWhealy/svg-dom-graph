use super::*;
use crate::red_black_tree::solver::{Event, Op, Trace};

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

fn at(s: &Snapshot, key: Key) -> (f64, f64) {
    centres(s).into_iter().find(|&(k, _)| k == key).expect("the key is drawn").1
}

#[test]
fn a_key_never_moves_sideways() {
    let t = fixture();
    for s in t.snapshots() {
        for (key, (x, _)) in centres(s) {
            let rank = UNIVERSE.iter().position(|&k| k == key).expect("in the fixture");
            assert_eq!(x, 100.0 + rank as f64 * 106.0);
        }
    }
}

#[test]
fn depth_gives_the_height() {
    let t = fixture();
    let s = t
        .snapshots()
        .iter()
        .find(|s| s.event == Event::CompleteInsert(12))
        .expect("complete");
    assert_eq!(at(s, 5).1, 96.0);
    assert_eq!(at(s, 15).1, 96.0 + 105.0);
    assert_eq!(at(s, 10).1, 96.0 + 210.0);
    assert_eq!(at(s, 12).1, 96.0 + 315.0);
}

#[test]
fn the_slot_of_a_deleted_key_stays_empty() {
    let t = fixture();
    let last = &t.snapshots()[t.last()];
    assert!(centres(last).iter().all(|&(k, _)| k != 15));
    assert_eq!(at(last, 20).0, 100.0 + 5.0 * 106.0, "20 keeps its slot");
    assert_eq!(at(last, 12).1, 96.0 + 105.0);
}

#[test]
fn everything_fits_in_the_scene() {
    let t = fixture();
    for s in t.snapshots() {
        for (_, (x, y)) in centres(s) {
            assert!(x - RADIUS * 1.5 > 0.0 && x + RADIUS * 1.5 < VIEW_WIDTH, "{x}");
            assert!(y - RADIUS * 1.5 > 0.0 && y + RADIUS * 1.5 < VIEW_HEIGHT, "{y}");
        }
    }
}

#[test]
fn keys_outside_the_fixture_are_spread_by_rank() {
    let t = Trace::build(&[Op::Insert(3), Op::Insert(1), Op::Insert(2)]).expect("valid");
    let s = &t.snapshots()[t.last()];
    let xs: Vec<f64> = centres(s).iter().map(|&(_, (x, _))| x).collect();
    assert_eq!(xs, vec![100.0, 206.0, 312.0]);
}

#[test]
fn an_empty_tree_has_no_centres() {
    assert!(centres(&fixture().snapshots()[0]).is_empty());
}

#[test]
fn the_buttons_fit_in_the_strip_and_do_not_overlap() {
    for (at, &(_, _, x, y, w)) in BUTTONS.iter().enumerate() {
        assert!(
            x >= 0.0 && x + w <= VIEW_WIDTH && y + BUTTON_HEIGHT < RANGE_MIDDLE - 10.0,
            "{at}"
        );
        for &(_, _, x2, y2, w2) in &BUTTONS[at + 1..] {
            let apart = y + BUTTON_HEIGHT <= y2 || y2 + BUTTON_HEIGHT <= y || x + w <= x2 || x2 + w2 <= x;
            assert!(apart, "two buttons overlap");
        }
    }
    const { assert!(RANGE_MIDDLE + 20.0 <= CONTROLS_HEIGHT) };
}
