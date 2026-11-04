use super::*;
use crate::tarjan::{
    condensation::Condensation,
    fixture,
    layout::{LEFT_PANEL, Layout},
    solver::Trace,
};
use demo_common::force::solver::FIXED_DT;

type Setup = (Vec<(f64, f64)>, Vec<(usize, usize)>, Bounds);

/// The final condensation, its nodes' centres in the scene, and the bounds that the page gives the springs.
fn setup() -> Setup {
    let graph = fixture::graph();
    let trace = Trace::build(&graph).unwrap();
    let last = trace.snapshot(trace.last()).unwrap();
    let condensation = Condensation::of(&graph, last);
    let layout = Layout::build(101.3, 91.8);
    let centres = condensation
        .placements()
        .unwrap()
        .iter()
        .map(|p| layout.dag_centre(p.centre))
        .collect();
    let bounds = Bounds::from_origin_and_size(
        190.0,
        layout.dag_origin.1 - 20.0,
        LEFT_PANEL - 5.0 - 190.0,
        layout.size.1 - layout.dag_origin.1,
    );
    (centres, condensation.arcs, bounds)
}

fn lengths(system: &SpringSystem) -> Vec<f64> {
    system
        .springs()
        .iter()
        .map(|s| (system.centre(s.from).unwrap() - system.centre(s.to).unwrap()).length())
        .collect()
}

fn spread(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt() / mean
}

#[test]
fn there_is_a_node_for_each_of_the_ten_components_and_a_spring_for_each_of_the_17_arrows() {
    let (centres, arcs, bounds) = setup();
    let system = build(&centres, &arcs, bounds).unwrap();
    assert_eq!((system.len(), system.springs().len()), (10, 17));
}

#[test]
fn every_spring_has_the_same_weight_and_the_same_rest_length() {
    let (centres, arcs, bounds) = setup();
    let system = build(&centres, &arcs, bounds).unwrap();
    for spring in system.springs() {
        assert_eq!(spring.weight, WEIGHT);
        assert!((spring.rest_length - REST).abs() < 1e-6, "{}", spring.rest_length);
    }
}

#[test]
fn the_springs_make_the_arrows_more_nearly_equal_and_come_to_rest_inside_the_area_without_overlap() {
    let (centres, arcs, bounds) = setup();
    let mut system = build(&centres, &arcs, bounds).unwrap();
    let before = spread(&lengths(&system));
    for _ in 0..(RUN_SECONDS / FIXED_DT) as usize {
        if !system.wants_frames() {
            break;
        }
        system.step(FIXED_DT);
    }
    let after = spread(&lengths(&system));
    assert!(
        after < before * 0.7,
        "the arrows' lengths varied by {before:.2} and now by {after:.2}"
    );
    assert!(system.max_penetration() < 2.0);
    for i in 0..system.len() {
        let c = system.centre(i).unwrap();
        assert!(
            c.x >= bounds.left + RADIUS - 1e-6 && c.x <= bounds.right - RADIUS + 1e-6,
            "{c:?}"
        );
        assert!(
            c.y >= bounds.top + RADIUS - 1e-6 && c.y <= bounds.bottom - RADIUS + 1e-6,
            "{c:?}"
        );
    }
}

#[test]
fn building_the_springs_is_the_same_each_time() {
    let (centres, arcs, bounds) = setup();
    let mut a = build(&centres, &arcs, bounds).unwrap();
    let mut b = build(&centres, &arcs, bounds).unwrap();
    for _ in 0..300 {
        a.step(FIXED_DT);
        b.step(FIXED_DT);
    }
    for i in 0..a.len() {
        assert_eq!(a.centre(i), b.centre(i));
    }
}
