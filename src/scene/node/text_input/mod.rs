//! Text input nodes: a labelled box holding a one-line text box the user types into.
//!
//! The text box is an HTML `<input>` inside an SVG `<foreignObject>`, so that the browser does what it already does
//! well: the caret, selection, the clipboard, an input method, and the accessibility tree. Nothing about editing is
//! redone here. The `<foreignObject>` is a child of the node's own `<g>`, so the node moves, zooms and pans with the
//! rest of the scene, and a connector can be anchored to its outer box like any other node's.
//!
//! The node is not a data node, and its text is not a [`DataNodeContent`](crate::scene::DataNodeContent). It lives in
//! the `<input>` and is read back from it. The graph model holds only the label.
//!
//! # Events
//!
//! The listener is attached to the node's `<g>`, and reacts to the `input` event that bubbles up from the text box. It
//! belongs to that handle, so it goes when the node does. It holds only a `Weak` reference to the scene, for the same
//! reason every other listener this crate installs does.
//!
//! Typing in the text box does not pan or zoom the scene. The scene's keyboard handling is on a separate element that no
//! other element is a descendant of, and its pointer handling is on the background surface behind the nodes.

use super::{
    EdgeAnchors, LABEL_FONT_SIZE, LABEL_ROW_HEIGHT, NodeOptions, OUTER_PADDING, data::validate_node_name,
    plain::shrink_label_to_fit, render_guard::RenderGuard, validate_edge_anchors,
};
use crate::{
    colours::{BOX_STROKE, NAMED_BOX_FILL, TEXT_FILL},
    error::Error,
    model::node::{NodeContent, NodeId},
    scene::{BoxHandles, DataDecor, Scene, Selection},
};
use std::{cell::Cell, rc::Rc};
use svg_dom::{
    DominantBaseline, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// The height of the text box itself.
const INPUT_HEIGHT: f64 = 30.0;

/// The narrowest a node can be: the text box has [`OUTER_PADDING`] either side of it, and needs room for a few
/// characters.
const MIN_WIDTH: f64 = 2.0 * OUTER_PADDING + 60.0;

/// The XML namespace HTML elements are created in.
const NS_XHTML: &str = "http://www.w3.org/1999/xhtml";

/// How the text box looks. It fills its `<foreignObject>`, and uses a monospace font like the rest of the scene's text.
const INPUT_STYLE: &str = "box-sizing: border-box; width: 100%; height: 100%; margin: 0; padding: 0 6px; \
                           font: 13px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; \
                           user-select: text; -webkit-user-select: text;";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text box inside the node `group`, if it has one.
fn find_input(group: &svg_dom::SvgNode) -> Option<HtmlInputElement> {
    group
        .as_element()
        .query_selector("input")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a text input node, and attaches `listener` to its `input` event.
///
/// The outer box has the label in a row above the text box, as a named data node does. A [`RenderGuard`] removes
/// whatever was drawn if any step fails. `listener` is not installed until everything it needs has been drawn.
#[allow(clippy::too_many_arguments)]
fn draw_text_input_box(
    svg: &SvgRoot,
    scratch: &mut String,
    top_left: Point,
    width: f64,
    label: &str,
    value: &str,
    edge_anchors: Option<EdgeAnchors>,
    listener: impl FnMut(web_sys::Event) + 'static,
) -> Result<(BoxHandles, Rect), Error> {
    let size = Size::new(width, LABEL_ROW_HEIGHT + INPUT_HEIGHT + OUTER_PADDING);
    let group = svg.group()?;
    let mut guard = RenderGuard::new(group.clone());

    let rect_el = svg.rect(Point::origin(), size)?;
    guard.track(rect_el.clone());
    rect_el.set_fill(NAMED_BOX_FILL)?;
    rect_el.set_stroke(BOX_STROKE)?;
    rect_el.set_stroke_width(1.5)?;
    group.append(&rect_el)?;
    guard.release();

    let label_el = svg.text(Point::new(width / 2.0, LABEL_ROW_HEIGHT / 2.0), label)?;
    guard.track(label_el.clone());
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(LABEL_FONT_SIZE)?;
    label_el.set_fill(TEXT_FILL)?;
    // Width only: the label row has no height to spare for the plain node's vertical margin, and only the width can
    // overflow, since the node is `width` wide whatever the label says.
    shrink_label_to_fit(&label_el, Size::new(width, f64::INFINITY))?;
    group.append(&label_el)?;
    guard.release();

    let fo = svg.foreign_object(
        Point::new(OUTER_PADDING, LABEL_ROW_HEIGHT),
        Size::new(width - 2.0 * OUTER_PADDING, INPUT_HEIGHT),
    )?;
    guard.track(fo.clone());
    let document = fo
        .as_element()
        .owner_document()
        .ok_or_else(|| svg_dom::Error::Dom("text input: element has no owner document".into()))?;
    let input = document
        .create_element_ns(Some(NS_XHTML), "input")
        .map_err(|e| svg_dom::Error::Dom(format!("text input: could not create an <input>: {e:?}")))?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| svg_dom::Error::CastFailed("HtmlInputElement"))?;
    input.set_type("text");
    input.set_value(value);
    for (name, content) in [
        ("aria-label", label),
        ("autocomplete", "off"),
        ("spellcheck", "false"),
        ("style", INPUT_STYLE),
    ] {
        input
            .set_attribute(name, content)
            .map_err(|e| svg_dom::Error::Dom(format!("text input: could not set {name}: {e:?}")))?;
    }
    fo.as_element()
        .append_child(&input)
        .map_err(|e| svg_dom::Error::Dom(format!("text input: could not add the <input>: {e:?}")))?;
    group.append(&fo)?;
    guard.release();

    // On the group, not the `<foreignObject>`. A listener lasts as long as the Rust handle it was added through, and the
    // group's handle is kept with the node, where the `<foreignObject>`'s would be dropped when this function returns.
    // The `input` event bubbles from the text box up through both.
    group.on_event("input", listener)?;

    group.set_attr("role", "group")?;
    group.set_attr("aria-label", label)?;
    group.set_title(label)?;
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", top_left.x, top_left.y))?;
    guard.disarm();

    Ok((
        BoxHandles {
            group,
            draggable: false,
            enterable: false,
            edge_anchors,
            binary_operator_inputs: None,
            operator_input_side: None,
            binary_operator_input_edges: None,
            outer_rect: rect_el,
            cell_rects: Vec::new(),
            cell_texts: Vec::new(),
            replaceable: false,
            cell_grid: None,
            decor: DataDecor::default(),
            cell_stroke_width: "",
            selection: Selection::None,
            secondary: Vec::new(),
            unreached: Vec::new(),
            aria_label: label.to_owned(),
            base_label_len: label.len(),
            label_stale: false,
            edge_anchors_stale: false,
            ref_name: label.to_owned(),
            child: None,
            plot: None,
        },
        Rect { origin: top_left, size },
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a node holding a one-line text box the user can type into, and returns its id. The node is `width` wide,
    /// with `label` above the text box, which starts holding `value`.
    ///
    /// `on_input` is called each time the text changes, with this scene, the node's id and the text as it is now. It is
    /// not called for a change [`set_text_input_value`](Self::set_text_input_value) makes, since a caller that makes a
    /// change knows about it. It is the one place to react to what is typed.
    ///
    /// The label is also the text box's accessible name, so it should say what the text is for.
    ///
    /// Equivalent to [`add_text_input_node_with`](Self::add_text_input_node_with) with [`NodeOptions::default`].
    ///
    /// # Callback lifecycle
    ///
    /// `on_input` is called with nothing of this scene borrowed, so it may call back into it. It may even draw the whole
    /// scene again, including a new text input node. But the text box being typed in is then gone, and with it the
    /// keyboard focus and the caret. A caller that does this should read [`text_input_caret`](Self::text_input_caret)
    /// before it draws, and give the new node [`focus_text_input`](Self::focus_text_input) afterward.
    ///
    /// # Ownership
    ///
    /// `on_input` is held by the text box's own event listener, which holds only a `Weak` reference to this scene. So the
    /// scene is not kept alive by it. See [`show_selection_toolbar`](Self::show_selection_toolbar)'s own "Ownership" for
    /// what a closure that captures a strong `Scene` clone does.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyNodeName`] if `label` is empty or only whitespace. Returns [`Error::InvalidNodeGeometry`] if
    /// `top_left` is not finite, or `width` is not finite or is too narrow for a few characters.
    pub fn add_text_input_node(
        &self,
        top_left: Point,
        width: f64,
        label: &str,
        value: &str,
        on_input: impl Fn(&Scene, NodeId, &str) + 'static,
    ) -> Result<NodeId, Error> {
        self.add_text_input_node_with(top_left, width, label, value, NodeOptions::default(), on_input)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a text input node, as [`add_text_input_node`](Self::add_text_input_node), but with `options` controlling how
    /// many connector fixing points its sides offer — see [`EdgeAnchors`].
    ///
    /// # Errors
    ///
    /// As [`add_text_input_node`](Self::add_text_input_node), plus [`Error::InvalidEdgeAnchors`] if
    /// `options.edge_anchors` is `Some(EdgeAnchors(0))`. Every check is made before anything is drawn.
    pub fn add_text_input_node_with(
        &self,
        top_left: Point,
        width: f64,
        label: &str,
        value: &str,
        options: NodeOptions,
        on_input: impl Fn(&Scene, NodeId, &str) + 'static,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;
        validate_node_name(label)?;
        if !top_left.x.is_finite() || !top_left.y.is_finite() || !width.is_finite() || width < MIN_WIDTH {
            return Err(Error::InvalidNodeGeometry(Rect {
                origin: top_left,
                size: Size::new(width, 0.0),
            }));
        }

        // The listener needs the node's id, which does not exist until the node has been added. So it reads it from
        // here, and the id is written in once the node is in the graph. A change before that cannot happen: nothing
        // runs between the two.
        let id_cell: Rc<Cell<Option<NodeId>>> = Rc::new(Cell::new(None));
        let weak = Rc::downgrade(&self.inner);
        let listener = {
            let id_cell = id_cell.clone();
            move |event: web_sys::Event| {
                let Some(inner) = weak.upgrade() else { return };
                let Some(id) = id_cell.get() else { return };
                let Some(input) = event.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) else {
                    return;
                };
                // Nothing of the scene is borrowed here, so `on_input` can use it freely.
                on_input(&Scene { inner }, id, &input.value());
            }
        };

        let mut inner = self.inner.borrow_mut();
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = draw_text_input_box(
            &inner.svg,
            &mut scratch,
            top_left,
            width,
            label,
            value,
            options.edge_anchors,
            listener,
        );
        inner.scratch = scratch;
        let (handles, rect) = result?;
        inner.attach(&handles.group)?;
        let id = inner.graph.add_node(rect, NodeContent::TextInput(label.to_owned()));
        inner.insert_node_handle(id, handles);
        id_cell.set(Some(id));
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The text in text input node `id`'s text box, as it is now.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene, or [`Error::NotATextInput`] if it names
    /// a node of any other kind.
    pub fn text_input_value(&self, id: NodeId) -> Result<String, Error> {
        let inner = self.inner.borrow();
        let input = text_input_of(&inner, id)?;
        Ok(input.value())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the caret is in text input node `id`'s text box, as `(start, end)`: the two ends of the selection, which are
    /// equal when nothing is selected. Both count UTF-16 code units from the start of the text, as the browser does.
    ///
    /// # Errors
    ///
    /// As [`text_input_value`](Self::text_input_value).
    pub fn text_input_caret(&self, id: NodeId) -> Result<(u32, u32), Error> {
        let inner = self.inner.borrow();
        let input = text_input_of(&inner, id)?;
        let end = input.value().encode_utf16().count() as u32;
        let start = input.selection_start().ok().flatten().unwrap_or(end);
        let stop = input.selection_end().ok().flatten().unwrap_or(end);
        Ok((start, stop))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Gives text input node `id`'s text box the keyboard focus, and puts the caret at `caret`, as
    /// [`text_input_caret`](Self::text_input_caret) reports it. With `None` the caret is left where the browser puts it.
    ///
    /// A caret past the end of the text is placed at the end.
    ///
    /// # Errors
    ///
    /// As [`text_input_value`](Self::text_input_value). Returns [`Error::Svg`] if the browser refuses to move the focus.
    pub fn focus_text_input(&self, id: NodeId, caret: Option<(u32, u32)>) -> Result<(), Error> {
        let inner = self.inner.borrow();
        let input = text_input_of(&inner, id)?;
        // Without this, the browser scrolls the page to the text box, and a redraw on each keystroke would jolt it.
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        input
            .focus_with_options(&options)
            .map_err(|e| svg_dom::Error::Dom(format!("text input: could not take the focus: {e:?}")))?;
        if let Some((start, stop)) = caret {
            let end = input.value().encode_utf16().count() as u32;
            input
                .set_selection_range(start.min(end), stop.min(end))
                .map_err(|e| svg_dom::Error::Dom(format!("text input: could not place the caret: {e:?}")))?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces the text in text input node `id`'s text box with `value`. This does not call the `on_input` the node was
    /// added with, because the caller is the one making the change.
    ///
    /// # Errors
    ///
    /// As [`text_input_value`](Self::text_input_value).
    pub fn set_text_input_value(&self, id: NodeId, value: &str) -> Result<(), Error> {
        let inner = self.inner.borrow();
        text_input_of(&inner, id)?.set_value(value);
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Node `id`'s text box.
fn text_input_of(inner: &crate::scene::scene_inner::SceneInner, id: NodeId) -> Result<HtmlInputElement, Error> {
    match &inner.graph.node(id).ok_or(Error::UnknownNode(id))?.content {
        NodeContent::TextInput(_) => {},
        _ => return Err(Error::NotATextInput(id)),
    }
    inner
        .node_handle(id)
        .and_then(|handles| find_input(&handles.group))
        .ok_or(Error::NotATextInput(id))
}
