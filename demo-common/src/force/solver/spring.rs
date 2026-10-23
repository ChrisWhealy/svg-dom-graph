//! One spring, joining two circles.

/// One spring, joining the particles at indices `from` and `to`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
    pub stiffness: f64,
    pub rest_length: f64,
}
