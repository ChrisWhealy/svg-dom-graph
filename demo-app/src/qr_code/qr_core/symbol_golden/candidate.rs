//! One candidate matrix of a golden symbol, with its own format information and its scores.

/// One candidate: the matrix with this mask and its own format information, and its scores.
pub(in super::super) struct CandidateGolden {
    pub(in super::super) mask: u8,
    pub(in super::super) format_bits: u16,
    pub(in super::super) rows: [&'static str; 21],
    pub(in super::super) n1: u32,
    pub(in super::super) n2: u32,
    pub(in super::super) n3: u32,
    pub(in super::super) n3_qrcode: u32,
    pub(in super::super) n3_segno: u32,
    pub(in super::super) n4: u32,
}
