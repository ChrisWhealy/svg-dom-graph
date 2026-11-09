//! Compares the checkpoint a runner observed with the one the reference model predicted.
//!
//! Exact sections are compared exactly. Geometry is compared coordinate by coordinate within the tolerance the
//! checkpoint carries, and a difference names the element, the coordinate, both values and how far apart they are.
//! Nothing here hashes geometry, since two values well within tolerance can round to different numbers.
//!
//! A difference says whether the rule behind the expected value is settled or provisional. A provisional rule that
//! proves wrong means the model needs correcting, so a runner counts both kinds as failures.

use crate::model::{Checkpoint, GeometryElement};
use serde::Serialize;
use std::collections::BTreeMap;

/// How far apart two view numbers may be. The view is not geometry that the browser rounds, so this is very tight.
const VIEW_TOLERANCE: f64 = 1e-9;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// The expected value follows from the operations alone.
    Modelled,
    /// The expected value comes from a rule that has still to be confirmed against the DOM.
    Provisional,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Difference {
    pub section: &'static str,
    pub basis: Basis,
    /// The node, edge or key that differs, when one does.
    pub element: Option<String>,
    pub detail: String,
    pub expected: String,
    pub actual: String,
    /// How far apart two numbers are, for a numeric difference.
    pub abs_diff: Option<f64>,
}

impl Difference {
    fn new(
        section: &'static str,
        basis: Basis,
        element: Option<&str>,
        detail: &str,
        expected: String,
        actual: String,
    ) -> Self {
        Self {
            section,
            basis,
            element: element.map(str::to_string),
            detail: detail.to_string(),
            expected,
            actual,
            abs_diff: None,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// One line, for a log.
    pub fn summary(&self) -> String {
        let element = self.element.as_deref().map_or(String::new(), |e| format!(" {e}"));
        let basis = if self.basis == Basis::Provisional { " (provisional rule)" } else { "" };
        format!(
            "{}{element}{basis}: {}: expected {}, got {}",
            self.section, self.detail, self.expected, self.actual
        )
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every way that `actual` differs from `expected`, which is empty when they agree.
///
/// The sections that the model does not predict, the DOM ids and the accessibility tree, are not compared. The ids are
/// only checked for duplicates, which the model expects there to be none of.
pub fn compare(expected: &Checkpoint, actual: &Checkpoint) -> Vec<Difference> {
    let mut found = Vec::new();
    compare_topology(expected, actual, &mut found);
    compare_content(expected, actual, &mut found);
    compare_interaction(expected, actual, &mut found);
    compare_geometry(expected, actual, &mut found);

    if expected.paint_order != actual.paint_order {
        found.push(Difference::new(
            "paint_order",
            Basis::Modelled,
            None,
            "the order of the nodes in the document",
            expected.paint_order.join(","),
            actual.paint_order.join(","),
        ));
    }
    if !actual.dom_identity.duplicates.is_empty() {
        found.push(Difference::new(
            "dom_identity",
            Basis::Modelled,
            None,
            "ids used more than once",
            "none".to_string(),
            actual.dom_identity.duplicates.join(","),
        ));
    }
    found
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn count_items(items: &[String]) -> BTreeMap<&str, i64> {
    items.iter().fold(BTreeMap::new(), |mut counts, item| {
        *counts.entry(item.as_str()).or_insert(0) += 1;
        counts
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The items of `want` that `got` lacks and the items of `got` that `want` lacks, each list taken as a multiset.
fn multiset_difference(want: &[String], got: &[String]) -> (Vec<String>, Vec<String>) {
    let (want_count, got_count) = (count_items(want), count_items(got));
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    for key in want_count
        .keys()
        .chain(got_count.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let delta = want_count.get(*key).copied().unwrap_or(0) - got_count.get(*key).copied().unwrap_or(0);
        for _ in 0..delta.max(0) {
            missing.push((*key).to_string());
        }
        for _ in 0..(-delta).max(0) {
            extra.push((*key).to_string());
        }
    }
    (missing, extra)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn compare_topology(expected: &Checkpoint, actual: &Checkpoint, found: &mut Vec<Difference>) {
    let (missing, extra) = multiset_difference(&expected.topology.nodes, &actual.topology.nodes);
    for node in missing {
        found.push(Difference::new(
            "topology",
            Basis::Modelled,
            Some(&node),
            "a node is missing",
            "present".to_string(),
            "absent".to_string(),
        ));
    }
    for node in extra {
        found.push(Difference::new(
            "topology",
            Basis::Modelled,
            Some(&node),
            "an unexpected node",
            "absent".to_string(),
            "present".to_string(),
        ));
    }

    let edges = |checkpoint: &Checkpoint| -> Vec<String> {
        checkpoint
            .topology
            .edges
            .iter()
            .map(|e| format!("{}->{}", e.from, e.to))
            .collect()
    };
    let (missing, extra) = multiset_difference(&edges(expected), &edges(actual));
    for edge in missing {
        found.push(Difference::new(
            "topology",
            Basis::Modelled,
            Some(&edge),
            "an edge is missing",
            "present".to_string(),
            "absent".to_string(),
        ));
    }
    for edge in extra {
        found.push(Difference::new(
            "topology",
            Basis::Modelled,
            Some(&edge),
            "an unexpected edge",
            "absent".to_string(),
            "present".to_string(),
        ));
    }
    for (label, want, got) in [
        ("dangling edges", &expected.topology.dangling, &actual.topology.dangling),
        (
            "duplicate keys",
            &expected.topology.duplicate_keys,
            &actual.topology.duplicate_keys,
        ),
    ] {
        if want != got {
            found.push(Difference::new(
                "topology",
                Basis::Modelled,
                None,
                label,
                want.join(","),
                got.join(","),
            ));
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn compare_content(expected: &Checkpoint, actual: &Checkpoint, found: &mut Vec<Difference>) {
    for key in expected
        .content
        .keys()
        .chain(actual.content.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let (want, got) = (expected.content.get(key), actual.content.get(key));
        if want != got {
            found.push(Difference::new(
                "content",
                Basis::Modelled,
                Some(key),
                "the text of a node",
                want.cloned().unwrap_or_else(|| "(none)".to_string()),
                got.cloned().unwrap_or_else(|| "(none)".to_string()),
            ));
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn compare_interaction(expected: &Checkpoint, actual: &Checkpoint, found: &mut Vec<Difference>) {
    let (want, got) = (&expected.interaction, &actual.interaction);
    let (mut want_selection, mut got_selection) = (want.selection.clone(), got.selection.clone());
    want_selection.sort();
    got_selection.sort();
    if want_selection != got_selection {
        found.push(Difference::new(
            "interaction",
            Basis::Modelled,
            None,
            "the selection",
            want_selection.join(","),
            got_selection.join(","),
        ));
    }
    if want.focused != got.focused {
        found.push(Difference::new(
            "interaction",
            Basis::Modelled,
            None,
            "the focused element",
            format!("{:?}", want.focused),
            format!("{:?}", got.focused),
        ));
    }
    match (&want.view, &got.view) {
        (None, None) => {},
        (Some(w), Some(g)) => {
            for (name, a, b) in [("x", w.x, g.x), ("y", w.y, g.y), ("scale", w.scale, g.scale)] {
                if (a - b).abs() > VIEW_TOLERANCE {
                    let mut difference = Difference::new(
                        "interaction",
                        Basis::Modelled,
                        Some("view"),
                        name,
                        a.to_string(),
                        b.to_string(),
                    );
                    difference.abs_diff = Some((a - b).abs());
                    found.push(difference);
                }
            }
        },
        (w, g) => found.push(Difference::new(
            "interaction",
            Basis::Modelled,
            Some("view"),
            "whether the view is compared",
            format!("{w:?}"),
            format!("{g:?}"),
        )),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn coordinate_names(kind: &str, count: usize) -> Vec<String> {
    match kind {
        "node_box" => ["x", "y", "w", "h"].iter().map(|s| s.to_string()).collect(),
        "label" => ["x", "y"].iter().map(|s| s.to_string()).collect(),
        _ => (0..count)
            .map(|i| format!("{}{}", if i % 2 == 0 { "x" } else { "y" }, i / 2 + 1))
            .collect(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every kind of element is predicted by a rule that Phase 2 confirmed in Chrome, for plain nodes and straight
/// connectors. A rule that has not been confirmed would be [`Basis::Provisional`] here.
fn basis_of(_kind: &str) -> Basis {
    Basis::Modelled
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn index_elements(checkpoint: &Checkpoint) -> BTreeMap<(String, &'static str), &GeometryElement> {
    checkpoint
        .geometry
        .elements
        .iter()
        .map(|e| ((e.key.clone(), e.kind), e))
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn compare_geometry(expected: &Checkpoint, actual: &Checkpoint, found: &mut Vec<Difference>) {
    let tolerance = expected.geometry.tolerance;
    let (want, got) = (index_elements(expected), index_elements(actual));
    for ((key, kind), want_element) in &want {
        let Some(got_element) = got.get(&(key.clone(), *kind)) else {
            found.push(Difference::new(
                "geometry",
                basis_of(kind),
                Some(key),
                &format!("a {kind} is missing"),
                "present".to_string(),
                "absent".to_string(),
            ));
            continue;
        };
        if want_element.coordinates.len() != got_element.coordinates.len() {
            found.push(Difference::new(
                "geometry",
                basis_of(kind),
                Some(key),
                &format!("the number of coordinates of a {kind}"),
                want_element.coordinates.len().to_string(),
                got_element.coordinates.len().to_string(),
            ));
            continue;
        }
        let names = coordinate_names(kind, want_element.coordinates.len());
        for (position, (a, b)) in want_element.coordinates.iter().zip(&got_element.coordinates).enumerate() {
            let distance = (a - b).abs();
            // A coordinate that is not a number can never be within tolerance, so it is always reported.
            if distance.is_nan() || distance > tolerance {
                let mut difference = Difference::new(
                    "geometry",
                    basis_of(kind),
                    Some(key),
                    &format!("{kind} {}", names[position]),
                    a.to_string(),
                    b.to_string(),
                );
                difference.abs_diff = Some(distance);
                found.push(difference);
            }
        }
    }
    for (key, kind) in got.keys() {
        if !want.contains_key(&(key.clone(), *kind)) {
            found.push(Difference::new(
                "geometry",
                basis_of(kind),
                Some(key),
                &format!("an unexpected {kind}"),
                "absent".to_string(),
                "present".to_string(),
            ));
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
