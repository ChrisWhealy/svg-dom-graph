//! The scalar fields that the demo contours, and the levels that go with each. Pure: nothing here touches the DOM.
//!
//! A grid is stored row by row, with row 0 at the top. Cell `(r, c)` has its top left corner at the sample `(r, c)`,
//! and a sample is drawn at `x = column`, `y = height - 1 - row`, so the picture has the field's top at the top.

use super::solver::{Grid, SolverError};

/// How many levels there are. Each has a slot: low, middle and high.
pub(crate) const LEVELS: usize = 3;

/// The names of the three slots, and what is drawn for each so that no level relies on colour alone.
pub(crate) const SLOT_NAMES: [&str; LEVELS] = ["low", "middle", "high"];
pub(crate) const SLOT_COLOURS: [&str; LEVELS] = ["#1d4ed8", "#b45309", "#047857"];
/// A solid line, a dashed one and a dotted one.
pub(crate) const SLOT_DASHES: [Option<&str>; LEVELS] = [None, Some("9 5"), Some("2 5")];
pub(crate) const SLOT_PATTERNS: [&str; LEVELS] = ["solid", "dashed", "dotted"];

/// The fields that can be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldId {
    Ripple,
    Twin,
    Coarse,
    /// A single cell, one of the five named test matrices at threshold zero.
    Lab(LabCase),
}

/// The five named matrices of the saddle lab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabCase {
    FivePositive,
    FiveNegative,
    TenPositive,
    TenNegative,
    Tie,
}

impl LabCase {
    pub(crate) const ALL: [LabCase; 5] = [
        Self::FivePositive,
        Self::FiveNegative,
        Self::TenPositive,
        Self::TenNegative,
        Self::Tie,
    ];

    /// The corners, as top left, top right, bottom right and bottom left.
    pub(crate) fn corners(self) -> [f64; 4] {
        match self {
            Self::FivePositive => [3.0, -1.0, 1.0, -2.0],
            Self::FiveNegative => [2.0, -3.0, 1.0, -2.0],
            Self::TenPositive => [-3.0, 1.0, -2.0, 2.0],
            Self::TenNegative => [-1.0, 3.0, -2.0, 1.0],
            Self::Tie => [1.0, -1.0, 1.0, -1.0],
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::FivePositive => "lab-5-positive",
            Self::FiveNegative => "lab-5-negative",
            Self::TenPositive => "lab-10-positive",
            Self::TenNegative => "lab-10-negative",
            Self::Tie => "lab-tie",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::FivePositive => "Lab: case 5, Q positive",
            Self::FiveNegative => "Lab: case 5, Q negative",
            Self::TenPositive => "Lab: case 10, Q positive",
            Self::TenNegative => "Lab: case 10, Q negative",
            Self::Tie => "Lab: exact tie, Q zero",
        }
    }
}

impl FieldId {
    /// Every field, in the order of the selector.
    pub(crate) fn all() -> Vec<FieldId> {
        let mut all = vec![Self::Ripple, Self::Twin, Self::Coarse];
        all.extend(LabCase::ALL.map(Self::Lab));
        all
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Ripple => "ripple",
            Self::Twin => "twin",
            Self::Coarse => "coarse",
            Self::Lab(case) => case.key(),
        }
    }

    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Self::all().into_iter().find(|f| f.key() == key)
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ripple => "Ripple plane",
            Self::Twin => "Twin hills",
            Self::Coarse => "Coarse saddle waves",
            Self::Lab(case) => case.label(),
        }
    }

    /// The levels the field starts with, in ascending order.
    pub(crate) fn levels(self) -> [f64; LEVELS] {
        match self {
            Self::Ripple => [-0.53, 0.101, 0.52],
            Self::Twin => [-0.12, 0.24, 0.66],
            Self::Coarse => [-0.10, 0.10, 0.20],
            Self::Lab(_) => [-0.5, 0.0, 0.5],
        }
    }

    /// The samples of the field.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the grid is not one the solver accepts, which no preset is.
    pub(crate) fn grid(self) -> Result<Grid, SolverError> {
        match self {
            Self::Ripple => sampled(25, 19, |x, y| (1.15 * x).sin() * (1.35 * y).cos() + 0.16 * x - 0.08 * y),
            Self::Twin => sampled(25, 19, |x, y| {
                1.55 * (-((x + 1.25).powi(2) / 1.15 + (y + 0.1).powi(2) / 0.92)).exp()
                    + 1.35 * (-((x - 1.1).powi(2) / 1.0 + (y - 0.25).powi(2) / 1.05)).exp()
                    - 0.28
            }),
            Self::Coarse => {
                let values = (0..7)
                    .flat_map(|r| (0..9).map(move |c| ((c as f64 - 3.5).sin()) * ((2.5 - r as f64).sin())))
                    .collect();
                Grid::new(9, 7, values)
            },
            Self::Lab(case) => {
                let [tl, tr, br, bl] = case.corners();
                Grid::new(2, 2, vec![tl, tr, bl, br])
            },
        }
    }

    /// Whether two fields are drawn on the same axes, so that a zoom and a pan mean the same on both.
    pub(crate) fn same_extent(self, other: Self) -> bool {
        self.size() == other.size()
    }

    /// The width and the height of the grid, which is also the extent of its axes.
    pub(crate) fn size(self) -> (usize, usize) {
        match self {
            Self::Ripple | Self::Twin => (25, 19),
            Self::Coarse => (9, 7),
            Self::Lab(_) => (2, 2),
        }
    }
}

/// A 25 by 19 grid of `f(x, y)`, with `x` from -3 to 3 and `y` from 2.25 at row 0 down to -2.25.
fn sampled(width: usize, height: usize, f: impl Fn(f64, f64) -> f64) -> Result<Grid, SolverError> {
    let values = (0..height)
        .flat_map(|r| {
            let y = 2.25 - 4.5 * r as f64 / (height - 1) as f64;
            let f = &f;
            (0..width).map(move |c| f(-3.0 + 6.0 * c as f64 / (width - 1) as f64, y))
        })
        .collect::<Vec<_>>();
    Grid::new(width, height, values)
}

#[cfg(test)]
mod unit_tests;
