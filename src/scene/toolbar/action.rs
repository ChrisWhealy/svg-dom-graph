use crate::geometry::view::ViewTransform;
use svg_dom::root::utils::Size;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a toolbar button does when activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolbarAction {
    ZoomIn,
    ZoomOut,
    Reset,
}

impl ToolbarAction {
    /// The buttons a toolbar holds, in display order.
    pub(super) const ALL: [Self; 3] = [Self::ZoomIn, Self::ZoomOut, Self::Reset];

    /// The visible glyph or word on the button.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::ZoomIn => "+",
            Self::ZoomOut => "\u{2212}", // MINUS SIGN, so it matches the width of "+"
            Self::Reset => "100%",
        }
    }

    /// The button's accessible name, since a bare "+" or "−" is not one.
    pub(super) fn aria_label(self) -> &'static str {
        match self {
            Self::ZoomIn => "Zoom in",
            Self::ZoomOut => "Zoom out",
            Self::Reset => "Reset zoom",
        }
    }

    /// The button's size, before any layout widens it.
    pub(super) fn natural_size(self, height: f64) -> Size {
        match self {
            Self::ZoomIn | Self::ZoomOut => Size::new(height, height),
            Self::Reset => Size::new(height * 2.5, height),
        }
    }

    /// Whether `view` leaves this action anything to do.
    pub(super) fn is_enabled(self, view: ViewTransform) -> bool {
        match self {
            Self::ZoomIn => view.can_zoom_in(),
            Self::ZoomOut => view.can_zoom_out(),
            Self::Reset => view != ViewTransform::IDENTITY,
        }
    }
}
