//! What a module of a QR symbol is for: a part of a fixed pattern, or a bit of a codeword.

use super::{DataRole, FunctionRole};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a module is for. The two halves are separate types so that [`BitGrid::assign`](super::BitGrid::assign) cannot
/// be given a function role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QrModuleKind {
    Function(FunctionRole),
    Payload(DataRole),
}

impl QrModuleKind {
    /// Every role, in the order the legend lists them.
    pub(crate) const ALL: [Self; 7] = [
        Self::Function(FunctionRole::Finder),
        Self::Function(FunctionRole::Separator),
        Self::Function(FunctionRole::Timing),
        Self::Function(FunctionRole::DarkModule),
        Self::Function(FunctionRole::Format),
        Self::Payload(DataRole::Data),
        Self::Payload(DataRole::ErrorCorrection),
    ];

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The name shown in the legend and read by assistive technology.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Function(FunctionRole::Finder) => "Finder",
            Self::Function(FunctionRole::Separator) => "Separator",
            Self::Function(FunctionRole::Timing) => "Timing",
            Self::Function(FunctionRole::DarkModule) => "Dark module",
            Self::Function(FunctionRole::Format) => "Format",
            Self::Payload(DataRole::Data) => "Data",
            Self::Payload(DataRole::ErrorCorrection) => "Error correction",
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What this role is for, in words for someone who has never met a QR code. It is shown when the role is chosen.
    pub(crate) const fn explanation(self) -> &'static str {
        match self {
            Self::Function(FunctionRole::Finder) => {
                "The three large squares in the corners. Each is a dark ring, a light ring and a dark 3 by 3 centre. \
                 A scanner looks for them first, to find the code and to see its size. The fourth corner has none, \
                 so the scanner can tell which way up the code is."
            },
            Self::Function(FunctionRole::Separator) => {
                "A border of light modules, one module wide, along the inner sides of each finder pattern. It keeps \
                 the dark ring of a finder apart from the dark modules of the data beside it, so that the two are \
                 not read as one shape."
            },
            Self::Function(FunctionRole::Timing) => {
                "Two lines of alternating dark and light modules between the finder patterns: one along row 6 and \
                 one down column 6. Both start and end dark. A scanner counts along them to measure the grid, which \
                 corrects for a code that is stretched or photographed at an angle."
            },
            Self::Function(FunctionRole::DarkModule) => {
                "One module that is always dark, at row 13 and column 8, beside the bottom left finder. The standard \
                 reserves it, so it is never part of the message and no mask ever changes it."
            },
            Self::Function(FunctionRole::Format) => {
                "Fifteen bits that say which error correction level and which mask the symbol uses. A scanner reads \
                 them before anything else. They are written twice, once around the top left finder and once split \
                 between the other two, so that damage to one copy can be survived. They have no colour until the \
                 mask is chosen."
            },
            Self::Payload(DataRole::Data) => {
                "The modules that hold the message: the mode, the length, the bytes, the terminator and the \
                 padding. 19 codewords of 8 bits fill 152 modules. They are placed two columns at a time, starting at \
                 the bottom right and zig-zagging up, then down, then up again."
            },
            Self::Payload(DataRole::ErrorCorrection) => {
                "Seven extra codewords, 56 modules, worked out from the data with Reed-Solomon coding. They let a \
                 scanner rebuild the message when part of the symbol is dirty, torn or hidden. Level L is designed to \
                 recover about 7% of the symbol."
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The value of the legend's own option for this role, which [`from_key`](Self::from_key) reads back.
    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::Function(FunctionRole::Finder) => "finder",
            Self::Function(FunctionRole::Separator) => "separator",
            Self::Function(FunctionRole::Timing) => "timing",
            Self::Function(FunctionRole::DarkModule) => "dark-module",
            Self::Function(FunctionRole::Format) => "format",
            Self::Payload(DataRole::Data) => "data",
            Self::Payload(DataRole::ErrorCorrection) => "error-correction",
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The role whose [`key`](Self::key) is `key`, or `None` for any other text, such as the legend's "none" option.
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }
}
