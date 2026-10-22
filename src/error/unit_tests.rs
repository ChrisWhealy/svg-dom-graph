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
/// Each variant's message says what went wrong, and names the value that was rejected where it has one. A failure names
/// the variant.
#[test]
fn each_variant_displays_what_went_wrong() -> Result<(), String> {
    let id = a_node_id();
    let nan_origin = Rect {
        origin: Point::new(f64::NAN, 0.0),
        size: Size::new(100.0, 100.0),
    };
    let negative_width = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(-1.0, 1.0),
    };
    let toolbar = crate::scene::ToolbarOptions::new(crate::scene::Side::West);
    let cell_9 = format!("{id:?}");
    for (error, fragments) in [
        (Error::UnknownNode(id), vec!["does not belong to this Scene"]),
        (Error::UnknownEdge(an_edge_id()), vec!["does not belong to this Scene"]),
        (Error::SelfLoopUnsupported(id), vec!["not yet supported"]),
        (Error::AlreadyDraggable(id), vec!["already draggable"]),
        (Error::InvalidCollisionPadding(f64::NAN), vec!["NaN"]),
        (Error::InvalidDragBounds(nan_origin), vec!["NaN"]),
        (Error::InvalidToolbarOptions(toolbar), vec!["West", "button_height"]),
        (Error::InvalidCornerRadius(f64::NAN), vec!["NaN"]),
        (Error::InvalidEdgeAnchors(0), vec!["0"]),
        (Error::InvalidNodeGeometry(negative_width), vec!["-1"]),
        (
            Error::InvalidSelection(id, crate::scene::Selection::Cell(9)),
            vec![cell_9.as_str(), "Cell(9)"],
        ),
    ] {
        let message = error.to_string();
        for fragment in fragments {
            check(
                message.contains(fragment),
                &format!("{error:?} displays {message:?}, which lacks {fragment:?}"),
            )?;
        }
    }
    Ok(())
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
