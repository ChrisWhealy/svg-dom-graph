use crate::geometry::side::Side;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures how [`Scene::show_scene_title`](crate::scene::Scene::show_scene_title) draws a scene's own title and
/// where it sits.
///
/// All lengths are in the `<svg>`'s own user space — the same units as its `viewBox`, or as pixels when it has none.
/// They never scale with the scene's zoom.
///
/// Build one either with [`SceneTitleOptions::default`] or with the `with_*` methods below. A struct literal does not
/// compile outside this crate.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, the same as
/// [`ConnectorOptions`](crate::scene::ConnectorOptions): removing it later is a breaking change, so every field this
/// type gains must itself stay `Copy`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct SceneTitleOptions {
    /// The edge the title is fixed to. Default: [`Side::North`].
    pub edge: Side,
    /// The space between the title and the edge it is fixed to. Must be a finite value `>= 0.0`. Default: `12.0`.
    pub margin: f64,
    /// The title's own font size. Must be a finite value `> 0.0`. Default: `20.0`. It is deliberately larger than a
    /// plain node's own label (`14.0`) or a data node's own cell text (`13.0`). So the title reads as a heading for the
    /// whole scene, not just another label.
    pub font_size: f64,
    /// Whether the title is drawn bold. Default: `true`.
    pub bold: bool,
    /// Whether the title is drawn underlined. Default: `true`.
    pub underline: bool,
    /// The title's own `aria-level`, for assistive technology that navigates a page by heading. Must be `>= 1`.
    /// Default: `2`.
    ///
    /// This crate cannot know where in the host page's own heading hierarchy a scene sits. A title drawn inside a panel
    /// that already has its own `<h2>` reads more correctly at level `3`. A different host page might want something
    /// else again. Adjust this to fit whatever heading structure actually surrounds the `<svg>`.
    ///
    /// Only one scene's own title is ever exposed to assistive technology at a time, whatever the nesting depth.
    /// [`Scene::enter`](crate::scene::Scene::enter)/[`exit`](crate::scene::Scene::exit) hide a scene's whole `<svg>`
    /// root via the `visibility` attribute. A `visibility: hidden` subtree, however it is set, is excluded from the
    /// accessibility tree entirely, the same as `display: none`. So a fixed `aria_level` can never make it ambiguous
    /// *which* scene's title is current; it only affects how that one title reads relative to the page around it.
    pub aria_level: u8,
}

impl SceneTitleOptions {
    /// Returns `self` with `edge` set to `edge`.
    #[must_use]
    pub fn with_edge(mut self, edge: Side) -> Self {
        self.edge = edge;
        self
    }

    /// Returns `self` with `margin` set to `margin`.
    #[must_use]
    pub fn with_margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }

    /// Returns `self` with `font_size` set to `font_size`.
    #[must_use]
    pub fn with_font_size(mut self, font_size: f64) -> Self {
        self.font_size = font_size;
        self
    }

    /// Returns `self` with `bold` set to `bold`.
    #[must_use]
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Returns `self` with `underline` set to `underline`.
    #[must_use]
    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// Returns `self` with `aria_level` set to `aria_level`.
    #[must_use]
    pub fn with_aria_level(mut self, aria_level: u8) -> Self {
        self.aria_level = aria_level;
        self
    }

    /// Whether every value [`Scene::show_scene_title`](crate::scene::Scene::show_scene_title) accepts: `font_size` a
    /// finite value `> 0.0`, `margin` a finite value `>= 0.0`, and `aria_level` `>= 1`.
    ///
    /// Crate-private, since `show_scene_title` is the one place it is needed and it reports a failure as an error.
    pub(crate) fn is_valid(&self) -> bool {
        self.font_size.is_finite()
            && self.font_size > 0.0
            && self.margin.is_finite()
            && self.margin >= 0.0
            && self.aria_level >= 1
    }
}

impl Default for SceneTitleOptions {
    fn default() -> Self {
        Self {
            edge: Side::North,
            margin: 12.0,
            font_size: 20.0,
            bold: true,
            underline: true,
            aria_level: 2,
        }
    }
}
