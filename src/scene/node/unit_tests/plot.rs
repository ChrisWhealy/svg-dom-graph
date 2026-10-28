//! Plot nodes: what is drawn, what is refused, what is plotted on one, and the clicks it reports.

use super::support::make_svg;
use crate::{
    error::Error,
    geometry::apply_matrix,
    scene::{
        AxisPosition, LabelAnchor, LegendEntry, MinorTicks, PlotAxis, PlotConfig, PlotConfigError, PlotItem, PlotLabel,
        PlotLine, PlotPlane, PlotPoints, Scene, TickLabels,
    },
    test_support::check,
};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Size};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn config() -> PlotConfig {
    PlotConfig::new(
        Size::new(400.0, 300.0),
        PlotAxis::linear(0.0, 1.0).with_major_interval(0.25),
        PlotAxis::linear(-2.0, 2.0).with_major_interval(1.0),
    )
}

/// Every element matching `selector` inside plot node `id`'s own group.
fn all(scene: &Scene, id: crate::NodeId, selector: &str) -> Vec<web_sys::Element> {
    let inner = scene.inner.borrow();
    let group = inner.node_handle(id).expect("a node").group.as_element().clone();
    let list = group.query_selector_all(selector).expect("a valid selector");
    (0..list.length())
        .filter_map(|i| list.item(i))
        .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
        .collect()
}

fn attribute(element: &web_sys::Element, name: &str) -> String {
    element.get_attribute(name).unwrap_or_default()
}

fn numbers(element: &web_sys::Element, name: &str) -> f64 {
    attribute(element, name).parse().unwrap_or(f64::NAN)
}

/// The scene's own content layer's children, counted, to see that a refused call left nothing behind.
fn content_children(scene: &Scene) -> u32 {
    scene.inner.borrow().content.as_element().child_element_count()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_plot_node_is_as_big_as_it_was_told_and_is_an_image_with_a_description() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-basic")).map_err(|e| e.to_string())?;
    let id = scene
        .add_plot_node(Point::new(30.0, 40.0), config().with_description("Three waves"))
        .map_err(|e| e.to_string())?;
    let rect = scene.node_rect(id).map_err(|e| e.to_string())?;
    check(rect.origin == Point::new(30.0, 40.0), "the origin")?;
    check(rect.size == Size::new(400.0, 300.0), "the size")?;
    let inner = scene.inner.borrow();
    let group = inner.node_handle(id).ok_or("no handles")?.group.as_element().clone();
    check(attribute(&group, "role") == "img", "the role")?;
    check(attribute(&group, "aria-label") == "Three waves", "the accessible name")?;
    check(attribute(&group, "transform") == "translate(30, 40)", "the position")
}

#[wasm_bindgen_test]
fn a_plot_with_no_description_is_named_from_its_axes() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-default-name")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    let inner = scene.inner.borrow();
    let group = inner.node_handle(id).ok_or("no handles")?.group.as_element().clone();
    check(
        attribute(&group, "aria-label")
            == "Real plane plot: horizontal axis linear from 0 to 1, vertical axis linear from \u{2212}2 to 2",
        &attribute(&group, "aria-label"),
    )
}

#[wasm_bindgen_test]
fn the_background_is_the_pale_blue_by_default_and_the_grid_is_a_little_darker_than_it() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-colours")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    let rects = all(&scene, id, "rect");
    check(rects.len() == 2, "an outer box and the plotted area")?;
    check(attribute(&rects[0], "fill") == "#eef4ff", "the outer box is the pale blue")?;
    check(attribute(&rects[1], "fill") == "#eef4ff", "so is the plotted area")?;
    let strokes: Vec<String> = all(&scene, id, "line").iter().map(|l| attribute(l, "stroke")).collect();
    check(
        strokes.iter().any(|s| s == "#cdd2db"),
        &format!("a major grid line, darker: {strokes:?}"),
    )?;
    check(strokes.iter().any(|s| s == "#4a5568"), "an axis line")
}

#[wasm_bindgen_test]
fn another_background_and_grid_colours_are_used_as_given() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-own-colours")).map_err(|e| e.to_string())?;
    let grid = crate::scene::PlotGrid::default()
        .with_major_colour("#ff0000")
        .with_minor_colour("#00ff00");
    let id = scene
        .add_plot_node(
            Point::origin(),
            PlotConfig::new(
                Size::new(300.0, 200.0),
                PlotAxis::linear(0.0, 2.0)
                    .with_major_interval(1.0)
                    .with_minor(MinorTicks::Divisions(2)),
                PlotAxis::linear(0.0, 2.0).with_major_interval(1.0),
            )
            .with_background("#ffffff")
            .with_grid(grid),
        )
        .map_err(|e| e.to_string())?;
    check(attribute(&all(&scene, id, "rect")[1], "fill") == "#ffffff", "the background")?;
    let strokes: Vec<String> = all(&scene, id, "line").iter().map(|l| attribute(l, "stroke")).collect();
    check(strokes.contains(&"#ff0000".to_owned()), "a major line has the colour given")?;
    check(strokes.contains(&"#00ff00".to_owned()), "a minor line has the colour given")
}

#[wasm_bindgen_test]
fn grid_lines_can_be_turned_off() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-no-grid")).map_err(|e| e.to_string())?;
    let id = scene
        .add_plot_node(
            Point::origin(),
            config().with_grid(crate::scene::PlotGrid::default().with_major(false).with_minor(false)),
        )
        .map_err(|e| e.to_string())?;
    let strokes: Vec<String> = all(&scene, id, "line").iter().map(|l| attribute(l, "stroke")).collect();
    check(
        strokes.iter().all(|s| s != "#cdd2db" && s != "#dde3ed"),
        &format!("no grid line: {strokes:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn the_ticks_are_where_the_axes_say_and_their_labels_are_written() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-ticks")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    let labels: Vec<String> = all(&scene, id, "text").iter().filter_map(|t| t.text_content()).collect();
    for wanted in ["0", "0.25", "0.5", "0.75", "1", "\u{2212}2", "\u{2212}1", "1", "2"] {
        check(labels.iter().any(|l| l == wanted), &format!("{wanted:?} in {labels:?}"))?;
    }
    Ok(())
}

#[wasm_bindgen_test]
fn custom_tick_labels_are_written_in_place_of_the_automatic_ones() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-custom-ticks")).map_err(|e| e.to_string())?;
    let x = PlotAxis::linear(0.0, 1.0).with_tick_labels(TickLabels::Custom(vec![
        (0.0, "start".to_owned()),
        (0.5, "half".to_owned()),
        (1.0, "end".to_owned()),
    ]));
    let id = scene
        .add_plot_node(
            Point::origin(),
            PlotConfig::new(Size::new(300.0, 200.0), x, PlotAxis::linear(0.0, 1.0)),
        )
        .map_err(|e| e.to_string())?;
    let labels: Vec<String> = all(&scene, id, "text").iter().filter_map(|t| t.text_content()).collect();
    check(labels.iter().any(|l| l == "half"), &format!("{labels:?}"))?;
    check(!labels.iter().any(|l| l == "0.25"), "the automatic ticks are not there")
}

#[wasm_bindgen_test]
fn a_logarithmic_axis_has_a_grid_line_at_each_power_of_ten_and_lighter_ones_between() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-log")).map_err(|e| e.to_string())?;
    let id = scene
        .add_plot_node(
            Point::origin(),
            PlotConfig::new(
                Size::new(400.0, 200.0),
                PlotAxis::logarithmic(1.0, 1000.0),
                PlotAxis::linear(0.0, 1.0),
            ),
        )
        .map_err(|e| e.to_string())?;
    let vertical: Vec<web_sys::Element> = all(&scene, id, "line")
        .into_iter()
        .filter(|l| numbers(l, "x1") == numbers(l, "x2") && numbers(l, "y1") != numbers(l, "y2"))
        .collect();
    let major = vertical.iter().filter(|l| attribute(l, "stroke") == "#cdd2db").count();
    let minor = vertical.iter().filter(|l| attribute(l, "stroke") == "#dde3ed").count();
    check(major == 4, &format!("{major} major lines for 1, 10, 100 and 1000"))?;
    check(minor == 24, &format!("{minor} minor lines, 2 to 9 in three decades"))?;
    // The decades are evenly spaced.
    let mut xs: Vec<f64> = vertical
        .iter()
        .filter(|l| attribute(l, "stroke") == "#cdd2db")
        .map(|l| numbers(l, "x1"))
        .collect();
    xs.sort_by(f64::total_cmp);
    check(
        (xs[1] - xs[0] - (xs[2] - xs[1])).abs() < 1e-6,
        "equal steps between the decades",
    )
}

#[wasm_bindgen_test]
fn a_complex_plot_names_its_axes_and_writes_the_imaginary_ticks_with_an_i() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-complex")).map_err(|e| e.to_string())?;
    let id = scene
        .add_plot_node(
            Point::origin(),
            PlotConfig::new(
                Size::new(300.0, 300.0),
                PlotAxis::linear(-2.5, 2.5).with_major_interval(1.0).with_arrow(true),
                PlotAxis::linear(-2.5, 2.5).with_major_interval(1.0).with_arrow(true),
            )
            .with_plane(PlotPlane::Complex),
        )
        .map_err(|e| e.to_string())?;
    let labels: Vec<String> = all(&scene, id, "text").iter().filter_map(|t| t.text_content()).collect();
    for wanted in ["Re", "Im", "1i", "\u{2212}2i", "2"] {
        check(labels.iter().any(|l| l == wanted), &format!("{wanted:?} in {labels:?}"))?;
    }
    check(!labels.iter().any(|l| l == "0" || l == "0i"), "no label where the axes cross")?;
    check(all(&scene, id, "path").len() == 2, "an arrowhead on each axis")?;
    let name = {
        let inner = scene.inner.borrow();
        attribute(inner.node_handle(id).ok_or("no handles")?.group.as_element(), "aria-label")
    };
    check(name.starts_with("Complex plane plot: real axis"), &name)
}

#[wasm_bindgen_test]
fn the_axis_lines_are_where_their_positions_put_them() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-axis-position")).map_err(|e| e.to_string())?;
    let id = scene
        .add_plot_node(
            Point::origin(),
            PlotConfig::new(
                Size::new(300.0, 200.0),
                PlotAxis::linear(0.0, 1.0),
                PlotAxis::linear(-1.0, 1.0).with_position(AxisPosition::AtMaximum),
            ),
        )
        .map_err(|e| e.to_string())?;
    let area = scene.plot_area(id).map_err(|e| e.to_string())?;
    let axes: Vec<web_sys::Element> = all(&scene, id, "line")
        .into_iter()
        .filter(|l| attribute(l, "stroke") == "#4a5568" && attribute(l, "stroke-width") == "1.5")
        .collect();
    check(axes.len() == 2, "two axis lines")?;
    let vertical = axes
        .iter()
        .find(|l| numbers(l, "x1") == numbers(l, "x2"))
        .ok_or("no vertical axis")?;
    check(
        (numbers(vertical, "x1") - (area.origin.x + area.size.width)).abs() < 1e-6,
        "the vertical axis is at the right edge",
    )
}

#[wasm_bindgen_test]
fn the_legend_has_a_sample_and_a_name_for_each_entry() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-legend")).map_err(|e| e.to_string())?;
    let id = scene
        .add_plot_node(
            Point::origin(),
            config().with_legend(vec![
                LegendEntry::line("wave A", "#e5534b"),
                LegendEntry::line("wave B", "#2f9e44").with_dash("7 4"),
                LegendEntry::dot("samples", "#f5a623"),
            ]),
        )
        .map_err(|e| e.to_string())?;
    let labels: Vec<String> = all(&scene, id, "text").iter().filter_map(|t| t.text_content()).collect();
    for wanted in ["wave A", "wave B", "samples"] {
        check(labels.iter().any(|l| l == wanted), &format!("{wanted:?} in {labels:?}"))?;
    }
    check(all(&scene, id, "line[stroke-dasharray=\"7 4\"]").len() == 1, "a dashed sample")?;
    check(all(&scene, id, "circle").len() == 1, "a dot sample")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_configuration_that_cannot_be_drawn_is_refused_and_leaves_nothing_behind() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-refused")).map_err(|e| e.to_string())?;
    let before = content_children(&scene);
    let bad = PlotConfig::new(Size::new(300.0, 200.0), PlotAxis::linear(1.0, 1.0), PlotAxis::linear(0.0, 1.0));
    let outcome = scene.add_plot_node(Point::origin(), bad);
    check(
        matches!(outcome, Err(Error::InvalidPlotConfig(PlotConfigError::Range(_)))),
        "the range is refused",
    )?;
    check(content_children(&scene) == before, "nothing was drawn")?;
    let outcome = scene.add_plot_node(Point::new(f64::NAN, 0.0), config());
    check(
        matches!(outcome, Err(Error::InvalidNodeGeometry(_))),
        "a position that is not a number is refused",
    )?;
    let outcome = scene.add_plot_node(Point::origin(), config().with_background("blue"));
    check(
        matches!(outcome, Err(Error::InvalidPlotConfig(PlotConfigError::Background))),
        "a colour name is refused",
    )?;
    check(content_children(&scene) == before, "still nothing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn items_are_drawn_in_order_and_replaced_as_a_whole() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-items")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    let items: Vec<PlotItem> = vec![
        PlotLine::new([(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)], "#e5534b")
            .with_dash("4 2")
            .into(),
        PlotPoints::new([(0.5, 1.0), (0.25, 0.5)], "#f5a623").into(),
        PlotLabel::new((0.5, 1.0), "peak")
            .with_anchor(LabelAnchor::Middle)
            .with_offset(0.0, -10.0)
            .into(),
    ];
    scene.set_plot_items(id, &items).map_err(|e| e.to_string())?;
    check(all(&scene, id, "polyline").len() == 1, "one line")?;
    check(all(&scene, id, "circle").len() == 2, "two dots")?;
    check(
        all(&scene, id, "text")
            .iter()
            .any(|t| t.text_content().as_deref() == Some("peak")),
        "the label",
    )?;
    check(all(&scene, id, "polyline[stroke-dasharray=\"4 2\"]").len() == 1, "the dash")?;

    scene
        .set_plot_items(id, &[PlotPoints::new([(0.1, 0.1)], "#00f").into()])
        .map_err(|e| e.to_string())?;
    check(all(&scene, id, "polyline").is_empty(), "the old line is gone")?;
    check(all(&scene, id, "circle").len() == 1, "only the new dot")?;
    scene.set_plot_items(id, &[]).map_err(|e| e.to_string())?;
    check(all(&scene, id, "circle").is_empty(), "no items")
}

#[wasm_bindgen_test]
fn a_line_that_leaves_the_plot_is_cut_at_its_edge_and_a_dot_outside_is_not_drawn() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-clipped")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    let area = scene.plot_area(id).map_err(|e| e.to_string())?;
    scene
        .set_plot_items(
            id,
            &[
                PlotLine::new([(-1.0, 0.0), (2.0, 0.0)], "#000").into(),
                PlotPoints::new([(5.0, 5.0), (0.5, 0.0)], "#000").into(),
                PlotLabel::new((9.0, 9.0), "gone").into(),
            ],
        )
        .map_err(|e| e.to_string())?;
    let line = all(&scene, id, "polyline");
    check(line.len() == 1, "one run")?;
    let points = attribute(&line[0], "points");
    let xs: Vec<f64> = points
        .split_whitespace()
        .filter_map(|pair| pair.split(',').next()?.parse().ok())
        .collect();
    let origin_x = scene.node_rect(id).map_err(|e| e.to_string())?.origin.x;
    check(
        xs.iter()
            .all(|x| *x >= area.origin.x - origin_x - 1e-6 && *x <= area.origin.x - origin_x + area.size.width + 1e-6),
        &points,
    )?;
    check(all(&scene, id, "circle").len() == 1, "only the dot that is inside")?;
    check(
        !all(&scene, id, "text")
            .iter()
            .any(|t| t.text_content().as_deref() == Some("gone")),
        "no label outside",
    )
}

#[wasm_bindgen_test]
fn a_gap_in_the_data_is_a_gap_in_the_line() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-gap")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    scene
        .set_plot_items(
            id,
            &[PlotLine::new([(0.0, 0.0), (0.2, 0.5), (f64::NAN, 0.0), (0.6, 0.5), (0.8, 1.0)], "#000").into()],
        )
        .map_err(|e| e.to_string())?;
    check(all(&scene, id, "polyline").len() == 2, "two runs either side of the gap")
}

#[wasm_bindgen_test]
fn items_on_a_node_that_is_not_a_plot_are_refused() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-not-a-plot")).map_err(|e| e.to_string())?;
    let plain = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "plain")
        .map_err(|e| e.to_string())?;
    check(
        matches!(scene.set_plot_items(plain, &[]), Err(Error::NotAPlot(n)) if n == plain),
        "set_plot_items",
    )?;
    check(matches!(scene.plot_area(plain), Err(Error::NotAPlot(_))), "plot_area")?;
    check(
        matches!(scene.plot_point_to_scene(plain, 0.0, 0.0), Err(Error::NotAPlot(_))),
        "plot_point_to_scene",
    )?;
    check(
        matches!(scene.on_plot_click(plain, |_, _, _| {}), Err(Error::NotAPlot(_))),
        "on_plot_click",
    )?;
    check(
        matches!(scene.set_plot_description(plain, "x"), Err(Error::NotAPlot(_))),
        "set_plot_description",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn the_plotted_area_and_a_value_are_found_in_the_scenes_own_space_after_the_node_moves() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-geometry")).map_err(|e| e.to_string())?;
    let id = scene
        .add_plot_node(Point::new(10.0, 20.0), config())
        .map_err(|e| e.to_string())?;
    let area = scene.plot_area(id).map_err(|e| e.to_string())?;
    let at = scene
        .plot_point_to_scene(id, 0.0, -2.0)
        .map_err(|e| e.to_string())?
        .ok_or("not placed")?;
    check(
        (at.x - area.origin.x).abs() < 1e-9 && (at.y - (area.origin.y + area.size.height)).abs() < 1e-9,
        "the corner",
    )?;
    let top_right = scene
        .plot_point_to_scene(id, 1.0, 2.0)
        .map_err(|e| e.to_string())?
        .ok_or("not placed")?;
    check(
        (top_right.x - (area.origin.x + area.size.width)).abs() < 1e-9 && (top_right.y - area.origin.y).abs() < 1e-9,
        "the other corner",
    )?;
    scene.move_node(id, Point::new(110.0, 120.0)).map_err(|e| e.to_string())?;
    let moved = scene.plot_area(id).map_err(|e| e.to_string())?;
    check(
        (moved.origin.x - area.origin.x - 100.0).abs() < 1e-9,
        "the area moved with the node",
    )?;
    check(
        scene
            .plot_point_to_scene(id, f64::NAN, 0.0)
            .map_err(|e| e.to_string())?
            .is_none(),
        "a value that cannot be placed",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dispatches a click on plot node `id` at the point `user` of the scene's own space, as a browser would.
fn click_at(scene: &Scene, id: crate::NodeId, user: Point) -> Result<(), String> {
    let (element, client) = {
        let inner = scene.inner.borrow();
        let ctm = inner.content.screen_ctm().ok_or("no screen matrix")?;
        (
            inner.node_handle(id).ok_or("no handles")?.group.as_element().clone(),
            apply_matrix(ctm, user),
        )
    };
    let init = web_sys::MouseEventInit::new();
    init.set_bubbles(true);
    init.set_client_x(client.x.round() as i32);
    init.set_client_y(client.y.round() as i32);
    let event = web_sys::MouseEvent::new_with_mouse_event_init_dict("click", &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[wasm_bindgen_test]
fn a_click_inside_the_plot_reports_the_values_there_and_one_in_the_margin_reports_nothing() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-click")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::new(5.0, 5.0), config()).map_err(|e| e.to_string())?;
    let seen: Rc<RefCell<Vec<(f64, f64)>>> = Rc::default();
    let sink = seen.clone();
    scene
        .on_plot_click(id, move |_, _, click| sink.borrow_mut().push((click.x, click.y)))
        .map_err(|e| e.to_string())?;

    let wanted = scene
        .plot_point_to_scene(id, 0.75, 1.0)
        .map_err(|e| e.to_string())?
        .ok_or("not placed")?;
    click_at(&scene, id, wanted)?;
    let got = seen.borrow().clone();
    check(got.len() == 1, &format!("one click was reported: {got:?}"))?;
    check(
        (got[0].0 - 0.75).abs() < 0.01 && (got[0].1 - 1.0).abs() < 0.02,
        &format!("the values at the click: {got:?}"),
    )?;

    let margin = Point::new(7.0, 7.0);
    click_at(&scene, id, margin)?;
    check(seen.borrow().len() == 1, "a click in the margin was not reported")
}

#[wasm_bindgen_test]
fn a_click_is_found_through_the_zoom_of_the_scene() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-click-zoomed")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::new(5.0, 5.0), config()).map_err(|e| e.to_string())?;
    let seen: Rc<RefCell<Vec<(f64, f64)>>> = Rc::default();
    let sink = seen.clone();
    scene
        .on_plot_click(id, move |_, _, click| sink.borrow_mut().push((click.x, click.y)))
        .map_err(|e| e.to_string())?;
    scene
        .set_view(crate::scene::ViewTransform {
            scale: 2.0,
            tx: -30.0,
            ty: -10.0,
        })
        .map_err(|e| e.to_string())?;
    let wanted = scene
        .plot_point_to_scene(id, 0.25, -1.0)
        .map_err(|e| e.to_string())?
        .ok_or("not placed")?;
    click_at(&scene, id, wanted)?;
    let got = seen.borrow().clone();
    check(
        got.len() == 1 && (got[0].0 - 0.25).abs() < 0.01 && (got[0].1 + 1.0).abs() < 0.02,
        &format!("{got:?}"),
    )
}

#[wasm_bindgen_test]
fn a_click_on_a_logarithmic_axis_is_a_value_on_it() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-click-log")).map_err(|e| e.to_string())?;
    let id = scene
        .add_plot_node(
            Point::origin(),
            PlotConfig::new(
                Size::new(400.0, 200.0),
                PlotAxis::logarithmic(1.0, 1000.0),
                PlotAxis::linear(0.0, 1.0),
            ),
        )
        .map_err(|e| e.to_string())?;
    let seen: Rc<RefCell<Vec<f64>>> = Rc::default();
    let sink = seen.clone();
    scene
        .on_plot_click(id, move |_, _, click| sink.borrow_mut().push(click.x))
        .map_err(|e| e.to_string())?;
    let wanted = scene
        .plot_point_to_scene(id, 100.0, 0.5)
        .map_err(|e| e.to_string())?
        .ok_or("not placed")?;
    click_at(&scene, id, wanted)?;
    let got = seen.borrow().clone();
    check(got.len() == 1 && (got[0] / 100.0 - 1.0).abs() < 0.02, &format!("{got:?}"))
}

#[wasm_bindgen_test]
fn a_second_click_handler_is_refused() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-click-twice")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    scene.on_plot_click(id, |_, _, _| {}).map_err(|e| e.to_string())?;
    check(
        matches!(scene.on_plot_click(id, |_, _, _| {}), Err(Error::AlreadyHasClickHandler(n)) if n == id),
        "the second handler is refused",
    )?;
    let area = &all(&scene, id, "rect")[1];
    check(
        attribute(area, "style") == "cursor: crosshair",
        "the plotted area shows that it can be clicked",
    )
}

#[wasm_bindgen_test]
fn the_handler_may_draw_on_the_plot_it_was_called_from() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-click-draws")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    scene
        .on_plot_click(id, |scene, id, click| {
            let _ = scene.set_plot_items(id, &[PlotPoints::new([click.point()], "#f00").into()]);
        })
        .map_err(|e| e.to_string())?;
    let wanted = scene
        .plot_point_to_scene(id, 0.5, 0.0)
        .map_err(|e| e.to_string())?
        .ok_or("not placed")?;
    click_at(&scene, id, wanted)?;
    click_at(&scene, id, wanted)?;
    check(all(&scene, id, "circle").len() == 1, "each click replaced the dot")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_new_description_replaces_the_old_and_keeps_what_the_connectors_added() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-describe")).map_err(|e| e.to_string())?;
    let plot = scene
        .add_plot_node(Point::new(200.0, 0.0), config().with_description("First"))
        .map_err(|e| e.to_string())?;
    let source = scene
        .add_node(Point::origin(), Size::new(60.0, 30.0), "source")
        .map_err(|e| e.to_string())?;
    scene.add_edge(source, plot).map_err(|e| e.to_string())?;
    scene.set_plot_description(plot, "Second").map_err(|e| e.to_string())?;
    let name = {
        let inner = scene.inner.borrow();
        attribute(inner.node_handle(plot).ok_or("no handles")?.group.as_element(), "aria-label")
    };
    check(name.starts_with("Second."), &name)?;
    check(name.contains("Input from source"), &name)?;
    check(!name.contains("First"), "the old description is gone")?;
    check(
        matches!(
            scene.set_plot_description(plot, "  "),
            Err(Error::InvalidPlotConfig(PlotConfigError::EmptyDescription))
        ),
        "an empty description is refused",
    )
}

#[wasm_bindgen_test]
fn a_plot_takes_the_focus_ring_like_any_other_node() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-focus")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    scene.set_focus(id, true).map_err(|e| e.to_string())?;
    check(
        attribute(&all(&scene, id, "rect")[0], "stroke") == "#d00b57",
        "the outer box is ringed",
    )?;
    scene.set_focus(id, false).map_err(|e| e.to_string())?;
    check(attribute(&all(&scene, id, "rect")[0], "stroke") == "#2a5db0", "and unringed")
}

#[wasm_bindgen_test]
fn an_extremely_distant_point_never_puts_a_non_finite_number_in_the_drawing() -> Result<(), String> {
    let scene = Scene::new(make_svg("plot-extreme")).map_err(|e| e.to_string())?;
    let id = scene.add_plot_node(Point::origin(), config()).map_err(|e| e.to_string())?;
    scene
        .set_plot_items(
            id,
            &[
                PlotLine::new([(0.5, 0.0), (1e308, 1e308)], "#000").into(),
                PlotLine::new([(0.5, 0.0), (f64::MAX, 0.0)], "#000").into(),
                PlotPoints::new([(1e308, 0.0)], "#000").into(),
                PlotLabel::new((f64::MIN, 0.0), "far").into(),
            ],
        )
        .map_err(|e| e.to_string())?;
    let lines = all(&scene, id, "polyline");
    check(lines.len() == 2, "each line is cut at the edge of the plot and drawn")?;
    for line in lines {
        let points = attribute(&line, "points");
        check(
            !points.contains("NaN") && !points.contains("inf") && !points.contains("Infinity"),
            &points,
        )?;
    }
    check(all(&scene, id, "circle").is_empty(), "a dot that far outside is not drawn")
}
