use super::*;
use svg_dom::root::utils::{Point, Size};

fn test_rect(x: f64, y: f64) -> Rect {
    Rect {
        origin: Point::new(x, y),
        size: Size::new(10.0, 10.0),
    }
}

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
