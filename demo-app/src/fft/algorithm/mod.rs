//! A radix-2 decimation-in-time fast Fourier transform in plain Rust, recorded stage by stage so that the demo can show
//! every one. Nothing here touches the DOM or the library's drawing.
//!
//! The transform is the forward, unnormalised one: `X[k] = Σ x[n]·e^(−2πikn/N)`, with no `1/N` anywhere. The input is
//! put into bit-reversed order first, then `log₂ N` stages follow, and the output is in natural order. Stage `s` joins
//! blocks of `m = 2^s` lanes, and every block holds `m/2` butterflies. Butterfly `j` of a block combines the lanes `j`
//! and `j + m/2` with the twiddle factor `W_m^j = e^(−2πij/m)`.
//!
//! The arithmetic is [`ComplexBuf::butterfly`], called once for each whole block. [`PairTrace`] records the numbers of
//! each butterfly from the state before the block is processed, worked out with the scalar complex operations. They are
//! then checked against the buffer, which is the authority.
//!
//! Stages are numbered from 1, and stage 0 is the bit-reversed input. So [`FftTrace::states`] holds `L + 1` states.
//! [`FftTrace::pairs`] holds `L` lists of butterflies, and the list for stage `s` is at index `s − 1`.

use demo_common::complex::{Complex, ComplexBuf};
use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why an input, or a run on it, was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Error {
    /// The number of samples is below 2, or not a power of two. A length of 1 has no butterfly to show.
    BadLength(usize),
    /// A sample has a part that is infinite or not a number.
    NotFinite { index: usize },
    /// The twiddle factors of a stage could not be made.
    Twiddles { order: usize },
    /// A butterfly worked out in scalar arithmetic differs from the buffer's lane, by more than rounding explains.
    Disagreement { stage: usize, lane: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadLength(n) => write!(f, "{n} samples: a transform needs a power of two, and at least 2"),
            Self::NotFinite { index } => write!(f, "sample {index} is not a finite number"),
            Self::Twiddles { order } => write!(f, "the twiddle factors of order {order} could not be made"),
            Self::Disagreement { stage, lane } => {
                write!(f, "stage {stage}, lane {lane}: the butterfly and the buffer disagree")
            },
        }
    }
}

impl std::error::Error for Error {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sizes the demo offers.
pub(super) const SIZES: [usize; 2] = [8, 16];

/// How closely a butterfly worked out one number at a time must agree with the buffer, relative to the larger of 1 and
/// the value. Both do the same operations in the same order, so only a bug could separate them.
const AGREEMENT: f64 = 1e-9;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Fixed signals for the tests: the demo's own input is the sum of three waves.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Preset {
    /// `x[1] = 1`, and every other sample zero. Its transform is the twiddle factors themselves.
    Impulse,
    /// Every sample `1`. All the energy lands in bin 0.
    Constant,
    /// `cos(2πn/N)`, a wave that fits the window exactly once. Bins 1 and `N − 1` each hold `N/2`.
    Cosine,
    /// Samples with positive and negative real and imaginary parts, so that every kind of product happens.
    Mixed,
}

#[cfg(test)]
impl Preset {
    /// Every preset, in the order the page lists them.
    pub(super) const ALL: [Self; 4] = [Self::Impulse, Self::Constant, Self::Cosine, Self::Mixed];

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The value of the page's list that stands for this preset.
    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Impulse => "impulse",
            Self::Constant => "constant",
            Self::Cosine => "cosine",
            Self::Mixed => "mixed",
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The preset that a page value names.
    pub(super) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| preset.key() == key)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `n` samples of this signal.
    pub(super) fn samples(self, n: usize) -> Vec<Complex> {
        (0..n)
            .map(|i| match self {
                Self::Impulse => Complex::from_real(f64::from(u8::from(i == 1))),
                Self::Constant => Complex::ONE,
                Self::Cosine => Complex::from_real((std::f64::consts::TAU * i as f64 / n as f64).cos()),
                Self::Mixed => {
                    // The remainders are taken as integers, then turned into numbers.
                    let re = 0.5 * ((i % 5) as f64 - 2.0);
                    let im = -0.25 * ((i % 7) as f64 - 3.0);
                    Complex::new(re, im)
                },
            })
            .collect()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One butterfly of one stage: where it is, what went in and what came out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PairTrace {
    /// The stage, from 1.
    pub(super) stage: usize,
    /// The first lane of the block this butterfly is in.
    pub(super) start: usize,
    /// The position in the block. It decides the twiddle factor, `W_m^j`.
    pub(super) j: usize,
    /// The lane of `a`, which receives `sum`.
    pub(super) upper: usize,
    /// The lane of `b`, which receives `difference`.
    pub(super) lower: usize,
    pub(super) a: Complex,
    pub(super) b: Complex,
    pub(super) w: Complex,
    /// `w·b`
    pub(super) product: Complex,
    /// `a + w·b`
    pub(super) sum: Complex,
    /// `a − w·b`
    pub(super) difference: Complex,
}

impl PairTrace {
    /// The width of the block this butterfly is in: `2^stage`.
    pub(super) const fn block_width(&self) -> usize {
        1 << self.stage
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A whole run: the input, every state and every butterfly. It is never changed after it is made.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FftTrace {
    pub(super) input: Vec<Complex>,
    /// `bit_reversed[i]` is the input sample that lane `i` starts with.
    pub(super) bit_reversed: Vec<usize>,
    /// The lanes before the first stage, and after each stage. The last is the transform.
    pub(super) states: Vec<Vec<Complex>>,
    /// The butterflies of each stage, in the order [`stage_pairs`] lists them.
    pub(super) pairs: Vec<Vec<PairTrace>>,
}

impl FftTrace {
    /// The number of lanes.
    pub(super) fn lanes(&self) -> usize {
        self.input.len()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The number of stages, `log₂ N`.
    pub(super) fn stages(&self) -> usize {
        self.pairs.len()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The butterflies of stage `stage`, counted from 1.
    pub(super) fn stage_pairs(&self, stage: usize) -> &[PairTrace] {
        stage
            .checked_sub(1)
            .and_then(|index| self.pairs.get(index))
            .map_or(&[], Vec::as_slice)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checks that `n` is a length the transform handles, and returns `log₂ n`.
fn log2_checked(n: usize) -> Result<u32, Error> {
    if n < 2 || !n.is_power_of_two() {
        return Err(Error::BadLength(n));
    }
    Ok(n.trailing_zeros())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The input sample that each of `n` lanes starts with: the index with its `log₂ n` bits reversed.
///
/// # Errors
///
/// Returns [`Error::BadLength`] unless `n` is a power of two of at least 2. The shift that reverses the bits is by
/// `usize::BITS − log₂ n`, which is the width of the word for `n = 1`, so that length is refused first.
pub(super) fn bit_reversal(n: usize) -> Result<Vec<usize>, Error> {
    let bits = log2_checked(n)?;
    Ok((0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The butterflies of one stage in the order the demo uses everywhere: `(upper, lower, j, start)`, block after block,
/// and `j` within a block.
pub(super) fn pair_lanes(n: usize, stage: usize) -> Vec<(usize, usize, usize, usize)> {
    let half = 1_usize << (stage - 1);
    (0..n)
        .step_by(2 * half)
        .flat_map(|start| (0..half).map(move |j| (start + j, start + j + half, j, start)))
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the first butterfly with an input that is not zero (signed zeros are zero), or 0 if there is none.
pub(super) fn first_informative_pair(pairs: &[PairTrace]) -> usize {
    let nonzero = |z: Complex| z.re != 0.0 || z.im != 0.0;
    pairs.iter().position(|pair| nonzero(pair.a) || nonzero(pair.b)).unwrap_or(0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs the forward transform of `input`, keeping every state and every butterfly.
///
/// # Errors
///
/// Returns `Err` for a length below 2 or not a power of two, and for a sample that is not finite. It also returns `Err`
/// if the butterflies worked out one at a time disagree with the buffer. Nothing is drawn before this returns.
pub(super) fn trace(input: &[Complex]) -> Result<FftTrace, Error> {
    let n = input.len();
    let bits = log2_checked(n)?;
    if let Some(index) = input.iter().position(|z| !z.is_finite()) {
        return Err(Error::NotFinite { index });
    }

    let order = bit_reversal(n)?;
    let reordered: Vec<Complex> = order.iter().map(|&i| input[i]).collect();
    let mut data = ComplexBuf::from_complex(&reordered);
    let mut states = vec![reordered];
    let mut pairs = Vec::with_capacity(bits as usize);

    for stage in 1..=bits as usize {
        let half = 1_usize << (stage - 1);
        let m = 2 * half;
        let mut twiddles = ComplexBuf::unit_roots(m, half).map_err(|_| Error::Twiddles { order: m })?;
        twiddles.conj();

        let before = data.to_complex();
        let mut stage_pairs = Vec::with_capacity(n / 2);
        for (upper, lower, j, start) in pair_lanes(n, stage) {
            let (a, b, w) = (before[upper], before[lower], twiddles.get(j));
            let product = w * b;
            let (sum, difference) = a.sum_and_difference(product);
            stage_pairs.push(PairTrace {
                stage,
                start,
                j,
                upper,
                lower,
                a,
                b,
                w,
                product,
                sum,
                difference,
            });
        }
        // Each block once, never each pair.
        for start in (0..n).step_by(m) {
            data.butterfly(start, half, &twiddles);
        }

        let after = data.to_complex();
        for pair in &stage_pairs {
            for (lane, expected) in [(pair.upper, pair.sum), (pair.lower, pair.difference)] {
                if !agree(after[lane], expected) {
                    return Err(Error::Disagreement { stage, lane });
                }
            }
        }
        states.push(after);
        pairs.push(stage_pairs);
    }

    Ok(FftTrace {
        input: input.to_vec(),
        bit_reversed: order,
        states,
        pairs,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether two numbers are the same, to within rounding.
fn agree(got: Complex, expected: Complex) -> bool {
    (got - expected).norm() <= AGREEMENT * expected.norm().max(1.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
