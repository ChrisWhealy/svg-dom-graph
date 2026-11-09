//! A run: the vector, the product kept up to date as entries of the vector change, and the steps (contract sections 4 and 5).

use crate::hash::vector_hash;
use crate::matrix::Matrix;
use crate::prng::{SplitMix64, stream};
use crate::spec::{Error, Spec};

/// Which entries of x a step changes. The contract's order is [`Random`](Self::Random), drawn from a seeded stream, and it is the only
/// one that the oracle and the hashes know. [`InOrder`](Self::InOrder) is for a person watching: a step changes `x0`, the next `x1`,
/// and so on round the window, so that what comes next can be told. It is never used by a recorded run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateOrder {
    #[default]
    Random,
    InOrder,
}

/// The state of a scenario between steps.
#[derive(Debug, Clone)]
pub struct Run {
    pub matrix: Matrix,
    pub x: Vec<i64>,
    pub y: Vec<i64>,
    /// The number of steps done. Step 0 is the initial state.
    pub step: u64,
    /// The entries of x that the last step set, in order, as `(index, new value)`. A later one of the same index replaces an
    /// earlier one in x, and both are listed.
    pub last_updates: Vec<(usize, i64)>,
    cells: u64,
    updates: u64,
    stream: SplitMix64,
    order: UpdateOrder,
    cursor: usize,
}

impl Run {
    /// The initial state of a scenario: the vector from stream 3 and the product of the matrix with it.
    #[must_use]
    pub fn new(spec: &Spec, matrix: Matrix) -> Self {
        let mut s3 = stream(spec.seed, 3);
        let x: Vec<i64> = (0..matrix.cols).map(|_| s3.between(0, 198) as i64 - 99).collect();
        let y = matrix.multiply(&x);
        Self {
            matrix,
            x,
            y,
            step: 0,
            last_updates: Vec::new(),
            cells: spec.cells,
            updates: spec.updates,
            stream: stream(spec.seed, 4),
            order: UpdateOrder::default(),
            cursor: 0,
        }
    }

    /// Chooses which entries the next steps change. The product is kept exactly either way.
    pub fn set_order(&mut self, order: UpdateOrder) {
        self.order = order;
    }

    /// Applies one step's updates, keeping y incrementally. With `verify`, y is also recomputed in full and compared, which costs a
    /// pass over every nonzero.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Internal`] if the incremental product differs from the full one.
    pub fn advance(&mut self, verify: bool) -> Result<(), Error> {
        self.last_updates.clear();
        for _ in 0..self.updates {
            let j = match self.order {
                UpdateOrder::Random => self.stream.below(self.cells) as usize,
                UpdateOrder::InOrder => {
                    let window = (self.cells as usize).min(self.x.len()).max(1);
                    let j = self.cursor % window;
                    self.cursor = (j + 1) % window;
                    j
                },
            };
            let v = self.stream.between(0, 198) as i64 - 99;
            let delta = v - self.x[j];
            if delta != 0 {
                let Self { matrix, y, .. } = self;
                for k in matrix.col_ptr[j]..matrix.col_ptr[j + 1] {
                    y[matrix.csc_rows[k] as usize] += matrix.csc_vals[k] * delta;
                }
            }
            self.x[j] = v;
            self.last_updates.push((j, v));
        }
        self.step += 1;
        if verify && self.y != self.matrix.multiply(&self.x) {
            return Err(Error::Internal(format!(
                "the incremental product differs from the full one at step {}",
                self.step
            )));
        }
        Ok(())
    }

    /// The hash of x.
    #[must_use]
    pub fn x_hash(&self) -> String {
        vector_hash(&self.x)
    }

    /// The hash of y.
    #[must_use]
    pub fn y_hash(&self) -> String {
        vector_hash(&self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_incremental_product_equals_the_full_one_at_every_step_of_every_panel_seed() {
        for seed in 0..300 {
            let spec = Spec::panel(seed);
            let mut run = Run::new(&spec, Matrix::generate(&spec));
            for _ in 0..spec.steps {
                run.advance(true).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            }
            assert_eq!(run.step, spec.steps);
        }
    }

    /// The same product by the plainest means there is: a dense matrix and two loops. The compressed form must agree with it.
    #[test]
    fn the_product_equals_a_dense_product_made_in_the_plainest_way() {
        for seed in 0..100 {
            let spec = Spec::panel(seed);
            let run = Run::new(&spec, Matrix::generate(&spec));
            let (r, c) = (run.matrix.rows, run.matrix.cols);
            let mut dense = vec![vec![0i64; c]; r];
            for (i, row) in dense.iter_mut().enumerate() {
                for (j, v) in run.matrix.row(i) {
                    row[j] = v;
                }
            }
            let want: Vec<i64> = dense
                .iter()
                .map(|row| row.iter().zip(&run.x).map(|(a, b)| a * b).sum())
                .collect();
            assert_eq!(run.y, want, "seed {seed}");
        }
    }

    #[test]
    fn updates_touch_only_the_visible_window_and_are_listed_in_order() {
        let spec = Spec {
            seed: 5,
            rows: 60,
            cols: 60,
            per_row: 4,
            hub_cols: 0,
            hub_percent: 0,
            label_len: 4,
            cells: 6,
            updates: 9,
            steps: 4,
        };
        let mut run = Run::new(&spec, Matrix::generate(&spec));
        let before = run.x.clone();
        run.advance(true).expect("a step");
        assert_eq!(run.last_updates.len(), 9);
        assert!(run.last_updates.iter().all(|(j, v)| *j < 6 && (-99..=99).contains(v)));
        assert_eq!(run.x[6..], before[6..], "entries outside the window never change");
        let mut replay = before;
        for (j, v) in &run.last_updates {
            replay[*j] = *v;
        }
        assert_eq!(replay, run.x);
    }

    #[test]
    fn in_order_changes_x0_then_x1_and_so_on_round_the_window_and_keeps_the_product_exact() {
        let spec = Spec {
            seed: 5,
            rows: 20,
            cols: 20,
            per_row: 3,
            hub_cols: 0,
            hub_percent: 0,
            label_len: 1,
            cells: 6,
            updates: 1,
            steps: 10,
        };
        let mut run = Run::new(&spec, Matrix::generate(&spec));
        run.set_order(UpdateOrder::InOrder);
        let mut changed = Vec::new();
        for _ in 0..14 {
            run.advance(true).expect("a verified step");
            changed.push(run.last_updates[0].0);
        }
        assert_eq!(changed, vec![0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5, 0, 1]);
    }

    #[test]
    fn the_default_order_is_the_contracts_and_is_not_in_order() {
        let spec = Spec::panel(3);
        let mut run = Run::new(&spec, Matrix::generate(&spec));
        assert_eq!(UpdateOrder::default(), UpdateOrder::Random);
        let mut seen = Vec::new();
        for _ in 0..12 {
            run.advance(false).expect("a step");
            seen.push(run.last_updates[0].0);
        }
        assert!(seen.windows(2).any(|w| w[1] != (w[0] + 1) % 10), "{seen:?}");
    }

    #[test]
    fn a_corrupted_incremental_product_is_reported_and_not_hidden() {
        let spec = Spec::panel(8);
        let mut run = Run::new(&spec, Matrix::generate(&spec));
        run.y[0] += 1;
        assert!(matches!(run.advance(true), Err(Error::Internal(_))));
    }
}
