//! The known answers of the sparse-matrix core: the cases of `spmv/cases.json`, whose expectations were derived by
//! hand, and the pins that the JavaScript oracle computed separately.

use serde_json::Value;
use spmv_core::budget::Budget;
use spmv_core::matrix::Matrix;
use spmv_core::spec::Spec;
use spmv_core::trace::outcome;

const CASES: &str = include_str!("../../spmv/cases.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("spmv/cases.json is JSON")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every case gives the outcome, code and message that its author derived by hand, and the statistics where it names
/// them.
#[test]
fn the_cases_give_the_outcomes_that_were_derived_by_hand() {
    let all = cases();
    let mut checked = 0;
    for (name, case) in all.as_object().expect("an object") {
        let spec = Spec::from_json(&case["spec"]).expect("a spec");
        let budget = Budget::from_json(&case["budget"]).expect("a budget");
        let got = outcome(&spec, &budget);
        let Some(want) = case.get("expect") else { continue };
        checked += 1;
        assert_eq!(got["outcome"], want["outcome"], "{name}: {got}");
        for key in ["code", "message"] {
            if want.get(key).is_some() {
                assert_eq!(got[key], want[key], "{name}: {key}");
            }
        }
        if let Some(stats) = want.get("stats") {
            assert_eq!(&got["stats"], stats, "{name}: stats");
        }
    }
    assert!(checked >= 20, "only {checked} cases had an expectation");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A matrix and a vector written out by hand give the hashes that the oracle computed for the same bytes, in another
/// language.
#[test]
fn the_hashes_of_a_matrix_written_by_hand_are_those_the_oracle_computed() {
    let m = Matrix::from_rows(4, &[vec![(0, 2), (2, -1)], vec![], vec![(0, 3), (1, 4), (3, 5)]]);
    assert_eq!(m.hash(), "2e2f3949569a9b726f35e4946ffa0d2f62a50ec221006d27ac0018891ab6067a");
    assert_eq!(m.multiply(&[1, 10, 100, -1]), vec![-98, 0, 38]);
    assert_eq!(
        spmv_core::hash::vector_hash(&[1, 10, 100, -1]),
        "f45761e606f61b883f930b91bf546bdb6bb76dbcd532be3f6132a99279061a14"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A specification that is refused is refused before any matrix is generated: a size that would take gigabytes is
/// answered at once.
#[test]
fn a_hostile_size_is_refused_before_anything_is_generated() {
    let spec = Spec {
        seed: 1,
        rows: 1_000_000,
        cols: 1_000_000,
        per_row: 64,
        hub_cols: 0,
        hub_percent: 0,
        label_len: 1,
        cells: 1000,
        updates: 0,
        steps: 0,
    };
    let start = std::time::Instant::now();
    let got = outcome(&spec, &Budget::default());
    assert_eq!(got["outcome"], "refused");
    assert_eq!(got["code"], "nodes");
    assert!(start.elapsed().as_millis() < 500, "the refusal took {:?}", start.elapsed());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same seed gives the same outcome, and the panel is deterministic across calls.
#[test]
fn an_outcome_is_deterministic() {
    for seed in [0, 1, 17, 99] {
        let spec = Spec::panel(seed);
        assert_eq!(outcome(&spec, &Budget::default()), outcome(&spec, &Budget::default()));
    }
}
