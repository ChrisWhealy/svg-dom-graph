use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the demo's nodes show at one position.
pub(in crate::sha1) struct Shown {
    /// Whether the message block has not been reached yet, so its zeros are placeholders and not words.
    pub(in crate::sha1) block_unreached: bool,
    /// The schedule words not built yet. They hold zero as a placeholder, which a built word can also legitimately be.
    pub(in crate::sha1) schedule_unreached: Vec<usize>,
    /// Whether the terms of the schedule word or round have no values to be taken from yet, so they hold placeholders.
    pub(in crate::sha1) terms_unreached: bool,
    /// Whether the digest has not been reached yet.
    pub(in crate::sha1) digest_unreached: bool,
    /// Whether the hexadecimal hash value has not been reached yet.
    pub(in crate::sha1) hash_unreached: bool,
    /// The four schedule words schedule word `n` is built from, their XOR and its rotation, while it is being built.
    pub(in crate::sha1) expansion: Option<Expansion>,
    pub(in crate::sha1) ring: Option<Ring>,
    /// The message block, or zeros before the walk reaches it.
    pub(in crate::sha1) block: [u32; BLOCK_WORDS],
    /// The message schedule so far.
    pub(in crate::sha1) schedule: [u32; SCHEDULE_WORDS],
    /// The schedule word in focus, if any.
    pub(in crate::sha1) schedule_focus: Option<usize>,
    /// The words the schedule word being built reads, marked secondary.
    pub(in crate::sha1) schedule_secondary: Vec<usize>,
    /// The round being run, or after the last round that last round, if any.
    pub(in crate::sha1) round: Option<Round>,
    /// The working variables going into the round, or after the last round, going into that last round. Zeros before
    /// the first round.
    pub(in crate::sha1) working: [u32; 5],
    /// The digest, or zeros before the digest step.
    pub(in crate::sha1) digest: [u32; 5],
    /// The digest as 40 hexadecimal digits at the last step, and 40 zeros before it.
    pub(in crate::sha1) hash: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Schedule word `n` being built, `ms[n] = ROTL(ms[n-3] XOR ms[n-8] XOR ms[n-14] XOR ms[n-16], 1)`, with every term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sha1) struct Expansion {
    pub(in crate::sha1) n: usize,
    /// `ms[n-3]`, `ms[n-8]`, `ms[n-14]` and `ms[n-16]`.
    pub(in crate::sha1) inputs: [u32; 4],
    /// The four inputs, XOR'ed together.
    pub(in crate::sha1) xor: u32,
    /// The XOR rotated left by one bit, which is schedule word `n` itself.
    pub(in crate::sha1) sum: u32,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the nodes show at `position` of the hash `trace` records.
pub(in crate::sha1) fn shown(trace: &Trace, position: usize) -> Shown {
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
        Phase::Expand(n) => (Some(n), vec![n - 3, n - 8, n - 14, n - 16]),
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
    let working = round.map_or([0; 5], |r| r.before);

    let expansion = match phase {
        Phase::Expand(n) => {
            let inputs = expansion_inputs(&trace.schedule, n);
            Some(Expansion {
                n,
                inputs,
                xor: expansion_xor(&inputs),
                sum: trace.schedule[n],
            })
        },
        _ => None,
    };

    Shown {
        block_unreached: !at_least(1),
        schedule_unreached: (built..SCHEDULE_WORDS).collect(),
        terms_unreached: round.is_none() && expansion.is_none(),
        digest_unreached: !matches!(phase, Phase::Final | Phase::Hash),
        hash_unreached: phase != Phase::Hash,
        expansion,
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
        round,
        working,
        digest: if matches!(phase, Phase::Final | Phase::Hash) { trace.digest } else { [0; 5] },
        hash: if phase == Phase::Hash { digest_hex(&trace.digest) } else { "0".repeat(40) },
    }
}
