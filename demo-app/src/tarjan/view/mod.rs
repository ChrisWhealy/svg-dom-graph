//! Where the source view is, and when and how the page moves it. Pure: a view is a number for the zoom and two for the
//! pan, so every rule here is worked out and tested without a browser.
//!
//! The page opens with the whole picture in view, as large as it can be. It never moves the view for a slider seek or a
//! restart. It moves it when the attention vertex is near the edge or off the screen, when a deliberate jump lands on a
//! component that has closed, and when a person asks to zoom to the vertex. A person's own pan or zoom always wins, and
//! turns following off.

use super::layout::Layout;

/// The scale that zooming to a vertex uses, at which a label is easy to read.
pub(crate) const MAX_FIT: f64 = 1.0;

/// The room that the opening view leaves round the picture, in screen pixels, and the inset that the safe rectangle
/// keeps.
pub(crate) const FIT_MARGIN: f64 = 24.0;
pub(crate) const SAFE_INSET: f64 = 0.15;

/// How far the left of the picture is from the left of the viewport when the page opens, in screen pixels.
pub(crate) const LEFT_MARGIN: f64 = 6.0;

/// How far a view is zoomed, and where it is panned to: a scene point `(x, y)` is on the screen at `(scale * x + tx,
/// scale * y + ty)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Camera {
    pub(crate) scale: f64,
    pub(crate) tx: f64,
    pub(crate) ty: f64,
}

impl Camera {
    /// Whether two views are the same, to within the rounding that comes of adding the strip of controls to the pan and
    /// taking it off again.
    pub(crate) fn same_as(&self, other: &Camera) -> bool {
        (self.scale - other.scale).abs() < 1e-9
            && (self.tx - other.tx).abs() < 1e-6
            && (self.ty - other.ty).abs() < 1e-6
    }

    /// Where a scene point is on the screen.
    pub(crate) fn to_screen(self, p: (f64, f64)) -> (f64, f64) {
        (self.scale * p.0 + self.tx, self.scale * p.1 + self.ty)
    }

    /// The camera at the same scale that has `p` in the middle of a viewport of this size.
    pub(crate) fn centred_on(&self, p: (f64, f64), viewport: (f64, f64)) -> Self {
        Self {
            scale: self.scale,
            tx: viewport.0 / 2.0 - self.scale * p.0,
            ty: viewport.1 / 2.0 - self.scale * p.1,
        }
    }
}

/// How well a point can be seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Visibility {
    /// Inside the safe rectangle, with its edges counting as inside.
    Inside,
    /// In the viewport but outside the safe rectangle.
    NearEdge,
    /// Outside the viewport.
    OffScreen,
}

/// The safe rectangle of a viewport, which is inset on every side, in screen pixels: the left, top, right and bottom.
pub(crate) fn safe_rect(viewport: (f64, f64)) -> (f64, f64, f64, f64) {
    (
        viewport.0 * SAFE_INSET,
        viewport.1 * SAFE_INSET,
        viewport.0 * (1.0 - SAFE_INSET),
        viewport.1 * (1.0 - SAFE_INSET),
    )
}

/// Whether the whole picture is in the viewport, in which case no pan can show a vertex any better.
fn whole_picture_is_in_view(camera: &Camera, viewport: (f64, f64), layout: &Layout) -> bool {
    let (left, top) = camera.to_screen((0.0, 0.0));
    let (right, bottom) = camera.to_screen(layout.size);
    left >= -1e-6 && top >= -1e-6 && right <= viewport.0 + 1e-6 && bottom <= viewport.1 + 1e-6
}

/// How well the centre of vertex `v` can be seen. While the whole picture is in the viewport every vertex is as well
/// placed as it can be, so none is near the edge.
pub(crate) fn visibility(camera: &Camera, viewport: (f64, f64), layout: &Layout, v: usize) -> Visibility {
    let (x, y) = camera.to_screen(layout.centre(v));
    let (left, top, right, bottom) = safe_rect(viewport);
    let eps = 1e-6;
    if whole_picture_is_in_view(camera, viewport, layout)
        || x >= left - eps && x <= right + eps && y >= top - eps && y <= bottom + eps
    {
        Visibility::Inside
    } else if x >= 0.0 && x <= viewport.0 && y >= 0.0 && y <= viewport.1 {
        Visibility::NearEdge
    } else {
        Visibility::OffScreen
    }
}

/// The smallest scale that the library allows, which a view that shows the whole picture is not smaller than.
pub(crate) const MIN_SCALE: f64 = 0.25;

/// The view that the page opens with: the whole picture, as large as it can be and no larger than a scale of 1, with
/// its left edge at the left of the viewport and its middle at the middle of the height. A picture too large for the
/// library's smallest scale is shown at that scale.
pub(crate) fn initial(layout: &Layout, viewport: (f64, f64)) -> Camera {
    let wanted = ((viewport.0 - 2.0 * FIT_MARGIN) / layout.size.0).min((viewport.1 - 2.0 * FIT_MARGIN) / layout.size.1);
    let scale = wanted.clamp(MIN_SCALE, MAX_FIT);
    let centred = Camera { scale, tx: 0.0, ty: 0.0 }.centred_on((layout.size.0 / 2.0, layout.size.1 / 2.0), viewport);
    Camera { tx: LEFT_MARGIN, ..centred }
}

/// What the page does to the view for one transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Plan {
    pub(crate) camera: Camera,
}

/// The smallest pan that brings the centre of `v` back inside the safe rectangle.
fn minimal_pan(camera: &Camera, viewport: (f64, f64), layout: &Layout, v: usize) -> Camera {
    let (x, y) = camera.to_screen(layout.centre(v));
    let (left, top, right, bottom) = safe_rect(viewport);
    let shift = |p: f64, lo: f64, hi: f64| {
        if p < lo {
            lo - p
        } else if p > hi {
            hi - p
        } else {
            0.0
        }
    };
    Camera {
        scale: camera.scale,
        tx: camera.tx + shift(x, left, right),
        ty: camera.ty + shift(y, top, bottom),
    }
}

/// What following does for a Play tick or a single Next or Previous, when the attention vertex is `v`. A vertex that is
/// inside the safe rectangle needs nothing. One that is near the edge is brought back by the smallest pan, and one that
/// is off the screen is put in the middle. The zoom is never changed.
pub(crate) fn follow(camera: &Camera, viewport: (f64, f64), layout: &Layout, v: Option<usize>) -> Option<Plan> {
    let v = v?;
    let camera = match visibility(camera, viewport, layout, v) {
        Visibility::Inside => return None,
        Visibility::NearEdge => minimal_pan(camera, viewport, layout, v),
        Visibility::OffScreen => camera.centred_on(layout.centre(v), viewport),
    };
    Some(Plan { camera })
}

/// What a deliberate jump to a component that has closed does: the middle of its vertices is put in the middle of the
/// viewport, whatever the view was, and the zoom is kept.
pub(crate) fn destination(camera: &Camera, viewport: (f64, f64), layout: &Layout, members: &[usize]) -> Option<Plan> {
    let bounds = layout.bounds_of(members, 0.0)?;
    Some(Plan {
        camera: camera.centred_on(bounds.centre(), viewport),
    })
}

/// Zooming to a vertex: the scale at which a label is easy to read, with the vertex in the middle.
pub(crate) fn zoom_to(viewport: (f64, f64), layout: &Layout, v: usize) -> Plan {
    Plan {
        camera: Camera {
            scale: MAX_FIT,
            tx: 0.0,
            ty: 0.0,
        }
        .centred_on(layout.centre(v), viewport),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the controller remembers about the view, so that it can tell its own changes from a person's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Memory {
    /// The view as the controller last wrote it, or read it when nothing had been written.
    pub(crate) acknowledged: Camera,
    pub(crate) follow: bool,
}

impl Memory {
    pub(crate) fn new(camera: Camera) -> Self {
        Self {
            acknowledged: camera,
            follow: true,
        }
    }

    /// Compares the view that is on the page with the one that was last written. If a person has changed it, the new
    /// view is taken as the baseline, the centred region is forgotten and following is turned off. Says whether that
    /// happened, and whether it turned following off.
    pub(crate) fn reconcile(&mut self, actual: Camera) -> Reconciled {
        if actual.same_as(&self.acknowledged) {
            return Reconciled::Unchanged;
        }
        self.acknowledged = actual;
        if std::mem::replace(&mut self.follow, false) {
            Reconciled::FollowTurnedOff
        } else {
            Reconciled::Changed
        }
    }

    /// Records a view that the controller wrote and the page committed.
    pub(crate) fn committed(&mut self, plan: Plan) {
        self.acknowledged = plan.camera;
    }
}

/// What reconciling found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reconciled {
    Unchanged,
    /// A person changed the view, and following was already off.
    Changed,
    /// A person changed the view, and that turned following off.
    FollowTurnedOff,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
