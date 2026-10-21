//! Where the nodes of a row go, so that the row is centred the way a person sees it. Pure, so none of it needs a browser.

/// The left edges of the nodes of one row, whose boxes are `widths` wide, with `gap` between one box and the next, so
/// that the row is centred on `axis`.
///
/// With an odd number of nodes the one in the middle is centred on the axis. With an even number the gap between the two
/// in the middle is. The others follow outward from there at the same gap. So the middle of the row is on the axis
/// whatever the widths are, which centring the whole extent of the row would not do if the widths on one side were more
/// than those on the other.
pub(super) fn centred_lefts(widths: &[f64], gap: f64, axis: f64) -> Vec<f64> {
    let count = widths.len();
    let mut lefts = vec![0.0; count];
    if count == 0 {
        return lefts;
    }
    let half = count / 2;
    // The first node to the right of the middle, and the last one on its left, if there is no node in the middle.
    let (mut next_left, mut previous_right) = if count % 2 == 1 {
        lefts[half] = axis - widths[half] / 2.0;
        (lefts[half] + widths[half] + gap, lefts[half] - gap)
    } else {
        (axis + gap / 2.0, axis - gap / 2.0)
    };
    for i in (if count % 2 == 1 { half + 1 } else { half })..count {
        lefts[i] = next_left;
        next_left += widths[i] + gap;
    }
    for i in (0..half).rev() {
        lefts[i] = previous_right - widths[i];
        previous_right = lefts[i] - gap;
    }
    lefts
}
