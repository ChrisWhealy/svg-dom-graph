use crate::geometry::side::Side;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a [`Scene`]'s toolbar is drawn and where it sits. See [`Scene::show_toolbar`].
///
/// All lengths are in the `<svg>`'s own user space — the same units as its `viewBox`, or as pixels when it has none.
/// They never scale with the scene's zoom.
///
/// Deriving `Copy` is a deliberate compatibility commitment, the same as [`DragOptions`](crate::scene::DragOptions).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ToolbarOptions {
    /// The edge the bar is fixed to. Default: [`Side::North`].
    pub edge: Side,
    /// The height of every button. Must be a finite value `> 0.0`. Default: `28.0`.
    ///
    /// Zoom buttons are square. Wider buttons, such as "Reset", scale in proportion.
    pub button_height: f64,
    /// The space between adjacent buttons. Must be a finite value `>= 0.0`. Default: `4.0`.
    pub gap: f64,
    /// The space between the bar and the edge it is fixed to. Must be a finite value `>= 0.0`. Default: `8.0`.
    pub margin: f64,
}

impl ToolbarOptions {
    /// Default options, fixed to `edge`.
    pub fn new(edge: Side) -> Self {
        Self { edge, ..Self::default() }
    }

    pub fn is_valid(&self) -> bool {
        self.button_height.is_finite()
            && self.button_height > 0.0
            && self.gap.is_finite()
            && self.gap >= 0.0
            && self.margin.is_finite()
            && self.margin >= 0.0
    }
}

impl Default for ToolbarOptions {
    fn default() -> Self {
        Self {
            edge: Side::North,
            button_height: 28.0,
            gap: 4.0,
            margin: 8.0,
        }
    }
}
