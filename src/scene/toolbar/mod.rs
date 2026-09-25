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

use super::{Scene, SceneInner};
use crate::{
    colours::{BOX_STROKE, FOCUS_RING, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    geometry::{
        centre, invert_matrix,
        side::Side,
        view::{ViewTransform, ZOOM_STEP},
    },
};
use action::ToolbarAction;
use button::ToolbarButton;
use layout::{layout, parse_view_box, visible_user_area};
pub use options::ToolbarOptions;
use std::{cell::RefCell, rc::Weak};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

/// Every toolbar button shares this look: an arrow-friendly pointer, and no accidental text selection when clicked
/// twice in quick succession.
const BUTTON_STYLE: &str = "cursor: pointer; user-select: none; -webkit-user-select: none;";

/// A toolbar button's border width at rest, and while it has keyboard focus.
const BUTTON_STROKE_WIDTH: f64 = 1.0;
const FOCUS_STROKE_WIDTH: f64 = 3.0;

/// The opacity of a button whose action would currently change nothing, such as zoom-in at maximum zoom.
const DISABLED_OPACITY: &str = "0.4";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The part of the `<svg>`'s own user space that is visible: its `viewBox` if it has one, otherwise `(0, 0)` to its
/// size in pixels.
///
/// `svg-dom` reads the size once, from the `width` and `height` attributes, and never measures the rendered size. An
/// `<svg>` sized purely by CSS therefore reports `0 x 0`, which would lay everything out in a zero-sized area. So when
/// there is no `viewBox` and either cached dimension is not positive, the rendered size is used instead.
fn visible_area(svg: &SvgRoot) -> Rect {
    if let Some(view_box) = svg.root.get_attribute("viewBox").and_then(|value| parse_view_box(&value)) {
        return view_box;
    }

    let (mut width, mut height) = (svg.width(), svg.height());
    if !(width > 0.0 && height > 0.0) {
        let rendered = svg.root.get_bounding_client_rect();
        (width, height) = (rendered.width(), rendered.height());
    }
    Rect {
        origin: Point::origin(),
        size: Size::new(width, height),
    }
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
    rect.set_stroke_width(BUTTON_STROKE_WIDTH)?;
    rect.set_attr("rx", "4")?;

    let label = svg.text(Point::origin(), action.label())?;
    group.append(&label)?;
    label.set_text_anchor(TextAnchor::Middle)?;
    label.set_dominant_baseline(DominantBaseline::Middle)?;
    label.set_font_size(height * 0.55)?;
    label.set_fill(TEXT_FILL)?;

    // Keyboard focus must be obvious. A browser's own outline on a focused SVG element varies, so draw one explicitly:
    // a thicker, distinctly coloured border while focused. The rect is held weakly, like the scene below.
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

    Ok(ToolbarButton {
        action,
        group,
        rect,
        label,
        // Not drawn yet: the first sync writes it.
        enabled: std::cell::Cell::new(None),
    })
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
    pub options: ToolbarOptions,
    pub buttons: Vec<ToolbarButton>,
}

impl Toolbar {
    /// Removes the whole bar, and with it every button and listener, from the DOM.
    fn remove(self) {
        self.group.remove();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl SceneInner {
    /// The rectangle of the `<svg>`'s own user space that is visible right now.
    ///
    /// This is what the browser actually shows, which is not always the `viewBox`: a `viewBox` origin other than
    /// `(0, 0)` shifts it, `preserveAspectRatio="meet"` (the default) shows more than the `viewBox` when the box is a
    /// different shape, and `"slice"` shows less. So it is found by mapping the rendered box back into user space
    /// through the `<svg>`'s own screen matrix, which accounts for all of those and for any CSS scaling at once.
    ///
    /// Falls back to [`visible_area`], which reads the `viewBox` or size attributes, if the browser cannot give a
    /// screen matrix — for example for an `<svg>` that is not being displayed.
    pub(super) fn visible_area(&self) -> Rect {
        self.rendered_user_area().unwrap_or_else(|| visible_area(&self.svg))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The part of the `<svg>`'s user space its rendered box shows, or `None` if it cannot be measured.
    fn rendered_user_area(&self) -> Option<Rect> {
        // The `<svg>` root as a node, so its own screen matrix can be read. It is the content layer's parent.
        let root = self.content.parent()?;
        let inverse_ctm = root.screen_ctm().and_then(invert_matrix)?;
        let rendered = self.svg.root.get_bounding_client_rect();
        visible_user_area(
            Rect {
                origin: Point::new(rendered.left(), rendered.top()),
                size: Size::new(rendered.width(), rendered.height()),
            },
            inverse_ctm,
        )
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes `view` the current view and writes it to the content layer's `transform` at once.
    ///
    /// Also brings every toolbar button's enabled state in line with it, and settles any earlier
    /// [`set_view_deferred`](Self::set_view_deferred) change still waiting for its frame. Does nothing if `view` is
    /// already current and already written.
    pub(super) fn set_view(&mut self, view: ViewTransform) -> Result<(), Error> {
        if view == self.view && !self.view_dirty {
            return Ok(());
        }
        self.view = view;
        self.view_dirty = true;
        self.flush_view()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes `view` the current view without touching the DOM. Returns whether there is now a write to flush.
    ///
    /// For a gesture that can fire many times per frame. The next [`flush_view`](Self::flush_view) writes only the
    /// latest view, so however many events arrived in between cost one DOM write. Reading [`view`](Self::view) in the
    /// meantime already gives the latest value, so consecutive events compose correctly.
    pub(super) fn set_view_deferred(&mut self, view: ViewTransform) -> bool {
        if view != self.view {
            self.view = view;
            self.view_dirty = true;
        }
        self.view_dirty
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the current view to the content layer, if it has changed since it was last written.
    ///
    /// If the DOM write fails the view stays marked as waiting, so the next flush tries again.
    pub(super) fn flush_view(&mut self) -> Result<(), Error> {
        if !self.view_dirty {
            return Ok(());
        }

        let mut scratch = std::mem::take(&mut self.scratch);
        self.view.write_attr(&mut scratch);
        let result = self.content.set_attr("transform", &scratch);
        self.scratch = scratch;
        result?;

        self.view_dirty = false;
        self.sync_view_label()?;
        self.sync_toolbar_state()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks each toolbar button enabled or disabled, to match what its action could currently do.
    ///
    /// A disabled button is dimmed and carries `aria-disabled`. It stays focusable, so keyboard focus is never lost
    /// from under someone who has just pressed it into its own limit.
    ///
    /// Each button remembers whether it was last drawn enabled, and writes its attributes only when that changes. A pan, or
    /// a zoom that leaves every button as it was, therefore neither writes to the DOM nor reads from it: reading an
    /// attribute back to compare it costs a call across the WASM and JavaScript boundary and a `String`, on every
    /// animation frame.
    fn sync_toolbar_state(&self) -> Result<(), Error> {
        let Some(toolbar) = &self.toolbar else { return Ok(()) };
        for button in &toolbar.buttons {
            let enabled = button.action.is_enabled(self.view);
            if button.enabled.get() == Some(enabled) {
                continue;
            }
            button.group.set_attr("aria-disabled", if enabled { "false" } else { "true" })?;
            button.group.set_attr("opacity", if enabled { "1" } else { DISABLED_OPACITY })?;
            // Only once both writes have succeeded, so a failure is tried again on the next frame.
            button.enabled.set(Some(enabled));
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Positions the bar and its buttons for the current visible area.
    pub(super) fn layout_toolbar(&mut self) -> Result<(), Error> {
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
    /// By default, showing the toolbar also switches on dragging empty background to pan the content, and holding Ctrl or
    /// Cmd while turning the mouse wheel to zoom about the pointer. Hiding the toolbar switches them off again.
    ///
    /// Those two gestures are not part of the toolbar and can be set independently of it — see
    /// [`set_pan_mode`](Self::set_pan_mode) and [`set_wheel_zoom_mode`](Self::set_wheel_zoom_mode). An application that
    /// supplies its own controls can hide this toolbar and keep both.
    ///
    /// The bar holds three buttons — zoom in, zoom out, and reset. It stays a fixed size however far the content is
    /// zoomed, and draws on top of it. Every button is keyboard operable: Tab to reach one, Enter or Space to activate.
    ///
    /// # Keeping the layout current
    ///
    /// **The scene cannot observe its `<svg>` being resized.** The bar is positioned against the `<svg>`'s visible area
    /// as it is at the moment of this call, and stays there until told otherwise. Nothing reports an error when it
    /// goes stale, so it is easy to miss. Call [`refresh_layout`](Self::refresh_layout) whenever the visible area
    /// changes.
    ///
    /// Whether it does depends on how the `<svg>` is sized:
    ///
    /// | The `<svg>` | When its size changes | Call `refresh_layout`? |
    /// |---|---|---|
    /// | has a `viewBox`, and its CSS size changes but not its shape | The browser scales the whole `<svg>`, toolbar included. | No |
    /// | has a `viewBox`, and its CSS size changes *shape* | The visible area changes (see below). The bar keeps its old place. | **Yes** |
    /// | has a `viewBox`, and the `viewBox` itself changes | The bar keeps its old place. | **Yes** |
    /// | has no `viewBox`, and its size changes | The bar keeps its old place. | **Yes** |
    ///
    /// So a responsive page whose `<svg>` keeps its aspect ratio, with a `viewBox`, needs no refresh as its CSS size
    /// changes.
    ///
    /// # The visible area
    ///
    /// The bar is placed against the part of the `<svg>`'s user space that is actually on screen, which is not always
    /// the `viewBox`. A `viewBox` origin other than `(0, 0)`, such as `-500 -300 1000 600`, is honoured. So is
    /// `preserveAspectRatio`: with the default `meet`, an `<svg>` shaped differently from its `viewBox` shows *more*
    /// than the `viewBox`, and with `slice` it shows *less*, so the bar follows the visible edge rather than
    /// disappearing off-screen. The centre that [`zoom_in`](Self::zoom_in) and [`zoom_out`](Self::zoom_out) zoom about
    /// is the centre of that same area.
    ///
    /// A stale layout is not only cosmetic. The surface that panning and wheel zoom work through is sized the same way,
    /// so if the `<svg>` grows and the layout is not refreshed, the new area has no surface behind it and those
    /// gestures stop working there.
    ///
    /// Without a `viewBox`, an `<svg>` sized purely by CSS is measured by its rendered size, at each call that lays it
    /// out.
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
        let group = inner.svg.group()?;
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
                return Err(err);
            },
        };

        inner.toolbar = Some(Toolbar { group, options, buttons });
        let laid_out = inner.layout_toolbar().and_then(|()| inner.sync_toolbar_state());
        if let Err(err) = laid_out {
            if let Some(toolbar) = inner.toolbar.take() {
                toolbar.remove();
            }
            return Err(err);
        }
        drop(inner);

        // Showing a toolbar switches on whichever gestures are set to follow it. If that fails, no toolbar is shown.
        if let Err(err) = self.sync_view_input() {
            self.hide_toolbar();
            return Err(err);
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Removes the toolbar from the DOM entirely. Does nothing if none is shown.
    ///
    /// The scene's current zoom is kept. Use [`reset_view`](Self::reset_view) to undo it.
    ///
    /// Also switches off whichever of panning and wheel zoom are set to follow the toolbar. A gesture forced
    /// [`On`](crate::scene::InputMode::On) stays active. See [`set_pan_mode`](Self::set_pan_mode) and
    /// [`set_wheel_zoom_mode`](Self::set_wheel_zoom_mode).
    pub fn hide_toolbar(&self) {
        let Some(toolbar) = self.inner.borrow_mut().toolbar.take() else { return };
        toolbar.remove();
        // This method cannot report an error. Switching gestures off only removes things, and the one case that
        // rebuilds — another gesture forced on while this one was following the toolbar — leaves it unavailable if
        // the DOM refuses, without disturbing anything else.
        let _ = self.sync_view_input();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether a toolbar is currently shown.
    pub fn has_toolbar(&self) -> bool {
        self.inner.borrow().toolbar.is_some()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves the shown toolbar to `edge`. Does nothing if none is shown.
    ///
    /// The bar is placed against the `<svg>`'s visible area as it is at this call, so this also picks up a change the
    /// bar has not yet been told about. It does not resize the surface that panning and wheel zoom work through. If the
    /// `<svg>` has been resized, call [`refresh_layout`](Self::refresh_layout), which does both. See "Keeping the layout
    /// current" under [`show_toolbar`](Self::show_toolbar).
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
    /// **The scene cannot observe its `<svg>` being resized, so call this — or better, [`refresh_layout`](
    /// Self::refresh_layout) — whenever the size or `viewBox` changes,** unless the `<svg>` has a `viewBox` and only its
    /// CSS size changed. Nothing reports an error when the layout goes stale. See "Keeping the layout current" under
    /// [`show_toolbar`](Self::show_toolbar) for exactly when it is needed.
    ///
    /// Also resizes the surface that panning and wheel zoom work through, so this is the same call as
    /// [`refresh_layout`](Self::refresh_layout), which describes it better now that those gestures no longer depend on
    /// the toolbar.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if repositioning fails.
    pub fn refresh_toolbar_layout(&self) -> Result<(), Error> {
        self.refresh_layout()
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
