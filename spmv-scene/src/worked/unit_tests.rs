use super::*;

/// The terms of a row of a matrix of eight columns: the column and the number of A.
fn row(n: usize) -> Vec<(usize, i64)> {
    (0..n).map(|k| (k, k as i64 + 1)).collect()
}

const X: [i64; 12] = [10, -20, 30, -40, 50, -60, 70, -80, 90, -100, 110, -120];

#[test]
fn a_short_row_shows_every_term_and_adds_them_one_after_another() {
    let sum = sum_of(&row(3), &X);
    assert_eq!(sum.shown, vec![(0, 1, 10), (1, 2, -40), (2, 3, 90)]);
    assert_eq!(sum.rest, None);
    assert_eq!(sum.running, vec![10, -30, 60]);
}

#[test]
fn a_row_of_one_term_has_no_addition() {
    let sum = sum_of(&row(1), &X);
    assert_eq!(sum.running, vec![10]);
}

#[test]
fn a_row_of_no_terms_adds_up_to_nothing() {
    let sum = sum_of(&[], &X);
    assert!(sum.shown.is_empty() && sum.rest.is_none() && sum.running.is_empty());
}

#[test]
fn a_long_row_folds_the_terms_past_the_limit_into_one_that_keeps_the_sum_whole() {
    let terms = row(MAX_TERMS + 3);
    let sum = sum_of(&terms, &X);
    assert_eq!(sum.shown.len(), MAX_TERMS);
    let folded: i64 = terms[MAX_TERMS..].iter().map(|(c, v)| v * X[*c]).sum();
    assert_eq!(sum.rest, Some((3, folded)));
    let whole: i64 = terms.iter().map(|(c, v)| v * X[*c]).sum();
    assert_eq!(sum.running.len(), MAX_TERMS + 1);
    assert_eq!(
        *sum.running.last().expect("a total"),
        whole,
        "the total is the whole row, not only the terms that are drawn"
    );
}

#[test]
fn every_running_total_is_the_sum_of_the_items_so_far() {
    let terms = row(MAX_TERMS + 2);
    let sum = sum_of(&terms, &X);
    let items: Vec<i64> = sum.shown.iter().map(|t| t.2).chain(sum.rest.map(|r| r.1)).collect();
    for (k, total) in sum.running.iter().enumerate() {
        assert_eq!(*total, items[..=k].iter().sum::<i64>(), "after item {k}");
    }
}
