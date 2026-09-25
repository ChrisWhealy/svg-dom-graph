//! Mouse and trackpad control of the scene's view: dragging the background to pan, and Ctrl or Cmd plus the wheel to
//! zoom.
//!
//! Both work through one transparent surface placed directly beneath the content layer. Nodes and connectors draw on
//! top of it and take their own pointer events, so only empty background reaches it.
//!
//! Each gesture has its own [`InputMode`], independent of the others and of the toolbar. By default each follows the
//! toolbar: on while one is shown, off otherwise. An application that supplies its own controls can instead force
//! either gesture on, or force it off even with the stock toolbar showing.

mod frame;
mod keyboard;
mod pan;
mod wheel;

use super::{Scene, SceneInner};
use crate::error::Error;
use frame::ViewFlusher;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use svg_dom::{SvgNode, SvgRoot, root::utils::Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// When a mouse or trackpad gesture is active. See [`Scene::set_pan_mode`] and [`Scene::set_wheel_zoom_mode`].
///
/// Deriving `Copy` is a deliberate compatibility commitment, the same as [`DragOptions`](crate::scene::DragOptions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Active while a toolbar is shown, and inactive otherwise. This is the default.
    #[default]
    WithToolbar,
    /// Always active, whether or not a toolbar is shown.
    On,
    /// Never active, even while a toolbar is shown.
    Off,
}

impl InputMode {
    fn is_active(self, toolbar_shown: bool) -> bool {
        match self {
            Self::WithToolbar => toolbar_shown,
            Self::On => true,
            Self::Off => false,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The surface behind the content layer, and which gestures are currently wired onto it.
///
/// Exists only while at least one gesture is active. Lives in [`SceneInner::view_input`].
pub(super) struct ViewInput {
    surface: SvgNode,
    /// The content layer, kept so removing this can also remove the wheel listener registered on it — see [`wheel`].
    content: SvgNode,
    /// The `<svg>` root, kept so removing this can take the keyboard handling off it again — see [`keyboard`].
    root: SvgNode,
    pan: bool,
    wheel: bool,
}

impl ViewInput {
    /// Removes the surface, and with it every listener registered on it, from the DOM.
    fn remove(self) {
        self.surface.remove();
        if self.wheel {
            self.content.remove_listeners("wheel");
        }
        keyboard::remove(&self.root);
    }

    /// Makes the surface cover `area`.
    fn place(&self, area: Rect, scratch: &mut String) -> Result<(), Error> {
        for (name, value) in [
            ("x", area.origin.x),
            ("y", area.origin.y),
            ("width", area.size.width),
            ("height", area.size.height),
        ] {
            self.surface.set_attr_display(scratch, name, value)?;
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates the surface for `area`, places it directly beneath `content`, and wires the requested gestures onto it. The
/// `<svg>` itself becomes a keyboard target too, so the same gestures can be reached without a pointer.
///
/// `scale` is the current zoom, for the `<svg>`'s accessible name.
///
/// Returns `Ok(None)` if `content` is somehow detached from the `<svg>`. A surface left on top of everything would
/// swallow every node's pointer events, so it is removed instead and the gestures are simply unavailable.
///
/// Removes everything it created if anything fails.
fn build(
    svg: &SvgRoot,
    content: &SvgNode,
    inner: &Weak<RefCell<SceneInner>>,
    pan: bool,
    wheel: bool,
    area: Rect,
    scale: f64,
) -> Result<Option<ViewInput>, Error> {
    let surface = svg.rect(area.origin, area.size)?;
    // Set just before the keyboard handling is installed, so a failure earlier on never removes attributes this did not
    // set — the `<svg>` may carry a `tabindex` or a `role` of the application's own.
    let keyboard_attempted = Cell::new(false);

    let wired = (|| {
        // `transparent`, not `none`: an SVG shape only receives pointer events where it is painted.
        surface.set_fill("transparent")?;
        surface.set_attr("aria-hidden", "true")?;
        let Some(root) = content.parent() else { return Ok::<_, Error>(None) };
        root.insert_before(&surface, content)?;

        // One flusher serves both gestures, so a pan and a wheel zoom in the same frame still cost a single DOM write.
        let flusher = ViewFlusher::new(inner.clone())?;
        if pan {
            pan::install(&surface, inner, &flusher)?;
        }
        if wheel {
            wheel::install(content, &surface, inner, &flusher)?;
        }
        keyboard_attempted.set(true);
        keyboard::install(&root, inner, pan, wheel, scale)?;
        Ok(Some(root))
    })();

    match wired {
        Ok(Some(root)) => Ok(Some(ViewInput {
            surface,
            content: content.clone(),
            root,
            pan,
            wheel,
        })),
        Ok(None) => {
            surface.remove();
            Ok(None)
        },
        Err(err) => {
            surface.remove();
            if wheel {
                content.remove_listeners("wheel");
            }
            if keyboard_attempted.get() {
                if let Some(root) = content.parent() {
                    keyboard::remove(&root);
                }
            }
            Err(err)
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl SceneInner {
    /// Brings the `<svg>`'s accessible name in line with the current zoom, so it reads "Graph view, zoom 125%".
    ///
    /// Does nothing if there is no keyboard handling, and writes only if the text changes. It is a name, not a live
    /// region, so it never interrupts a screen reader with an announcement on every zoom step.
    ///
    /// Runs on every flushed frame of a pan or zoom, so it builds the text in the scene's reused scratch buffer rather
    /// than allocating a new `String` each time.
    pub(super) fn sync_view_label(&mut self) -> Result<(), Error> {
        if self.view_input.is_none() {
            return Ok(());
        }

        let mut scratch = std::mem::take(&mut self.scratch);
        keyboard::write_label(self.view.scale, &mut scratch);
        let result = self
            .view_input
            .as_ref()
            .map_or(Ok(()), |input| input.root.set_attr_if_changed("aria-label", &scratch));
        self.scratch = scratch;
        Ok(result?)
    }

    /// Whether panning is active right now, given its mode and whether a toolbar is shown.
    pub(super) fn pan_active(&self) -> bool {
        self.pan_mode.is_active(self.toolbar.is_some())
    }

    /// Whether wheel zoom is active right now, given its mode and whether a toolbar is shown.
    pub(super) fn wheel_zoom_active(&self) -> bool {
        self.wheel_zoom_mode.is_active(self.toolbar.is_some())
    }

    /// Makes the surface cover the visible area as it is now. Does nothing if there is no surface.
    pub(super) fn resize_view_input(&mut self) -> Result<(), Error> {
        let area = self.visible_area();
        let mut scratch = std::mem::take(&mut self.scratch);
        let result = self.view_input.as_ref().map_or(Ok(()), |input| input.place(area, &mut scratch));
        self.scratch = scratch;
        result
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Sets when dragging empty background pans the content. The default is [`InputMode::WithToolbar`].
    ///
    /// Panning lets content zoomed past the edge of the visible area always be brought back. Nodes and connectors take
    /// their own pointer events, so dragging one still drags only that node.
    ///
    /// # Keyboard
    ///
    /// While panning is active the `<svg>` itself also joins the Tab order, so a keyboard user can pan too: the arrow keys
    /// move the view like scrolling, 40 units a press, and Shift moves it five times further. Otherwise zooming in from
    /// the toolbar could leave content that a keyboard user cannot get back to. The `<svg>` is given the `application`
    /// role, and its accessible name reports the current zoom, such as "Graph view, zoom 125%".
    ///
    /// This sets `tabindex`, `role`, `aria-label`, and `aria-description` on the `<svg>`, and removes them when no gesture
    /// needs them. Only a key pressed while the `<svg>` itself has focus is handled, and Ctrl, Cmd, and Alt combinations
    /// are left alone.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if the DOM cannot be updated, in which case nothing is left behind.
    pub fn set_pan_mode(&self, mode: InputMode) -> Result<(), Error> {
        self.inner.borrow_mut().pan_mode = mode;
        self.sync_view_input()
    }

    /// Sets when Ctrl or Cmd plus the mouse wheel zooms the content about the pointer. The default is
    /// [`InputMode::WithToolbar`].
    ///
    /// Cmd is the Mac convention and Ctrl is the Windows and Linux one. Browsers report a trackpad pinch as ctrl+wheel,
    /// so pinch-to-zoom works too. While active, a wheel event with either modifier is cancelled, so the browser does
    /// not also zoom the page. A wheel without a modifier is left alone, so the page still scrolls.
    ///
    /// The keyboard equivalent is `+` (or `=`), `-`, and `0`, which zoom in, zoom out, and restore the original zoom
    /// while the `<svg>` has focus. See [`set_pan_mode`](Self::set_pan_mode) for how the `<svg>` becomes a keyboard target.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if the DOM cannot be updated, in which case nothing is left behind.
    pub fn set_wheel_zoom_mode(&self, mode: InputMode) -> Result<(), Error> {
        self.inner.borrow_mut().wheel_zoom_mode = mode;
        self.sync_view_input()
    }

    /// The pan mode last set. This is the setting, not whether panning is active right now — see
    /// [`pan_enabled`](Self::pan_enabled).
    pub fn pan_mode(&self) -> InputMode {
        self.inner.borrow().pan_mode
    }

    /// The wheel-zoom mode last set. This is the setting, not whether wheel zoom is active right now — see
    /// [`wheel_zoom_enabled`](Self::wheel_zoom_enabled).
    pub fn wheel_zoom_mode(&self) -> InputMode {
        self.inner.borrow().wheel_zoom_mode
    }

    /// Whether dragging empty background pans the content right now: its mode, combined with whether a toolbar is
    /// shown.
    pub fn pan_enabled(&self) -> bool {
        self.inner.borrow().pan_active()
    }

    /// Whether Ctrl or Cmd plus the wheel zooms the content right now: its mode, combined with whether a toolbar is
    /// shown.
    pub fn wheel_zoom_enabled(&self) -> bool {
        self.inner.borrow().wheel_zoom_active()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Brings the wired gestures in line with what is now wanted: adds or removes the surface, and the listeners on it,
    /// as the two modes and the toolbar's visibility require.
    ///
    /// Called whenever either mode changes, or a toolbar is shown or hidden. Does nothing if what is wired already
    /// matches. Otherwise it rebuilds the surface from scratch, so a gesture in progress at that moment is dropped.
    pub(super) fn sync_view_input(&self) -> Result<(), Error> {
        let weak = Rc::downgrade(&self.inner);
        let mut inner = self.inner.borrow_mut();
        let (pan, wheel) = (inner.pan_active(), inner.wheel_zoom_active());

        let wired = inner.view_input.as_ref().map(|input| (input.pan, input.wheel));
        if wired == Some((pan, wheel)) || (wired.is_none() && !pan && !wheel) {
            return Ok(());
        }
        if let Some(old) = inner.view_input.take() {
            // A wheel or pan event may have changed the view only moments ago, leaving its write to the next animation
            // frame. That frame is about to be cancelled along with the handling that asked for it, so write the view
            // now. Otherwise what is drawn and what `zoom_scale()` reports would disagree.
            let _ = inner.flush_view();
            old.remove();
        }
        if !pan && !wheel {
            return Ok(());
        }

        let area = inner.visible_area();
        let scale = inner.view.scale;
        inner.view_input = build(&inner.svg, &inner.content, &weak, pan, wheel, area, scale)?;
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Repositions the toolbar and resizes the pan and wheel-zoom surface for the `<svg>`'s visible area as it is now.
    ///
    /// **The scene cannot observe its `<svg>` being resized, so call this whenever the size or `viewBox` changes,**
    /// for example after `SvgRoot::set_viewport` or `SvgRoot::set_view_box`, or from a `resize` handler. The one
    /// exception is an `<svg>` with a `viewBox` whose CSS size changes without changing its shape, since the browser
    /// then scales everything together. See "Keeping the layout current" under [`show_toolbar`](Self::show_toolbar) for the full picture.
    ///
    /// A stale layout is not only cosmetic. A toolbar is left where it was, and the surface that panning and wheel zoom
    /// work through no longer covers a `<svg>` that has grown, so those gestures stop working in the new area.
    ///
    /// Each part is skipped if there is nothing to update, so it is always safe to call.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if updating fails.
    pub fn refresh_layout(&self) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();
        inner.layout_toolbar()?;
        inner.resize_view_input()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
