//! The net: an ordinary place/transition Petri net of a bounded buffer that four producers fill and three consumers
//! empty. Pure: nothing here touches the DOM, a clock or the scene.
//!
//! Every arc has weight 1. A transition is enabled exactly when each of its input places holds a token. Firing moves
//! one token out of each input place and one into each output place, all at once, or not at all.
//!
//! The buffer is two places, `Free` and `Used`. A `Put` needs a `Free` token and makes a `Used` one, and a `Take` does
//! the reverse. The net itself, and no check in the page, is what stops a full buffer from taking another item.

use std::fmt;

/// How many producers and consumers there are, and how many slots the buffer has.
pub(crate) const PRODUCERS: usize = 4;
pub(crate) const CONSUMERS: usize = 3;
pub(crate) const CAPACITY: u8 = 8;

/// How many places and transitions there are: three places and transitions for each producer, two for each consumer.
pub(crate) const PLACES: usize = PRODUCERS * 3 + CONSUMERS * 2 + 2;
pub(crate) const TRANSITIONS: usize = PRODUCERS * 3 + CONSUMERS * 2;

/// How many arcs there are: eight for each producer, and six for each consumer.
pub(crate) const ARCS: usize = PRODUCERS * 8 + CONSUMERS * 6;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The place at this index in a marking. Producers come first, with `Idle`, `Busy` and `Ready` each, then consumers,
/// with `Idle` and `Processing` each, and last `Free` and `Used`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PlaceId(pub(crate) u8);

/// The transition at this index. Producers come first, with `Start`, `Finish` and `Put` each, then consumers, with
/// `Take` and `Complete` each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TransitionId(pub(crate) u8);

/// What a place holds, which is the state of one worker or part of the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaceKind {
    ProducerIdle,
    ProducerBusy,
    ProducerReady,
    ConsumerIdle,
    ConsumerProcessing,
    Free,
    Used,
}

/// What a transition does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransitionKind {
    Start,
    Finish,
    Put,
    Take,
    Complete,
}

impl PlaceId {
    pub(crate) const FREE: Self = Self((PRODUCERS * 3 + CONSUMERS * 2) as u8);
    pub(crate) const USED: Self = Self(Self::FREE.0 + 1);

    pub(crate) fn producer_idle(i: usize) -> Self {
        Self((i * 3) as u8)
    }

    pub(crate) fn producer_busy(i: usize) -> Self {
        Self((i * 3 + 1) as u8)
    }

    pub(crate) fn producer_ready(i: usize) -> Self {
        Self((i * 3 + 2) as u8)
    }

    pub(crate) fn consumer_idle(j: usize) -> Self {
        Self((PRODUCERS * 3 + j * 2) as u8)
    }

    pub(crate) fn consumer_processing(j: usize) -> Self {
        Self((PRODUCERS * 3 + j * 2 + 1) as u8)
    }

    pub(crate) fn index(self) -> usize {
        usize::from(self.0)
    }

    /// Every place, in the order of the marking.
    pub(crate) fn all() -> impl Iterator<Item = Self> {
        (0..PLACES as u8).map(Self)
    }

    pub(crate) fn kind(self) -> PlaceKind {
        let at = self.index();
        match at {
            _ if at < PRODUCERS * 3 => {
                [PlaceKind::ProducerIdle, PlaceKind::ProducerBusy, PlaceKind::ProducerReady][at % 3]
            },
            _ if self == Self::FREE => PlaceKind::Free,
            _ if self == Self::USED => PlaceKind::Used,
            _ => [PlaceKind::ConsumerIdle, PlaceKind::ConsumerProcessing][(at - PRODUCERS * 3) % 2],
        }
    }

    /// Which producer or consumer the place belongs to, or `None` for `Free` and `Used`.
    pub(crate) fn worker(self) -> Option<usize> {
        let at = self.index();
        match at {
            _ if at < PRODUCERS * 3 => Some(at / 3),
            _ if at < PRODUCERS * 3 + CONSUMERS * 2 => Some((at - PRODUCERS * 3) / 2),
            _ => None,
        }
    }

    /// The name that a person reads: `P0.Idle`, `C2.Processing`, `Free`.
    pub(crate) fn name(self) -> String {
        let (worker, word) = match self.kind() {
            PlaceKind::ProducerIdle => ('P', "Idle"),
            PlaceKind::ProducerBusy => ('P', "Busy"),
            PlaceKind::ProducerReady => ('P', "Ready"),
            PlaceKind::ConsumerIdle => ('C', "Idle"),
            PlaceKind::ConsumerProcessing => ('C', "Processing"),
            PlaceKind::Free => return "Free".to_owned(),
            PlaceKind::Used => return "Used".to_owned(),
        };
        format!("{worker}{}.{word}", self.worker().unwrap_or(0))
    }
}

impl TransitionId {
    pub(crate) fn start(i: usize) -> Self {
        Self((i * 3) as u8)
    }

    pub(crate) fn finish(i: usize) -> Self {
        Self((i * 3 + 1) as u8)
    }

    pub(crate) fn put(i: usize) -> Self {
        Self((i * 3 + 2) as u8)
    }

    pub(crate) fn take(j: usize) -> Self {
        Self((PRODUCERS * 3 + j * 2) as u8)
    }

    pub(crate) fn complete(j: usize) -> Self {
        Self((PRODUCERS * 3 + j * 2 + 1) as u8)
    }

    pub(crate) fn index(self) -> usize {
        usize::from(self.0)
    }

    /// Every transition, in order.
    pub(crate) fn all() -> impl Iterator<Item = Self> {
        (0..TRANSITIONS as u8).map(Self)
    }

    pub(crate) fn kind(self) -> TransitionKind {
        let at = self.index();
        if at < PRODUCERS * 3 {
            [TransitionKind::Start, TransitionKind::Finish, TransitionKind::Put][at % 3]
        } else {
            [TransitionKind::Take, TransitionKind::Complete][(at - PRODUCERS * 3) % 2]
        }
    }

    /// Which producer or consumer the transition belongs to.
    pub(crate) fn worker(self) -> usize {
        let at = self.index();
        if at < PRODUCERS * 3 { at / 3 } else { (at - PRODUCERS * 3) / 2 }
    }

    /// The name that a person reads: `Start[0]`, `Take[2]`.
    pub(crate) fn name(self) -> String {
        let word = match self.kind() {
            TransitionKind::Start => "Start",
            TransitionKind::Finish => "Finish",
            TransitionKind::Put => "Put",
            TransitionKind::Take => "Take",
            TransitionKind::Complete => "Complete",
        };
        format!("{word}[{}]", self.worker())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The input and the output places of one transition. The actor's own place comes first in each list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransitionSpec {
    pub(crate) pre: Vec<PlaceId>,
    pub(crate) post: Vec<PlaceId>,
}

/// Why a net could not be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NetError {
    /// There are not the right number of places or transitions.
    WrongCount { places: usize, transitions: usize },
    /// An arc names a place that does not exist.
    UnknownPlace(TransitionId, PlaceId),
    /// A transition has the same place twice on one side, which would be an arc of weight 2.
    DuplicateArc(TransitionId, PlaceId),
    /// A transition has no input or no output place.
    NoArcs(TransitionId),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongCount { places, transitions } => {
                write!(
                    f,
                    "the net has {places} places and {transitions} transitions, not {PLACES} and {TRANSITIONS}"
                )
            },
            Self::UnknownPlace(t, p) => write!(f, "{} has an arc to the place {}, which does not exist", t.name(), p.0),
            Self::DuplicateArc(t, p) => write!(f, "{} has two arcs with {}", t.name(), p.name()),
            Self::NoArcs(t) => write!(f, "{} has no inputs or no outputs", t.name()),
        }
    }
}

impl std::error::Error for NetError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The fixed topology: for each transition, its input places and its output places.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Net {
    transitions: Vec<TransitionSpec>,
}

impl Net {
    /// The bounded producer and consumer net.
    ///
    /// # Panics
    ///
    /// Never: the table below passes [`Net::from_specs`], which a test checks.
    pub(crate) fn bounded_buffer() -> Self {
        Self::from_specs(PLACES, Self::table()).expect("the fixed table is a valid net")
    }

    /// The table of §3 of the design: one row for each transition, instantiated once for each worker.
    fn table() -> Vec<TransitionSpec> {
        let mut rows = Vec::with_capacity(TRANSITIONS);
        for i in 0..PRODUCERS {
            rows.push(TransitionSpec {
                pre: vec![PlaceId::producer_idle(i)],
                post: vec![PlaceId::producer_busy(i)],
            });
            rows.push(TransitionSpec {
                pre: vec![PlaceId::producer_busy(i)],
                post: vec![PlaceId::producer_ready(i)],
            });
            rows.push(TransitionSpec {
                pre: vec![PlaceId::producer_ready(i), PlaceId::FREE],
                post: vec![PlaceId::producer_idle(i), PlaceId::USED],
            });
        }
        for j in 0..CONSUMERS {
            rows.push(TransitionSpec {
                pre: vec![PlaceId::consumer_idle(j), PlaceId::USED],
                post: vec![PlaceId::consumer_processing(j), PlaceId::FREE],
            });
            rows.push(TransitionSpec {
                pre: vec![PlaceId::consumer_processing(j)],
                post: vec![PlaceId::consumer_idle(j)],
            });
        }
        rows
    }

    /// A net from `places` and the rows of `transitions`, checked.
    ///
    /// # Errors
    ///
    /// Returns which rule the rows break: the wrong number of parts, an unknown place, a place twice on one side of a
    /// transition, or a transition with nothing on a side.
    pub(crate) fn from_specs(places: usize, transitions: Vec<TransitionSpec>) -> Result<Self, NetError> {
        if places != PLACES || transitions.len() != TRANSITIONS {
            return Err(NetError::WrongCount {
                places,
                transitions: transitions.len(),
            });
        }
        for (at, spec) in transitions.iter().enumerate() {
            let t = TransitionId(at as u8);
            if spec.pre.is_empty() || spec.post.is_empty() {
                return Err(NetError::NoArcs(t));
            }
            for side in [&spec.pre, &spec.post] {
                for (n, &place) in side.iter().enumerate() {
                    if place.index() >= places {
                        return Err(NetError::UnknownPlace(t, place));
                    }
                    if side[..n].contains(&place) {
                        return Err(NetError::DuplicateArc(t, place));
                    }
                }
            }
        }
        Ok(Self { transitions })
    }

    pub(crate) fn spec(&self, t: TransitionId) -> &TransitionSpec {
        &self.transitions[t.index()]
    }

    /// The arcs, each as `(from_place, to_transition)` for an input arc or `(to_place, from_transition)` for an output
    /// one, in the order that the scene draws them: for each transition, its inputs and then its outputs.
    pub(crate) fn arcs(&self) -> Vec<Arc> {
        let mut arcs = Vec::with_capacity(ARCS);
        for t in TransitionId::all() {
            let spec = self.spec(t);
            arcs.extend(spec.pre.iter().map(|&p| Arc {
                place: p,
                transition: t,
                input: true,
            }));
            arcs.extend(spec.post.iter().map(|&p| Arc {
                place: p,
                transition: t,
                input: false,
            }));
        }
        arcs
    }

    /// Whether `t` can fire at `marking`: every input place holds a token.
    pub(crate) fn is_enabled(&self, marking: &Marking, t: TransitionId) -> bool {
        self.spec(t).pre.iter().all(|&p| marking.get(p) >= 1)
    }

    /// Every transition that can fire at `marking`.
    pub(crate) fn enabled(&self, marking: &Marking) -> TransitionSet {
        TransitionId::all()
            .filter(|&t| self.is_enabled(marking, t))
            .fold(TransitionSet::default(), |set, t| set.with(t))
    }

    /// The input places of `t` that hold no token at `marking`, the actor's own place first.
    pub(crate) fn missing(&self, marking: &Marking, t: TransitionId) -> Vec<PlaceId> {
        self.spec(t).pre.iter().copied().filter(|&p| marking.get(p) < 1).collect()
    }

    /// Fires `t` at `marking` and gives the next marking. Nothing is changed in `marking`.
    ///
    /// # Errors
    ///
    /// Returns [`FireError::Disabled`] with the missing places if `t` is not enabled, and [`FireError::Invariant`] if
    /// the next marking breaks one, in which case the marking is not used.
    pub(crate) fn fire(&self, marking: &Marking, t: TransitionId) -> Result<Marking, FireError> {
        let missing = self.missing(marking, t);
        if !missing.is_empty() {
            return Err(FireError::Disabled(missing));
        }
        let spec = self.spec(t);
        let mut next = *marking;
        for &p in &spec.pre {
            next.tokens[p.index()] -= 1;
        }
        for &p in &spec.post {
            next.tokens[p.index()] += 1;
        }
        next.check().map_err(FireError::Invariant)?;
        Ok(next)
    }
}

/// One arc of the net, which is always between a place and a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Arc {
    pub(crate) place: PlaceId,
    pub(crate) transition: TransitionId,
    /// Whether the arc goes from the place to the transition, and not from the transition to the place.
    pub(crate) input: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A set of transitions, as bits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TransitionSet(u32);

impl TransitionSet {
    pub(crate) fn with(self, t: TransitionId) -> Self {
        Self(self.0 | 1 << t.index())
    }

    pub(crate) fn contains(self, t: TransitionId) -> bool {
        self.0 >> t.index() & 1 == 1
    }

    /// The transitions that are in `self` and not in `other`.
    pub(crate) fn minus(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    #[cfg(test)]
    pub(crate) fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The members, in order.
    pub(crate) fn iter(self) -> impl Iterator<Item = TransitionId> {
        TransitionId::all().filter(move |&t| self.contains(t))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many tokens each place holds, in the order of [`PlaceId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Marking {
    pub(crate) tokens: [u8; PLACES],
}

impl Marking {
    /// The starting marking: every worker idle, eight free slots and none used.
    pub(crate) fn initial() -> Self {
        let mut tokens = [0; PLACES];
        for i in 0..PRODUCERS {
            tokens[PlaceId::producer_idle(i).index()] = 1;
        }
        for j in 0..CONSUMERS {
            tokens[PlaceId::consumer_idle(j).index()] = 1;
        }
        tokens[PlaceId::FREE.index()] = CAPACITY;
        Self { tokens }
    }

    pub(crate) fn get(&self, p: PlaceId) -> u8 {
        self.tokens[p.index()]
    }

    pub(crate) fn used(&self) -> u8 {
        self.get(PlaceId::USED)
    }

    pub(crate) fn free(&self) -> u8 {
        self.get(PlaceId::FREE)
    }

    /// How many consumers are processing an item.
    pub(crate) fn processing(&self) -> usize {
        (0..CONSUMERS)
            .filter(|&j| self.get(PlaceId::consumer_processing(j)) == 1)
            .count()
    }

    /// Checks the rules that hold at every reachable marking: each worker is in exactly one state, and the buffer's two
    /// places always add up to its capacity.
    ///
    /// # Errors
    ///
    /// Returns the first rule that is broken.
    pub(crate) fn check(&self) -> Result<(), Violation> {
        for i in 0..PRODUCERS {
            let sum = self.get(PlaceId::producer_idle(i))
                + self.get(PlaceId::producer_busy(i))
                + self.get(PlaceId::producer_ready(i));
            if sum != 1 {
                return Err(Violation::ProducerExclusivity(i));
            }
        }
        for j in 0..CONSUMERS {
            if self.get(PlaceId::consumer_idle(j)) + self.get(PlaceId::consumer_processing(j)) != 1 {
                return Err(Violation::ConsumerExclusivity(j));
            }
        }
        if u16::from(self.free()) + u16::from(self.used()) != u16::from(CAPACITY) {
            return Err(Violation::Capacity);
        }
        Ok(())
    }
}

/// A rule that a marking breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Violation {
    ProducerExclusivity(usize),
    ConsumerExclusivity(usize),
    Capacity,
    /// Put count minus Take count is not what the buffer holds.
    BufferAccounting,
    /// Take count minus Complete count is not the number of consumers processing.
    ProcessingAccounting,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProducerExclusivity(i) => write!(f, "producer {i} is not in exactly one state"),
            Self::ConsumerExclusivity(j) => write!(f, "consumer {j} is not in exactly one state"),
            Self::Capacity => write!(f, "Free and Used do not add up to {CAPACITY}"),
            Self::BufferAccounting => write!(f, "Puts minus Takes is not what the buffer holds"),
            Self::ProcessingAccounting => write!(f, "Takes minus Completes is not what is being processed"),
        }
    }
}

/// Why a transition did not fire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FireError {
    /// It was not enabled. These are the input places that held no token, the actor's own place first.
    Disabled(Vec<PlaceId>),
    /// The next marking broke a rule, so it was not used.
    Invariant(Violation),
}

impl fmt::Display for FireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled(missing) => {
                let names: Vec<String> = missing.iter().map(|p| p.name()).collect();
                write!(f, "it is not enabled: {} has no token", names.join(" and "))
            },
            Self::Invariant(violation) => write!(f, "{violation}"),
        }
    }
}

impl std::error::Error for FireError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a worker cannot go on, which is worked out from a marking and is never kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blocked {
    /// A producer holds a prepared item and the buffer is full.
    Full(usize),
    /// A consumer is idle and the buffer is empty.
    Empty(usize),
}

/// The workers that wait on the buffer at `marking`: producers in `Ready` while `Free` is 0, and idle consumers while
/// `Used` is 0.
pub(crate) fn blocked(marking: &Marking) -> Vec<Blocked> {
    let mut out = Vec::new();
    if marking.free() == 0 {
        out.extend(
            (0..PRODUCERS)
                .filter(|&i| marking.get(PlaceId::producer_ready(i)) == 1)
                .map(Blocked::Full),
        );
    }
    if marking.used() == 0 {
        out.extend(
            (0..CONSUMERS)
                .filter(|&j| marking.get(PlaceId::consumer_idle(j)) == 1)
                .map(Blocked::Empty),
        );
    }
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
