//! One input of the golden vectors, with its eight candidates and the mask each convention chooses.

use super::candidate::CandidateGolden;

/// One input, with its eight candidates and the mask each convention chooses.
pub(in super::super) struct SymbolGolden {
    pub(in super::super) input: &'static [u8],
    pub(in super::super) candidates: [CandidateGolden; 8],
    pub(in super::super) mask: u8,
    pub(in super::super) mask_qrcode: u8,
    pub(in super::super) mask_qrcode_chosen: u8,
    pub(in super::super) mask_segno: u8,
}
