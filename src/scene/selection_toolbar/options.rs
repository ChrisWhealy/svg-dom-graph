use crate::geometry::side::Side;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a [`super::Scene`]'s selection toolbar is drawn and where it sits. See
/// [`super::Scene::show_selection_toolbar`].
///
/// All lengths are in the `<svg>`'s own user space — the same units as its `viewBox`, or as pixels when it has none.
/// They never scale with the scene's zoom.
///
/// Deriving `Copy` is a deliberate compatibility commitment, the same as [`ToolbarOptions`](crate::scene::ToolbarOptions).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct SelectionToolbarOptions {
    /// The edge the bar is fixed to. Default: [`Side::South`].
    pub edge: Side,
    /// The height of every button. Must be a finite value `> 0.0`. Default: `28.0`.
    pub button_height: f64,
    /// The space between adjacent buttons. Must be a finite value `>= 0.0`. Default: `4.0`.
    pub gap: f64,
    /// The space between the bar and the edge it is fixed to. Must be a finite value `>= 0.0`. Default: `8.0`.
    pub margin: f64,
}

impl SelectionToolbarOptions {
    /// Default options, fixed to `edge`.
    pub fn new(edge: Side) -> Self {
        Self { edge, ..Self::default() }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether every length is one [`Scene::show_selection_toolbar`](crate::scene::Scene::show_selection_toolbar)
    /// accepts: `button_height` a finite value `> 0.0`, and `gap` and `margin` finite values `>= 0.0`.
    ///
    /// Crate-private, since `show_selection_toolbar` is the one place it is needed and it reports a failure as an
    /// error.
    pub(crate) fn is_valid(&self) -> bool {
        self.button_height.is_finite()
            && self.button_height > 0.0
            && self.gap.is_finite()
            && self.gap >= 0.0
            && self.margin.is_finite()
            && self.margin >= 0.0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Default for SelectionToolbarOptions {
    fn default() -> Self {
        Self {
            edge: Side::South,
            button_height: 28.0,
            gap: 4.0,
            margin: 8.0,
        }
    }
}
