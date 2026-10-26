//! The marks drawn over the bytes of a hex dump.

use crate::{
    error::Error,
    model::content::{HexDumpFormat, full_width},
    scene::{
        HexMarks, Pool,
        node::{GRID_FONT_FAMILY, GRID_FONT_SIZE, render_guard::RenderGuard},
    },
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot,
    root::utils::{Point, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the marks a hex dump shows its selected byte with, over its lines, hidden until a byte is selected. A light block
/// and a dark copy of the characters on it, for the hex digits and again for the character, so that a mark reads as
/// inverse video does in a terminal. `line_width` is the width of a full line, which is `full_width(format)` characters.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
pub(super) fn draw_hex_marks(
    svg: &SvgRoot,
    group: &SvgNode,
    guard: &mut RenderGuard,
    format: HexDumpFormat,
    line_width: f64,
) -> Result<HexMarks, Error> {
    let mut mark = |fill: &str| -> Result<(SvgNode, SvgNode), Error> {
        let (block, ink) = make_mark(svg, fill)?;
        guard.track(block.clone());
        group.append(&block)?;
        guard.release();
        guard.track(ink.clone());
        group.append(&ink)?;
        guard.release();
        Ok((block, ink))
    };
    let [hex_fill, ascii_fill] = HexMarks::FILLS;
    let hex = mark(hex_fill)?;
    let ascii = mark(ascii_fill)?;
    #[allow(clippy::cast_precision_loss)]
    let char_width = line_width / full_width(format) as f64;
    Ok(HexMarks {
        hex,
        ascii,
        char_width,
        format,
        group: group.clone(),
        scratch: String::new(),
        secondary: Pool::default(),
        unreached: Pool::default(),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A hidden block of `fill` and the hidden dark text that goes over it, neither yet in the document.
pub(super) fn make_mark(svg: &SvgRoot, fill: &str) -> Result<(SvgNode, SvgNode), Error> {
    let block = svg.rect(Point::origin(), Size::new(0.0, 0.0))?;
    block.set_fill(fill)?;
    block.set_attr("pointer-events", "none")?;
    block.set_attr("aria-hidden", "true")?;
    block.set_attr("visibility", "hidden")?;

    let ink = svg.text(Point::origin(), "")?;
    ink.set_dominant_baseline(DominantBaseline::Middle)?;
    ink.set_font_family(GRID_FONT_FAMILY)?;
    ink.set_font_size(GRID_FONT_SIZE)?;
    ink.set_fill(HexMarks::INK)?;
    ink.set_attr("style", "white-space: pre")?;
    ink.set_attr("pointer-events", "none")?;
    ink.set_attr("aria-hidden", "true")?;
    ink.set_attr("visibility", "hidden")?;
    Ok((block, ink))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A block and its text, appended to `group`, for a hex dump's secondary byte. Used when the marks drawn so far run out.
pub(crate) fn new_mark(svg: &SvgRoot, group: &SvgNode, fill: &str) -> Result<(SvgNode, SvgNode), svg_dom::Error> {
    let (block, ink) = make_mark(svg, fill).map_err(|e| svg_dom::Error::Dom(e.to_string()))?;
    group.append(&block)?;
    group.append(&ink)?;
    Ok((block, ink))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A hidden block of `fill` at the opacity of an unreached cell, appended to `group`, which dims what it is drawn over.
pub(crate) fn new_dim_block(svg: &SvgRoot, group: &SvgNode, fill: &str) -> Result<SvgNode, svg_dom::Error> {
    let block = svg.rect(Point::origin(), Size::new(0.0, 0.0))?;
    block.set_fill(fill)?;
    block.set_attr("opacity", "0.65")?;
    block.set_attr("pointer-events", "none")?;
    block.set_attr("aria-hidden", "true")?;
    block.set_attr("visibility", "hidden")?;
    group.append(&block)?;
    Ok(block)
}
