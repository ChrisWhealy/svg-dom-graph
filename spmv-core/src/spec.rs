//! The specification of a scenario, its ranges, and the two kinds of error: a malformed specification and a refused
//! one.

use crate::budget::Refusal;
use crate::prng::stream;

/// Why a specification was not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A field is outside its range (contract section 1).
    Invalid(String),
    /// The specification is valid and is over a budget (contract section 8).
    Refused(Refusal),
    /// The product kept incrementally differs from a full recomputation, which is a defect and not a bad input.
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(m) | Self::Internal(m) => f.write_str(m),
            Self::Refused(r) => f.write_str(&r.message),
        }
    }
}

impl std::error::Error for Error {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scenario. The fields are those of the table in contract section 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    pub seed: u64,
    pub rows: u64,
    pub cols: u64,
    pub per_row: u64,
    pub hub_cols: u64,
    pub hub_percent: u64,
    pub label_len: u64,
    pub cells: u64,
    pub updates: u64,
    pub steps: u64,
}

fn within(name: &str, v: u64, lo: u64, hi: u64) -> Result<(), Error> {
    if v < lo || v > hi {
        return Err(Error::Invalid(format!("{name} {v} is outside {lo} to {hi}")));
    }
    Ok(())
}

impl Spec {
    /// Checks the ranges, in the order of the table.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] naming the first field that is out of range.
    pub fn validate(&self) -> Result<(), Error> {
        within("rows", self.rows, 1, 1_000_000)?;
        within("cols", self.cols, 1, 1_000_000)?;
        within("per_row", self.per_row, 0, 64)?;
        within("hub_cols", self.hub_cols, 0, self.cols)?;
        within("hub_percent", self.hub_percent, 0, 100)?;
        within("label_len", self.label_len, 1, 100_000)?;
        within("cells", self.cells, 1, self.rows.min(self.cols))?;
        within("updates", self.updates, 0, self.cells)?;
        within("steps", self.steps, 0, 100_000)
    }

    /// The specification of a panel seed (contract section 7).
    #[must_use]
    pub fn panel(seed: u64) -> Self {
        let mut s = stream(seed, 7);
        let rows = s.between(1, 120);
        let cols = s.between(1, 120);
        let per_row = s.between(0, 8);
        let hub_cols = if s.below(2) == 0 { 0 } else { s.between(1, cols) };
        let hub_percent = s.between(0, 100);
        let label_len = s.between(1, 40);
        let cells = s.between(1, rows.min(cols));
        let updates = s.between(0, cells);
        let steps = s.between(1, 12);
        Self {
            seed,
            rows,
            cols,
            per_row,
            hub_cols,
            hub_percent,
            label_len,
            cells,
            updates,
            steps,
        }
    }

    /// The specification from JSON with the fields of the contract's table, each a number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] if a field is missing or is not a non-negative integer.
    pub fn from_json(v: &serde_json::Value) -> Result<Self, Error> {
        let field = |name: &str| {
            v.get(name)
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| Error::Invalid(format!("{name} is missing or is not a non-negative integer")))
        };
        Ok(Self {
            seed: field("seed")?,
            rows: field("rows")?,
            cols: field("cols")?,
            per_row: field("per_row")?,
            hub_cols: field("hub_cols")?,
            hub_percent: field("hub_percent")?,
            label_len: field("label_len")?,
            cells: field("cells")?,
            updates: field("updates")?,
            steps: field("steps")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok() -> Spec {
        Spec {
            seed: 1,
            rows: 10,
            cols: 8,
            per_row: 3,
            hub_cols: 2,
            hub_percent: 50,
            label_len: 5,
            cells: 4,
            updates: 2,
            steps: 3,
        }
    }

    #[test]
    fn a_good_specification_is_valid_and_each_field_has_its_range() {
        assert_eq!(ok().validate(), Ok(()));
        let cases: [(Spec, &str); 9] = [
            (Spec { rows: 0, ..ok() }, "rows 0 is outside 1 to 1000000"),
            (Spec { cols: 1_000_001, ..ok() }, "cols 1000001 is outside 1 to 1000000"),
            (Spec { per_row: 65, ..ok() }, "per_row 65 is outside 0 to 64"),
            (Spec { hub_cols: 9, ..ok() }, "hub_cols 9 is outside 0 to 8"),
            (Spec { hub_percent: 101, ..ok() }, "hub_percent 101 is outside 0 to 100"),
            (Spec { label_len: 0, ..ok() }, "label_len 0 is outside 1 to 100000"),
            (Spec { cells: 9, ..ok() }, "cells 9 is outside 1 to 8"),
            (Spec { updates: 5, ..ok() }, "updates 5 is outside 0 to 4"),
            (Spec { steps: 100_001, ..ok() }, "steps 100001 is outside 0 to 100000"),
        ];
        for (spec, message) in cases {
            assert_eq!(spec.validate(), Err(Error::Invalid(message.to_owned())));
        }
    }

    #[test]
    fn every_panel_seed_is_valid() {
        for seed in 0..2000 {
            assert_eq!(Spec::panel(seed).validate(), Ok(()), "seed {seed}");
        }
    }
}
