use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One round of compression, with every intermediate value kept so the demo can show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sha1) struct Round {
    /// The working variables `a..e` going in.
    pub(in crate::sha1) before: [u32; 5],
    /// The five terms of `temp`, in the order the demo draws them: `ROTL(a, 5)`, `f(b, c, d)`, `e`, `K` and `W`.
    pub(in crate::sha1) terms: [u32; 5],
    /// The sum of the five terms, ignoring overflow.
    pub(in crate::sha1) temp: u32,
    /// The working variables going out: `e=d, d=c, c=ROTL(b, 30), b=a, a=temp`.
    pub(in crate::sha1) after: [u32; 5],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Round `index` over `state` (`a..e`) with schedule word `w`. All additions wrap.
pub(in crate::sha1) fn round(state: [u32; 5], index: usize, w: u32) -> Round {
    let [a, b, c, d, e] = state;
    let section = section_of(index);
    let terms = [
        a.rotate_left(5),
        RoundFunction::of_section(section).apply(b, c, d),
        e,
        ROUND_CONSTANTS[section],
        w,
    ];
    let temp = terms.iter().fold(0_u32, |sum, term| sum.wrapping_add(*term));
    Round {
        before: state,
        terms,
        temp,
        after: [temp, a, b.rotate_left(30), c, d],
    }
}
