//! The sparse matrix, in compressed form by rows and by columns, and its generation from a seed (contract sections 3 and 6).

use crate::hash::Writer;
use crate::prng::stream;
use crate::spec::Spec;

/// The counts that a budget and a report need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub nnz: u64,
    pub max_row_degree: u64,
    pub max_col_degree: u64,
    pub empty_rows: u64,
    pub empty_cols: u64,
}

/// A sparse matrix of integers. The rows are kept in compressed form, in ascending column order, and so are the columns, so that a
/// change of one entry of the vector reaches the rows it affects without a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub row_ptr: Vec<usize>,
    pub col_idx: Vec<u32>,
    pub vals: Vec<i64>,
    pub col_ptr: Vec<usize>,
    pub csc_rows: Vec<u32>,
    pub csc_vals: Vec<i64>,
}

impl Matrix {
    /// Builds the matrix from rows that are already sorted and without repeats, as `(column, value)` pairs.
    #[must_use]
    pub fn from_rows(cols: usize, rows: &[Vec<(u32, i64)>]) -> Self {
        let mut row_ptr = vec![0];
        let (mut col_idx, mut vals) = (Vec::new(), Vec::new());
        for row in rows {
            for (c, v) in row {
                col_idx.push(*c);
                vals.push(*v);
            }
            row_ptr.push(col_idx.len());
        }
        let mut count = vec![0usize; cols + 1];
        for c in &col_idx {
            count[*c as usize + 1] += 1;
        }
        for j in 0..cols {
            count[j + 1] += count[j];
        }
        let col_ptr = count.clone();
        let mut next = count;
        let (mut csc_rows, mut csc_vals) = (vec![0u32; col_idx.len()], vec![0i64; col_idx.len()]);
        for (i, row) in rows.iter().enumerate() {
            for (c, v) in row {
                let at = next[*c as usize];
                csc_rows[at] = u32::try_from(i).unwrap_or(u32::MAX);
                csc_vals[at] = *v;
                next[*c as usize] += 1;
            }
        }
        Self {
            rows: rows.len(),
            cols,
            row_ptr,
            col_idx,
            vals,
            col_ptr,
            csc_rows,
            csc_vals,
        }
    }

    /// Generates the matrix of a specification. The specification has to be valid and within a budget: nothing here limits the
    /// size, and a hostile one is for [`crate::budget::check_spec`] to refuse first.
    #[must_use]
    pub fn generate(spec: &Spec) -> Self {
        let (rows, cols) = (spec.rows as usize, spec.cols as usize);
        let (mut columns, mut values) = (stream(spec.seed, 1), stream(spec.seed, 2));
        let mut table: Vec<Vec<(u32, i64)>> = Vec::with_capacity(rows);
        for _ in 0..rows {
            let mut chosen: Vec<u32> = Vec::new();
            for _ in 0..spec.per_row {
                let u = columns.below(100);
                let c = if u < spec.hub_percent && spec.hub_cols > 0 {
                    columns.below(spec.hub_cols)
                } else {
                    columns.below(spec.cols)
                };
                let c = u32::try_from(c).unwrap_or(u32::MAX);
                if !chosen.contains(&c) {
                    chosen.push(c);
                }
            }
            chosen.sort_unstable();
            let row = chosen
                .into_iter()
                .map(|c| {
                    let v = values.between(1, 9) as i64;
                    let negative = values.below(2) == 1;
                    (c, if negative { -v } else { v })
                })
                .collect();
            table.push(row);
        }
        Self::from_rows(cols, &table)
    }

    #[must_use]
    pub fn nnz(&self) -> usize {
        self.col_idx.len()
    }

    #[must_use]
    pub fn stats(&self) -> Stats {
        let row_deg = |i: usize| (self.row_ptr[i + 1] - self.row_ptr[i]) as u64;
        let col_deg = |j: usize| (self.col_ptr[j + 1] - self.col_ptr[j]) as u64;
        Stats {
            nnz: self.nnz() as u64,
            max_row_degree: (0..self.rows).map(row_deg).max().unwrap_or(0),
            max_col_degree: (0..self.cols).map(col_deg).max().unwrap_or(0),
            empty_rows: (0..self.rows).filter(|i| row_deg(*i) == 0).count() as u64,
            empty_cols: (0..self.cols).filter(|j| col_deg(*j) == 0).count() as u64,
        }
    }

    /// The nonzeros of row `i`, as `(column, value)`.
    pub fn row(&self, i: usize) -> impl Iterator<Item = (usize, i64)> + '_ {
        (self.row_ptr[i]..self.row_ptr[i + 1]).map(|k| (self.col_idx[k] as usize, self.vals[k]))
    }

    /// The nonzeros of column `j`, as `(row, value)`.
    pub fn col(&self, j: usize) -> impl Iterator<Item = (usize, i64)> + '_ {
        (self.col_ptr[j]..self.col_ptr[j + 1]).map(|k| (self.csc_rows[k] as usize, self.csc_vals[k]))
    }

    /// The position of the nonzero at row `i` and column `j` among all the nonzeros in row order, which is the order a scene makes its
    /// edges in. `None` if that entry is zero.
    #[must_use]
    pub fn edge_index(&self, i: usize, j: usize) -> Option<usize> {
        let range = self.row_ptr[i]..self.row_ptr[i + 1];
        let at = self.col_idx[range.clone()].binary_search(&u32::try_from(j).ok()?).ok()?;
        Some(range.start + at)
    }

    /// The full product `A·x`.
    #[must_use]
    pub fn multiply(&self, x: &[i64]) -> Vec<i64> {
        (0..self.rows).map(|i| self.row(i).map(|(c, v)| v * x[c]).sum()).collect()
    }

    /// The hash of the matrix (contract section 6).
    #[must_use]
    pub fn hash(&self) -> String {
        let mut w = Writer::new();
        w.u8(1).u64(self.rows as u64).u64(self.cols as u64).u64(self.nnz() as u64);
        for i in 0..self.rows {
            w.u32(u32::try_from(self.row_ptr[i + 1] - self.row_ptr[i]).unwrap_or(u32::MAX));
            for (c, v) in self.row(i) {
                w.u64(c as u64).i64(v);
            }
        }
        w.digest()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The label of a node (contract section 7): `x` and the column, or `y` and the row, padded on the right with `-` to `len`.
#[must_use]
pub fn label(kind: char, index: usize, len: usize) -> String {
    let mut text = format!("{kind}{index}");
    while text.len() < len {
        text.push('-');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A matrix written out by hand: the product, the columns, the degrees and the hash all follow from it.
    ///
    /// ```text
    ///       c0  c1  c2  c3
    ///  r0 [  2   0  -1   0 ]
    ///  r1 [  0   0   0   0 ]
    ///  r2 [  3   4   0   5 ]
    /// ```
    fn by_hand() -> Matrix {
        Matrix::from_rows(4, &[vec![(0, 2), (2, -1)], vec![], vec![(0, 3), (1, 4), (3, 5)]])
    }

    #[test]
    fn a_matrix_written_by_hand_multiplies_and_counts_as_it_should() {
        let m = by_hand();
        let x = [1, 10, 100, -1];
        // r0 = 2*1 + -1*100 = -98. r1 = 0. r2 = 3*1 + 4*10 + 5*-1 = 38.
        assert_eq!(m.multiply(&x), vec![-98, 0, 38]);
        assert_eq!(
            m.stats(),
            Stats {
                nnz: 5,
                max_row_degree: 3,
                max_col_degree: 2,
                empty_rows: 1,
                empty_cols: 0
            }
        );
        assert_eq!(
            m.col(0).collect::<Vec<_>>(),
            vec![(0, 2), (2, 3)],
            "column 0 in ascending row order"
        );
        assert_eq!(m.col(2).collect::<Vec<_>>(), vec![(0, -1)]);
    }

    #[test]
    fn the_compressed_columns_hold_exactly_the_nonzeros_of_the_rows() {
        for seed in 0..50 {
            let m = Matrix::generate(&Spec::panel(seed));
            let mut from_rows: Vec<(usize, usize, i64)> =
                (0..m.rows).flat_map(|i| m.row(i).map(move |(c, v)| (i, c, v))).collect();
            let mut from_cols: Vec<(usize, usize, i64)> =
                (0..m.cols).flat_map(|j| m.col(j).map(move |(i, v)| (i, j, v))).collect();
            from_rows.sort_unstable();
            from_cols.sort_unstable();
            assert_eq!(from_rows, from_cols, "seed {seed}");
        }
    }

    #[test]
    fn a_generated_matrix_obeys_the_contract() {
        for seed in 0..200 {
            let spec = Spec::panel(seed);
            let m = Matrix::generate(&spec);
            assert_eq!((m.rows as u64, m.cols as u64), (spec.rows, spec.cols));
            for i in 0..m.rows {
                let row: Vec<(usize, i64)> = m.row(i).collect();
                assert!(
                    row.len() as u64 <= spec.per_row,
                    "seed {seed}: row {i} has more than per_row nonzeros"
                );
                assert!(
                    row.windows(2).all(|w| w[0].0 < w[1].0),
                    "seed {seed}: row {i} is not strictly ascending"
                );
                assert!(
                    row.iter().all(|(c, v)| *c < m.cols && (1..=9).contains(&v.abs())),
                    "seed {seed}: row {i}"
                );
            }
        }
    }

    #[test]
    fn hubs_get_more_edges_than_other_columns() {
        let base = Spec {
            seed: 3,
            rows: 400,
            cols: 400,
            per_row: 6,
            hub_cols: 4,
            hub_percent: 0,
            label_len: 4,
            cells: 4,
            updates: 1,
            steps: 1,
        };
        let flat = Matrix::generate(&base).stats().max_col_degree;
        let heavy = Matrix::generate(&Spec { hub_percent: 80, ..base }).stats().max_col_degree;
        assert!(heavy > 4 * flat, "hub-heavy {heavy} against flat {flat}");
    }

    #[test]
    fn the_same_seed_gives_the_same_matrix_and_another_seed_another() {
        let s = Spec::panel(11);
        assert_eq!(Matrix::generate(&s).hash(), Matrix::generate(&s).hash());
        assert_ne!(Matrix::generate(&s).hash(), Matrix::generate(&Spec { seed: 12, ..s }).hash());
    }

    #[test]
    fn an_edge_index_is_the_position_in_row_order_and_none_for_a_zero() {
        let m = by_hand();
        // The nonzeros in row order: (0,0) (0,2) (2,0) (2,1) (2,3).
        assert_eq!(
            [(0, 0), (0, 2), (2, 0), (2, 1), (2, 3)].map(|(i, j)| m.edge_index(i, j)),
            [Some(0), Some(1), Some(2), Some(3), Some(4)]
        );
        assert_eq!([(0, 1), (1, 0), (2, 2)].map(|(i, j)| m.edge_index(i, j)), [None, None, None]);
        for seed in 0..40 {
            let m = Matrix::generate(&Spec::panel(seed));
            let all: Vec<usize> = (0..m.rows)
                .flat_map(|i| m.row(i).map(move |(j, _)| (i, j)))
                .map(|(i, j)| m.edge_index(i, j).expect("a nonzero"))
                .collect();
            assert_eq!(all, (0..m.nnz()).collect::<Vec<_>>(), "seed {seed}");
        }
    }

    #[test]
    fn labels_are_padded_and_never_cut() {
        assert_eq!(label('x', 7, 6), "x7----");
        assert_eq!(label('y', 12345, 3), "y12345");
        assert_eq!(label('x', 0, 2).len(), 2);
    }
}
