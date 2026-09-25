//! Dragging the scene's background pans its content.
//!
//! Zooming in pushes content past the edge of the visible area. Without a way to move it back, that content would be
//! unreachable. So while a toolbar is shown, the scene also holds a transparent surface behind the content layer.
//! Dragging that surface — which is to say, dragging any empty background, since nodes and connectors draw on top of it
//! and take their own pointer events — translates the content layer.

use super::{frame::ViewFlusher, wheel};
use crate::{
    error::Error,
    geometry::{invert_matrix, view::ViewTransform},
    scene::{SceneInner, client_to_user_space},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use svg_dom::{
    SvgNode, SvgRoot, WeakSvgNode,
    root::utils::{Matrix2D, Point, Size},
};

/// The surface's own style while idle: a grab cursor, and no touch scrolling of the page so a touch drag pans instead.
const IDLE_STYLE: &str = "cursor: grab; touch-action: none; user-select: none; -webkit-user-select: none;";

/// The surface's own style while a pan is in progress.
const PANNING_STYLE: &str = "cursor: grabbing; touch-action: none; user-select: none; -webkit-user-select: none;";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a pan started. Kept for as long as the pan lasts.
#[derive(Clone, Copy)]
struct PanStart {
    pointer_id: i32,
    /// The pointer's position when the pan began, in the `<svg>`'s user space.
    pointer: Point,
    /// The content layer's transform when the pan began. Every move is applied to this, not to the latest view, so the
    /// content follows the pointer exactly however many moves arrive.
    view: ViewTransform,
    /// Converts client pixels to the `<svg>`'s user space. The surface never moves, so it stays valid all pan long.
    inverse_ctm: Matrix2D,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates the transparent pan surface, places it directly beneath `content`, and wires its pointer listeners.
///
/// The surface's size is left for the toolbar's own layout to write. Removes it again if anything fails.
///
/// The listeners hold only a `Weak` reference to the scene's shared state, for the same reason
/// [`Scene::make_draggable_with`](crate::scene::Scene::make_draggable_with)'s do.
pub(super) fn build_pan_surface(
    svg: &SvgRoot,
    content: &SvgNode,
    inner: &Weak<RefCell<SceneInner>>,
) -> Result<SvgNode, Error> {
    let surface = svg.rect(Point::origin(), Size::new(1.0, 1.0))?;
    // One flusher serves both gestures, so a pan and a wheel zoom in the same frame still cost a single DOM write.
    let flusher = ViewFlusher::new(inner.clone())?;
    wire(content, &surface, inner, &flusher).inspect_err(|_| surface.remove())?;
    wheel::install(content, &surface, inner, &flusher).inspect_err(|_| {
        surface.remove();
        content.remove_listeners("wheel");
    })?;
    Ok(surface)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn wire(
    content: &SvgNode,
    surface: &SvgNode,
    inner: &Weak<RefCell<SceneInner>>,
    flusher: &ViewFlusher,
) -> Result<(), Error> {
    // `transparent`, not `none`: an SVG shape only receives pointer events where it is painted.
    surface.set_fill("transparent")?;
    surface.set_attr("aria-hidden", "true")?;
    surface.set_attr("style", IDLE_STYLE)?;
    // `content` is always a child of the `<svg>` root, so this always finds it. Were it ever detached, a surface left
    // on top of everything would swallow every node's pointer events. So it is removed instead, and panning is lost.
    let Some(parent) = content.parent() else {
        surface.remove();
        return Ok(());
    };
    parent.insert_before(surface, content)?;

    let start: Rc<Cell<Option<PanStart>>> = Rc::new(Cell::new(None));

    {
        let start = start.clone();
        let surface_weak = surface.downgrade();
        let inner = inner.clone();
        surface.on_pointerdown(move |evt| {
            if start.get().is_some() || evt.button() != 0 {
                return;
            }
            // Stops the browser starting its own text selection from this pointerdown.
            evt.prevent_default();
            let Some(surface) = surface_weak.upgrade() else { return };
            let Some(inner) = inner.upgrade() else { return };
            let Some(inverse_ctm) = surface.screen_ctm().and_then(invert_matrix) else {
                return;
            };
            let client = Point::new(evt.client_x() as f64, evt.client_y() as f64);

            let _ = surface.as_element().set_pointer_capture(evt.pointer_id());
            let _ = surface.set_attr("style", PANNING_STYLE);
            start.set(Some(PanStart {
                pointer_id: evt.pointer_id(),
                pointer: client_to_user_space(client, inverse_ctm),
                view: inner.borrow().view,
                inverse_ctm,
            }));
        })?;
    }

    {
        let start = start.clone();
        let inner = inner.clone();
        let flusher = flusher.clone();
        surface.on_pointermove(move |evt| {
            let Some(pan) = start.get() else { return };
            if evt.pointer_id() != pan.pointer_id {
                return;
            }
            evt.prevent_default();
            let Some(inner) = inner.upgrade() else { return };
            let client = Point::new(evt.client_x() as f64, evt.client_y() as f64);
            let now = client_to_user_space(client, pan.inverse_ctm);
            // Updates the view now but leaves the DOM write to one animation frame: a pointer can move more often than
            // the browser paints. The pan follows the pointer exactly, since each move is applied to where it started.
            let view = pan.view.translated(now.x - pan.pointer.x, now.y - pan.pointer.y);
            if inner.borrow_mut().set_view_deferred(view) {
                flusher.schedule();
            }
        })?;
    }

    {
        let start = start.clone();
        let surface_weak = surface.downgrade();
        let flusher = flusher.clone();
        surface.on_pointerup(move |evt| end_pan(&start, &surface_weak, &flusher, evt.pointer_id()))?;
    }

    {
        let surface_weak = surface.downgrade();
        let flusher = flusher.clone();
        surface.on_pointercancel(move |evt| end_pan(&start, &surface_weak, &flusher, evt.pointer_id()))?;
    }

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Ends the pan `start` holds, writing its final position to the DOM first, if `pointer_id` is the pointer that began it.
///
/// Ignores any other pointer, such as a second finger lifting while the first is still panning.
fn end_pan(start: &Cell<Option<PanStart>>, surface: &WeakSvgNode, flusher: &ViewFlusher, pointer_id: i32) {
    let Some(pan) = start.get() else { return };
    if pan.pointer_id != pointer_id {
        return;
    }
    start.set(None);
    // Settles the DOM at the pan's final position now, rather than leaving it up to one frame behind.
    flusher.flush_now();
    if let Some(surface) = surface.upgrade() {
        let _ = surface.as_element().release_pointer_capture(pointer_id);
        let _ = surface.set_attr("style", IDLE_STYLE);
    }
}
