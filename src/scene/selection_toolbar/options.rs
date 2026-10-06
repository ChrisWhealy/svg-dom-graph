use crate::geometry::side::Side;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An optional second, coarser way for a selection toolbar to step: two extra buttons that move `step` cells at a
/// time. See [`SelectionToolbarOptions::stride`].
///
/// For a node whose cells fall into equal groups — SHA3's `Keccak-f`, say, with five functions in each of 24
/// rounds — `Prev`/`Next` step one cell and these step a whole group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionStride {
    /// How many cells each extra button moves. Must be `>= 1`.
    pub step: usize,
    /// The word for one group, such as `"Round"`. The buttons read `Prev Round` and `Next Round`, and their accessible
    /// names `Previous round` and `Next round`.
    pub label: &'static str,
}

impl SelectionStride {
    /// A stride moving `step` cells, called `label`.
    pub fn new(step: usize, label: &'static str) -> Self {
        Self { step, label }
    }
}

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
    /// Two extra buttons, `Prev <label>` and `Next <label>`, that step by [`SelectionStride::step`] cells instead of
    /// one. `None` — the default — draws just `Prev`, `Next` and `Restart`. When set, `step` must be `>= 1`.
    pub stride: Option<SelectionStride>,
}

impl SelectionToolbarOptions {
    /// Default options, fixed to `edge`.
    pub fn new(edge: Side) -> Self {
        Self { edge, ..Self::default() }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// These options with `stride` set to `stride` — see [`SelectionToolbarOptions::stride`].
    ///
    /// Provided because the struct is `#[non_exhaustive]`, so a caller outside this crate cannot build one with struct
    /// update syntax.
    #[must_use]
    pub fn with_stride(mut self, stride: SelectionStride) -> Self {
        self.stride = Some(stride);
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether every length is one [`Scene::show_selection_toolbar`](crate::scene::Scene::show_selection_toolbar)
    /// accepts: `button_height` a finite value `> 0.0`, `gap` and `margin` finite values `>= 0.0`, and any `stride`'s
    /// `step` at least `1`.
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
            && self.stride.is_none_or(|stride| stride.step >= 1)
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
            stride: None,
        }
    }
}
