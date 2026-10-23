//! `Scene::add_ellipse_node`: a real `<ellipse>` with a centred label, grown (never shrunk) to hold the label inside the
//! curved outline, its place in the model, and what connectors, focus, dragging and the other node methods do with it.
//!
//! The geometry itself, and the routing rules, are checked without a browser, in `src/geometry/ellipse` and
//! `src/scene/connector`. These tests are about what reaches the page.

use crate::common::{
    check, check_close, first_point_of_path, last_point_of_path, make_svg, nth_group, path_d, the_connector,
};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    EdgeEnd, EndpointProblem, Error, NodeId,
    scene::{
        ClockPosition, ConnectorOptions, ConnectorType, EdgeAnchors, EllipseOptions, EllipseRadii, NodeValues, Scene,
        Side,
    },
};
use wasm_bindgen_test::wasm_bindgen_test;

fn new_scene(id: &str) -> Result<Scene, String> {
    Scene::new(make_svg(id, Size::new(700.0, 500.0), Size::new(700.0, 500.0))).map_err(|e| e.to_string())
}

fn e(error: Error) -> String {
    error.to_string()
}

fn minute(m: u8) -> ClockPosition {
    ClockPosition::try_from(m).expect("a minute from 0 to 59")
}

fn on_outline(point: (f64, f64), centre: Point, rx: f64, ry: f64) -> bool {
    let value = ((point.0 - centre.x) / rx).powi(2) + ((point.1 - centre.y) / ry).powi(2);
    (value - 1.0).abs() < 1e-6
}

fn child(group: &web_sys::Element, tag: &str) -> Result<web_sys::Element, String> {
    group
        .query_selector(&format!(":scope > {tag}"))
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| format!("no {tag} in the group"))
}

fn number(element: &web_sys::Element, name: &str) -> Result<f64, String> {
    element
        .get_attribute(name)
        .ok_or_else(|| format!("no {name}"))?
        .parse::<f64>()
        .map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn an_ellipse_is_drawn_as_a_real_ellipse_with_its_label_in_a_group() -> Result<(), String> {
    let scene = new_scene("ellipse-draw")?;
    scene
        .add_ellipse_node(Point::new(200.0, 150.0), EllipseRadii::new(100.0, 60.0), "Start")
        .map_err(e)?;
    let group = nth_group("ellipse-draw", 0)?;
    let ellipse = child(&group, "ellipse")?;
    check(
        child(&group, "text")?.text_content().as_deref() == Some("Start"),
        "the label is the text",
    )?;
    check(
        group.query_selector(":scope > rect").map_err(|e| format!("{e:?}"))?.is_none(),
        "no rectangle outline",
    )?;
    // The ellipse is as large as asked for, since "Start" fits easily, with its own centre at (rx, ry) in the group.
    check_close(number(&ellipse, "rx")?, 100.0)?;
    check_close(number(&ellipse, "ry")?, 60.0)?;
    check_close(number(&ellipse, "cx")?, 100.0)?;
    check_close(number(&ellipse, "cy")?, 60.0)
}

#[wasm_bindgen_test]
fn node_rect_is_the_rectangle_that_encloses_the_ellipse_and_the_centre_is_where_it_was_put() -> Result<(), String> {
    let scene = new_scene("ellipse-rect")?;
    let id = scene
        .add_ellipse_node(Point::new(200.0, 150.0), EllipseRadii::new(100.0, 60.0), "Start")
        .map_err(e)?;
    let rect = scene.node_rect(id).map_err(e)?;
    check(
        rect == Rect {
            origin: Point::new(100.0, 90.0),
            size: Size::new(200.0, 120.0),
        },
        &format!("{rect:?}"),
    )
}

#[wasm_bindgen_test]
fn a_label_that_does_not_fit_makes_the_ellipse_larger_about_the_same_centre_and_keeps_the_ratio() -> Result<(), String>
{
    let scene = new_scene("ellipse-grow")?;
    let radii = EllipseRadii::new(40.0, 20.0);
    let id = scene
        .add_ellipse_node(Point::new(300.0, 200.0), radii, "A rather long operation label")
        .map_err(e)?;
    let rect = scene.node_rect(id).map_err(e)?;
    let (rx, ry) = (rect.size.width / 2.0, rect.size.height / 2.0);
    check(rx > 40.0 && ry > 20.0, &format!("it did not grow: {rx} {ry}"))?;
    check_close(rx / ry, 2.0)?;
    check_close(rect.origin.x + rx, 300.0)?;
    check_close(rect.origin.y + ry, 200.0)?;

    // The text, with the margin all round, is inside the curved outline at every corner, as the browser measures it.
    let group = nth_group("ellipse-grow", 0)?;
    let text = child(&group, "text")?;
    let measured = text.get_bounding_client_rect();
    let ellipse = child(&group, "ellipse")?.get_bounding_client_rect();
    let (hx, hy) = (measured.width() / 2.0 + 6.0, measured.height() / 2.0 + 6.0);
    let (erx, ery) = (ellipse.width() / 2.0, ellipse.height() / 2.0);
    check(
        (hx / erx).powi(2) + (hy / ery).powi(2) <= 1.02,
        "the label and its margin cross the outline",
    )
}

#[wasm_bindgen_test]
fn a_circle_stays_a_circle_when_the_label_makes_it_grow() -> Result<(), String> {
    let scene = new_scene("ellipse-circle")?;
    let id = scene
        .add_ellipse_node(Point::new(200.0, 200.0), EllipseRadii::circle(20.0), "Distance")
        .map_err(e)?;
    let rect = scene.node_rect(id).map_err(e)?;
    check(rect.size.width > 40.0, "it did not grow")?;
    check_close(rect.size.width, rect.size.height)
}

#[wasm_bindgen_test]
fn a_single_letter_fits_a_circle_of_radius_22_without_growing() -> Result<(), String> {
    let scene = new_scene("ellipse-letter")?;
    let id = scene
        .add_ellipse_node(Point::new(100.0, 100.0), EllipseRadii::circle(22.0), "A")
        .map_err(e)?;
    check_close(scene.node_rect(id).map_err(e)?.size.width, 44.0)
}

#[wasm_bindgen_test]
fn the_label_is_centred_on_the_ellipse() -> Result<(), String> {
    let scene = new_scene("ellipse-centred")?;
    scene
        .add_ellipse_node(Point::new(300.0, 200.0), EllipseRadii::new(90.0, 50.0), "Centred")
        .map_err(e)?;
    let group = nth_group("ellipse-centred", 0)?;
    let ellipse = child(&group, "ellipse")?.get_bounding_client_rect();
    let text = child(&group, "text")?.get_bounding_client_rect();
    let (ex, ey) = (ellipse.x() + ellipse.width() / 2.0, ellipse.y() + ellipse.height() / 2.0);
    let (tx, ty) = (text.x() + text.width() / 2.0, text.y() + text.height() / 2.0);
    check(
        (ex - tx).abs() < 1.0 && (ey - ty).abs() < 1.0,
        &format!("({ex}, {ey}) and ({tx}, {ty})"),
    )
}

#[wasm_bindgen_test]
fn the_inner_margin_is_configurable_and_a_larger_one_makes_a_larger_ellipse() -> Result<(), String> {
    let scene = new_scene("ellipse-margin")?;
    let radii = EllipseRadii::circle(20.0);
    let small = scene
        .add_ellipse_node_with(
            Point::new(100.0, 100.0),
            radii,
            "Label",
            EllipseOptions::default().with_inner_margin(0.0),
        )
        .map_err(e)?;
    let large = scene
        .add_ellipse_node_with(
            Point::new(300.0, 100.0),
            radii,
            "Label",
            EllipseOptions::default().with_inner_margin(20.0),
        )
        .map_err(e)?;
    check(
        scene.node_rect(large).map_err(e)?.size.width > scene.node_rect(small).map_err(e)?.size.width,
        "no larger",
    )
}

#[wasm_bindgen_test]
fn the_fill_option_sets_the_fill_of_the_ellipse() -> Result<(), String> {
    let scene = new_scene("ellipse-fill")?;
    scene
        .add_ellipse_node_with(
            Point::new(100.0, 100.0),
            EllipseRadii::circle(30.0),
            "A",
            EllipseOptions::default().with_fill(Some("#d9c9f3")),
        )
        .map_err(e)?;
    check(
        child(&nth_group("ellipse-fill", 0)?, "ellipse")?
            .get_attribute("fill")
            .as_deref()
            == Some("#d9c9f3"),
        "the fill was not applied",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn group_count(id: &str) -> Result<u32, String> {
    let list = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("a document")?
        .query_selector_all(&format!("#{id} > g.svg-dom-graph-content > g"))
        .map_err(|e| format!("{e:?}"))?;
    Ok(list.length())
}

#[wasm_bindgen_test]
fn invalid_geometry_and_margins_are_refused_and_draw_nothing() -> Result<(), String> {
    let scene = new_scene("ellipse-invalid")?;
    let centre = Point::new(100.0, 100.0);
    for radii in [
        EllipseRadii::new(0.0, 10.0),
        EllipseRadii::new(-5.0, 10.0),
        EllipseRadii::new(10.0, f64::NAN),
        EllipseRadii::circle(f64::INFINITY),
    ] {
        check(
            matches!(
                scene.add_ellipse_node(centre, radii, "x"),
                Err(Error::InvalidEllipseGeometry { .. })
            ),
            &format!("{radii:?} was not refused"),
        )?;
    }
    check(
        matches!(
            scene.add_ellipse_node(Point::new(f64::NAN, 0.0), EllipseRadii::circle(10.0), "x"),
            Err(Error::InvalidEllipseGeometry { .. })
        ),
        "a centre that is not finite was not refused",
    )?;
    for margin in [-1.0, f64::NAN, f64::INFINITY] {
        check(
            matches!(
                scene.add_ellipse_node_with(
                    centre,
                    EllipseRadii::circle(10.0),
                    "x",
                    EllipseOptions::default().with_inner_margin(margin)
                ),
                Err(Error::InvalidEllipseMargin(_))
            ),
            &format!("margin {margin} was not refused"),
        )?;
    }
    check(group_count("ellipse-invalid")? == 0, "a refused call left an element behind")
}

#[wasm_bindgen_test]
fn an_enormous_label_that_overflows_the_radii_is_refused_and_draws_nothing() -> Result<(), String> {
    let scene = new_scene("ellipse-overflow")?;
    let result = scene.add_ellipse_node(Point::new(0.0, 0.0), EllipseRadii::circle(f64::MAX), "x");
    check(
        matches!(result, Err(Error::InvalidEllipseGeometry { .. })),
        "radii that overflow were accepted",
    )?;
    check(group_count("ellipse-overflow")? == 0, "a refused call left an element behind")
}

#[wasm_bindgen_test]
fn an_empty_label_gives_a_valid_ellipse() -> Result<(), String> {
    let scene = new_scene("ellipse-empty")?;
    let id = scene
        .add_ellipse_node(Point::new(100.0, 100.0), EllipseRadii::circle(10.0), "")
        .map_err(e)?;
    let rect = scene.node_rect(id).map_err(e)?;
    check(rect.size.width.is_finite() && rect.size.width >= 20.0, &format!("{rect:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn two_ellipses(id: &str) -> Result<(Scene, NodeId, NodeId), String> {
    let scene = new_scene(id)?;
    let a = scene
        .add_ellipse_node(Point::new(100.0, 100.0), EllipseRadii::new(60.0, 30.0), "A")
        .map_err(e)?;
    let b = scene
        .add_ellipse_node(Point::new(400.0, 300.0), EllipseRadii::circle(40.0), "B")
        .map_err(e)?;
    Ok((scene, a, b))
}

#[wasm_bindgen_test]
fn a_straight_edge_between_two_ellipses_lands_on_both_outlines() -> Result<(), String> {
    let (scene, a, b) = two_ellipses("ellipse-straight")?;
    scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(e)?;
    let d = path_d(&the_connector("ellipse-straight")?)?;
    check(
        on_outline(first_point_of_path(&d)?, Point::new(100.0, 100.0), 60.0, 30.0),
        &format!("start of {d}"),
    )?;
    check(
        on_outline(last_point_of_path(&d)?, Point::new(400.0, 300.0), 40.0, 40.0),
        &format!("end of {d}"),
    )
}

#[wasm_bindgen_test]
fn an_elbow_leaves_an_ellipse_by_a_cardinal_point() -> Result<(), String> {
    let (scene, a, b) = two_ellipses("ellipse-elbow")?;
    scene.add_edge(a, b).map_err(e)?;
    let d = path_d(&the_connector("ellipse-elbow")?)?;
    // The target is further east than south of the first ellipse, so it leaves by its east point (160, 100).
    let (x, y) = first_point_of_path(&d)?;
    check((x - 160.0).abs() < 1e-6 && (y - 100.0).abs() < 1e-6, &d)
}

#[wasm_bindgen_test]
fn an_explicit_clock_position_puts_a_straight_end_on_that_minute_and_it_stays_there_after_a_move() -> Result<(), String>
{
    let (scene, a, b) = two_ellipses("ellipse-clock")?;
    scene
        .add_edge_with(
            a,
            b,
            ConnectorOptions::default()
                .with_connector_type(ConnectorType::Straight)
                .with_from_clock(Some(minute(10))),
        )
        .map_err(e)?;
    // Minute 10 is 60 degrees clockwise from north. The ray from the centre of a 60 x 30 ellipse crosses it at
    // (t sin 60, -t cos 60) from the centre, where t = 1 / hypot(sin60 / 60, cos60 / 30).
    let t = 1.0 / ((60.0_f64.to_radians().sin() / 60.0).powi(2) + (60.0_f64.to_radians().cos() / 30.0).powi(2)).sqrt();
    let expected = |centre: Point| {
        (
            centre.x + t * 60.0_f64.to_radians().sin(),
            centre.y - t * 60.0_f64.to_radians().cos(),
        )
    };
    let start = first_point_of_path(&path_d(&the_connector("ellipse-clock")?)?)?;
    let want = expected(Point::new(100.0, 100.0));
    check(
        (start.0 - want.0).abs() < 1e-6 && (start.1 - want.1).abs() < 1e-6,
        &format!("{start:?} is not {want:?}"),
    )?;

    // The first ellipse is moved. The end is still at minute 10 of it, now from its new centre, and not somewhere else.
    scene.move_node(a, Point::new(100.0, 250.0)).map_err(e)?;
    let moved = first_point_of_path(&path_d(&the_connector("ellipse-clock")?)?)?;
    let want = expected(Point::new(160.0, 280.0));
    check(
        (moved.0 - want.0).abs() < 1e-6 && (moved.1 - want.1).abs() < 1e-6,
        &format!("{moved:?} is not {want:?}"),
    )
}

#[wasm_bindgen_test]
fn a_forced_side_on_an_ellipse_lands_on_its_cardinal_point_even_for_a_straight_edge() -> Result<(), String> {
    let (scene, a, b) = two_ellipses("ellipse-forced")?;
    scene
        .add_edge_with(
            a,
            b,
            ConnectorOptions::default()
                .with_connector_type(ConnectorType::Straight)
                .with_from_side(Some(Side::North)),
        )
        .map_err(e)?;
    let (x, y) = first_point_of_path(&path_d(&the_connector("ellipse-forced")?)?)?;
    check((x - 100.0).abs() < 1e-6 && (y - 70.0).abs() < 1e-6, &format!("({x}, {y})"))
}

#[wasm_bindgen_test]
fn an_edge_between_an_ellipse_and_a_rectangle_works_in_both_directions() -> Result<(), String> {
    let scene = new_scene("ellipse-mixed")?;
    let ellipse = scene
        .add_ellipse_node(Point::new(100.0, 100.0), EllipseRadii::circle(40.0), "E")
        .map_err(e)?;
    let rectangle = scene
        .add_node(Point::new(400.0, 250.0), Size::new(80.0, 40.0), "R")
        .map_err(e)?;
    for connector_type in [ConnectorType::Straight, ConnectorType::Elbow { corner_radius: 0.0 }] {
        scene
            .add_edge_with(
                ellipse,
                rectangle,
                ConnectorOptions::default().with_connector_type(connector_type),
            )
            .map_err(e)?;
        scene
            .add_edge_with(
                rectangle,
                ellipse,
                ConnectorOptions::default().with_connector_type(connector_type),
            )
            .map_err(e)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_clock_position_on_a_rectangle_is_refused_and_draws_no_edge() -> Result<(), String> {
    let scene = new_scene("ellipse-clock-rect")?;
    let rectangle = scene.add_node(Point::new(10.0, 10.0), Size::new(60.0, 30.0), "R").map_err(e)?;
    let ellipse = scene
        .add_ellipse_node(Point::new(300.0, 100.0), EllipseRadii::circle(30.0), "E")
        .map_err(e)?;
    let result = scene.add_edge_with(
        rectangle,
        ellipse,
        ConnectorOptions::default().with_from_clock(Some(minute(15))),
    );
    check(
        matches!(
            result,
            Err(Error::InvalidEllipseEndpoint {
                endpoint: EdgeEnd::From,
                problem: EndpointProblem::ClockOnRectangle
            })
        ),
        &format!("{result:?}"),
    )?;
    let result = scene.add_edge_with(ellipse, rectangle, ConnectorOptions::default().with_to_clock(Some(minute(15))));
    check(
        matches!(
            result,
            Err(Error::InvalidEllipseEndpoint {
                endpoint: EdgeEnd::To,
                problem: EndpointProblem::ClockOnRectangle
            })
        ),
        &format!("{result:?}"),
    )?;
    check(
        web_sys::window()
            .and_then(|w| w.document())
            .ok_or("a document")?
            .query_selector("#ellipse-clock-rect > g.svg-dom-graph-content > path")
            .map_err(|e| format!("{e:?}"))?
            .is_none(),
        "a refused edge was drawn",
    )
}

#[wasm_bindgen_test]
fn a_fraction_on_an_ellipse_and_a_clock_with_a_side_are_refused_by_name() -> Result<(), String> {
    let (scene, a, b) = two_ellipses("ellipse-combos")?;
    let fraction = scene.add_edge_with(
        a,
        b,
        ConnectorOptions::default()
            .with_from_side(Some(Side::East))
            .with_from_position(Some(0.5)),
    );
    check(
        matches!(
            fraction,
            Err(Error::InvalidEllipseEndpoint {
                endpoint: EdgeEnd::From,
                problem: EndpointProblem::FractionOnEllipse
            })
        ),
        &format!("{fraction:?}"),
    )?;
    let both = scene.add_edge_with(
        a,
        b,
        ConnectorOptions::default()
            .with_connector_type(ConnectorType::Straight)
            .with_to_clock(Some(minute(5)))
            .with_to_side(Some(Side::West)),
    );
    check(
        matches!(
            both,
            Err(Error::InvalidEllipseEndpoint {
                endpoint: EdgeEnd::To,
                problem: EndpointProblem::ClockWithSide
            })
        ),
        &format!("{both:?}"),
    )
}

#[wasm_bindgen_test]
fn an_elbow_with_an_off_cardinal_clock_is_refused_when_added_and_when_the_type_is_switched() -> Result<(), String> {
    let (scene, a, b) = two_ellipses("ellipse-elbow-clock")?;
    let added = scene.add_edge_with(a, b, ConnectorOptions::default().with_from_clock(Some(minute(10))));
    check(matches!(added, Err(Error::InvalidEllipseElbowClock(_))), &format!("{added:?}"))?;

    // A straight edge with minute 10 cannot become an elbow, and nothing about it changes when it is refused.
    let edge = scene
        .add_edge_with(
            a,
            b,
            ConnectorOptions::default()
                .with_connector_type(ConnectorType::Straight)
                .with_from_clock(Some(minute(10))),
        )
        .map_err(e)?;
    let before = path_d(&the_connector("ellipse-elbow-clock")?)?;
    let switched = scene.set_connector_type(edge, ConnectorType::Elbow { corner_radius: 0.0 });
    check(
        matches!(switched, Err(Error::InvalidEllipseElbowClock(_))),
        &format!("{switched:?}"),
    )?;
    check(
        path_d(&the_connector("ellipse-elbow-clock")?)? == before,
        "the path changed on a refused switch",
    )?;
    // A cardinal is fine for an elbow, and so is any minute for a straight edge that stays straight.
    scene.set_connector_type(edge, ConnectorType::Straight).map_err(e)
}

#[wasm_bindgen_test]
fn fixing_points_cannot_be_set_on_an_ellipse() -> Result<(), String> {
    let (scene, a, _b) = two_ellipses("ellipse-anchors")?;
    let result = scene.set_edge_anchors(a, Some(EdgeAnchors(3)));
    check(
        matches!(result, Err(Error::EdgeAnchorsUnsupportedForEllipse(id)) if id == a),
        &format!("{result:?}"),
    )
}

#[wasm_bindgen_test]
fn an_ellipse_is_not_a_data_node_so_its_values_cannot_be_replaced() -> Result<(), String> {
    let (scene, a, _b) = two_ellipses("ellipse-values")?;
    let result = scene.set_data_values(a, NodeValues::U8(vec![1, 2, 3]));
    check(matches!(result, Err(Error::IncompatibleNodeValues(_))), &format!("{result:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn focus_draws_a_ring_on_the_ellipse_outline_and_move_node_and_draggable_work() -> Result<(), String> {
    let (scene, a, _b) = two_ellipses("ellipse-focus")?;
    scene.set_focus(a, true).map_err(e)?;
    let ellipse = child(&nth_group("ellipse-focus", 0)?, "ellipse")?;
    check(ellipse.get_attribute("stroke-width").as_deref() == Some("3"), "no focus ring")?;
    scene.set_focus(a, false).map_err(e)?;
    check(
        ellipse.get_attribute("stroke-width").as_deref() == Some("1.5"),
        "the ring stayed",
    )?;

    scene.make_draggable(a).map_err(e)?;
    scene.move_node(a, Point::new(10.0, 20.0)).map_err(e)?;
    let rect = scene.node_rect(a).map_err(e)?;
    check(
        rect.origin == Point::new(10.0, 20.0) && rect.size == Size::new(120.0, 60.0),
        &format!("{rect:?}"),
    )
}

#[wasm_bindgen_test]
fn moving_an_ellipse_reroutes_its_connectors_to_the_outline_at_the_new_place() -> Result<(), String> {
    let (scene, a, b) = two_ellipses("ellipse-reroute")?;
    scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(e)?;
    scene.move_node(b, Point::new(300.0, 50.0)).map_err(e)?;
    let d = path_d(&the_connector("ellipse-reroute")?)?;
    // The circle's origin is (300, 50) and its radius 40, so its centre is (340, 90).
    check(on_outline(last_point_of_path(&d)?, Point::new(340.0, 90.0), 40.0, 40.0), &d)?;
    check(on_outline(first_point_of_path(&d)?, Point::new(100.0, 100.0), 60.0, 30.0), &d)
}

#[wasm_bindgen_test]
fn the_group_keeps_its_accessible_name_and_the_relationship_to_the_other_end() -> Result<(), String> {
    let (scene, a, b) = two_ellipses("ellipse-aria")?;
    scene.add_edge(a, b).map_err(e)?;
    let group = nth_group("ellipse-aria", 0)?;
    let label = group.get_attribute("aria-label").unwrap_or_default();
    check(label.contains("A") && label.contains("Output to B"), &label)?;
    let title = child(&group, "title")?.text_content().unwrap_or_default();
    check(title.contains("Output to B"), &title)
}

#[wasm_bindgen_test]
fn a_rectangle_edge_is_unchanged_when_ellipses_exist_elsewhere_in_the_scene() -> Result<(), String> {
    let scene = new_scene("ellipse-regress")?;
    let a = scene.add_node(Point::new(10.0, 10.0), Size::new(80.0, 40.0), "A").map_err(e)?;
    let b = scene
        .add_node(Point::new(300.0, 200.0), Size::new(80.0, 40.0), "B")
        .map_err(e)?;
    scene
        .add_ellipse_node(Point::new(500.0, 100.0), EllipseRadii::circle(30.0), "E")
        .map_err(e)?;
    scene.add_edge(a, b).map_err(e)?;
    // The ordinary elbow: the ray from A's centre (50, 30) to B's (340, 220) crosses A's south side first, so it leaves
    // by the middle of that side.
    let (x, y) = first_point_of_path(&path_d(&the_connector("ellipse-regress")?)?)?;
    check((x - 50.0).abs() < 1e-6 && (y - 50.0).abs() < 1e-6, &format!("({x}, {y})"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The points of a path of straight segments, `M x y L x y ...`, as the scene writes one with a corner radius of zero.
fn path_points(d: &str) -> Result<Vec<(f64, f64)>, String> {
    let numbers: Vec<f64> = d
        .split_whitespace()
        .filter(|token| *token != "M" && *token != "L")
        .map(|token| token.parse::<f64>().map_err(|e| format!("{token}: {e}")))
        .collect::<Result<_, _>>()?;
    Ok(numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect())
}

#[wasm_bindgen_test]
fn an_elbow_that_leaves_a_box_past_its_target_goes_out_then_turns_round_and_never_through_the_box() -> Result<(), String>
{
    let scene = new_scene("ellipse-hairpin")?;
    let clock = scene
        .add_ellipse_node(Point::new(400.0, 300.0), EllipseRadii::new(110.0, 55.0), "Clock")
        .map_err(e)?;
    // Above the ellipse, with its east side at x = 460, which is right of the ellipse's north point at x = 400.
    let rectangle = scene
        .add_node(Point::new(350.0, 40.0), Size::new(110.0, 40.0), "R")
        .map_err(e)?;
    scene
        .add_edge_with(
            rectangle,
            clock,
            ConnectorOptions::default()
                .with_from_side(Some(Side::East))
                .with_to_side(Some(Side::North)),
        )
        .map_err(e)?;
    let points = path_points(&path_d(&the_connector("ellipse-hairpin")?)?)?;
    check(points.len() == 5, &format!("{points:?}"))?;
    check(
        points[0] == (460.0, 60.0) && points[1] == (480.0, 60.0),
        &format!("it does not go out by east: {points:?}"),
    )?;
    check(
        points[2] == (480.0, 225.0) && points[3] == (400.0, 225.0),
        &format!("{points:?}"),
    )?;
    check(
        points[4] == (400.0, 245.0),
        &format!("it does not end at the north point: {points:?}"),
    )?;
    // None of the five segments passes through the box (350..460, 40..80).
    for pair in points.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let inside = if ay == by {
            ay > 40.0 && ay < 80.0 && ax.min(bx) < 460.0 && ax.max(bx) > 350.0
        } else {
            ax > 350.0 && ax < 460.0 && ay.min(by) < 80.0 && ay.max(by) > 40.0
        };
        check(!inside, &format!("a segment runs through the box: {pair:?}"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_circle_node_is_an_ellipse_node_with_equal_radii() -> Result<(), String> {
    let scene = new_scene("circle-same")?;
    let circle = scene.add_circle_node(Point::new(100.0, 100.0), 30.0, "Same").map_err(e)?;
    let ellipse = scene
        .add_ellipse_node(Point::new(300.0, 100.0), EllipseRadii::circle(30.0), "Same")
        .map_err(e)?;
    let (a, b) = (scene.node_rect(circle).map_err(e)?, scene.node_rect(ellipse).map_err(e)?);
    check(a.size == b.size, &format!("{:?} and {:?}", a.size, b.size))?;
    check_close(a.size.width, a.size.height)?;
    check_close(a.origin.x + a.size.width / 2.0, 100.0)?;
    let group = nth_group("circle-same", 0)?;
    let drawn = child(&group, "ellipse")?;
    check_close(number(&drawn, "rx")?, number(&drawn, "ry")?)
}

#[wasm_bindgen_test]
fn a_circle_node_grows_to_hold_its_label_and_stays_a_circle() -> Result<(), String> {
    let scene = new_scene("circle-grow")?;
    let id = scene.add_circle_node(Point::new(200.0, 200.0), 20.0, "Distance").map_err(e)?;
    let rect = scene.node_rect(id).map_err(e)?;
    check(rect.size.width > 40.0, "it did not grow")?;
    check_close(rect.size.width, rect.size.height)
}

#[wasm_bindgen_test]
fn a_circle_node_takes_the_ellipse_options() -> Result<(), String> {
    let scene = new_scene("circle-options")?;
    scene
        .add_circle_node_with(
            Point::new(100.0, 100.0),
            22.0,
            "A",
            EllipseOptions::default().with_fill(Some("#d9c9f3")),
        )
        .map_err(e)?;
    check(
        child(&nth_group("circle-options", 0)?, "ellipse")?
            .get_attribute("fill")
            .as_deref()
            == Some("#d9c9f3"),
        "the fill was not applied",
    )?;
    let refused = scene.add_circle_node_with(
        Point::new(300.0, 100.0),
        22.0,
        "A",
        EllipseOptions::default().with_inner_margin(-1.0),
    );
    check(matches!(refused, Err(Error::InvalidEllipseMargin(_))), &format!("{refused:?}"))
}

#[wasm_bindgen_test]
fn an_invalid_circle_radius_or_centre_is_refused_with_the_radius_in_both_places_and_draws_nothing() -> Result<(), String>
{
    let scene = new_scene("circle-invalid")?;
    for radius in [0.0, -3.0, f64::NAN, f64::INFINITY] {
        let result = scene.add_circle_node(Point::new(100.0, 100.0), radius, "x");
        check(
            matches!(result, Err(Error::InvalidEllipseGeometry { rx, ry, .. }) if (rx.is_nan() && ry.is_nan()) || (rx == radius && ry == radius)),
            &format!("radius {radius}: {result:?}"),
        )?;
    }
    let result = scene.add_circle_node(Point::new(f64::NAN, 0.0), 10.0, "x");
    check(
        matches!(result, Err(Error::InvalidEllipseGeometry { .. })),
        &format!("{result:?}"),
    )?;
    check(group_count("circle-invalid")? == 0, "a refused call left an element behind")
}

#[wasm_bindgen_test]
fn a_circle_node_joins_connectors_like_any_other_node() -> Result<(), String> {
    let scene = new_scene("circle-edge")?;
    let a = scene.add_circle_node(Point::new(100.0, 100.0), 30.0, "A").map_err(e)?;
    let b = scene.add_circle_node(Point::new(400.0, 100.0), 30.0, "B").map_err(e)?;
    scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(e)?;
    let d = path_d(&the_connector("circle-edge")?)?;
    check(on_outline(first_point_of_path(&d)?, Point::new(100.0, 100.0), 30.0, 30.0), &d)?;
    check(on_outline(last_point_of_path(&d)?, Point::new(400.0, 100.0), 30.0, 30.0), &d)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_detour_round_a_tall_ellipse_stays_outside_the_painted_curve() -> Result<(), String> {
    let scene = new_scene("ellipse-round")?;
    // A tall ellipse, centred at (300, 260), and a rectangle behind it and below its centre, as the routing tests have
    // them. The connector leaves the ellipse by its east point and enters the rectangle from the north.
    let ellipse = scene
        .add_ellipse_node(Point::new(300.0, 260.0), EllipseRadii::new(20.0, 100.0), "")
        .map_err(e)?;
    let rectangle = scene
        .add_node(Point::new(160.0, 260.0), Size::new(80.0, 40.0), "R")
        .map_err(e)?;
    scene
        .add_edge_with(
            ellipse,
            rectangle,
            ConnectorOptions::default()
                .with_from_side(Some(Side::East))
                .with_to_side(Some(Side::North)),
        )
        .map_err(e)?;
    let rect = scene.node_rect(ellipse).map_err(e)?;
    let (cx, cy) = (rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0);
    let (rx, ry) = (rect.size.width / 2.0, rect.size.height / 2.0);
    let points = path_points(&path_d(&the_connector("ellipse-round")?)?)?;
    check(points.len() == 6, &format!("{points:?}"))?;
    // Every point along every leg is outside the ellipse, except the first point, which is on it.
    for pair in points.windows(2) {
        for step in 1..=400 {
            let t = f64::from(step) / 400.0;
            let (x, y) = (pair[0].0 + (pair[1].0 - pair[0].0) * t, pair[0].1 + (pair[1].1 - pair[0].1) * t);
            let inside = ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) < 1.0 - 1e-9;
            check(!inside, &format!("({x}, {y}) of {pair:?} is inside the painted ellipse"))?;
        }
    }
    // It goes up and over the top of the ellipse, which is at y = 160, and not across its middle.
    check(
        points.iter().any(|&(_, y)| y < 160.0),
        &format!("it does not go over the top: {points:?}"),
    )
}
