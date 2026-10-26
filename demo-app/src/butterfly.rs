//! `panel-butterfly` / `#butterfly-diagram`: a butterfly, the building block of a fast Fourier transform, drawn as one
//! unit. See [`build_butterfly_demo`]'s own doc comment.

use demo_common::util::{clear_error, document, ensure_svg_in, report_error, required_input, resize_svg, stringify};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    complex::Complex,
    scene::{
        Butterfly, ButterflyOperands, ButterflyResults, ComplexNumberFormat, ComplexStyle, DataFormat, DataNodeContent,
        NodeValues, Scene, Side, ToolbarOptions,
    },
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::HtmlInputElement;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("butterfly.rs");

const STAGE_ID: &str = "butterfly-stage";
const DIAGRAM_ID: &str = "butterfly-diagram";
const SLIDER_ID: &str = "butterfly-twiddle";
const READOUT_ID: &str = "butterfly-twiddle-readout";
const ERROR_ID: &str = "butterfly-error";

/// The size of the transform whose twiddle factors the slider steps through, so the slider reaches `0..N`.
pub(crate) const N: usize = 8;
/// The twiddle factor the slider starts at.
const START: usize = 1;

/// The two inputs. They are fixed, so the slider changes only the twiddle factor.
const A: Complex = Complex::new(2.0, 1.0);
const B: Complex = Complex::new(1.0, 2.0);

const MARGIN: f64 = 20.0;
const GAP: f64 = 70.0;
const NODE_GAP: f64 = 24.0;
/// Room east of the outputs for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;
/// How many lines of text high the pictures in the nodes are.
const PICTURE_SCALE: f64 = 1.5;

thread_local! {
    // The nodes, kept so that the slider can give them new values. Same reasoning as `tree::SCENE`.
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static LISTENER: RefCell<Option<Listener>> = const { RefCell::new(None) };
}

/// The slider's listener, kept alive for as long as the page is.
type Listener = Closure<dyn FnMut(web_sys::Event)>;

/// Everything the slider changes.
struct State {
    scene: Scene,
    twiddle: NodeId,
    butterfly: Butterfly,
    sum_out: NodeId,
    difference_out: NodeId,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The numbers of one butterfly: `product = w·b`, `sum = a + product` and `difference = a − product`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Values {
    twiddle: Complex,
    product: Complex,
    sum: Complex,
    difference: Complex,
}

/// The twiddle factor `W_N^k = e^(−2πik/N)` of an `N`-point transform.
///
/// # Errors
///
/// Returns `Err` if `N` is zero, which it never is.
fn twiddle_factor(k: usize) -> Result<Complex, String> {
    Ok(Complex::unit_root(k % N, N).map_err(stringify)?.conj())
}

/// The butterfly of [`A`] and [`B`] with the twiddle factor `W_N^k`.
fn values(k: usize) -> Result<Values, String> {
    let twiddle = twiddle_factor(k)?;
    let product = twiddle * B;
    let (sum, difference) = A.sum_and_difference(product);
    Ok(Values {
        twiddle,
        product,
        sum,
        difference,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How wide a number is written, in characters. A node is drawn for the widest number it will show, since a node never
/// changes size.
fn width_of(z: Complex) -> usize {
    format!("{:.3}", z.re).len() + format!("{:.3}", z.im).len()
}

/// The numbers that `pick` chooses from each butterfly of the slider's range, widest first.
fn widest(pick: impl Fn(&Values) -> Complex) -> Result<Complex, String> {
    let mut best = pick(&values(0)?);
    for k in 1..N {
        let z = pick(&values(k)?);
        if width_of(z) > width_of(best) {
            best = z;
        }
    }
    Ok(best)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn content(z: Complex) -> DataNodeContent {
    DataNodeContent::new(
        NodeValues::Complex64(vec![z]),
        DataFormat::ComplexNumber(
            ComplexNumberFormat::Rectangular,
            ComplexStyle::default().with_icon_scale(PICTURE_SCALE),
        ),
    )
}

fn only(z: Complex) -> NodeValues {
    NodeValues::Complex64(vec![z])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the butterfly demo: the inputs `a` and `b`, the twiddle factor `w`, a butterfly, and the two outputs.
///
/// - **The butterfly.** [`Scene::add_butterfly_node`] draws a multiplication, an addition and a subtraction in one
///   frame. Drag the frame, and the three operator nodes go with it.
/// - **The twiddle factor.** A slider above the diagram steps `k` through `0..8`, and `w` is `e^(−2πik/8)`, a twiddle
///   factor of an eight-point transform. The demo works out `w·b`, `a + w·b` and `a − w·b` in plain Rust, and gives each
///   node its new number with [`Scene::set_data_values`]. The library never does the arithmetic.
///
/// Every node is first drawn for the widest number it will show, and then given the starting number, because a node
/// never changes size.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if the page is missing the stage or the slider.
pub(crate) fn build_butterfly_demo() -> Result<(), String> {
    let document = document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 500.0))?;
    let scene = Scene::new(SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?).map_err(stringify)?;

    let start = values(START)?;
    let node = |scene: &Scene, at: Point, name: &str, z: Complex| {
        scene.add_named_data_node(at, name, content(z)).map_err(stringify)
    };

    // The inputs, one above the next, in a column.
    let mut y = MARGIN;
    let a = node(&scene, Point::new(MARGIN, y), "a", A)?;
    y += scene.node_rect(a).map_err(stringify)?.size.height + NODE_GAP;
    let b = node(&scene, Point::new(MARGIN, y), "b", B)?;
    y += scene.node_rect(b).map_err(stringify)?.size.height + NODE_GAP;
    let twiddle = node(&scene, Point::new(MARGIN, y), "w", widest(|v| v.twiddle)?)?;
    let inputs_right = [a, b, twiddle].iter().try_fold(0.0_f64, |right, &id| -> Result<f64, String> {
        let rect = scene.node_rect(id).map_err(stringify)?;
        Ok(right.max(rect.origin.x + rect.size.width))
    })?;
    let inputs_bottom = y + scene.node_rect(twiddle).map_err(stringify)?.size.height;

    let results = ButterflyResults::new(
        content(widest(|v| v.product)?),
        content(widest(|v| v.sum)?),
        content(widest(|v| v.difference)?),
    );
    let butterfly = scene
        .add_butterfly_node(
            Point::new(inputs_right + GAP, MARGIN),
            ButterflyOperands::new(a, b, twiddle),
            results,
        )
        .map_err(stringify)?;

    // The outputs, level with the nodes that produce them.
    let frame = scene.node_rect(butterfly.frame).map_err(stringify)?;
    let outputs_x = frame.origin.x + frame.size.width + GAP;
    let level = |id: NodeId| -> Result<f64, String> { Ok(scene.node_rect(id).map_err(stringify)?.origin.y) };
    let sum_out = node(
        &scene,
        Point::new(outputs_x, level(butterfly.sum)?),
        "a + w·b",
        widest(|v| v.sum)?,
    )?;
    let difference_out = node(
        &scene,
        Point::new(outputs_x, level(butterfly.difference)?),
        "a − w·b",
        widest(|v| v.difference)?,
    )?;
    scene.add_edge(butterfly.sum, sum_out).map_err(stringify)?;
    scene.add_edge(butterfly.difference, difference_out).map_err(stringify)?;

    let right = [sum_out, difference_out]
        .iter()
        .try_fold(0.0_f64, |right, &id| -> Result<f64, String> {
            let rect = scene.node_rect(id).map_err(stringify)?;
            Ok(right.max(rect.origin.x + rect.size.width))
        })?;
    let bottom = inputs_bottom.max(frame.origin.y + frame.size.height);
    resize_svg(&document, DIAGRAM_ID, Size::new(right + MARGIN + TOOLBAR_ROOM, bottom + MARGIN))?;
    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    let state = State {
        scene,
        twiddle,
        butterfly,
        sum_out,
        difference_out,
    };
    show(&state, &start)?;
    STATE.with_borrow_mut(|slot| *slot = Some(state));

    let slider = required_input(&document, SLIDER_ID)?;
    slider.set_min("0");
    slider.set_max(&(N - 1).to_string());
    slider.set_value(&START.to_string());
    moved(&slider);
    let target = slider.clone();
    let listener: Listener = Closure::new(move |_event: web_sys::Event| moved(&target));
    slider
        .add_event_listener_with_callback("input", listener.as_ref().unchecked_ref())
        .map_err(|e| format!("could not listen to the slider: {e:?}"))?;
    LISTENER.with_borrow_mut(|slot| *slot = Some(listener));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives every node that the slider changes its number from `v`.
fn show(state: &State, v: &Values) -> Result<(), String> {
    let set = |id: NodeId, z: Complex| state.scene.set_data_values(id, only(z)).map_err(stringify);
    set(state.twiddle, v.twiddle)?;
    set(state.butterfly.product, v.product)?;
    set(state.butterfly.sum, v.sum)?;
    set(state.butterfly.difference, v.difference)?;
    set(state.sum_out, v.sum)?;
    set(state.difference_out, v.difference)
}

/// What happens when the slider moves: the readout shows `k`, and the nodes show the butterfly for it.
fn moved(slider: &HtmlInputElement) {
    let k = slider.value_as_number().round().clamp(0.0, (N - 1) as f64) as usize;
    if let Some(readout) = document().ok().and_then(|document| document.get_element_by_id(READOUT_ID)) {
        readout.set_text_content(Some(&k.to_string()));
    }
    let _ = slider.set_attribute("aria-valuetext", &format!("k = {k}"));
    let result =
        values(k).and_then(|v| STATE.with_borrow(|state| state.as_ref().map_or(Ok(()), |state| show(state, &v))));
    match result {
        Ok(()) => clear_error(ERROR_ID),
        Err(message) => report_error(ERROR_ID, &message),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn near(a: Complex, b: Complex) -> bool {
        (a - b).norm() < 1e-12
    }

    #[test]
    fn the_twiddle_factors_are_the_conjugate_roots_of_unity() -> Result<(), String> {
        assert!(near(twiddle_factor(0)?, Complex::ONE));
        assert!(near(twiddle_factor(2)?, Complex::new(0.0, -1.0)));
        assert!(near(twiddle_factor(4)?, Complex::new(-1.0, 0.0)));
        assert!(near(twiddle_factor(6)?, Complex::new(0.0, 1.0)));
        for k in 0..N {
            assert!((twiddle_factor(k)?.norm() - 1.0).abs() < 1e-12);
        }
        Ok(())
    }

    #[test]
    fn the_twiddle_factor_wraps_after_n() -> Result<(), String> {
        assert_eq!(twiddle_factor(N + 3)?, twiddle_factor(3)?);
        Ok(())
    }

    #[test]
    fn a_butterfly_with_w_equal_one_adds_and_subtracts_b() -> Result<(), String> {
        let v = values(0)?;
        assert!(near(v.product, B));
        assert!(near(v.sum, A + B));
        assert!(near(v.difference, A - B));
        Ok(())
    }

    #[test]
    fn the_sum_and_difference_always_add_up_to_twice_a_and_differ_by_twice_the_product() -> Result<(), String> {
        for k in 0..N {
            let v = values(k)?;
            assert!(near(v.sum + v.difference, A.scale(2.0)), "k = {k}");
            assert!(near(v.sum - v.difference, v.product.scale(2.0)), "k = {k}");
            assert!(
                (v.product.norm() - B.norm()).abs() < 1e-12,
                "multiplying by W only turns b, k = {k}"
            );
        }
        Ok(())
    }

    #[test]
    fn the_widest_number_is_at_least_as_wide_as_every_number_the_slider_reaches() -> Result<(), String> {
        let picks: [fn(&Values) -> Complex; 4] = [|v| v.twiddle, |v| v.product, |v| v.sum, |v| v.difference];
        for pick in picks {
            let widest = widest(pick)?;
            for k in 0..N {
                assert!(width_of(pick(&values(k)?)) <= width_of(widest), "k = {k}");
            }
        }
        Ok(())
    }

    #[test]
    fn the_slider_starts_inside_its_range() {
        const { assert!(START < N) };
    }
}
