use super::*;
use crate::{model::graph::Graph, test_support::check};
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
fn unknown_node_display_says_it_does_not_belong_to_this_scene() -> Result<(), String> {
    let message = Error::UnknownNode(a_node_id()).to_string();
    check(message.contains("does not belong to this Scene"), &message)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn unknown_edge_display_says_it_does_not_belong_to_this_scene() -> Result<(), String> {
    let message = Error::UnknownEdge(an_edge_id()).to_string();
    check(message.contains("does not belong to this Scene"), &message)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn self_loop_unsupported_display_says_it_is_not_yet_supported() -> Result<(), String> {
    let message = Error::SelfLoopUnsupported(a_node_id()).to_string();
    check(message.contains("not yet supported"), &message)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn already_draggable_display_says_the_node_is_already_draggable() -> Result<(), String> {
    let message = Error::AlreadyDraggable(a_node_id()).to_string();
    check(message.contains("already draggable"), &message)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn invalid_collision_padding_display_names_the_rejected_value() -> Result<(), String> {
    let message = Error::InvalidCollisionPadding(f64::NAN).to_string();
    check(message.contains("NaN"), &message)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn invalid_corner_radius_display_names_the_rejected_value() -> Result<(), String> {
    let message = Error::InvalidCornerRadius(f64::NAN).to_string();
    check(message.contains("NaN"), &message)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn invalid_node_geometry_display_names_the_rejected_rect() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(-1.0, 1.0),
    };
    let message = Error::InvalidNodeGeometry(rect).to_string();
    check(message.contains("-1"), &message)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn svg_error_display_passes_through_the_wrapped_message() -> Result<(), String> {
    let svg_err = svg_dom::Error::ElementNotFound("diagram".into());
    let expected = svg_err.to_string();
    let wrapped = Error::from(svg_err).to_string();
    check(wrapped == expected, &format!("expected {expected:?}, got {wrapped:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn svg_error_source_returns_the_wrapped_error() -> Result<(), String> {
    use std::error::Error as _;
    let svg_err = svg_dom::Error::ElementNotFound("diagram".into());
    let expected = svg_err.to_string();
    let wrapped = Error::from(svg_err);
    let source = wrapped
        .source()
        .ok_or("Error::Svg should expose its wrapped error via source()")?;
    let got = source.to_string();
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_variant_that_does_not_wrap_another_error_has_no_source() -> Result<(), String> {
    use std::error::Error as _;
    check(
        Error::UnknownNode(a_node_id()).source().is_none(),
        "UnknownNode should not expose a source",
    )
}
