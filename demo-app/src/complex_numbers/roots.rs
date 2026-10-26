//! "The roots of unity": the `n` complex numbers that give 1 when raised to the power `n`, drawn on the unit circle with a
//! node of them in rectangular form and another in polar form.
//!
//! A slider above the diagram sets `n` from 1 to [`MOST`]. The slider is an ordinary HTML control that is not in the
//! scene, so that it is not drawn again as the scene is. A node cannot be given more or fewer numbers than it was drawn
//! with, so a change of `n` draws the scene again from nothing, and carries the zoom and pan over to the new one.

use super::{
    FEATURE_SIDE, GAP, MARGIN, NODE_GAP, TOOLBAR_ROOM,
    argand::{Argand, Guides, Mark, draw},
    sliders::Listener,
};
use demo_common::util::{clear_error, document, ensure_svg_in, report_error, required_element, resize_svg, stringify};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    complex::Complex,
    scene::{
        ComplexNumberFormat, ComplexStyle, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Side,
        ToolbarOptions,
    },
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::HtmlInputElement;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The most roots the slider reaches. The number is arbitrary: it is as many as the diagram can label without the labels
/// running together.
pub(super) const MOST: usize = 20;

/// How many roots the demo starts with, an eight-point transform's.
pub(super) const START: usize = 8;

pub(super) const STAGE_ID: &str = "complex-roots-stage";
const DIAGRAM_ID: &str = "complex-roots-diagram";
const SLIDER_ID: &str = "complex-roots-count";
const READOUT_ID: &str = "complex-roots-readout";
const ERROR_ID: &str = "complex-error";

/// How many numbers of a node are in a row.
const COLUMNS: usize = 4;

/// How many lines of text high the pictures are.
const PICTURE_SCALE: f64 = 1.5;

/// The plane shows this far either way, and the axes run a little less.
const EXTENT: f64 = 1.45;
const AXIS_HALF: f64 = 1.3;

/// The size the stage is first made at, before the first scene is drawn and measured. Every scene after sets its own.
const FIRST_STAGE: (f64, f64) = (1400.0, 700.0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
thread_local! {
    // The scene as it is now, kept for as long as the page is.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
    static LISTENER: RefCell<Option<Listener>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `n` roots of unity, anticlockwise from 1.
pub(super) fn roots(n: usize) -> Result<Vec<Complex>, String> {
    Complex::unit_roots(n, n).map_err(stringify)
}

/// What is described to assistive technology for a diagram of the `n` roots of unity.
pub(super) fn describe_roots(n: usize) -> String {
    let roots = if n == 1 { "root" } else { "roots" };
    format!("Argand diagram of the {n} {roots} of unity of order {n}, evenly spaced round the unit circle")
}

/// The size the stage needs for a scene whose nodes reach as far as `right` and `bottom`: room for them, the margin, and
/// the zoom toolbar east of them. It is never shorter than the diagram, which is as tall as the nodes of a few numbers.
///
/// More numbers make a node taller, since it holds four in a row, so the stage grows with `n` and a node is never cut off.
pub(super) fn stage_size(right: f64, bottom: f64) -> Size {
    Size::new(right + MARGIN + TOOLBAR_ROOM, bottom.max(MARGIN + FEATURE_SIDE) + MARGIN)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `n` roots from nothing, in place of the one there was, with the zoom and pan it had.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the panel, or if any library call fails.
fn rebuild(n: usize) -> Result<(), String> {
    let document = document()?;
    let view = SCENE.with_borrow(|slot| slot.as_ref().map(Scene::view));
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let drawing = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?).map_err(stringify)?;
    let content_group = document
        .query_selector(&format!("#{DIAGRAM_ID} .svg-dom-graph-content"))
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| format!("#{DIAGRAM_ID} has no scene content group"))?;

    let numbers = roots(n)?;
    let diagram = Argand {
        top_left: Point::new(MARGIN, MARGIN),
        side: FEATURE_SIDE,
        extent: EXTENT,
        axis_half: AXIS_HALF,
        unit_circle: true,
        marks: numbers
            .iter()
            .enumerate()
            .map(|(k, &z)| Mark {
                z,
                label: format!("w{k}"),
                guides: Guides::None,
            })
            .collect(),
        description: describe_roots(n),
    };
    let group = draw(&drawing, &diagram)?;
    content_group.append_child(group.as_element()).map_err(|e| format!("{e:?}"))?;

    let nodes_x = MARGIN + FEATURE_SIDE + GAP;
    let (mut node_y, mut right) = (MARGIN, nodes_x);
    for (name, format) in [
        ("Rectangular", ComplexNumberFormat::Rectangular),
        ("Polar", ComplexNumberFormat::Polar),
    ] {
        let content = DataNodeContent::new(
            NodeValues::Complex64(numbers.clone()),
            DataFormat::ComplexNumber(format, ComplexStyle::default().with_icon_scale(PICTURE_SCALE)),
        )
        .with_layout(GridLayout::Columns(COLUMNS));
        let (_, size) = super::add_node(&scene, Point::new(nodes_x, node_y), name, content)?;
        right = right.max(nodes_x + size.width);
        node_y += size.height + NODE_GAP;
    }

    // The stage is as big as what was drawn in it, so a node of many numbers is not cut off.
    resize_svg(&document, DIAGRAM_ID, stage_size(right, node_y - NODE_GAP))?;

    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What happens when the slider moves: the readout shows `n`, and the scene is drawn for it.
fn moved(slider: &HtmlInputElement) {
    let n = slider.value_as_number().round().clamp(1.0, MOST as f64) as usize;
    if let Some(readout) = document().ok().and_then(|document| document.get_element_by_id(READOUT_ID)) {
        readout.set_text_content(Some(&n.to_string()));
    }
    let _ = slider.set_attribute("aria-valuetext", &format!("{n} roots"));
    match rebuild(n) {
        Ok(()) => clear_error(ERROR_ID),
        Err(message) => report_error(ERROR_ID, &message),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the roots of unity for the number the slider starts at, and wires the slider.
///
/// # Errors
///
/// Returns `Err` if the page is missing the slider, or if any library call fails.
pub(super) fn build() -> Result<(), String> {
    let document = document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(FIRST_STAGE.0, FIRST_STAGE.1))?;
    let slider = demo_common::util::required_input(&document, SLIDER_ID)?;
    slider.set_min(&1.to_string());
    slider.set_max(&MOST.to_string());
    slider.set_value(&START.to_string());
    if let Some(readout) = document.get_element_by_id(READOUT_ID) {
        readout.set_text_content(Some(&START.to_string()));
    }
    rebuild(START)?;

    let target = slider.clone();
    let listener: Listener = Closure::new(move |_event: web_sys::Event| moved(&target));
    slider
        .add_event_listener_with_callback("input", listener.as_ref().unchecked_ref())
        .map_err(|e| format!("could not listen to the slider: {e:?}"))?;
    LISTENER.with_borrow_mut(|slot| *slot = Some(listener));
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn the_slider_starts_at_eight_roots_and_reaches_at_least_that_many() {
        assert_eq!(START, 8);
        const { assert!(START <= MOST) };
    }

    #[test]
    fn there_are_n_roots_of_n_and_they_are_on_the_unit_circle() -> Result<(), String> {
        for n in 1..=MOST {
            let numbers = roots(n)?;
            assert_eq!(numbers.len(), n);
            for z in numbers {
                assert!((z.norm() - 1.0).abs() < 1e-12, "{z} is not on the unit circle");
            }
        }
        Ok(())
    }

    #[test]
    fn the_first_root_is_one_and_n_of_them_sum_to_zero() -> Result<(), String> {
        for n in 2..=MOST {
            let numbers = roots(n)?;
            assert_eq!(numbers[0], Complex::ONE);
            let total: Complex = numbers.into_iter().sum();
            assert!(total.norm() < 1e-12, "the {n} roots sum to {total}");
        }
        Ok(())
    }

    #[test]
    fn the_description_says_how_many_roots_and_of_what_order() {
        assert_eq!(
            describe_roots(1),
            "Argand diagram of the 1 root of unity of order 1, evenly spaced round the unit circle"
        );
        assert!(describe_roots(8).contains("8 roots of unity of order 8"));
    }

    #[test]
    fn the_stage_has_room_for_the_nodes_the_margin_and_the_toolbar() {
        let size = stage_size(900.0, 700.0);

        assert!((size.width - (900.0 + MARGIN + TOOLBAR_ROOM)).abs() < 1e-12);
        assert!((size.height - (700.0 + MARGIN)).abs() < 1e-12);
    }

    #[test]
    fn the_stage_grows_taller_with_the_nodes_and_is_never_shorter_than_the_diagram() {
        let diagram = FEATURE_SIDE + 2.0 * MARGIN;

        // Nodes that stop short of the diagram's foot leave the stage as tall as the diagram needs.
        assert!((stage_size(900.0, 100.0).height - diagram).abs() < 1e-12);
        // Taller nodes make it taller, by as much as they are.
        let (short, tall) = (stage_size(900.0, 600.0), stage_size(900.0, 900.0));
        assert!((tall.height - short.height - 300.0).abs() < 1e-12);
    }

    #[test]
    fn the_stage_grows_wider_with_the_nodes() {
        assert!(stage_size(1200.0, 500.0).width > stage_size(900.0, 500.0).width);
    }
}
