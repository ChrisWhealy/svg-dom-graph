use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the demo's nodes show at one position.
pub(in crate::sha2_256) struct Shown {
    pub(in crate::sha2_256) ring: Option<Ring>,
    /// The message block, or zeros before the walk reaches it.
    pub(in crate::sha2_256) block: [u32; BLOCK_WORDS],
    /// The message schedule so far.
    pub(in crate::sha2_256) schedule: [u32; SCHEDULE_WORDS],
    /// The schedule word in focus, if any.
    pub(in crate::sha2_256) schedule_focus: Option<usize>,
    /// The words the schedule word being built reads, marked secondary.
    pub(in crate::sha2_256) schedule_secondary: Vec<usize>,
    /// The round constant in focus, if any.
    pub(in crate::sha2_256) k_focus: Option<usize>,
    /// The round being run, if any.
    pub(in crate::sha2_256) round: Option<Round>,
    /// The working variables going into the round, or at the end the working variables after the last one. Zeros before
    /// the first round.
    pub(in crate::sha2_256) working: [u32; 8],
    /// The digest, or zeros before the last step.
    pub(in crate::sha2_256) digest: [u32; 8],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the nodes show at `position` of the hash `trace` records.
pub(in crate::sha2_256) fn shown(trace: &Trace, position: usize) -> Shown {
    let phase = phase(position);
    let at_least = |p: usize| position >= p;

    let mut schedule = [0_u32; SCHEDULE_WORDS];
    let built = match phase {
        Phase::Message | Phase::Block => 0,
        Phase::Copy => BLOCK_WORDS,
        Phase::Expand(n) => n + 1,
        Phase::Round(_) | Phase::Final => SCHEDULE_WORDS,
    };
    schedule[..built].copy_from_slice(&trace.schedule[..built]);

    let (schedule_focus, schedule_secondary) = match phase {
        Phase::Expand(n) => (Some(n), vec![n - 16, n - 15, n - 7, n - 2]),
        Phase::Round(i) => (Some(i), Vec::new()),
        _ => (None, Vec::new()),
    };

    let (round, working) = match phase {
        Phase::Round(i) => (Some(trace.rounds[i]), trace.rounds[i].before),
        Phase::Final => (None, trace.compressed),
        _ => (None, [0; 8]),
    };

    Shown {
        ring: match phase {
            Phase::Message => Some(Ring::Message),
            Phase::Block => Some(Ring::Block),
            Phase::Final => Some(Ring::Digest),
            _ => None,
        },
        block: if at_least(1) { trace.block } else { [0; BLOCK_WORDS] },
        schedule,
        schedule_focus,
        schedule_secondary,
        k_focus: if let Phase::Round(i) = phase { Some(i) } else { None },
        round,
        working,
        digest: if phase == Phase::Final { trace.digest } else { [0; 8] },
    }
}
