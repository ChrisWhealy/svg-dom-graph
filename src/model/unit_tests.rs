use super::edge::EdgeId;
use crate::model::graph::Graph;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn test_rect(x: f64, y: f64) -> Rect {
    Rect {
        origin: Point::new(x, y),
        size: Size::new(10.0, 10.0),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn add_node_returns_distinct_ids() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(test_rect(0.0, 0.0), "A");
    let b = graph.add_node(test_rect(10.0, 10.0), "B");
    if a == b {
        return Err("two distinct nodes got the same id".into());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn node_returns_none_for_an_id_from_a_different_graph() -> Result<(), String> {
    let mut other_graph = Graph::new();
    let foreign_id = other_graph.add_node(test_rect(0.0, 0.0), "foreign");

    let empty_graph = Graph::new();
    if empty_graph.node(foreign_id).is_some() {
        return Err("node() found data for an id this graph never issued".into());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn edge_returns_none_for_an_id_from_a_different_graph() -> Result<(), String> {
    let mut other_graph = Graph::new();
    let a = other_graph.add_node(test_rect(0.0, 0.0), "A");
    let b = other_graph.add_node(test_rect(10.0, 10.0), "B");
    let foreign_id = other_graph.add_edge(a, b);

    let empty_graph = Graph::new();
    if empty_graph.edge(foreign_id).is_some() {
        return Err("edge() found data for an id this graph never issued".into());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn incident_edges_is_empty_for_a_node_with_no_edges() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(test_rect(0.0, 0.0), "A");
    check_eq(graph.incident_edges(a), &[] as &[EdgeId])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn add_edge_registers_incidence_on_both_endpoints() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(test_rect(0.0, 0.0), "A");
    let b = graph.add_node(test_rect(10.0, 10.0), "B");
    let edge = graph.add_edge(a, b);

    check_eq(graph.incident_edges(a), &[edge] as &[EdgeId])?;
    check_eq(graph.incident_edges(b), &[edge] as &[EdgeId])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn incident_edges_collects_every_edge_touching_a_shared_node() -> Result<(), String> {
    let mut graph = Graph::new();
    let root = graph.add_node(test_rect(0.0, 0.0), "root");
    let left = graph.add_node(test_rect(10.0, 10.0), "left");
    let right = graph.add_node(test_rect(20.0, 20.0), "right");
    let left_edge = graph.add_edge(root, left);
    let right_edge = graph.add_edge(root, right);

    check_eq(graph.incident_edges(root), &[left_edge, right_edge] as &[EdgeId])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn set_node_rect_updates_the_stored_rect() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(test_rect(0.0, 0.0), "A");

    let moved = test_rect(99.0, 99.0);
    graph.set_node_rect(a, moved);

    check_eq(graph.node(a).map(|n| n.rect), Some(moved))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn set_node_rect_does_nothing_for_an_id_from_a_different_graph() -> Result<(), String> {
    let mut other_graph = Graph::new();
    let foreign_id = other_graph.add_node(test_rect(0.0, 0.0), "foreign");

    // Must not panic when given an id this graph never issued.
    let mut empty_graph = Graph::new();
    empty_graph.set_node_rect(foreign_id, test_rect(1.0, 1.0));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn edge_records_its_endpoints() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(test_rect(0.0, 0.0), "A");
    let b = graph.add_node(test_rect(10.0, 10.0), "B");
    let edge = graph.add_edge(a, b);

    let stored = graph.edge(edge).ok_or("edge() returned None for an id it just issued")?;
    check_eq(stored.from, a)?;
    check_eq(stored.to, b)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn foreign_node_id_at_the_same_sequence_position_does_not_resolve_locally() -> Result<(), String> {
    let mut graph_a = Graph::new();
    // graph_a's first node: same sequence position graph_b's first node below will also get.
    let a0 = graph_a.add_node(test_rect(0.0, 0.0), "a0");

    let mut graph_b = Graph::new();
    let b0 = graph_b.add_node(test_rect(50.0, 50.0), "b0");

    // If ids were only distinguished by sequence number, a0 and b0 would be indistinguishable: both are their
    // graph's first node. graph_b must still refuse a0, and must still resolve its own b0 correctly.
    if graph_b.node(a0).is_some() {
        return Err(
            "a NodeId from graph_a resolved to a node in graph_b, despite sharing b0's sequence position".into(),
        );
    }
    check_eq(graph_b.node(b0).map(|n| n.rect), Some(test_rect(50.0, 50.0)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn foreign_edge_id_at_the_same_sequence_position_does_not_resolve_locally() -> Result<(), String> {
    let mut graph_a = Graph::new();
    let a1 = graph_a.add_node(test_rect(0.0, 0.0), "a1");
    let a2 = graph_a.add_node(test_rect(10.0, 10.0), "a2");
    // graph_a's first edge: same sequence position graph_b's first edge below will also get.
    let edge_a = graph_a.add_edge(a1, a2);

    let mut graph_b = Graph::new();
    let b1 = graph_b.add_node(test_rect(50.0, 50.0), "b1");
    let b2 = graph_b.add_node(test_rect(60.0, 60.0), "b2");
    let edge_b = graph_b.add_edge(b1, b2);

    if graph_b.edge(edge_a).is_some() {
        return Err(
            "an EdgeId from graph_a resolved to an edge in graph_b, despite sharing edge_b's sequence position".into(),
        );
    }
    check_eq(graph_b.edge(edge_b).map(|e| (e.from, e.to)), Some((b1, b2)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn add_node_accepts_a_borrowed_non_static_label() -> Result<(), String> {
    let mut graph = Graph::new();

    // Built at runtime and borrowed from a local `String`: this could never satisfy `&'static str`, which is
    // exactly the constraint `add_node` used to impose on every caller, including one reading a label from data
    // fetched at runtime.
    let owned_label = format!("node-{}", 42);
    let id = graph.add_node(test_rect(0.0, 0.0), owned_label.as_str());

    check_eq(graph.node(id).map(|n| n.label.as_str()), Some("node-42"))
}
