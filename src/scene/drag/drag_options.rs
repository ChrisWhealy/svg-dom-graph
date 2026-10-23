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
    /// `None` (the default) keeps the node inside the part of the scene that is visible, as the view is when it is
    /// dragged, so the whole box stays on screen. See [`confine_to_view`](Self::confine_to_view). A node dropped outside
    /// the visible area stays rendered but clipped, and can no longer be clicked to pick it up again, which is what this
    /// default prevents.
    ///
    /// `Some(bounds)` clamps every drag move, and any collision-resolution push, to stay inside `bounds`. A node larger
    /// than `bounds` on some axis pins to `bounds`'s own near edge on that axis instead — see [`crate::geometry`]'s own
    /// `clamp_to_bounds` for the exact rule.
    ///
    /// `Scene::make_draggable_with` rejects a `Some(bounds)` whose origin or size is not finite, or whose width or
    /// height is negative, with [Error::InvalidDragBounds](crate::error::Error::InvalidDragBounds) — see that method's
    /// own `# Errors` section. A zero width or height is accepted: `clamp_to_bounds` already gives that a deterministic
    /// result.
    pub bounds: Option<Rect>,
    /// Whether a node that has no [`bounds`](Self::bounds) of its own is kept inside the visible area of the scene. `true`
    /// by default.
    ///
    /// The visible area is the part of the `<svg>` that the browser shows, found when the drag starts, and carried through
    /// any zoom or pan made during the drag. A box larger than it on some axis pins to its near edge on that axis.
    /// Without this, a scene that has no zoom or pan controls would lose any node dropped past its edge for good.
    ///
    /// `false` leaves a node with no `bounds` free to go anywhere. Only an application that provides another way to bring
    /// a node back should do that. See [`with_confine_to_view`](Self::with_confine_to_view). It has no effect on a node
    /// that has `bounds`, which are always used.
    pub confine_to_view: bool,
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

    /// Returns `self` with `confine_to_view` set to `confine`: whether a node with no [`bounds`](Self::bounds) is kept
    /// inside the scene's visible area. It is `true` by default.
    ///
    /// ```
    /// use svg_dom_graph::scene::DragOptions;
    /// assert!(DragOptions::default().confine_to_view);
    /// assert!(!DragOptions::default().with_confine_to_view(false).confine_to_view);
    /// ```
    #[must_use]
    pub fn with_confine_to_view(mut self, confine: bool) -> Self {
        self.confine_to_view = confine;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
    /// [`CollisionPolicy::PushClear`] with 6 user-space units of padding, and no `bounds`, so a node is kept inside the
    /// scene's visible area, since `confine_to_view` is `true`. This is
    /// [Scene::make_draggable](crate::scene::Scene::make_draggable)'s own behaviour.
    fn default() -> Self {
        Self {
            collision: CollisionPolicy::PushClear { padding: 6.0 },
            bounds: None,
            confine_to_view: true,
        }
    }
}
