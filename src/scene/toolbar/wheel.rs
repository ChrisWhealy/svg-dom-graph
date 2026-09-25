//! Holding a modifier key and turning the mouse wheel zooms about the pointer.
//!
//! The modifier is Ctrl or Cmd (Meta), whichever the platform uses. Cmd is the Mac convention. Ctrl is the Windows and
//! Linux one, and browsers also report a trackpad pinch gesture as a stream of small ctrl+wheel events, so honouring it
//! gives pinch-to-zoom for free. Accepting either key on every platform costs nothing and avoids sniffing which one
//! this is.
//!
//! Without a modifier the wheel is left alone, so the page scrolls as usual.

use crate::{
    error::Error,
    geometry::{invert_matrix, view::wheel_zoom_factor},
    scene::{SceneInner, client_to_user_space},
};
use std::{cell::RefCell, rc::Weak};
use svg_dom::{SvgNode, WeakSvgNode, root::utils::Point};
use web_sys::WheelEvent;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Listens for modifier-plus-wheel over both the pan `surface` and the `content` layer, zooming about the pointer.
///
/// Both are needed. Nodes and connectors draw on top of the surface, so a wheel over one reaches `content` and never
/// the surface. Between them they cover the whole visible area, and the toolbar's own buttons, which sit above both,
/// are left out.
///
/// The listeners hold only `Weak` references, for the same reason [`Scene::make_draggable_with`](
/// crate::scene::Scene::make_draggable_with)'s do. Remove the `content` listener with
/// `content.remove_listeners("wheel")`. The surface's own listener goes when the surface does.
pub(super) fn install(content: &SvgNode, surface: &SvgNode, inner: &Weak<RefCell<SceneInner>>) -> Result<(), Error> {
    for target in [content, surface] {
        let surface = surface.downgrade();
        let inner = inner.clone();
        // Not passive: `prevent_default` is what stops ctrl+wheel zooming the whole page.
        target.on_wheel(move |event| zoom_on_wheel(&surface, &inner, &event))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn zoom_on_wheel(surface: &WeakSvgNode, inner: &Weak<RefCell<SceneInner>>, event: &WheelEvent) {
    if !(event.ctrl_key() || event.meta_key()) {
        return;
    }
    // Claims the event even if the zoom below cannot be applied, so a modified wheel over the scene never zooms the
    // page instead.
    event.prevent_default();

    let Some(surface) = surface.upgrade() else { return };
    let Some(inner) = inner.upgrade() else { return };
    // The surface never moves or scales, so its own screen matrix converts client pixels to the `<svg>`'s user space
    // however far the content is zoomed or panned.
    let Some(inverse_ctm) = surface.screen_ctm().and_then(invert_matrix) else {
        return;
    };
    let client = Point::new(event.client_x() as f64, event.client_y() as f64);
    let pivot = client_to_user_space(client, inverse_ctm);

    let factor = wheel_zoom_factor(event.delta_y(), event.delta_mode());
    let mut inner = inner.borrow_mut();
    let next = inner.view.zoomed_about(factor, pivot);
    // A listener has nowhere to report an error to, and a failed DOM write leaves the previous view in place.
    let _ = inner.set_view(next);
}
