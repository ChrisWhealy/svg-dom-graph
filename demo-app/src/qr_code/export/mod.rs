//! The finished symbol as a file: a static SVG with the quiet zone that a scanner needs.
//!
//! The interactive matrix is exactly 21 by 21 modules, and has no quiet zone. An export adds one: four light modules on
//! every side, so 29 by 29 in all. Everything here is plain text work with no DOM, so that it can be tested, and so
//! that a decoder can be run on the files it makes, see `decode_exports.py`.
//!
//! The file is pure black on pure white. Every module is a whole number of pixels wide and the edges are not
//! antialiased, so a scanner sees sharp squares at any size. A run of dark modules in a row is one rectangle.
//!
//! The text of the symbol is user input, and is untrusted. It is written only as escaped character data, never as
//! markup, and a control character is written as a visible escape, since XML cannot hold one.

mod download;

pub(super) use download::download;

use super::{
    grid::{BitGrid, CellState, SIZE},
    paths::dark_path_offset,
    qr_core::input::escaped,
};

/// How many light modules there are on every side of the symbol.
pub(crate) const QUIET_ZONE: usize = 4;

/// The width of the whole picture, in modules, and the width of one module in pixels, which is a whole number.
pub(crate) const MODULES: usize = SIZE + 2 * QUIET_ZONE;
pub(crate) const MODULE_PIXELS: usize = 10;

/// The name of the file the browser is asked to save.
pub(crate) const FILE_NAME: &str = "qr-code.svg";

/// The only two colours in the file.
pub(crate) const DARK: &str = "#000000";
pub(crate) const LIGHT: &str = "#ffffff";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a symbol cannot be exported. Nothing is made then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportError {
    /// The grid is not 21 by 21, so it is not a Version 1 symbol.
    NotASymbol { width: usize, height: usize },
    /// A module has no colour yet, so the symbol is not finished.
    Unfinished { row: usize, col: usize },
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NotASymbol { width, height } => {
                write!(
                    f,
                    "a grid of {width} by {height} modules is not a Version 1 symbol, which is {SIZE} by {SIZE}"
                )
            },
            Self::Unfinished { row, col } => {
                write!(f, "module ({row}, {col}) has no colour yet, so the symbol is not finished")
            },
        }
    }
}

impl std::error::Error for ExportError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `text` as character data of XML: the four characters that could start markup or end an attribute are written as
/// entities. `text` has no control characters, since [`svg`] writes those as visible escapes first.
pub(crate) fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The SVG file of the finished symbol `grid`, which was made from `text`.
///
/// The picture is [`MODULES`] modules wide and tall, each [`MODULE_PIXELS`] pixels, with a white background and one
/// path of black rectangles. `text` is named in the file's description so that a reader of the file can tell what it
/// holds.
///
/// # Errors
///
/// Returns `Err` if `grid` is not 21 by 21 or has a module without a colour.
pub(crate) fn svg(grid: &BitGrid, text: &str) -> Result<String, ExportError> {
    if grid.width() != SIZE || grid.height() != SIZE {
        return Err(ExportError::NotASymbol {
            width: grid.width(),
            height: grid.height(),
        });
    }
    if let Some((row, col)) = grid
        .cells()
        .find(|&(row, col)| !matches!(grid.get(row, col), Some(CellState::Dark | CellState::Light)))
    {
        return Err(ExportError::Unfinished { row, col });
    }

    let pixels = MODULES * MODULE_PIXELS;
    let description = xml_escape(&escaped(text.as_bytes()));
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{pixels}\" height=\"{pixels}\" \
         viewBox=\"0 0 {MODULES} {MODULES}\" shape-rendering=\"crispEdges\" role=\"img\" \
         aria-labelledby=\"title description\">\n\
         <title id=\"title\">QR code, Version 1, level L</title>\n\
         <desc id=\"description\">{description}</desc>\n\
         <rect width=\"{MODULES}\" height=\"{MODULES}\" fill=\"{LIGHT}\"/>\n\
         <path fill=\"{DARK}\" d=\"{}\"/>\n\
         </svg>\n",
        dark_path_offset(grid, QUIET_ZONE)
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
