//! A fixed-size button bar — Prev, Next, Restart — bound to exactly one data node, for stepping through its own
//! cells.
//!
//! The bar is a sibling of the scene's content layer under the `<svg>` root, never a child of it — same reasoning
//! as [`super::toolbar`]'s own bar. It is optional: [`Scene::show_selection_toolbar`] creates it,
//! [`Scene::hide_selection_toolbar`] removes it entirely.
//!
//! There is no cursor stored here, or anywhere else. Every button reads the managed node's own current
//! [`Selection`], via [`DataNodeContent::flat_index`](crate::scene::DataNodeContent::flat_index), and writes the next one back through
//! [`Scene::set_selection`] — see [`Scene::show_selection_toolbar`]'s own doc comment for why.

mod action;
mod button;
mod layout;
mod options;

use super::{Scene, SceneInner, Selection, Side};
use crate::{
    colours::{BOX_STROKE, FOCUS_RING, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    model::node::{NodeContent, NodeId},
};
use action::SelectionToolbarAction;
use button::SelectionToolbarButton;
use layout::layout;
pub use options::SelectionToolbarOptions;
use std::{cell::RefCell, rc::Rc, rc::Weak};
use svg_dom::{DominantBaseline, SvgNode, SvgRoot, TextAnchor, root::utils::Point};

/// Every selection toolbar button shares this look — same reasoning as [`super::toolbar::BUTTON_STYLE`].
const BUTTON_STYLE: &str = "cursor: pointer; user-select: none; -webkit-user-select: none;";
const BUTTON_STROKE_WIDTH: f64 = 1.0;
const FOCUS_STROKE_WIDTH: f64 = 3.0;
/// The opacity of a disabled button
const DISABLED_OPACITY: &str = "0.4";
/// Every button's own natural size — a fixed-width word, not square like the zoom toolbar's "+"/"−".
const BUTTON_WIDTH: f64 = 64.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a `SelectionTransition` reports: a change to the managed node's own flat position, never which button
/// caused it.
///
/// `Restart` from `Some(3)` and `Prev` from `Some(0)` both produce `{from: Some(_), to: None}` and are
/// deliberately indistinguishable here — both leave the managed node at the same unstarted state, and a host's own
/// "reset everything" response to `to.is_none()` is correct for either. A caller that needs to tell them apart has
/// no way to, by design — see [`Scene::show_selection_toolbar`]'s own doc comment.
/// The host's own `on_step` callback, reference-counted so all three buttons can share one copy — see
/// [`SelectionToolbar::on_step`]'s own doc comment for why it lives only there, never on `SceneInner` itself.
type OnStep = Rc<dyn Fn(&Scene, NodeId, SelectionTransition)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionTransition {
    /// The flat position the managed node's own [`Selection`] named before this change — `None` for the unstarted
    /// state, or for a `Selection` [`DataNodeContent::flat_index`](crate::scene::DataNodeContent::flat_index) does
    /// not recognise.
    pub from: Option<usize>,
    /// The flat position the managed node's own [`Selection`] now names — `None` for the unstarted state.
    pub to: Option<usize>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A rendered selection toolbar. Lives in [`SceneInner::selection_toolbar`] for as long as it is shown.
pub(super) struct SelectionToolbar {
    pub group: SvgNode,
    pub options: SelectionToolbarOptions,
    /// The one data node this toolbar steps. Fixed for as long as this toolbar is shown — replacing it means
    /// calling [`Scene::show_selection_toolbar`] again, which replaces the whole toolbar.
    pub node: NodeId,
    /// The host's own callback, shared by all three buttons. Not stored anywhere else — see
    /// [`Scene::show_selection_toolbar`]'s own doc comment ("Ownership") for why keeping it out of `SceneInner`
    /// itself matters.
    #[allow(dead_code)] // Held only so the buttons' own `Rc::clone`s of it stay backed by this owner too.
    pub on_step: OnStep,
    pub buttons: Vec<SelectionToolbarButton>,
}

impl SelectionToolbar {
    /// Removes the whole bar, and with it every button and listener, from the DOM.
    fn remove(self) {
        self.group.remove();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the position of `toolbar`'s own bar and every one of its buttons, for `area`.
fn place(toolbar: &SelectionToolbar, area: svg_dom::root::utils::Rect, scratch: &mut String) -> Result<(), Error> {
    let options = toolbar.options;
    // Exactly three buttons, always — see `SelectionToolbarAction::ALL` — so this is a fixed-size array, not a
    // `Vec` collected from `toolbar.buttons`: every button shares the same size, so there's nothing in `toolbar`
    // itself this genuinely needs to read, and `layout` only ever needs a `&[Size]` slice.
    let sizes = [svg_dom::root::utils::Size::new(BUTTON_WIDTH, options.button_height); 3];
    let laid_out = layout(options.edge, area, &sizes, options.gap, options.margin);

    toolbar.group.set_transform_fmt(
        scratch,
        format_args!("translate({}, {})", laid_out.bar.origin.x, laid_out.bar.origin.y),
    )?;
    let orientation = match options.edge {
        Side::North | Side::South => "horizontal",
        Side::East | Side::West => "vertical",
    };
    toolbar.group.set_attr("aria-orientation", orientation)?;

    for (button, rect) in toolbar.buttons.iter().zip(&laid_out.buttons) {
        button
            .group
            .set_transform_fmt(scratch, format_args!("translate({}, {})", rect.origin.x, rect.origin.y))?;
        button.rect.set_attr_display(scratch, "width", rect.size.width)?;
        button.rect.set_attr_display(scratch, "height", rect.size.height)?;
        let text_at = crate::geometry::centre(svg_dom::root::utils::Rect {
            origin: Point::origin(),
            size: rect.size,
        });
        button.label.set_attr_display(scratch, "x", text_at.x)?;
        button.label.set_attr_display(scratch, "y", text_at.y)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws one button and wires its click and keyboard activation to `action`, against the node managed by `node`.
///
/// The button's size and position are left for [`place`] to write.
fn build_button(
    svg: &SvgRoot,
    bar: &SvgNode,
    inner: &Weak<RefCell<SceneInner>>,
    node: NodeId,
    action: SelectionToolbarAction,
    on_step: &OnStep,
    height: f64,
) -> Result<SelectionToolbarButton, Error> {
    let group = svg.group()?;
    bar.append(&group)?;
    group.set_attr("role", "button")?;
    group.set_attr("tabindex", "0")?;
    group.set_attr("aria-label", action.aria_label())?;
    group.set_attr("style", BUTTON_STYLE)?;

    let rect = svg.rect(Point::origin(), svg_dom::root::utils::Size::new(BUTTON_WIDTH, height))?;
    group.append(&rect)?;
    rect.set_fill(PLAIN_BOX_FILL)?;
    rect.set_stroke(BOX_STROKE)?;
    rect.set_stroke_width(BUTTON_STROKE_WIDTH)?;
    rect.set_attr("rx", "4")?;

    let label = svg.text(Point::origin(), action.label())?;
    group.append(&label)?;
    label.set_text_anchor(TextAnchor::Middle)?;
    label.set_dominant_baseline(DominantBaseline::Middle)?;
    label.set_font_size(height * 0.5)?;
    label.set_fill(TEXT_FILL)?;

    // Keyboard focus must be obvious — same reasoning and shape as `toolbar::build_button`'s own focus ring.
    let focused_rect = rect.downgrade();
    group.on_focus(move |_| {
        if let Some(rect) = focused_rect.upgrade() {
            let _ = rect.set_stroke(FOCUS_RING);
            let _ = rect.set_stroke_width(FOCUS_STROKE_WIDTH);
        }
    })?;
    let blurred_rect = rect.downgrade();
    group.on_blur(move |_| {
        if let Some(rect) = blurred_rect.upgrade() {
            let _ = rect.set_stroke(BOX_STROKE);
            let _ = rect.set_stroke_width(BUTTON_STROKE_WIDTH);
        }
    })?;

    // Both listeners hold only a `Weak` reference to the scene's shared state, exactly like `toolbar::build_button`'s
    // own — a strong one would keep `SceneInner` alive for as long as the page holds this listener. `on_step` is
    // reference-counted separately (see `SelectionToolbar::on_step`'s own doc comment): each button's own `Rc::clone`
    // is owned by that button's DOM closure alone, never by `SceneInner` itself, so there is no retained-callback
    // cycle through it either — see `Scene::show_selection_toolbar`'s own doc comment ("Ownership").
    let on_click_inner = inner.clone();
    let on_click_step = on_step.clone();
    group.on_click(move |_| apply(&on_click_inner, node, action, &on_click_step))?;

    let on_key_inner = inner.clone();
    let on_key_step = on_step.clone();
    group.on_keydown(move |event| {
        if event.key() == "Enter" || event.key() == " " {
            event.prevent_default();
            apply(&on_key_inner, node, action, &on_key_step);
        }
    })?;

    Ok(SelectionToolbarButton {
        action,
        group,
        rect,
        label,
        enabled: std::cell::Cell::new(None),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `action` against the node managed by the scene behind `inner`, if it is still alive, then calls `on_step`.
///
/// Follows a strict two-phase shape: everything that reads `SceneInner` happens inside one short borrow, which is
/// dropped before `Scene::set_selection` (which takes its own borrow) or `on_step` ever run. `on_step` is
/// explicitly meant to reenter this crate — `set_selection`, `replace_container_child`, even
/// `show_selection_toolbar` again — so it must never run while any part of `SceneInner` is still borrowed. See
/// `Scene::show_selection_toolbar`'s own doc comment ("Callback lifecycle").
///
/// A disabled button's own activation reaches here too — `action.next_position` returning `None` is exactly what
/// stops it: neither `set_selection` nor `on_step` runs, and nothing about the node changes.
fn apply(inner: &Weak<RefCell<SceneInner>>, node: NodeId, action: SelectionToolbarAction, on_step: &OnStep) {
    let Some(inner_rc) = inner.upgrade() else { return };
    let scene = Scene { inner: inner_rc };

    let (from, to, new_selection) = {
        let borrowed = scene.inner.borrow();
        let Some(node_data) = borrowed.graph.node(node) else { return };
        let NodeContent::Data(content) = &node_data.content else { return };
        let Some(handles) = borrowed.node_handle(node) else { return };
        let from = content.flat_index(&handles.selection);
        let Some(to) = action.next_position(from, content.len()) else { return };
        let new_selection = match to {
            Some(i) => content.natural_selection(i).unwrap_or(Selection::None),
            None => Selection::None,
        };
        (from, to, new_selection)
    };
    // `borrowed` (and every reference derived from it) is dropped here — `set_selection` and `on_step` below are
    // both safe to reenter this crate's own state.

    if scene.set_selection(node, new_selection).is_err() {
        // Nowhere to report this to, and a failed write leaves the previous selection in place — same convention
        // as `toolbar::apply`'s own `let _ = ...`. `on_step` did not run, since nothing actually changed.
        return;
    }
    on_step(&scene, node, SelectionTransition { from, to });
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl SceneInner {
    /// Marks each selection toolbar button enabled or disabled, to match what its action could currently do against
    /// the managed node's own current [`Selection`].
    ///
    /// Same "write only what changed" shape as the zoom toolbar's own equivalent internal sync step — each button
    /// remembers whether it was last drawn enabled, and writes its attributes only when that changes. Called from
    /// every successful [`Scene::set_selection`], not only from a
    /// click through this toolbar's own buttons: an external `set_selection` call against the managed node is, by
    /// design, exactly as valid a way to move it as one of these buttons — see `Scene::show_selection_toolbar`'s own
    /// doc comment.
    pub(super) fn sync_selection_toolbar_state(&self) -> Result<(), Error> {
        let Some(toolbar) = &self.selection_toolbar else { return Ok(()) };
        let Some(node_data) = self.graph.node(toolbar.node) else { return Ok(()) };
        let NodeContent::Data(content) = &node_data.content else { return Ok(()) };
        let Some(handles) = self.node_handle(toolbar.node) else { return Ok(()) };
        let current = content.flat_index(&handles.selection);
        let len = content.len();

        for button in &toolbar.buttons {
            let enabled = button.action.is_enabled(current, len);
            if button.enabled.get() == Some(enabled) {
                continue;
            }
            button.group.set_attr("aria-disabled", if enabled { "false" } else { "true" })?;
            button.group.set_attr("opacity", if enabled { "1" } else { DISABLED_OPACITY })?;
            button.enabled.set(Some(enabled));
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Positions the bar and its buttons for the current visible area.
    pub(super) fn layout_selection_toolbar(&mut self) -> Result<(), Error> {
        let area = self.visible_area();
        let mut scratch = std::mem::take(&mut self.scratch);
        let result = self
            .selection_toolbar
            .as_ref()
            .map_or(Ok(()), |toolbar| place(toolbar, area, &mut scratch));
        self.scratch = scratch;
        result
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Shows a selection toolbar bound to `node` — Prev, Next, and Restart, for stepping through `node`'s own data
    /// cells — replacing any selection toolbar already shown.
    ///
    /// # The managed node's `Selection` is the only state
    ///
    /// There is no cursor kept alongside `node`'s own [`Selection`]. Every button reads `node`'s *current*
    /// `Selection`, via [`DataNodeContent::flat_index`](crate::scene::DataNodeContent::flat_index), derives the flat
    /// position it names, computes the next one, and writes it straight back through [`Scene::set_selection`].
    /// `flat_index` is the inverse of
    /// [`DataNodeContent::natural_selection`](crate::scene::DataNodeContent::natural_selection), over the selections
    /// `natural_selection` can produce for `node`'s own current grid shape — see both their own doc comments.
    ///
    /// Because there is only one value, not two, there is nothing to fall out of sync. A `Scene::set_selection` call
    /// against `node` made after this toolbar is shown simply becomes the new starting point the next button press
    /// advances from — see [`SelectionTransition`]'s own doc comment for exactly what the callback sees when that
    /// external `Selection` is not one this toolbar's own buttons would ever have produced.
    ///
    /// Showing the toolbar resets `node` to [`Selection::None`] — "unstarted", before element `0` is ever processed
    /// — as its own first *committed* act, regardless of whatever `Selection` `node` already held. See "Failure
    /// guarantee" below for why "first committed act" is not the same as "first thing this call does."
    ///
    /// # Disabled buttons
    ///
    /// Activating a disabled button does nothing: it does not change `node`'s own `Selection`, and does not invoke
    /// `on_step`. Prev and Restart are disabled once `node` is already unstarted — there is nothing before it to
    /// walk back to. Next is disabled at the last element, and outright for a `node` with no values at all, which
    /// also holds it at the unstarted state forever; showing this toolbar on such a node is not an error — every
    /// button just starts, and stays, disabled.
    ///
    /// # `on_step`
    ///
    /// Called after every actual change to `node`'s own `Selection` — never for a disabled button's own activation —
    /// with the [`Scene`] the change happened in, `node`, and a [`SelectionTransition`] describing the change.
    /// Mandatory, not an optional `_with` variant: the library owns `node`'s own `Selection`, `on_step` is how the
    /// host owns the consequences of it changing, and there is no meaningful toolbar without both halves. A `node`
    /// whose selection changes need no downstream response can still pass `|_, _, _| {}`.
    ///
    /// ## Callback lifecycle
    ///
    /// `on_step` is always called with every part of this `Scene`'s own state already fully updated and unborrowed —
    /// it is free to reenter this crate, including calling back into this same `Scene`.
    ///
    /// ## Ownership
    ///
    /// `on_step` is not stored on this `Scene`'s own shared state. It is wrapped once and cloned into each of the
    /// three buttons' own DOM click/keydown closures, which already hold only a `Weak` reference back to the scene
    /// — the same shape [`make_enterable`](Self::make_enterable)'s own listeners use. So there is no retained-
    /// callback cycle through this crate's own state. What remains the host's own responsibility: a closure that
    /// captures a strong `Scene` clone (rather than using the `&Scene` argument it is handed at call time) keeps
    /// that `Scene`'s underlying state alive for as long as the page's own DOM listener lives — no different from
    /// any other `on_click`/`on_keydown` closure a host writes today.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSelectionToolbarOptions`] if `options` holds a non-finite length, a `button_height`
    /// that is not `> 0.0`, or a negative `gap` or `margin`. Returns [`Error::UnknownNode`] if `node` does not name a
    /// node in this scene, or [`Error::InvalidSelection`] if it names a `Label` or `Container` node rather than a
    /// data node. All three are checked before drawing anything or touching any selection toolbar already shown.
    ///
    /// # Failure guarantee
    ///
    /// Follows this crate's usual construction ordering: validate, build and install the new toolbar's DOM, and only
    /// once that has fully succeeded, commit `Selection::None` on `node`. If this call returns `Err`, any
    /// previously shown selection toolbar and `node`'s own existing `Selection` are both left exactly as they were.
    pub fn show_selection_toolbar(
        &self,
        node: NodeId,
        options: SelectionToolbarOptions,
        on_step: impl Fn(&Scene, NodeId, SelectionTransition) + 'static,
    ) -> Result<(), Error> {
        if !options.is_valid() {
            return Err(Error::InvalidSelectionToolbarOptions(options));
        }
        {
            let inner = self.inner.borrow();
            match &inner.graph.node(node).ok_or(Error::UnknownNode(node))?.content {
                NodeContent::Data(_) => {},
                NodeContent::Label(_) | NodeContent::Container(_) => {
                    return Err(Error::InvalidSelection(node, Selection::None));
                },
            }
        }

        let on_step: OnStep = Rc::new(on_step);
        let weak = Rc::downgrade(&self.inner);

        let mut inner = self.inner.borrow_mut();
        let group = inner.svg.group()?;
        let built = (|| {
            group.set_attr("role", "toolbar")?;
            group.set_attr("aria-label", "Selection controls")?;
            let buttons = SelectionToolbarAction::ALL
                .into_iter()
                .map(|action| build_button(&inner.svg, &group, &weak, node, action, &on_step, options.button_height))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(buttons)
        })();

        let buttons = match built {
            Ok(buttons) => buttons,
            Err(err) => {
                group.remove();
                return Err(err);
            },
        };

        let new_toolbar = SelectionToolbar {
            group,
            options,
            node,
            on_step,
            buttons,
        };
        let previous = inner.selection_toolbar.replace(new_toolbar);

        if let Err(err) = inner.layout_selection_toolbar() {
            if let Some(new) = inner.selection_toolbar.take() {
                new.remove();
            }
            inner.selection_toolbar = previous;
            return Err(err);
        }
        drop(inner);

        if let Err(err) = self.set_selection(node, Selection::None) {
            let mut inner = self.inner.borrow_mut();
            if let Some(new) = inner.selection_toolbar.take() {
                new.remove();
            }
            inner.selection_toolbar = previous;
            return Err(err);
        }

        // Only now is the new toolbar fully committed — built, installed, laid out, and the managed node's own
        // `Selection` reset. Only now is it safe to remove whatever toolbar this one replaced: `previous`'s own DOM
        // group (and with it, its buttons' own listeners and their `Rc<dyn Fn>` clone of its own `on_step`) must
        // stay intact until this point, since either of the two failure branches above puts it straight back into
        // `inner.selection_toolbar` as the still-live, still-shown toolbar.
        if let Some(previous) = previous {
            previous.remove();
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Removes the selection toolbar from the DOM entirely, and with it every button and its listeners. Does
    /// nothing if none is shown.
    ///
    /// This also drops this toolbar's own `Rc<dyn Fn>` wrapper around `on_step` — since it was never held anywhere
    /// but the removed buttons' own DOM closures (see [`show_selection_toolbar`](Self::show_selection_toolbar)'s own
    /// doc comment, "Ownership"), `on_step` itself becomes droppable as soon as nothing else in the host's own code
    /// still holds a reference into it.
    ///
    /// The managed node's current `Selection` is left exactly as it was.
    pub fn hide_selection_toolbar(&self) {
        let Some(toolbar) = self.inner.borrow_mut().selection_toolbar.take() else {
            return;
        };
        toolbar.remove();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether a selection toolbar is currently shown.
    pub fn has_selection_toolbar(&self) -> bool {
        self.inner.borrow().selection_toolbar.is_some()
    }
}
