use super::*;
use crate::session::{Extent, PreflightError};

#[test]
fn the_scenarios_are_offered_in_order_with_the_shapes_that_the_design_gives() {
    let entries = catalogue().expect("the catalogue");
    let shape: Vec<_> = entries
        .iter()
        .map(|e| (e.spec.id.as_str(), e.spec.servers, e.advanced))
        .collect();
    assert_eq!(
        shape,
        [
            ("E0", 2, false),
            ("E3-NOOP", 1, false),
            ("scaled", 3, false),
            ("five", 5, false),
            ("W6", 3, true)
        ]
    );
}

#[test]
fn every_scenario_replays_to_exactly_the_totals_that_it_is_known_by() {
    let found: Vec<_> = catalogue()
        .expect("the catalogue")
        .iter()
        .map(|e| e.spec.preflight().map(|x| (x.slices, x.pops)))
        .collect();
    assert_eq!(
        found,
        [
            Ok((12, 19)),
            Ok((9, 9)),
            Ok((487, 1_009)),
            Ok((493, 1_103)),
            Ok((5_464, 11_214))
        ]
    );
}

#[test]
fn the_ceiling_of_each_scenario_is_its_total_so_that_one_more_step_is_refused() {
    for mut entry in catalogue().expect("the catalogue") {
        let (slices, pops) = (entry.spec.expected.slices, entry.spec.expected.pops);
        assert_eq!((entry.spec.expected.max_slices, entry.spec.expected.max_pops), (slices, pops));
        entry.spec.expected.max_slices = slices - 1;
        entry.spec.expected.max_pops = pops;
        assert_eq!(
            entry.spec.preflight(),
            Err(PreflightError::Ceiling {
                what: "slices",
                limit: slices - 1
            }),
            "{}",
            entry.spec.id
        );
    }
}

#[test]
fn a_total_that_is_not_the_known_one_is_a_mismatch_and_not_a_pass() {
    let mut e0 = catalogue().expect("the catalogue").remove(0);
    e0.spec.expected.slices = 11;
    e0.spec.expected.max_slices = 100;
    e0.spec.expected.max_pops = 100;
    assert_eq!(
        e0.spec.preflight(),
        Err(PreflightError::Mismatch {
            found: Extent { slices: 12, pops: 19 },
            expected: Extent { slices: 11, pops: 19 }
        })
    );
}
