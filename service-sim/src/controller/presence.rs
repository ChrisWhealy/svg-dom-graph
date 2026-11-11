//! Whether the host can be drawn on now.

/// Whether the host can be drawn on now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// Connected to the document, and the panel that holds it is shown.
    Live,
    /// Connected, and the panel is hidden or the document is.
    Hidden,
    /// Not in the document. A detached element still accepts writes, so this is checked and not discovered.
    Detached,
    /// Gone for good.
    Disposed,
}
