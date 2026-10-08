// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The node that is ringed as a whole at a step, for the steps that name a whole node rather than a cell of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sha2_256) enum Ring {
    Message,
    Block,
    Digest,
}
