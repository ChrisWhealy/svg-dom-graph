//! What the Pythagoras demo draws for an angle: the right triangle in the unit circle, and a square on each of its
//! sides. Pure: the page draws it, and nothing here touches the DOM.
//!
//! Point C goes round the unit circle, and W is the point under it on the horizontal axis, or over it. The centre O, W
//! and C make a right triangle, with the right angle at W. Its hypotenuse is the radius OC, which is 1 long, and its
//! other two sides are the width OW and the height WC of C. A square is drawn on each side, away from the triangle, so
//! that the squares do not cover it. The square on the hypotenuse has an area of 1, and the other two have the areas of
//! the cosine squared and the sine squared, which add up to 1.
//!
//! Points are in the units of the circle, with `y` going up.

/// A place, as `(x, y)`.
pub(super) type Point = (f64, f64);

/// How near to nothing a number has to be to count as nothing, when a point is on a line.
const NEAR: f64 = 1e-9;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The triangle and the three squares for one angle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Figure {
    /// The centre of the circle, the point under or over C on the horizontal axis, and C.
    pub(super) centre: Point,
    pub(super) foot: Point,
    pub(super) c: Point,
    /// The corners of the square on the hypotenuse OC, and on the width OW, and on the height WC. Each is in order
    /// round the square, from the end of the side that it is on that is first.
    pub(super) hypotenuse: [Point; 4],
    pub(super) width: [Point; 4],
    pub(super) height: [Point; 4],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The square on the side from `from` to `to`, on the side of it that is away from `other`, which is the third corner
/// of the triangle: the corners `from` and `to`, and then the two that are a side's length from them, at a right angle.
///
/// If `other` is on the line of the side, which is when the triangle is flat, there is no away, so the square goes to
/// the side that `tie` points to. If that does not decide it either, it goes to the left of the side as it is read from
/// `from` to `to`.
fn square_on(from: Point, to: Point, other: Point, tie: Point) -> [Point; 4] {
    let side = (to.0 - from.0, to.1 - from.1);
    // A turn a quarter of the way anticlockwise, which is to the left of the side.
    let left = (-side.1, side.0);
    let towards_other = left.0 * (other.0 - from.0) + left.1 * (other.1 - from.1);
    let towards_tie = left.0 * tie.0 + left.1 * tie.1;
    let sign = if towards_other.abs() > NEAR {
        // The other corner is on the left, so away is to the right.
        if towards_other > 0.0 { -1.0 } else { 1.0 }
    } else if towards_tie < -NEAR {
        -1.0
    } else {
        1.0
    };
    let out = (sign * left.0, sign * left.1);
    [from, to, (to.0 + out.0, to.1 + out.1), (from.0 + out.0, from.1 + out.1)]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The figure for C at `degrees`, anticlockwise from the right.
pub(super) fn figure(degrees: f64) -> Figure {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (centre, foot, c) = ((0.0, 0.0), (cos, 0.0), (cos, sin));
    Figure {
        centre,
        foot,
        c,
        // Flat, at 0 and 180 degrees, the square on the hypotenuse goes up, as it does a little after 0.
        hypotenuse: square_on(centre, c, foot, (0.0, 1.0)),
        // Flat, at 0 and 180 degrees, the square on the width goes down, as it does a little after 0.
        width: square_on(centre, foot, c, (0.0, -1.0)),
        // Flat, at 90 and 270 degrees, the square on the height goes to the right, as it does a little before 90.
        height: square_on(foot, c, centre, (1.0, 0.0)),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The area of the square with the corners `square`, which is the length of its side squared.
pub(super) fn area(square: &[Point; 4]) -> f64 {
    let side = (square[1].0 - square[0].0, square[1].1 - square[0].1);
    side.0 * side.0 + side.1 * side.1
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The middle of the square with the corners `square`.
pub(super) fn middle(square: &[Point; 4]) -> Point {
    let (x, y) = square.iter().fold((0.0, 0.0), |(x, y), p| (x + p.0, y + p.1));
    (x / 4.0, y / 4.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The three areas of `figure`, as the text beside the squares says: the square on the width, the square on the height,
/// and the square on the hypotenuse, each to two places.
pub(super) fn areas_text(figure: &Figure) -> (String, String, String) {
    (
        format!("{:.2}", area(&figure.width)),
        format!("{:.2}", area(&figure.height)),
        format!("{:.2}", area(&figure.hypotenuse)),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The equation of the areas, with its numbers in it: `0.50 + 0.50 = 1.00`.
pub(super) fn sum_text(figure: &Figure) -> String {
    let (a, b, c) = areas_text(figure);
    format!("{a} + {b} = {c}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
