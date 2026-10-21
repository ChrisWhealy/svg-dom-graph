//! The array that a sorting demo sorts, and making another. Nothing here touches the DOM or the library.

/// How many elements the demos sort.
pub(crate) const LEN: usize = 10;

/// The smallest and largest element the demos make. The height of the bar for an element is in proportion to it.
pub(crate) const LOWEST: u8 = 10;
pub(crate) const HIGHEST: u8 = 99;

/// The array a demo opens on. No element is in place, and the elements are all different.
pub(crate) const INITIAL: [u8; LEN] = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`LEN`] different elements chosen at random, between [`LOWEST`] and [`HIGHEST`]. `random` gives numbers from 0 up to
/// but not including 1.
pub(crate) fn sample(mut random: impl FnMut() -> f64) -> Vec<u8> {
    let mut pool: Vec<u8> = (LOWEST..=HIGHEST).collect();
    // A partial shuffle: each pick is one of the elements not yet picked.
    for taken in 0..LEN {
        let left = pool.len() - taken;
        let pick = taken + ((random() * left as f64) as usize).min(left - 1);
        pool.swap(taken, pick);
    }
    pool.truncate(LEN);
    pool
}
