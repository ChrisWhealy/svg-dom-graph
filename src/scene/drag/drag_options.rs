use crate::scene::{CollisionPolicy, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures the pointer-drag behaviour [Scene::make_draggable_with](crate::scene::Scene::make_draggable_with) wires
/// up for one node.
///
/// `#[non_exhaustive]` is used here for the same reason as [`CollisionPolicy`]. This type is expected to grow. Further
/// drag configuration — snapping, axis restriction — is likely to follow `bounds`.
///
/// Build one either with [`DragOptions::default`] or with [`with_collision`](Self::with_collision) and
/// [`with_bounds`](Self::with_bounds). A struct literal does not compile outside this crate.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, not an oversight. Removing `Copy` later is a breaking
/// change, so every field this type gains must also implement `Copy`.
///
/// Plain enums, numbers, points, and rectangles all stay `Copy`, so this commitment is expected to hold. A future field
/// needing a closure, an owned collection, or a user-defined strategy object would force `Copy` to be dropped, and that
/// would be a breaking release.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct DragOptions {
    /// What happens when a drop leaves the dragged node overlapping another one — see [`CollisionPolicy`].
    pub collision: CollisionPolicy,
    /// Confines the dragged node's own top-left corner so the whole box stays inside `bounds`, for as long as this node
    /// stays draggable.
    ///
    /// `None` (the default) leaves dragging unconstrained: a node can be dropped anywhere, including outside its own
    /// `<svg>`'s visible area. Once dropped there, it stays rendered but clipped, so it can no longer be clicked to
    /// pick it up again.
    ///
    /// `Some(bounds)` clamps every drag move, and any collision-resolution push, to stay inside `bounds`. A node larger
    /// than `bounds` on some axis pins to `bounds`'s own near edge on that axis instead — see [`crate::geometry`]'s own
    /// `clamp_to_bounds` for the exact rule.
    ///
    /// `Scene::make_draggable_with` rejects a `Some(bounds)` whose origin or size is not finite, or whose width or
    /// height is negative, with [Error::InvalidDragBounds](crate::error::Error::InvalidDragBounds) — see that method's
    /// own `# Errors` section. A zero width or height is accepted: `clamp_to_bounds` already gives that a
    /// deterministic result.
    pub bounds: Option<Rect>,
}

impl DragOptions {
    /// Returns `self` with `collision` set to `collision`.
    ///
    /// ```
    /// use svg_dom_graph::scene::{CollisionPolicy, DragOptions};
    /// let options = DragOptions::default().with_collision(CollisionPolicy::Allow);
    /// assert_eq!(options.collision, CollisionPolicy::Allow);
    /// ```
    #[must_use]
    pub fn with_collision(mut self, collision: CollisionPolicy) -> Self {
        self.collision = collision;
        self
    }

    /// Returns `self` with `bounds` set to `bounds`.
    ///
    /// ```
    /// use svg_dom::root::utils::{Point, Rect, Size};
    /// use svg_dom_graph::scene::DragOptions;
    /// let bounds = Rect { origin: Point::new(0.0, 0.0), size: Size::new(400.0, 300.0) };
    /// let options = DragOptions::default().with_bounds(Some(bounds));
    /// assert_eq!(options.bounds, Some(bounds));
    /// ```
    #[must_use]
    pub fn with_bounds(mut self, bounds: Option<Rect>) -> Self {
        self.bounds = bounds;
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Default for DragOptions {
    /// [`CollisionPolicy::PushClear`] with 6 user-space units of padding, and no `bounds` — unconstrained dragging,
    /// [Scene::make_draggable](crate::scene::Scene::make_draggable)'s own behaviour.
    fn default() -> Self {
        Self {
            collision: CollisionPolicy::PushClear { padding: 6.0 },
            bounds: None,
        }
    }
}
