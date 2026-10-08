use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One round of compression, with every intermediate value kept so the demo can show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sha2_256) struct Round {
    /// The working variables `a..h` going in.
    pub(in crate::sha2_256) before: [u32; 8],
    /// `bigSigma1(e)`.
    pub(in crate::sha2_256) big_sigma1: u32,
    /// `choice(e, f, g)`.
    pub(in crate::sha2_256) choice: u32,
    /// `h + bigSigma1(e) + choice(e, f, g) + k + w`.
    pub(in crate::sha2_256) temp1: u32,
    /// `bigSigma0(a)`.
    pub(in crate::sha2_256) big_sigma0: u32,
    /// `majority(a, b, c)`.
    pub(in crate::sha2_256) majority: u32,
    /// `bigSigma0(a) + majority(a, b, c)`.
    pub(in crate::sha2_256) temp2: u32,
    /// The working variables going out: `h=g, g=f, f=e, e=d+temp1, d=c, c=b, b=a, a=temp1+temp2`.
    pub(in crate::sha2_256) after: [u32; 8],
}

/// One round over `state` (`a..h`) with round constant `k` and schedule word `w`. All additions wrap.
pub(in crate::sha2_256) fn round(state: [u32; 8], k: u32, w: u32) -> Round {
    let [a, b, c, d, e, f, g, h] = state;
    let big_sigma1 = big_sigma1(e);
    let choice = choice(e, f, g);
    let temp1 = h.wrapping_add(big_sigma1).wrapping_add(choice).wrapping_add(k).wrapping_add(w);
    let big_sigma0 = big_sigma0(a);
    let majority = majority(a, b, c);
    let temp2 = big_sigma0.wrapping_add(majority);
    Round {
        before: state,
        big_sigma1,
        choice,
        temp1,
        big_sigma0,
        majority,
        temp2,
        after: [temp1.wrapping_add(temp2), a, b, c, d.wrapping_add(temp1), e, f, g],
    }
}
