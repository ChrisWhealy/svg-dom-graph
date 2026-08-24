use super::*;
use crate::model::graph::Graph;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn a_node_id() -> NodeId {
    let mut graph = Graph::new();
    graph.add_node(
        Rect {
            origin: Point::new(0.0, 0.0),
            size: Size::new(1.0, 1.0),
        },
        "n",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn an_edge_id() -> EdgeId {
    let mut graph = Graph::new();
    let a = graph.add_node(
        Rect {
            origin: Point::new(0.0, 0.0),
            size: Size::new(1.0, 1.0),
        },
        "a",
    );
    let b = graph.add_node(
        Rect {
            origin: Point::new(2.0, 0.0),
            size: Size::new(1.0, 1.0),
        },
        "b",
    );
    graph.add_edge(a, b)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn unknown_node_display_says_it_does_not_belong_to_this_scene() {
    let message = Error::UnknownNode(a_node_id()).to_string();
    assert!(message.contains("does not belong to this Scene"), "{message}");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn unknown_edge_display_says_it_does_not_belong_to_this_scene() {
    let message = Error::UnknownEdge(an_edge_id()).to_string();
    assert!(message.contains("does not belong to this Scene"), "{message}");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn self_loop_unsupported_display_says_it_is_not_yet_supported() {
    let message = Error::SelfLoopUnsupported(a_node_id()).to_string();
    assert!(message.contains("not yet supported"), "{message}");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn svg_error_display_passes_through_the_wrapped_message() {
    let svg_err = svg_dom::Error::ElementNotFound("diagram".into());
    let expected = svg_err.to_string();
    let wrapped = Error::from(svg_err).to_string();
    assert_eq!(wrapped, expected);
}
