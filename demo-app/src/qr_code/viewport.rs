//! The matrix viewport: a fixed set of SVG elements that draw a [`BitGrid`], however many modules it holds.
//!
//! There are three base paths, one for each colour a module can have, one path that outlines the modules of the chosen
//! role, and one that outlines the highlighted modules. Each path is rewritten only when its own geometry changes, so a
//! step that only moves the highlight writes one attribute.

use super::{
    grid::{BitGrid, QrModuleKind, SIZE},
    paths::{base_paths, highlight_path, matrix_text, role_path},
    slot::{Slot, Target, make},
};
use demo_common::util::required_element;

/// The size of one module in the scene, in pixels. It is a whole number, so that the modules have sharp edges.
pub(crate) const MODULE_PIXELS: usize = 20;

/// The fill of a module without a colour, which must not be mistaken for light.
const UNASSIGNED_FILL: &str = "#c9d3e0";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything one render needs to know.
pub(crate) struct Frame<'a> {
    pub(crate) grid: &'a BitGrid,
    pub(crate) highlights: &'a [(usize, usize)],
    pub(crate) role: Option<QrModuleKind>,
    /// The accessible name of the whole viewport.
    pub(crate) summary: &'a str,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The viewport's elements and the slots that write to them.
pub(crate) struct Viewport {
    svg: web_sys::Element,
    unassigned: Slot,
    light: Slot,
    dark: Slot,
    role: Slot,
    highlight: Slot,
    summary: Slot,
    matrix_text: Slot,
}

impl Viewport {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the empty viewport in the scene's `layer`, with its top left corner at `origin`, and finds
    /// `#matrix_text_id`, the element that holds the grid as text.
    ///
    /// The viewport is a group scaled up so that one module is one unit, so it zooms and pans with the scene.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `#matrix_text_id` is missing from the page, or if an element could not be made or added.
    pub(crate) fn new(
        document: &web_sys::Document,
        layer: &web_sys::Element,
        origin: (f64, f64),
        matrix_text_id: &str,
    ) -> Result<Self, String> {
        let svg = make(
            document,
            "g",
            &[
                ("id", "qr-code-matrix"),
                (
                    "transform",
                    &format!("translate({} {}) scale({MODULE_PIXELS})", origin.0, origin.1),
                ),
                ("role", "img"),
                ("shape-rendering", "crispEdges"),
            ],
        )?;
        let background = make(
            document,
            "rect",
            &[("width", &SIZE.to_string()), ("height", &SIZE.to_string()), ("fill", "#ffffff")],
        )?;
        svg.append_child(&background)
            .map_err(|e| format!("could not add the background: {e:?}"))?;

        let path = |name: &str, fill: &str, stroke: &str, width: &str| -> Result<web_sys::Element, String> {
            let element = make(
                document,
                "path",
                &[
                    ("id", name),
                    ("fill", fill),
                    ("stroke", stroke),
                    ("stroke-width", width),
                    ("stroke-linejoin", "miter"),
                ],
            )?;
            svg.append_child(&element).map_err(|e| format!("could not add {name}: {e:?}"))?;
            Ok(element)
        };
        let unassigned = path("qr-code-unassigned", UNASSIGNED_FILL, "none", "0")?;
        let light = path("qr-code-light", "#ffffff", "none", "0")?;
        let dark = path("qr-code-dark", "#111111", "none", "0")?;
        let role = path("qr-code-role", "none", "#d9480f", "0.1")?;
        let highlight = path("qr-code-highlight", "none", "#1971c2", "0.14")?;
        layer
            .append_child(&svg)
            .map_err(|e| format!("could not add the viewport: {e:?}"))?;

        let d = Target::Attribute("d");
        Ok(Self {
            unassigned: Slot::new(&unassigned, d),
            light: Slot::new(&light, d),
            dark: Slot::new(&dark, d),
            role: Slot::new(&role, d),
            highlight: Slot::new(&highlight, d),
            summary: Slot::new(&svg, Target::Attribute("aria-label")),
            matrix_text: Slot::new(&required_element(document, matrix_text_id)?, Target::Text),
            svg,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws `frame`, and returns how many attributes and texts it had to write.
    ///
    /// Every write is of the whole state, so a failed render is put right by the next one, whatever it changed.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the DOM refuses a write.
    pub(crate) fn render(&self, frame: &Frame) -> Result<usize, String> {
        let base = base_paths(frame.grid);
        let writes = [
            self.unassigned.set(&base.unassigned)?,
            self.light.set(&base.light)?,
            self.dark.set(&base.dark)?,
            self.role.set(&role_path(frame.grid, frame.role))?,
            self.highlight.set(&highlight_path(frame.highlights))?,
            self.summary.set(frame.summary)?,
            self.matrix_text.set(&matrix_text(frame.grid))?,
        ];
        Ok(writes.into_iter().filter(|&wrote| wrote).count())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The number of elements inside the viewport, which does not depend on the grid.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the elements cannot be counted.
    pub(crate) fn element_count(&self) -> Result<u32, String> {
        Ok(self
            .svg
            .query_selector_all("*")
            .map_err(|e| format!("could not count the viewport's elements: {e:?}"))?
            .length())
    }
}
