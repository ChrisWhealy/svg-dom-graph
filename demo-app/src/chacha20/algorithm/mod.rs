//! ChaCha20 (RFC 8439) in plain Rust, split into the same named pieces the demo draws.
//!
//! The cipher is built from one small function, the quarter round. It mixes four 32-bit words, `a`, `b`, `c` and `d`,
//! with three operations: addition, XOR and rotation. [`quarter_round_trace`] records every one of its 12 steps, so the
//! demo can draw each of them. [`quarter_round`], [`block`] and [`encrypt`] are built on that same function.
//!
//! The tests check every function against the values the RFC publishes.
//!
//! # Scope
//!
//! The 256-bit key and 96-bit nonce of RFC 8439, with a 32-bit block counter. The original ChaCha of Bernstein, with
//! its 64-bit nonce and 64-bit counter, is not covered.

/// The four words that start every ChaCha20 state: the ASCII text `expand 32-byte k`, as little-endian words.
pub(super) const CONSTANTS: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

/// Words in the state: a 4 by 4 matrix.
pub(super) const STATE_WORDS: usize = 16;

/// Bytes in a key.
pub(super) const KEY_LEN: usize = 32;

/// Bytes in a nonce.
pub(super) const NONCE_LEN: usize = 12;

/// Bytes in one block of keystream.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) const BLOCK_LEN: usize = 64;

/// Column rounds and diagonal rounds come in pairs, and ChaCha20 runs ten pairs: twenty rounds.
const DOUBLE_ROUNDS: usize = 10;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One of the four words a quarter round mixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Word {
    A,
    B,
    C,
    D,
}

impl Word {
    /// All four words, in the order the diagram lays its columns out.
    pub(super) const ALL: [Word; 4] = [Word::A, Word::B, Word::C, Word::D];

    /// This word's place in a quarter round's input and output.
    pub(super) const fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
            Self::C => 2,
            Self::D => 3,
        }
    }

    /// The word's name as the RFC writes it.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
            Self::C => "c",
            Self::D => "d",
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What one step of a quarter round does to the word it updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operation {
    /// Adds another word, discarding any carry out of 32 bits.
    Add(Word),
    /// XORs in another word.
    Xor(Word),
    /// Rotates the word left by this many bits.
    RotateLeft(u8),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The 12 steps of a quarter round, in order: the word each updates, and how.
///
/// The RFC writes them as `a += b; d ^= a; d <<<= 16; c += d; b ^= c; b <<<= 12; a += b; d ^= a; d <<<= 8; c += d;
/// b ^= c; b <<<= 7`.
pub(super) const STEPS: [(Word, Operation); 12] = [
    (Word::A, Operation::Add(Word::B)),
    (Word::D, Operation::Xor(Word::A)),
    (Word::D, Operation::RotateLeft(16)),
    (Word::C, Operation::Add(Word::D)),
    (Word::B, Operation::Xor(Word::C)),
    (Word::B, Operation::RotateLeft(12)),
    (Word::A, Operation::Add(Word::B)),
    (Word::D, Operation::Xor(Word::A)),
    (Word::D, Operation::RotateLeft(8)),
    (Word::C, Operation::Add(Word::D)),
    (Word::B, Operation::Xor(Word::C)),
    (Word::B, Operation::RotateLeft(7)),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One step of a quarter round, with the value it produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Step {
    /// The word this step updates.
    pub(super) word: Word,
    /// What the step does.
    pub(super) operation: Operation,
    /// The word's value after the step.
    pub(super) result: u32,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every step of one quarter round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct QuarterRoundTrace {
    /// The words `a`, `b`, `c` and `d` going in.
    pub(super) input: [u32; 4],
    /// The 12 steps, in order.
    pub(super) steps: [Step; 12],
    /// The words `a`, `b`, `c` and `d` coming out.
    pub(super) output: [u32; 4],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs one quarter round on `input`, which holds `a`, `b`, `c` and `d` in that order, recording each step.
pub(super) fn quarter_round_trace(input: [u32; 4]) -> QuarterRoundTrace {
    let mut words = input;
    let mut steps = [Step {
        word: Word::A,
        operation: Operation::RotateLeft(0),
        result: 0,
    }; 12];

    for (step, &(word, operation)) in steps.iter_mut().zip(STEPS.iter()) {
        let current = words[word.index()];
        let result = match operation {
            Operation::Add(other) => current.wrapping_add(words[other.index()]),
            Operation::Xor(other) => current ^ words[other.index()],
            Operation::RotateLeft(bits) => current.rotate_left(u32::from(bits)),
        };
        words[word.index()] = result;
        *step = Step { word, operation, result };
    }

    QuarterRoundTrace { input, steps, output: words }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The words of a state that each of the four column rounds mixes: the four columns of the 4 by 4 matrix.
pub(super) const COLUMNS: [[usize; 4]; 4] = [[0, 4, 8, 12], [1, 5, 9, 13], [2, 6, 10, 14], [3, 7, 11, 15]];

/// The words of a state that each of the four diagonal rounds mixes: the four diagonals of the 4 by 4 matrix.
pub(super) const DIAGONALS: [[usize; 4]; 4] = [[0, 5, 10, 15], [1, 6, 11, 12], [2, 7, 8, 13], [3, 4, 9, 14]];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads `bytes` as little-endian 32-bit words.
fn le_words<const N: usize>(bytes: &[u8]) -> [u32; N] {
    let mut words = [0_u32; N];
    for (word, chunk) in words.iter_mut().zip(bytes.chunks_exact(4)) {
        *word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    words
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The state before any round has run: the constants, the key, the block counter and the nonce.
pub(super) fn initial_state(key: &[u8; KEY_LEN], counter: u32, nonce: &[u8; NONCE_LEN]) -> [u32; STATE_WORDS] {
    let key_words: [u32; 8] = le_words(key);
    let nonce_words: [u32; 3] = le_words(nonce);

    let mut state = [0_u32; STATE_WORDS];
    state[0..4].copy_from_slice(&CONSTANTS);
    state[4..12].copy_from_slice(&key_words);
    state[12] = counter;
    state[13..16].copy_from_slice(&nonce_words);
    state
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The state after 20 rounds, before the initial state is added back in.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn run_rounds(initial: &[u32; STATE_WORDS]) -> [u32; STATE_WORDS] {
    rounds(initial).mixed
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The block function: 20 rounds, then each word added to the word it started as.
///
/// Without that last addition, the rounds alone could be run backwards, and the key recovered.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn block(initial: &[u32; STATE_WORDS]) -> [u32; STATE_WORDS] {
    rounds(initial).keystream
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which words of the state a round mixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RoundKind {
    /// The four columns of the 4 by 4 matrix. Every odd round is a column round.
    Column,
    /// The four diagonals of the matrix. Every even round is a diagonal round.
    Diagonal,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One quarter round of the 20 rounds: which state it mixes and what it does to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct QuarterRoundStage {
    /// The round this quarter round belongs to, from 1 to [`ROUNDS`].
    pub(super) round: usize,
    /// Whether that round mixes columns or diagonals.
    pub(super) kind: RoundKind,
    /// Which of the round's four quarter rounds this is, from 1 to [`QUARTER_ROUNDS_PER_ROUND`].
    pub(super) number: usize,
    /// The positions in the state of the words `a`, `b`, `c` and `d`.
    pub(super) indices: [usize; 4],
    /// The whole state before this quarter round.
    pub(super) before: [u32; STATE_WORDS],
    /// Every step of the quarter round, run on the four words at `indices`.
    pub(super) trace: QuarterRoundTrace,
    /// The whole state after this quarter round: `before`, with those four words replaced.
    pub(super) after: [u32; STATE_WORDS],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every quarter round of ChaCha20, in order, and the state they lead to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Rounds {
    /// The state before any round has run.
    pub(super) initial: [u32; STATE_WORDS],
    /// The [`QUARTER_ROUNDS`] quarter rounds, each starting from the state the one before it left.
    pub(super) stages: Vec<QuarterRoundStage>,
    /// The state after the last quarter round.
    pub(super) mixed: [u32; STATE_WORDS],
    /// `mixed`, with each word added to the word of `initial` it started as: one block of keystream.
    pub(super) keystream: [u32; STATE_WORDS],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The 20 rounds of ChaCha20.
pub(super) const ROUNDS: usize = DOUBLE_ROUNDS * 2;

/// Every round mixes the state in four quarter rounds.
pub(super) const QUARTER_ROUNDS_PER_ROUND: usize = 4;

/// The quarter rounds in all 20 rounds.
pub(super) const QUARTER_ROUNDS: usize = ROUNDS * QUARTER_ROUNDS_PER_ROUND;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs all 20 rounds on `initial`, recording each quarter round.
///
/// This is the one place the rounds are run. [`run_rounds`] and [`block`] read their results from it.
pub(super) fn rounds(initial: &[u32; STATE_WORDS]) -> Rounds {
    let mut state = *initial;
    let mut stages = Vec::with_capacity(QUARTER_ROUNDS);

    for round in 1..=ROUNDS {
        let (kind, groups) = if round % 2 == 1 {
            (RoundKind::Column, COLUMNS)
        } else {
            (RoundKind::Diagonal, DIAGONALS)
        };
        for (index, indices) in groups.into_iter().enumerate() {
            let before = state;
            let [a, b, c, d] = indices;
            let trace = quarter_round_trace([state[a], state[b], state[c], state[d]]);
            for (position, word) in indices.into_iter().zip(trace.output) {
                state[position] = word;
            }
            stages.push(QuarterRoundStage {
                round,
                kind,
                number: index + 1,
                indices,
                before,
                trace,
                after: state,
            });
        }
    }

    let mut keystream = state;
    for (word, start) in keystream.iter_mut().zip(initial) {
        *word = word.wrapping_add(*start);
    }
    Rounds {
        initial: *initial,
        stages,
        mixed: state,
        keystream,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One block of keystream: the block function's words, written as little-endian bytes.
///
/// The demo draws the block function's words, not these bytes, so only the tests call it.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn keystream_block(initial: &[u32; STATE_WORDS]) -> [u8; BLOCK_LEN] {
    let mut bytes = [0_u8; BLOCK_LEN];
    for (chunk, word) in bytes.chunks_exact_mut(4).zip(block(initial)) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Encrypts, or decrypts, `data`: each byte is XOR'ed with the keystream, which starts at block `counter`.
///
/// Both directions are the same operation, because XOR'ing the same keystream in twice restores the original.
///
/// Returns `None` if `data` needs a block whose counter would pass `u32::MAX`. RFC 8439 allows a key and nonce no more
/// than 2^32 blocks, and wrapping to zero would repeat a keystream block.
///
/// The demo draws one quarter round, not the XOR with a message, so only the tests call this.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn encrypt(key: &[u8; KEY_LEN], counter: u32, nonce: &[u8; NONCE_LEN], data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    for (index, chunk) in data.chunks(BLOCK_LEN).enumerate() {
        let block_counter = u32::try_from(index).ok().and_then(|offset| counter.checked_add(offset))?;
        let keystream = keystream_block(&initial_state(key, block_counter, nonce));
        out.extend(chunk.iter().zip(keystream).map(|(byte, key_byte)| byte ^ key_byte));
    }
    Some(out)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
