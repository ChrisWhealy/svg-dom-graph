//! The 62 snapshots of the walk, and the one before it. Each is the whole state of the scene at one position, worked
//! out from the encoder's own stages with no DOM, so every one can be tested and any one can be drawn again from
//! nothing.
//!
//! | Snapshots | What they show                                                              |
//! |-----------|-----------------------------------------------------------------------------|
//! | 0         | the input, as text                                                          |
//! | 1         | its bytes                                                                   |
//! | 2 to 5    | the bitstream, a part at a time: the mode, the count, the data, the terminator |
//! | 6         | the 19 data codewords, with the padding                                     |
//! | 7 to 25   | one Reed-Solomon division step for each data codeword                        |
//! | 26        | the function modules, with the format modules reserved and not coloured     |
//! | 27 to 52  | one codeword placed for each snapshot                                       |
//! | 53 to 60  | one mask candidate for each snapshot, with its format information and score |
//! | 61        | the winning candidate                                                       |
//!
//! The count never depends on the input. Every node of the scene has the same number of cells for every input, so the
//! scene is built once and only its values change.

use super::{
    function::{FORMAT_BITS, format_copy_a, format_copy_b},
    grid::{BitGrid, SIZE},
    qr_core::{
        ERROR_CORRECTION_CODEWORDS, Encoded, TOTAL_CODEWORDS,
        data::DATA_CODEWORDS,
        input::{MAX_BYTES, escaped},
        mask::MASKS,
        symbol::{Symbol, SymbolError, place_codeword},
    },
};

/// How many snapshots there are, and where each stage starts.
pub(crate) const COUNT: usize = 62;
const BITSTREAM_START: usize = 2;
const BITSTREAM_STEPS: usize = 4;
const DATA_START: usize = BITSTREAM_START + BITSTREAM_STEPS;
const RS_START: usize = DATA_START + 1;
const FUNCTIONS: usize = RS_START + DATA_CODEWORDS;
const PLACEMENT_START: usize = FUNCTIONS + 1;
const MASK_START: usize = PLACEMENT_START + TOTAL_CODEWORDS;
const FINAL: usize = MASK_START + MASKS;

/// The number of cells of each node, which never changes.
pub(crate) const BYTE_CELLS: usize = MAX_BYTES;
pub(crate) const BITSTREAM_CELLS: usize = DATA_CODEWORDS * 8;
pub(crate) const SCORE_COLUMNS: usize = 5;
pub(crate) const SCORE_CELLS: usize = MASKS * SCORE_COLUMNS;
pub(crate) const FORMAT_CELLS: usize = FORMAT_BITS;

/// The snapshots that the buttons that jump to a stage select.
pub(crate) const JUMPS: [(&str, usize); 6] = [
    ("Input", 0),
    ("Reed-Solomon", RS_START),
    ("Function modules", FUNCTIONS),
    ("Placement", PLACEMENT_START),
    ("Masks", MASK_START),
    ("Final", FINAL),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The part of the walk a snapshot is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    Input,
    Bytes,
    Bitstream,
    DataCodewords,
    ReedSolomon,
    FunctionModules,
    Placement,
    Masks,
    Final,
}

impl Stage {
    /// The stage that snapshot `index` is in.
    pub(crate) const fn of(index: usize) -> Self {
        match index {
            0 => Self::Input,
            1 => Self::Bytes,
            BITSTREAM_START..DATA_START => Self::Bitstream,
            DATA_START => Self::DataCodewords,
            RS_START..FUNCTIONS => Self::ReedSolomon,
            FUNCTIONS => Self::FunctionModules,
            PLACEMENT_START..MASK_START => Self::Placement,
            MASK_START..FINAL => Self::Masks,
            _ => Self::Final,
        }
    }

    /// The name of the stage, which the live region and the heading of its explanation use.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Input => "Input",
            Self::Bytes => "Bytes",
            Self::Bitstream => "Bitstream",
            Self::DataCodewords => "Data codewords",
            Self::ReedSolomon => "Reed-Solomon",
            Self::FunctionModules => "Function modules",
            Self::Placement => "Placement",
            Self::Masks => "Masks",
            Self::Final => "Final symbol",
        }
    }

    /// What the stage does and why, for someone who has never met a QR code.
    pub(crate) const fn explanation(self) -> &'static str {
        match self {
            Self::Input => {
                "Type up to 17 ASCII characters. Each becomes one byte, so what is shown is what is encoded. A \
                 character above U+007F is refused, because it would need an encoding that cannot be seen here."
            },
            Self::Bytes => {
                "The bytes of the text, in hexadecimal. A QR code stores bytes, not characters. The places the text \
                 does not use are dimmed."
            },
            Self::Bitstream => {
                "The bytes become bits, a part at a time. First 0100, which says the bytes are in byte mode, then the \
                 number of bytes in 8 bits, then the bytes, then up to four zero bits that mark the end of the data."
            },
            Self::DataCodewords => {
                "The bits are cut into 8 bit codewords. A Version 1 symbol at level L holds exactly 19, so what is \
                 left is filled with the pad codewords EC and 11 in turn."
            },
            Self::ReedSolomon => {
                "Error correction codewords let a scanner repair a damaged symbol. They are the remainder when the \
                 data, as a polynomial over GF(256), is divided by a fixed generator. Each step takes in one data \
                 byte and updates the 7 bytes of the remainder."
            },
            Self::FunctionModules => {
                "The fixed patterns go in first: the finders, the separators, the timing patterns and the dark \
                 module. The format modules are reserved, but have no colour yet, since they say which mask is used, \
                 and that is chosen last."
            },
            Self::Placement => {
                "The 26 codewords fill the modules that are left, two columns at a time, from the bottom right, up and \
                 then down, skipping the timing column. The first bit of a codeword goes in the first module."
            },
            Self::Masks => {
                "Large plain areas and false finders are hard to scan. Each of eight masks flips a different pattern \
                 of the data modules, and each candidate gets its own format information. N1 scores runs, N2 blocks, \
                 N3 false finders and N4 the balance of dark and light. The lowest total is best."
            },
            Self::Final => {
                "The candidate with the lowest total is the symbol, and the lowest mask wins a tie. It is 21 by 21 \
                 modules. A quiet zone of four light modules is added around it only when it is exported."
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One of the four parts of the bitstream, which the walk shows one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Region {
    /// The 4 bits that say the data is in byte mode.
    Mode,
    /// The 8 bits that say how many bytes there are.
    Count,
    /// The bits of the bytes themselves.
    Data,
    /// The zero bits that end the data.
    Terminator,
}

/// The bits of the mode indicator and of the count.
const MODE_BITS: usize = 4;
const COUNT_BITS: usize = 8;

impl Region {
    /// The parts, in the order they are shown.
    pub(crate) const ALL: [Self; BITSTREAM_STEPS] = [Self::Mode, Self::Count, Self::Data, Self::Terminator];

    /// The cells of the bitstream that this part is, for `used` bytes and a terminator of `terminator` bits.
    pub(crate) const fn cells(self, used: usize, terminator: usize) -> std::ops::Range<usize> {
        let data = MODE_BITS + COUNT_BITS;
        match self {
            Self::Mode => 0..MODE_BITS,
            Self::Count => MODE_BITS..data,
            Self::Data => data..data + 8 * used,
            Self::Terminator => data + 8 * used..data + 8 * used + terminator,
        }
    }

    /// What this part is, as a heading and a few sentences, for `used` bytes and a terminator of `terminator` bits.
    pub(crate) fn describe(self, used: usize, terminator: usize) -> (String, String) {
        match self {
            Self::Mode => (
                "Mode indicator, 4 bits".to_owned(),
                "0100 says that what follows is in byte mode: plain bytes, with no special packing.".to_owned(),
            ),
            Self::Count => (
                "Character count, 8 bits".to_owned(),
                format!("{used} in binary. It says how many bytes follow, so a scanner knows where the data ends."),
            ),
            Self::Data => (
                format!("Data, {} bits", 8 * used),
                if used == 0 {
                    "The text is empty, so there are no bytes of data.".to_owned()
                } else {
                    format!("The {used} bytes of the text, one after the other, each with its highest bit first.")
                },
            ),
            Self::Terminator => (
                format!("Terminator, {terminator} bits"),
                "Zero bits that end the data. There are fewer than four only if the symbol is already full.".to_owned(),
            ),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nodes of the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NodeKey {
    Input,
    Bytes,
    Bitstream,
    DataCodewords,
    Division,
    ErrorCorrection,
    Codewords,
    MaskScores,
    Format,
}

/// An edge of the graph, from one node to another.
pub(crate) type EdgeKey = (NodeKey, NodeKey);

/// Every edge of the graph, which is the flow of the data.
pub(crate) const EDGES: [EdgeKey; 9] = [
    (NodeKey::Input, NodeKey::Bytes),
    (NodeKey::Bytes, NodeKey::Bitstream),
    (NodeKey::Bitstream, NodeKey::DataCodewords),
    (NodeKey::DataCodewords, NodeKey::Division),
    (NodeKey::Division, NodeKey::ErrorCorrection),
    (NodeKey::DataCodewords, NodeKey::Codewords),
    (NodeKey::ErrorCorrection, NodeKey::Codewords),
    (NodeKey::Codewords, NodeKey::MaskScores),
    (NodeKey::MaskScores, NodeKey::Format),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the nodes hold for the whole of one input. These are the same at every snapshot. The snapshots only say how
/// much of them has been reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Content {
    pub(crate) text: String,
    /// The bytes of the input, then zeros up to 17.
    pub(crate) bytes: [u8; BYTE_CELLS],
    /// The bitstream and its terminator, one bit to a `u8`, then zeros up to 152.
    pub(crate) bitstream: Vec<u8>,
    pub(crate) data: [u8; DATA_CODEWORDS],
    pub(crate) codewords: [u8; TOTAL_CODEWORDS],
    /// N1, N2, N3, N4 and the total, for each mask in turn.
    pub(crate) scores: [u16; SCORE_CELLS],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cells of each node that have not been reached yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Unreached {
    pub(crate) bytes: Vec<usize>,
    pub(crate) bitstream: Vec<usize>,
    pub(crate) data: Vec<usize>,
    pub(crate) division: Vec<usize>,
    pub(crate) error_correction: Vec<usize>,
    pub(crate) codewords: Vec<usize>,
    pub(crate) scores: Vec<usize>,
    pub(crate) format: Vec<usize>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The whole state of the scene at one position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DemoSnapshot {
    pub(crate) stage: Stage,
    /// What happened at this position, in a sentence or two.
    pub(crate) label: String,
    /// The matrix at this position.
    pub(crate) grid: BitGrid,
    /// The modules this position changed or is about.
    pub(crate) highlights: Vec<(usize, usize)>,
    pub(crate) active_nodes: Vec<NodeKey>,
    pub(crate) active_edges: Vec<EdgeKey>,
    /// The data byte taken in and the factor it gave, at a Reed-Solomon step.
    pub(crate) division: [u8; 2],
    /// The remainder register, which ends as the error correction codewords.
    pub(crate) register: [u8; ERROR_CORRECTION_CODEWORDS],
    /// The format information of the candidate on show, most significant bit first, one bit to a `u8`.
    pub(crate) format: [u8; FORMAT_CELLS],
    /// The codeword being placed, if one is.
    pub(crate) placing: Option<usize>,
    /// The data codeword that the division is taking in, if it is.
    pub(crate) dividing: Option<usize>,
    /// The cells of the bitstream that are marked, which are the part that the position is about.
    pub(crate) marked_bits: Vec<usize>,
    /// What that part is, as a heading and a few sentences, which the scene shows to the right of the bitstream.
    pub(crate) note: Option<(String, String)>,
    /// The mask whose row of the scores is marked, once the winner is known.
    pub(crate) winner: Option<usize>,
    pub(crate) unreached: Unreached,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The content and every snapshot of one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotSet {
    pub(crate) content: Content,
    /// The position before the first, which is also what Restart returns to.
    pub(crate) unstarted: DemoSnapshot,
    /// Exactly [`COUNT`] snapshots.
    pub(crate) snapshots: Vec<DemoSnapshot>,
}

impl SnapshotSet {
    /// The snapshot at `position`, where `None` is before the first.
    pub(crate) fn at(&self, position: Option<usize>) -> &DemoSnapshot {
        position.and_then(|index| self.snapshots.get(index)).unwrap_or(&self.unstarted)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `cells` of 0 to `count`, except for the first `reached`.
fn after(reached: usize, count: usize) -> Vec<usize> {
    (reached..count).collect()
}

/// The 30 modules of both copies of the format information.
fn format_modules() -> Vec<(usize, usize)> {
    (0..FORMAT_BITS)
        .flat_map(|bit| [format_copy_a(bit), format_copy_b(bit)])
        .collect()
}

/// The 15 bits of format word `word`, most significant first.
fn format_cells(word: u16) -> [u8; FORMAT_CELLS] {
    std::array::from_fn(|cell| u8::from(word >> (FORMAT_BITS - 1 - cell) & 1 == 1))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(" ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every snapshot of `encoded`, whose symbol is `symbol`.
///
/// # Errors
///
/// Returns `Err` if a codeword cannot be placed, which means the layout is wrong. No snapshots are made then.
pub(crate) fn build(text: &str, encoded: &Encoded, symbol: &Symbol) -> Result<SnapshotSet, SymbolError> {
    let used = encoded.input.len();
    let mut bytes = [0; BYTE_CELLS];
    bytes[..used].copy_from_slice(&encoded.input);

    let shown_bits = encoded.data.bitstream.len() + encoded.data.terminator_bits + encoded.data.boundary_bits;
    let mut bitstream = encoded.data.bitstream.clone();
    bitstream.resize(shown_bits, 0);
    bitstream.resize(BITSTREAM_CELLS, 0);

    let mut scores = [0_u16; SCORE_CELLS];
    for (mask, candidate) in symbol.candidates.iter().enumerate() {
        let p = candidate.penalty;
        let row = [p.n1, p.n2, p.n3, p.n4, p.total()];
        for (column, value) in row.into_iter().enumerate() {
            scores[mask * SCORE_COLUMNS + column] = u16::try_from(value).unwrap_or(u16::MAX);
        }
    }
    let content = Content {
        text: text.to_owned(),
        bytes,
        bitstream,
        data: encoded.data.codewords,
        codewords: encoded.codewords(),
        scores,
    };

    // Every node is not reached at first, apart from the input.
    let nothing_reached = Unreached {
        bytes: after(0, BYTE_CELLS),
        bitstream: after(0, BITSTREAM_CELLS),
        data: after(0, DATA_CODEWORDS),
        division: after(0, 2),
        error_correction: after(0, ERROR_CORRECTION_CODEWORDS),
        codewords: after(0, TOTAL_CODEWORDS),
        scores: after(0, SCORE_CELLS),
        format: after(0, FORMAT_CELLS),
    };
    let empty = BitGrid::new(SIZE, SIZE);
    let unstarted = DemoSnapshot {
        stage: Stage::Input,
        label: "Not started. Every module is empty.".to_owned(),
        grid: empty.clone(),
        highlights: Vec::new(),
        active_nodes: Vec::new(),
        active_edges: Vec::new(),
        division: [0; 2],
        register: [0; ERROR_CORRECTION_CODEWORDS],
        format: [0; FORMAT_CELLS],
        placing: None,
        dividing: None,
        marked_bits: Vec::new(),
        note: None,
        winner: None,
        unreached: nothing_reached.clone(),
    };

    // The matrix after each of the 26 codewords is placed.
    let mut placed_grids = Vec::with_capacity(TOTAL_CODEWORDS);
    let mut grid = symbol.functions.clone();
    for (index, &word) in content.codewords.iter().enumerate() {
        place_codeword(&mut grid, &symbol.placement, index, word)?;
        placed_grids.push(grid.clone());
    }

    let last_step = encoded.division.last();
    let final_division = last_step.map_or([0; 2], |step| [step.data_byte, step.factor]);
    let final_register = encoded.error_correction;
    let format_modules = format_modules();

    let snapshots = (0..COUNT)
        .map(|index| {
            let stage = Stage::of(index);
            let mut snapshot = DemoSnapshot {
                stage,
                label: String::new(),
                grid: empty.clone(),
                highlights: Vec::new(),
                active_nodes: Vec::new(),
                active_edges: Vec::new(),
                division: [0; 2],
                register: [0; ERROR_CORRECTION_CODEWORDS],
                format: [0; FORMAT_CELLS],
                placing: None,
                dividing: None,
                marked_bits: Vec::new(),
                note: None,
                winner: None,
                unreached: nothing_reached.clone(),
            };
            let u = &mut snapshot.unreached;
            // What has been reached by this position. A node is reached at the position that makes it, and stays so.
            if index >= 1 {
                u.bytes = after(used, BYTE_CELLS);
            }
            if index >= DATA_START {
                u.bitstream = after(shown_bits, BITSTREAM_CELLS);
            }
            if index >= DATA_START {
                u.data = Vec::new();
            }
            if index >= RS_START {
                u.division = Vec::new();
                u.error_correction = Vec::new();
            }
            if index >= FUNCTIONS - 1 {
                u.codewords = Vec::new();
            }
            let scored = if index >= MASK_START { (index - MASK_START + 1).min(MASKS) } else { 0 };
            u.scores = after(scored * SCORE_COLUMNS, SCORE_CELLS);
            if index >= MASK_START {
                u.format = Vec::new();
            }

            match stage {
                Stage::Input => {
                    snapshot.label = format!("Input: {} bytes, {}.", used, escaped(&encoded.input));
                    snapshot.active_nodes = vec![NodeKey::Input];
                },
                Stage::Bytes => {
                    snapshot.label = format!("Bytes: {used} of 17, {}.", hex(&encoded.input));
                    snapshot.active_nodes = vec![NodeKey::Bytes];
                    snapshot.active_edges = vec![EDGES[0]];
                },
                Stage::Bitstream => {
                    let region = Region::ALL[index - BITSTREAM_START];
                    let cells = region.cells(used, encoded.data.terminator_bits);
                    // The bits are shown a part at a time, so the ones after this part are not reached yet.
                    snapshot.unreached.bitstream = after(cells.end, BITSTREAM_CELLS);
                    snapshot.marked_bits = cells.collect();
                    snapshot.note = Some(region.describe(used, encoded.data.terminator_bits));
                    snapshot.label = match region {
                        Region::Mode => "Bitstream, mode: 0100 says that the bytes are in byte mode.".to_owned(),
                        Region::Count => format!("Bitstream, count: {used} in 8 bits is how many bytes follow."),
                        Region::Data => format!("Bitstream, data: the {used} bytes are {} bits.", 8 * used),
                        Region::Terminator => format!(
                            "Bitstream, terminator: {} zero bits end the data, {shown_bits} bits in all.",
                            encoded.data.terminator_bits
                        ),
                    };
                    snapshot.active_nodes = vec![NodeKey::Bitstream];
                    snapshot.active_edges = if region == Region::Mode { vec![EDGES[1]] } else { Vec::new() };
                },
                Stage::DataCodewords => {
                    snapshot.label = format!(
                        "Data codewords: {} used and {} pad codewords make {DATA_CODEWORDS}.",
                        DATA_CODEWORDS - encoded.data.pad_codewords,
                        encoded.data.pad_codewords
                    );
                    snapshot.active_nodes = vec![NodeKey::DataCodewords];
                    snapshot.active_edges = vec![EDGES[2]];
                },
                Stage::ReedSolomon => {
                    let step = &encoded.division[index - RS_START];
                    snapshot.division = [step.data_byte, step.factor];
                    snapshot.dividing = Some(index - RS_START);
                    snapshot.label = format!(
                        "Reed-Solomon step {} of {DATA_CODEWORDS}: the data byte {:02X} gives the factor {:02X}, and \
                         the remainder is {}.",
                        index - RS_START + 1,
                        step.data_byte,
                        step.factor,
                        hex(&step.register)
                    );
                    snapshot.active_nodes = vec![NodeKey::Division, NodeKey::ErrorCorrection];
                    snapshot.active_edges = vec![EDGES[3], EDGES[4]];
                    if index == FUNCTIONS - 1 {
                        snapshot.active_edges.extend([EDGES[5], EDGES[6]]);
                        snapshot.active_nodes.push(NodeKey::Codewords);
                    }
                },
                Stage::FunctionModules => {
                    snapshot.label =
                        "Function modules: finders, separators, timing and the dark module. The 30 format \
                                      modules are reserved."
                            .to_owned();
                    snapshot.grid = symbol.functions.clone();
                    snapshot.highlights = format_modules.clone();
                },
                Stage::Placement => {
                    let word_index = index - PLACEMENT_START;
                    let word = content.codewords[word_index];
                    snapshot.grid = placed_grids[word_index].clone();
                    snapshot.highlights = symbol.placement[word_index * 8..word_index * 8 + 8].to_vec();
                    snapshot.placing = Some(word_index);
                    snapshot.active_nodes = vec![NodeKey::Codewords];
                    snapshot.label = format!(
                        "Codeword {} of {TOTAL_CODEWORDS}, {word:02X}, is placed in 8 modules. It is {}.",
                        word_index + 1,
                        if word_index < DATA_CODEWORDS { "data" } else { "error correction" }
                    );
                },
                Stage::Masks => {
                    let mask = index - MASK_START;
                    let candidate = &symbol.candidates[mask];
                    let p = candidate.penalty;
                    snapshot.grid = candidate.grid.clone();
                    snapshot.highlights = format_modules.clone();
                    snapshot.format = format_cells(candidate.format_bits);
                    snapshot.active_nodes = vec![NodeKey::MaskScores, NodeKey::Format];
                    snapshot.active_edges = vec![EDGES[7], EDGES[8]];
                    snapshot.label = format!(
                        "Mask {mask}: N1 {}, N2 {}, N3 {}, N4 {}, total {}.",
                        p.n1,
                        p.n2,
                        p.n3,
                        p.n4,
                        p.total()
                    );
                },
                Stage::Final => {
                    let chosen = symbol.chosen;
                    let candidate = &symbol.candidates[chosen];
                    snapshot.grid = candidate.grid.clone();
                    snapshot.format = format_cells(candidate.format_bits);
                    snapshot.winner = Some(chosen);
                    snapshot.active_nodes = vec![NodeKey::MaskScores, NodeKey::Format];
                    snapshot.active_edges = vec![EDGES[8]];
                    snapshot.label = format!(
                        "Final symbol: mask {chosen} has the lowest total, {}. The lowest mask wins a tie.",
                        candidate.penalty.total()
                    );
                },
            }
            // From the step that finishes the division, and for ever after, the division node shows its last step.
            if index >= FUNCTIONS {
                snapshot.division = final_division;
                snapshot.register = final_register;
            } else if stage == Stage::ReedSolomon {
                snapshot.register = encoded.division[index - RS_START]
                    .register
                    .as_slice()
                    .try_into()
                    .unwrap_or([0; 7]);
            }
            snapshot
        })
        .collect();

    Ok(SnapshotSet { content, unstarted, snapshots })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The format word of mask `mask`, as cells.
#[cfg(test)]
pub(crate) fn format_cells_of(mask: u8) -> Option<[u8; FORMAT_CELLS]> {
    super::qr_core::format::format_word(mask).map(format_cells)
}
