//! A heading-style text label for the whole `Scene` — a title, not a graph node.
//!
//! Like [`super::toolbar`]'s own bar, the title is a sibling of the content layer under the `<svg>` root, never a
//! child of it: zooming or panning the content never moves, scales, or otherwise disturbs it. It is optional —
//! [`Scene::show_scene_title`] draws it, [`Scene::hide_scene_title`] removes it entirely — and, like every other
//! nested `Scene`, a container node's own child carries its own independent title, unrelated to its parent's.

mod layout;
mod options;

use super::{Scene, SceneInner};
use crate::{
    colours::TEXT_FILL,
    error::Error,
    geometry::{centre, side::Side},
};
use layout::layout;
pub use options::SceneTitleOptions;
use svg_dom::{DominantBaseline, SvgNode, TextAnchor, root::utils::Point};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A rendered scene title. Lives in [`SceneInner::scene_title`] for as long as it is shown.
pub(super) struct SceneTitle {
    pub node: SvgNode,
    pub options: SceneTitleOptions,
}

impl SceneTitle {
    /// Removes the title from the DOM entirely.
    fn remove(self) {
        self.node.remove();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl SceneInner {
    /// Repositions the shown title for the `<svg>`'s visible area as it is now. Does nothing if none is shown.
    ///
    /// Reads the title's own real, rendered bounding box (`getBBox()`, via [`SvgNode::bounding_box`]) rather than
    /// estimating from `font_size` and character count, so this stays correct for whatever font the browser
    /// actually substitutes — the same reasoning `shrink_label_to_fit`'s own doc comment gives for a node's label.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if reading the bounding box or writing the new position fails.
    pub(super) fn layout_scene_title(&mut self) -> Result<(), Error> {
        let Some(title) = self.scene_title.as_ref() else { return Ok(()) };
        let area = self.visible_area();
        let size = title.node.bounding_box()?.size;
        let rect = layout(title.options.edge, area, size, title.options.margin);
        let at = centre(rect);

        let mut scratch = std::mem::take(&mut self.scratch);
        let result = (|| {
            title.node.set_attr_display(&mut scratch, "x", at.x)?;
            title.node.set_attr_display(&mut scratch, "y", at.y)
        })();
        self.scratch = scratch;
        Ok(result?)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Shows a heading-style title for this scene, replacing any title already shown.
    ///
    /// Unlike a node's own label, `text` describes the scene as a whole — the same role a chart's own title, or a
    /// panel's own `<h2>` in the surrounding page, already plays. `options` controls its size, weight, underline,
    /// position, and `aria-level` — see [`SceneTitleOptions`]'s own doc comment for every field and its default.
    ///
    /// Drawn with `text-anchor="middle"`/`dominant-baseline="middle"`, so `options.edge`/`options.margin` position
    /// its own centre, the same convention [`show_toolbar`](Self::show_toolbar)'s own button labels already use.
    ///
    /// # Keeping the layout current
    ///
    /// **The scene cannot observe its `<svg>` being resized.** The title is positioned against the `<svg>`'s
    /// visible area as it is at the moment of this call, and stays there until told otherwise. Call
    /// [`refresh_layout`](Self::refresh_layout) whenever the visible area changes — see
    /// [`show_toolbar`](Self::show_toolbar)'s own doc comment ("Keeping the layout current") for exactly when that
    /// is, which applies here unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSceneTitleOptions`] if `options` holds a `font_size` that is not a finite value
    /// `> 0.0`, a `margin` that is not a finite value `>= 0.0`, or an `aria_level` of `0`. Checked first, so a
    /// rejected call leaves any existing title exactly as it was.
    ///
    /// If drawing fails partway, nothing is left behind and no title is shown; any previous title stays exactly as
    /// it was until this call has fully succeeded, the same "nothing changes until the new one is fully built"
    /// guarantee [`show_toolbar`](Self::show_toolbar) already gives.
    pub fn show_scene_title(&self, text: impl Into<String>, options: SceneTitleOptions) -> Result<(), Error> {
        if !options.is_valid() {
            return Err(Error::InvalidSceneTitleOptions(options));
        }
        let text = text.into();

        let mut inner = self.inner.borrow_mut();
        let node = inner.svg.text(Point::origin(), &text)?;
        let built = (|| {
            node.set_font_size(options.font_size)?;
            node.set_text_anchor(TextAnchor::Middle)?;
            node.set_dominant_baseline(DominantBaseline::Middle)?;
            node.set_fill(TEXT_FILL)?;
            node.set_attr("role", "heading")?;
            node.set_attr("aria-level", &options.aria_level.to_string())?;
            if options.bold {
                node.set_attr("font-weight", "bold")?;
            }
            if options.underline {
                node.set_attr("text-decoration", "underline")?;
            }
            Ok::<(), Error>(())
        })();
        if let Err(err) = built {
            node.remove();
            return Err(err);
        }

        let previous = inner.scene_title.replace(SceneTitle { node, options });
        if let Err(err) = inner.layout_scene_title() {
            let failed = inner.scene_title.take();
            inner.scene_title = previous;
            if let Some(failed) = failed {
                failed.remove();
            }
            return Err(err);
        }

        // Only now is the new title fully committed — built, installed, and laid out. Only now is it safe to
        // remove whatever title this one replaced — the same ordering `show_selection_toolbar`'s own doc comment
        // ("Ownership") already follows, so either failure branch above can still put `previous` back as the
        // still-live, still-shown title.
        if let Some(previous) = previous {
            previous.remove();
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Removes the title from the DOM entirely. Does nothing if none is shown.
    pub fn hide_scene_title(&self) {
        let Some(title) = self.inner.borrow_mut().scene_title.take() else { return };
        title.remove();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether a title is currently shown.
    pub fn has_scene_title(&self) -> bool {
        self.inner.borrow().scene_title.is_some()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
