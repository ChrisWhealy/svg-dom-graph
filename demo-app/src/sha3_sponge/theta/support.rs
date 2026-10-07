//! Shared by [`super::theta_c`] and [`super::theta_d`]: the live state a nested child's own selection toolbar carries
//! across steps. `super::xor_loop` reuses `crate::util`'s own `<svg>`-cloning helpers directly, but not this module's
//! own [`SteppedChildState`] — see [`XorLoopState`](super::xor_loop::XorLoopState)'s own doc comment for why.

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Live state a nested child's own selection toolbar shares across steps — the same shape as
/// `crate::selection::ThetaCDemo`, reused here for both `ThetaC` and `ThetaD`, each stepped from inside its own nested
/// view instead of standalone.
pub(super) struct SteppedChildState {
    /// Row `i`'s own result — see `crate::selection::theta_c_outputs`'s own doc comment (for `ThetaC`) or
    /// [`super::theta_d::outputs`]'s own (for `ThetaD`). Which rows currently show is derived fresh from these and the
    /// walk's own current position on every step — see `crate::selection::display_outputs`'s own doc comment — rather
    /// than tracked here as a second, separately mutated flag per row.
    pub(super) outputs: [u64; 5],
    /// The `A[x][y]` the whole `Theta` walk runs over — handed down from whoever built the `Theta` scene, never a fixed
    /// demo value. Each rebuild redraws over this same input.
    pub(super) input: [[u64; 5]; 5],
    /// The id of whichever `<svg>` currently backs the nested child — see [`super::theta_c::rebuild_child`]'s/
    /// [`super::theta_d::rebuild_child`]'s own doc comment for why every step needs a fresh one.
    pub(super) child_svg_id: String,
}
