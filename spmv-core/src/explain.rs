//! The arithmetic of one node written out, so that a person who selects a node sees what it computes.
//!
//! Nothing here is hashed or compared with the oracle: it is presentation, derived from the matrix and the vector, and tested
//! natively against them. The sums it states are checked against the product the core keeps, so the text cannot say a number that the
//! product does not hold.

use crate::matrix::Matrix;

/// The most terms written out for one row or column. A longer one says how many more there are.
pub const MAX_TERMS: usize = 6;

fn signed(v: i64) -> String {
    if v < 0 { format!("−{}", -v) } else { v.to_string() }
}

/// A value in brackets when it is negative, so that `5·(−4)` reads as a product.
fn bracketed(v: i64) -> String {
    if v < 0 { format!("({})", signed(v)) } else { v.to_string() }
}

/// The row `i` as an equation: the formula with the matrix's values, then the same with the vector's, then the result.
///
/// ```text
/// y3 = 5·x1 − 2·x7 + 9·x12 = 5·(−4) + (−2)·31 + 9·6 = −28
/// ```
///
/// A row with no nonzeros is zero, and a long row writes `MAX_TERMS` terms and says how many it left out. The result is always the
/// whole sum.
#[must_use]
pub fn row(matrix: &Matrix, x: &[i64], i: usize) -> String {
    let terms: Vec<(usize, i64)> = matrix.row(i).collect();
    if terms.is_empty() {
        return format!("y{i} = 0, since row {i} has no nonzeros");
    }
    let total: i64 = terms.iter().map(|(c, v)| v * x[*c]).sum();
    let shown = &terms[..terms.len().min(MAX_TERMS)];
    let formula = shown
        .iter()
        .enumerate()
        .map(|(n, (c, v))| match (n, *v < 0) {
            (0, false) => format!("{v}·x{c}"),
            (0, true) => format!("−{}·x{c}", -v),
            (_, false) => format!("+ {v}·x{c}"),
            (_, true) => format!("− {}·x{c}", -v),
        })
        .collect::<Vec<_>>()
        .join(" ");
    let values = shown
        .iter()
        .map(|(c, v)| format!("{}·{}", bracketed(*v), bracketed(x[*c])))
        .collect::<Vec<_>>()
        .join(" + ");
    let more = terms.len() - shown.len();
    let tail = if more > 0 { format!(" and {more} more terms") } else { String::new() };
    format!("y{i} = {formula}{tail} = {values}{tail} = {}", signed(total))
}

/// The most rows written out for one column. A column that feeds more says how many more.
pub const MAX_ROWS: usize = 4;

/// The sum of row `i`, as the terms with the numbers in and the total, without its name: `5·(−4) + (−2)·31 = −28`.
fn sum_of(matrix: &Matrix, x: &[i64], i: usize) -> String {
    let terms: Vec<(usize, i64)> = matrix.row(i).collect();
    let total: i64 = terms.iter().map(|(c, v)| v * x[*c]).sum();
    let shown = &terms[..terms.len().min(MAX_TERMS)];
    let values = shown
        .iter()
        .map(|(c, v)| format!("{}·{}", bracketed(*v), bracketed(x[*c])))
        .collect::<Vec<_>>()
        .join(" + ");
    let more = terms.len() - shown.len();
    let tail = if more > 0 { format!(" + {more} more terms") } else { String::new() };
    format!("{values}{tail} = {}", signed(total))
}

/// How the value of x`j` goes into the outputs, one line for each row that uses it: the number of the matrix that it is multiplied
/// by, the product, and the whole sum that the product is one term of.
///
/// ```text
/// x0 = 74 is multiplied by a number of A in each row where column 0 is not blank, and each product is one term of that row's sum:
/// row 0: 6 × 74 = 444, a term of y0 = 6·74 + (−6)·31 = −342
/// ```
#[must_use]
pub fn column_story(matrix: &Matrix, x: &[i64], j: usize) -> String {
    let feeds: Vec<(usize, i64)> = matrix.col(j).collect();
    let xj = signed(x[j]);
    if feeds.is_empty() {
        return format!("x{j} = {xj}. Column {j} of A is blank, so x{j} is not used: it goes into no y.");
    }
    let mut text = format!(
        "x{j} = {xj}. It is multiplied by a number of A in each row where column {j} is not blank, and each product is one term of \
         that row's sum:"
    );
    for (i, v) in feeds.iter().take(MAX_ROWS) {
        text.push_str(&format!(
            "\nrow {i}: {} × {} = {}, a term of y{i} = {}",
            bracketed(*v),
            bracketed(x[j]),
            signed(v * x[j]),
            sum_of(matrix, x, *i)
        ));
    }
    if feeds.len() > MAX_ROWS {
        text.push_str(&format!("\nand {} more rows", feeds.len() - MAX_ROWS));
    }
    text
}

/// What column `j` does: its value, and the rows that it feeds with the weight it has in each.
///
/// ```text
/// x7 = 31 feeds 3 rows: y3 (×−2), y9 (×4), y14 (×1)
/// ```
#[must_use]
pub fn column(matrix: &Matrix, x: &[i64], j: usize) -> String {
    let feeds: Vec<(usize, i64)> = matrix.col(j).collect();
    if feeds.is_empty() {
        return format!("x{j} = {} feeds no row, since column {j} has no nonzeros", signed(x[j]));
    }
    let shown = feeds
        .iter()
        .take(MAX_TERMS)
        .map(|(i, v)| format!("y{i} (×{})", signed(*v)))
        .collect::<Vec<_>>()
        .join(", ");
    let more = feeds.len().saturating_sub(MAX_TERMS);
    let tail = if more > 0 { format!(" and {more} more") } else { String::new() };
    let rows = if feeds.len() == 1 { "row" } else { "rows" };
    format!("x{j} = {} feeds {} {rows}: {shown}{tail}", signed(x[j]), feeds.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The matrix of the hand-written test of `matrix.rs`:
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
    fn a_row_is_written_as_a_formula_then_with_the_values_then_the_result() {
        let x = [1, 10, 100, -1];
        assert_eq!(row(&by_hand(), &x, 0), "y0 = 2·x0 − 1·x2 = 2·1 + (−1)·100 = −98");
        assert_eq!(row(&by_hand(), &x, 2), "y2 = 3·x0 + 4·x1 + 5·x3 = 3·1 + 4·10 + 5·(−1) = 38");
        assert_eq!(row(&by_hand(), &x, 1), "y1 = 0, since row 1 has no nonzeros");
    }

    #[test]
    fn a_leading_negative_term_is_written_with_a_minus_sign() {
        let m = Matrix::from_rows(2, &[vec![(0, -3), (1, 2)]]);
        assert_eq!(row(&m, &[4, -5], 0), "y0 = −3·x0 + 2·x1 = (−3)·4 + 2·(−5) = −22");
    }

    #[test]
    fn a_column_story_follows_the_value_into_each_sum_it_is_a_term_of() {
        let x = [1, 10, 100, -1];
        assert_eq!(
            column_story(&by_hand(), &x, 0),
            "x0 = 1. It is multiplied by a number of A in each row where column 0 is not blank, and each product is one term of \
             that row's sum:\nrow 0: 2 × 1 = 2, a term of y0 = 2·1 + (−1)·100 = −98\nrow 2: 3 × 1 = 3, a term of y2 = 3·1 + 4·10 + \
             5·(−1) = 38"
        );
        let empty = Matrix::from_rows(2, &[vec![(0, 1)]]);
        assert!(column_story(&empty, &[5, -7], 1).contains("is blank, so x1 is not used"));
    }

    #[test]
    fn a_column_story_says_how_many_rows_it_left_out() {
        let rows: Vec<Vec<(u32, i64)>> = (0..7).map(|_| vec![(0, 2)]).collect();
        let text = column_story(&Matrix::from_rows(1, &rows), &[3], 0);
        assert_eq!(text.matches("a term of").count(), MAX_ROWS);
        assert!(text.ends_with("and 3 more rows"), "{text}");
    }

    #[test]
    fn a_column_lists_the_rows_it_feeds_with_their_weights() {
        let x = [1, 10, 100, -1];
        assert_eq!(column(&by_hand(), &x, 0), "x0 = 1 feeds 2 rows: y0 (×2), y2 (×3)");
        assert_eq!(column(&by_hand(), &x, 2), "x2 = 100 feeds 1 row: y0 (×−1)");
        assert_eq!(column(&by_hand(), &[0, 0, 0, 0], 1), "x1 = 0 feeds 1 row: y2 (×4)");
        let empty = Matrix::from_rows(2, &[vec![(0, 1)]]);
        assert_eq!(
            column(&empty, &[5, -7], 1),
            "x1 = −7 feeds no row, since column 1 has no nonzeros"
        );
    }

    /// The result that a row's text states is the number the product holds, for every row of many generated matrices, and a long row
    /// says how many terms it left out.
    #[test]
    fn the_result_written_is_always_the_product_the_core_holds() {
        use crate::run::Run;
        use crate::spec::Spec;
        let mut long_rows = 0;
        for seed in 0..120 {
            let spec = Spec::panel(seed);
            let run = Run::new(&spec, Matrix::generate(&spec));
            for i in 0..run.matrix.rows {
                let text = row(&run.matrix, &run.x, i);
                if run.matrix.row(i).count() == 0 {
                    assert!(text.contains("no nonzeros") && run.y[i] == 0, "seed {seed} row {i}: {text}");
                } else {
                    let stated: i64 = text
                        .rsplit("= ")
                        .next()
                        .expect("a result")
                        .replace('−', "-")
                        .parse()
                        .expect("a number");
                    assert_eq!(stated, run.y[i], "seed {seed} row {i}: {text}");
                }
                if run.matrix.row(i).count() > MAX_TERMS {
                    long_rows += 1;
                    assert!(text.contains("more terms"), "{text}");
                }
            }
        }
        assert!(
            long_rows > 0,
            "no generated row was long, so the truncation was never exercised"
        );
    }

    #[test]
    fn a_long_column_says_how_many_rows_it_left_out() {
        let rows: Vec<Vec<(u32, i64)>> = (0..10).map(|_| vec![(0, 1)]).collect();
        let m = Matrix::from_rows(1, &rows);
        assert_eq!(
            column(&m, &[2], 0),
            "x0 = 2 feeds 10 rows: y0 (×1), y1 (×1), y2 (×1), y3 (×1), y4 (×1), y5 (×1) and 4 more"
        );
    }
}
