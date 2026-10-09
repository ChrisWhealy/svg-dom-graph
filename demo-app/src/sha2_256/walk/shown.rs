use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the demo's nodes show at one position.
pub(in crate::sha2_256) struct Shown {
    /// The four schedule words schedule word `n` is built from, the two sigma results, and their sum, while it is being
    /// built.
    pub(in crate::sha2_256) expansion: Option<Expansion>,
    /// The round whose constant and schedule word the round terms show, while running a round or after the last one.
    pub(in crate::sha2_256) round_index: Option<usize>,
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
    /// The round being run, or after the last round that last round, if any.
    pub(in crate::sha2_256) round: Option<Round>,
    /// The working variables going into the round, or after the last round, going into that last round. Zeros before
    /// the first round.
    pub(in crate::sha2_256) working: [u32; 8],
    /// The digest, or zeros before the digest step.
    pub(in crate::sha2_256) digest: [u32; 8],
    /// The digest as 64 hexadecimal digits at the last step, and 64 zeros before it.
    pub(in crate::sha2_256) hash: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Schedule word `n` being built, `ms[n] = ms[n-16] + sigma0(ms[n-15]) + ms[n-7] + sigma1(ms[n-2])`, with every term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sha2_256) struct Expansion {
    pub(in crate::sha2_256) n: usize,
    pub(in crate::sha2_256) w16: u32,
    pub(in crate::sha2_256) w15: u32,
    pub(in crate::sha2_256) w7: u32,
    pub(in crate::sha2_256) w2: u32,
    pub(in crate::sha2_256) sigma0: u32,
    pub(in crate::sha2_256) sigma1: u32,
    /// The sum, which is schedule word `n` itself.
    pub(in crate::sha2_256) sum: u32,
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
        Phase::Round(_) | Phase::Final | Phase::Hash => SCHEDULE_WORDS,
    };
    schedule[..built].copy_from_slice(&trace.schedule[..built]);

    let (schedule_focus, schedule_secondary) = match phase {
        Phase::Expand(n) => (Some(n), vec![n - 16, n - 15, n - 7, n - 2]),
        Phase::Round(i) => (Some(i), Vec::new()),
        _ => (None, Vec::new()),
    };

    // After the last round the diagram keeps showing that round, so its "next working variables" are the compressed
    // state the digest is built from.
    let round_index = match phase {
        Phase::Round(i) => Some(i),
        Phase::Final | Phase::Hash => Some(ROUNDS - 1),
        _ => None,
    };
    let round = round_index.map(|i| trace.rounds[i]);
    let working = round.map_or([0; 8], |r| r.before);

    let expansion = match phase {
        Phase::Expand(n) => {
            let s = &trace.schedule;
            Some(Expansion {
                n,
                w16: s[n - 16],
                w15: s[n - 15],
                w7: s[n - 7],
                w2: s[n - 2],
                sigma0: sigma0(s[n - 15]),
                sigma1: sigma1(s[n - 2]),
                sum: s[n],
            })
        },
        _ => None,
    };

    Shown {
        expansion,
        round_index,
        ring: match phase {
            Phase::Message => Some(Ring::Message),
            Phase::Block => Some(Ring::Block),
            Phase::Final => Some(Ring::Digest),
            Phase::Hash => Some(Ring::Hash),
            _ => None,
        },
        block: if at_least(1) { trace.block } else { [0; BLOCK_WORDS] },
        schedule,
        schedule_focus,
        schedule_secondary,
        k_focus: if let Phase::Round(i) = phase { Some(i) } else { None },
        round,
        working,
        digest: if matches!(phase, Phase::Final | Phase::Hash) { trace.digest } else { [0; 8] },
        hash: if phase == Phase::Hash { digest_hex(&trace.digest) } else { "0".repeat(64) },
    }
}
