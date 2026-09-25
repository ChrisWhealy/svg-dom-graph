//! A fixed-size button bar along one edge of a [`Scene`], holding the zoom controls.
//!
//! The bar is a sibling of the scene's content layer under the `<svg>` root, never a child of it. Zooming or panning
//! the content therefore never scales, moves, or otherwise disturbs the bar, and it draws on top of the content.
//!
//! The bar is optional. [`Scene::show_toolbar`] creates it, [`Scene::hide_toolbar`] removes it from the DOM entirely,
//! and [`Scene::set_toolbar_edge`] moves it. The zoom operations behind its buttons are public too — see
//! [`Scene::zoom_in`], [`Scene::zoom_out`], and [`Scene::reset_view`] — so they work with or without the bar.

mod action;
mod button;
mod layout;
mod options;
mod pan;
mod wheel;

use super::{Scene, SceneInner};
use crate::{
    colours::{BOX_STROKE, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    geometry::{
        centre,
        side::Side,
        view::{ViewTransform, ZOOM_STEP},
    },
};
use action::ToolbarAction;
use button::ToolbarButton;
use layout::{layout, parse_view_box};
pub use options::ToolbarOptions;
use pan::build_pan_surface;
use std::{cell::RefCell, rc::Weak};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

/// Every toolbar button shares this look: an arrow-friendly pointer, and no accidental text selection when clicked
/// twice in quick succession.
const BUTTON_STYLE: &str = "cursor: pointer; user-select: none; -webkit-user-select: none;";

/// The opacity of a button whose action would currently change nothing, such as zoom-in at maximum zoom.
const DISABLED_OPACITY: &str = "0.4";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn visible_area(svg: &SvgRoot) -> Rect {
    svg.root
        .get_attribute("viewBox")
        .and_then(|value| parse_view_box(&value))
        .unwrap_or(Rect {
            origin: Point::origin(),
            size: Size::new(svg.width(), svg.height()),
        })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the position of `toolbar`'s own bar and every one of its buttons, for `area`.
fn place(toolbar: &Toolbar, area: Rect, scratch: &mut String) -> Result<(), Error> {
    let options = toolbar.options;
    let sizes: Vec<Size> = toolbar
        .buttons
        .iter()
        .map(|button| button.action.natural_size(options.button_height))
        .collect();
    let laid_out = layout(options.edge, area, &sizes, options.gap, options.margin);

    for (name, value) in [
        ("x", area.origin.x),
        ("y", area.origin.y),
        ("width", area.size.width),
        ("height", area.size.height),
    ] {
        toolbar.pan_surface.set_attr_display(scratch, name, value)?;
    }

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
        let text_at = centre(Rect {
            origin: Point::origin(),
            size: rect.size,
        });
        button.label.set_attr_display(scratch, "x", text_at.x)?;
        button.label.set_attr_display(scratch, "y", text_at.y)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws one button and wires its click and keyboard activation to `action`.
///
/// The button's size and position are left for [`place`] to write.
fn build_button(
    svg: &SvgRoot,
    bar: &SvgNode,
    inner: &Weak<RefCell<SceneInner>>,
    action: ToolbarAction,
    height: f64,
) -> Result<ToolbarButton, Error> {
    let group = svg.group()?;
    bar.append(&group)?;
    group.set_attr("role", "button")?;
    group.set_attr("tabindex", "0")?;
    group.set_attr("aria-label", action.aria_label())?;
    group.set_attr("style", BUTTON_STYLE)?;

    let rect = svg.rect(Point::origin(), action.natural_size(height))?;
    group.append(&rect)?;
    rect.set_fill(PLAIN_BOX_FILL)?;
    rect.set_stroke(BOX_STROKE)?;
    rect.set_stroke_width(1.0)?;
    rect.set_attr("rx", "4")?;

    let label = svg.text(Point::origin(), action.label())?;
    group.append(&label)?;
    label.set_text_anchor(TextAnchor::Middle)?;
    label.set_dominant_baseline(DominantBaseline::Middle)?;
    label.set_font_size(height * 0.55)?;
    label.set_fill(TEXT_FILL)?;

    // Both listeners hold only a `Weak` reference to the scene's shared state. A strong one would form a cycle
    // through `SceneInner::toolbar` and leak the scene — see `Scene::make_draggable_with`'s own comment on this.
    let on_click = inner.clone();
    group.on_click(move |_| apply(&on_click, action))?;

    let on_key = inner.clone();
    group.on_keydown(move |event| {
        if event.key() == "Enter" || event.key() == " " {
            // Stops Space scrolling the page, and Enter activating anything else.
            event.prevent_default();
            apply(&on_key, action);
        }
    })?;

    Ok(ToolbarButton { action, group, rect, label })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `action` against the scene behind `inner`, if it is still alive.
fn apply(inner: &Weak<RefCell<SceneInner>>, action: ToolbarAction) {
    let Some(inner) = inner.upgrade() else { return };
    let scene = Scene { inner };
    // A listener has nowhere to report an error to, and a failed DOM write leaves the previous view in place.
    let _ = match action {
        ToolbarAction::ZoomIn => scene.zoom_in(),
        ToolbarAction::ZoomOut => scene.zoom_out(),
        ToolbarAction::Reset => scene.reset_view(),
    };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A rendered toolbar. Lives in [`SceneInner::toolbar`] for as long as it is shown.
pub(super) struct Toolbar {
    pub group: SvgNode,
    /// The transparent surface behind the content layer that a drag on empty background pans — see [`pan`].
    pub pan_surface: SvgNode,
    /// The content layer, kept so removing the toolbar can also remove the wheel-zoom listener registered on it — see
    /// [`wheel`].
    pub content: SvgNode,
    pub options: ToolbarOptions,
    pub buttons: Vec<ToolbarButton>,
}

impl Toolbar {
    /// Removes the whole bar, and with it every button, the pan surface, and their listeners, from the DOM.
    fn remove(self) {
        self.group.remove();
        self.pan_surface.remove();
        self.content.remove_listeners("wheel");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl SceneInner {
    /// The rectangle of the `<svg>`'s own user space that is currently visible: its `viewBox`, or failing that
    /// `(0, 0, width, height)`.
    pub(super) fn visible_area(&self) -> Rect {
        visible_area(&self.svg)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes `view` the content layer's transform and brings every toolbar button's enabled state in line with it.
    ///
    /// Does nothing if `view` is already current.
    pub(super) fn set_view(&mut self, view: ViewTransform) -> Result<(), Error> {
        if view == self.view {
            return Ok(());
        }

        let mut scratch = std::mem::take(&mut self.scratch);
        view.write_attr(&mut scratch);
        let result = self.content.set_attr("transform", &scratch);
        self.scratch = scratch;
        result?;

        self.view = view;
        self.sync_toolbar_state()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks each toolbar button enabled or disabled, to match what its action could currently do.
    ///
    /// A disabled button is dimmed and carries `aria-disabled`. It stays focusable, so keyboard focus is never lost
    /// from under someone who has just pressed it into its own limit.
    fn sync_toolbar_state(&self) -> Result<(), Error> {
        let Some(toolbar) = &self.toolbar else { return Ok(()) };
        for button in &toolbar.buttons {
            let enabled = button.action.is_enabled(self.view);
            button.group.set_attr("aria-disabled", if enabled { "false" } else { "true" })?;
            button.group.set_attr("opacity", if enabled { "1" } else { DISABLED_OPACITY })?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Positions the bar and its buttons for the current visible area.
    fn layout_toolbar(&mut self) -> Result<(), Error> {
        let area = self.visible_area();
        let mut scratch = std::mem::take(&mut self.scratch);
        let result = self
            .toolbar
            .as_ref()
            .map_or(Ok(()), |toolbar| place(toolbar, area, &mut scratch));
        self.scratch = scratch;
        result
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Shows the toolbar with `options`, replacing any toolbar already shown.
    ///
    /// While the toolbar is shown, holding Ctrl or Cmd and turning the mouse wheel also zooms, about the pointer. A
    /// trackpad pinch works too, since browsers report it as ctrl+wheel. A wheel without a modifier is left alone.
    ///
    /// While the toolbar is shown, dragging empty background also pans the content, so content zoomed past the edge of
    /// the visible area can always be brought back. Nodes and connectors take their own pointer events, so dragging
    /// one still drags only that node.
    ///
    /// The bar holds three buttons — zoom in, zoom out, and reset. It stays a fixed size however far the content is
    /// zoomed, and draws on top of it. Every button is keyboard operable: Tab to reach one, Enter or Space to activate.
    ///
    /// The bar is positioned against the `<svg>`'s visible area as it is now. Call
    /// [`refresh_toolbar_layout`](Self::refresh_toolbar_layout) after that area changes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidToolbarOptions`] if `options` holds a non-finite length, a `button_height` that is not
    /// `> 0.0`, or a negative `gap` or `margin`. Checked first, so a rejected call leaves any existing toolbar exactly
    /// as it was.
    ///
    /// If drawing fails partway, nothing is left behind and no toolbar is shown.
    pub fn show_toolbar(&self, options: ToolbarOptions) -> Result<(), Error> {
        if !options.is_valid() {
            return Err(Error::InvalidToolbarOptions(options));
        }

        let mut inner = self.inner.borrow_mut();
        if let Some(existing) = inner.toolbar.take() {
            existing.remove();
        }

        let weak = std::rc::Rc::downgrade(&self.inner);
        let pan_surface = build_pan_surface(&inner.svg, &inner.content, &weak)?;
        let group = match inner.svg.group() {
            Ok(group) => group,
            Err(err) => {
                pan_surface.remove();
                return Err(err.into());
            },
        };
        let built = (|| {
            group.set_attr("role", "toolbar")?;
            group.set_attr("aria-label", "Scene controls")?;
            let buttons = ToolbarAction::ALL
                .into_iter()
                .map(|action| build_button(&inner.svg, &group, &weak, action, options.button_height))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(buttons)
        })();

        let buttons = match built {
            Ok(buttons) => buttons,
            Err(err) => {
                group.remove();
                pan_surface.remove();
                return Err(err);
            },
        };

        inner.toolbar = Some(Toolbar {
            group,
            pan_surface,
            content: inner.content.clone(),
            options,
            buttons,
        });
        let laid_out = inner.layout_toolbar().and_then(|()| inner.sync_toolbar_state());
        if let Err(err) = laid_out {
            if let Some(toolbar) = inner.toolbar.take() {
                toolbar.remove();
            }
            return Err(err);
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Removes the toolbar from the DOM entirely. Does nothing if none is shown.
    ///
    /// The scene's current zoom is kept. Use [`reset_view`](Self::reset_view) to undo it.
    pub fn hide_toolbar(&self) {
        if let Some(toolbar) = self.inner.borrow_mut().toolbar.take() {
            toolbar.remove();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether a toolbar is currently shown.
    pub fn has_toolbar(&self) -> bool {
        self.inner.borrow().toolbar.is_some()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves the shown toolbar to `edge`. Does nothing if none is shown.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if repositioning fails.
    pub fn set_toolbar_edge(&self, edge: Side) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();
        let Some(toolbar) = inner.toolbar.as_mut() else { return Ok(()) };
        if toolbar.options.edge == edge {
            return Ok(());
        }
        toolbar.options.edge = edge;
        inner.layout_toolbar()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Repositions the shown toolbar against the `<svg>`'s visible area as it is now. Does nothing if none is shown.
    ///
    /// Call this after changing the `<svg>`'s size (`SvgRoot::set_viewport`) or `viewBox`, since the scene cannot
    /// observe either.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if repositioning fails.
    pub fn refresh_toolbar_layout(&self) -> Result<(), Error> {
        self.inner.borrow_mut().layout_toolbar()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Zooms the content in by one step, about the centre of the `<svg>`'s visible area.
    ///
    /// Stops at a maximum scale of `4.0`. Calling it there changes nothing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if the DOM write fails, in which case the previous zoom is kept.
    pub fn zoom_in(&self) -> Result<(), Error> {
        self.zoom_by(ZOOM_STEP)
    }

    /// Zooms the content out by one step, about the centre of the `<svg>`'s visible area.
    ///
    /// Stops at a minimum scale of `0.25`. Calling it there changes nothing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if the DOM write fails, in which case the previous zoom is kept.
    pub fn zoom_out(&self) -> Result<(), Error> {
        self.zoom_by(1.0 / ZOOM_STEP)
    }

    /// Returns the content to its original scale and position.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if the DOM write fails, in which case the previous zoom is kept.
    pub fn reset_view(&self) -> Result<(), Error> {
        self.inner.borrow_mut().set_view(ViewTransform::IDENTITY)
    }

    /// The content's current scale factor: `1.0` when unzoomed.
    pub fn zoom_scale(&self) -> f64 {
        self.inner.borrow().view.scale
    }

    fn zoom_by(&self, factor: f64) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();
        let pivot = centre(inner.visible_area());
        let next = inner.view.zoomed_about(factor, pivot);
        inner.set_view(next)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
