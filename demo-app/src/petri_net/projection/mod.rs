//! What a snapshot looks like, and what is said about it. Pure: nothing here touches the DOM.
//!
//! A snapshot is projected to a [`Presentation`], which is a list of slots and the fields of the text panel. The page
//! draws the whole presentation, or only the slots that differ from the one before, and the two must give the same
//! page. Nothing about the net is read back from a class or an attribute.

use super::{
    fixtures::Scenario,
    net::{
        ARCS, Arc, Blocked, CAPACITY, CONSUMERS, Net, PLACES, PRODUCERS, PlaceId, PlaceKind, TRANSITIONS, TransitionId,
        TransitionSet, blocked,
    },
    trace::{Snapshot, Trace},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the page says when a button has nowhere to go.
pub(crate) const AT_FIRST: &str = "Already at the first snapshot.";
pub(crate) const AT_LAST: &str = "Already at the final snapshot.";
pub(crate) const NO_CHECKPOINTS_IN_MANUAL: &str = "Return to script to use checkpoints.";
pub(crate) const FIRE_IN_SCRIPT: &str = "Enter manual mode to fire a transition.";
pub(crate) const FIRE_NOTHING_SELECTED: &str = "Select a transition first.";
pub(crate) const FIRE_A_PLACE: &str = "A place cannot fire. Select a transition.";
pub(crate) const PLAY_IN_MANUAL: &str =
    "Play is for scripts. In manual mode, fire a transition or step through the branch.";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A place or a transition, which is what a person can select and what the keyboard moves between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Place(PlaceId),
    Transition(TransitionId),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How much of the wiring is emphasised. It changes how the arcs look and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wiring {
    Overview,
    Focused,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How fast the script plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Speed {
    One,
    Two,
    Five,
    Ten,
}

impl Speed {
    pub(crate) const ALL: [Self; 4] = [Self::One, Self::Two, Self::Five, Self::Ten];

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::One => "1",
            Self::Two => "2",
            Self::Five => "5",
            Self::Ten => "10",
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::One => "1 event/s",
            Self::Two => "2 events/s",
            Self::Five => "5 events/s",
            Self::Ten => "10 events/s",
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The time between two events, in milliseconds.
    pub(crate) fn interval_ms(self) -> u32 {
        match self {
            Self::One => 1000,
            Self::Two => 500,
            Self::Five => 200,
            Self::Ten => 100,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a person has chosen to look at. None of it changes the net or the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct View {
    pub(crate) selected: Option<Target>,
    /// The one target that is a tab stop. It follows the focus, and is not the same as the selection.
    pub(crate) roving: Target,
    pub(crate) wiring: Wiring,
    pub(crate) speed: Speed,
}

impl View {
    pub(crate) fn first() -> Self {
        Self {
            selected: None,
            roving: Target::Transition(TransitionId::start(0)),
            wiring: Wiring::Overview,
            speed: Speed::Two,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which history the page is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// An immutable script.
    Script(Scenario),
    /// A branch that a person grows, forked from this snapshot of this script.
    Manual { origin: Scenario, fork: usize },
}

impl Source {
    pub(crate) fn is_manual(self) -> bool {
        matches!(self, Self::Manual { .. })
    }

    /// The scenario that the page is in, or that the manual branch came from.
    pub(crate) fn scenario(self) -> Scenario {
        match self {
            Self::Script(s) | Self::Manual { origin: s, .. } => s,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The order that the keyboard visits the targets in: each producer's ring, then Free and Used, then each consumer's.
pub(crate) fn target_order() -> Vec<Target> {
    let mut order = Vec::with_capacity(PLACES + TRANSITIONS);
    for i in 0..PRODUCERS {
        order.extend([
            Target::Place(PlaceId::producer_idle(i)),
            Target::Transition(TransitionId::start(i)),
            Target::Place(PlaceId::producer_busy(i)),
            Target::Transition(TransitionId::finish(i)),
            Target::Place(PlaceId::producer_ready(i)),
            Target::Transition(TransitionId::put(i)),
        ]);
    }
    order.extend([Target::Place(PlaceId::FREE), Target::Place(PlaceId::USED)]);
    for j in 0..CONSUMERS {
        order.extend([
            Target::Place(PlaceId::consumer_idle(j)),
            Target::Transition(TransitionId::take(j)),
            Target::Place(PlaceId::consumer_processing(j)),
            Target::Transition(TransitionId::complete(j)),
        ]);
    }
    order
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The target that `key` moves the focus to from `from`, if it moves it. The ends do not wrap.
pub(crate) fn move_roving(order: &[Target], from: Target, key: &str) -> Option<Target> {
    let at = order.iter().position(|&t| t == from)?;
    let to = match key {
        "ArrowRight" | "ArrowDown" => at.checked_add(1).filter(|&n| n < order.len())?,
        "ArrowLeft" | "ArrowUp" => at.checked_sub(1)?,
        "Home" => 0,
        "End" => order.len() - 1,
        _ => return None,
    };
    (to != at).then(|| order[to])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The slots of a presentation, in order: a slot for each place, each gate, each arc, each cell of the buffer, and its
/// caption.
pub(crate) const fn place_slot(p: PlaceId) -> usize {
    p.0 as usize
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) const fn gate_slot(t: TransitionId) -> usize {
    PLACES + t.0 as usize
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) const fn arc_slot(index: usize) -> usize {
    PLACES + TRANSITIONS + index
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) const fn cell_slot(cell: usize) -> usize {
    PLACES + TRANSITIONS + ARCS + cell
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) const CAPTION_SLOT: usize = PLACES + TRANSITIONS + ARCS + CAPACITY as usize;
pub(crate) const SLOTS: usize = CAPTION_SLOT + 1;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What one slot is: some attributes, and perhaps text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SlotState {
    pub(crate) attrs: Vec<(&'static str, String)>,
    pub(crate) text: Option<String>,
}

impl SlotState {
    fn new(attrs: Vec<(&'static str, String)>, text: Option<String>) -> Self {
        Self { attrs, text }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text panel's fields, in order: the DOM ID of each and the name that a person reads.
pub(crate) const PANEL: [(&str, &str); 10] = [
    ("pn-p-snapshot", "Snapshot"),
    ("pn-p-history", "History"),
    ("pn-p-event", "This event"),
    ("pn-p-tokens", "Tokens"),
    ("pn-p-changes", "Enabled changes"),
    ("pn-p-enabled", "Enabled now"),
    ("pn-p-blocked", "Waiting"),
    ("pn-p-buffer", "Buffer"),
    ("pn-p-counts", "Counts"),
    ("pn-p-inspector", "Selected"),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that the page shows for one snapshot and one view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Presentation {
    pub(crate) slots: Vec<SlotState>,
    pub(crate) panel: Vec<String>,
    pub(crate) status: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One change to make to a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Write {
    Attr(usize, &'static str, String),
    RemoveAttr(usize, &'static str),
    Text(usize, String),
    Panel(usize, String),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The writes that draw all of `p`.
pub(crate) fn full(p: &Presentation) -> Vec<Write> {
    let mut writes = Vec::new();
    for (at, slot) in p.slots.iter().enumerate() {
        writes.extend(slot.attrs.iter().map(|(name, value)| Write::Attr(at, name, value.clone())));
        if let Some(text) = &slot.text {
            writes.push(Write::Text(at, text.clone()));
        }
    }
    writes.extend(p.panel.iter().enumerate().map(|(at, text)| Write::Panel(at, text.clone())));
    writes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The writes that turn a page that shows `before` into one that shows `after`.
pub(crate) fn diff(before: &Presentation, after: &Presentation) -> Vec<Write> {
    let mut writes = Vec::new();
    for (at, (old, new)) in before.slots.iter().zip(&after.slots).enumerate() {
        if old == new {
            continue;
        }
        for (name, value) in &new.attrs {
            if old.attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v) != Some(value) {
                writes.push(Write::Attr(at, name, value.clone()));
            }
        }
        for (name, _) in &old.attrs {
            if !new.attrs.iter().any(|(n, _)| n == name) {
                writes.push(Write::RemoveAttr(at, name));
            }
        }
        if let Some(text) = &new.text {
            if old.text.as_ref() != Some(text) {
                writes.push(Write::Text(at, text.clone()));
            }
        }
    }
    for (at, (old, new)) in before.panel.iter().zip(&after.panel).enumerate() {
        if old != new {
            writes.push(Write::Panel(at, new.clone()));
        }
    }
    writes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What changed at one snapshot, worked out from it and the one before it in the same history.
struct Step {
    enabled: TransitionSet,
    newly_enabled: TransitionSet,
    newly_disabled: TransitionSet,
    blocked: Vec<Blocked>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn step(net: &Net, trace: &Trace, index: usize) -> Option<Step> {
    let now = trace.snapshot(index)?;
    let enabled = net.enabled(&now.marking);
    let (newly_enabled, newly_disabled) = match trace.before(index) {
        Some(before) => {
            let was = net.enabled(&before.marking);
            (enabled.minus(was), was.minus(enabled))
        },
        None => (TransitionSet::default(), TransitionSet::default()),
    };
    Some(Step {
        enabled,
        newly_enabled,
        newly_disabled,
        blocked: blocked(&now.marking),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn names(places: &[PlaceId]) -> String {
    list(&places.iter().map(|p| p.name()).collect::<Vec<_>>())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn list(items: &[String]) -> String {
    match items {
        [] => "none".to_owned(),
        [only] => only.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn set_names(set: TransitionSet) -> String {
    list(&set.iter().map(TransitionId::name).collect::<Vec<_>>())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a worker waits, in words.
fn waiting(b: Blocked) -> String {
    match b {
        Blocked::Full(i) => format!("P{i} waits: buffer full"),
        Blocked::Empty(j) => format!("C{j} waits: buffer empty"),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn tokens_word(n: u8) -> String {
    format!("{n} token{}", if n == 1 { "" } else { "s" })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The input and the output places of `t`, as a person reads them.
fn arcs_of(net: &Net, t: TransitionId) -> (String, String) {
    let spec = net.spec(t);
    (names(&spec.pre), names(&spec.post))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the inspector says of the selected target.
fn inspector(net: &Net, snapshot: &Snapshot, step: &Step, selected: Option<Target>) -> String {
    match selected {
        None => "Nothing selected. Select a place or a transition to see its inputs, outputs and tokens.".to_owned(),
        Some(Target::Transition(t)) => {
            let (inputs, outputs) = arcs_of(net, t);
            let mut text = format!("{}: inputs {inputs}; outputs {outputs}. ", t.name());
            if step.enabled.contains(t) {
                text.push_str("Enabled: every input holds a token.");
            } else {
                let missing = net.missing(&snapshot.marking, t);
                text.push_str(&format!("Disabled: {} has no token.", names(&missing)));
            }
            text
        },
        Some(Target::Place(p)) => {
            let n = snapshot.marking.get(p);
            let arcs = net.arcs();
            let consumers: Vec<String> = arcs
                .iter()
                .filter(|a| a.place == p && a.input)
                .map(|a| a.transition.name())
                .collect();
            let producers: Vec<String> = arcs
                .iter()
                .filter(|a| a.place == p && !a.input)
                .map(|a| a.transition.name())
                .collect();
            let mut text = format!(
                "{}: {}. Consumed by {}; produced by {}.",
                p.name(),
                tokens_word(n),
                list(&consumers),
                list(&producers)
            );
            if let Some(b) = step.blocked.iter().find(|b| blocked_place(**b) == p) {
                text.push(' ');
                text.push_str(&waiting(*b));
                text.push('.');
            }
            text
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The place that a blocked worker waits in, which is the one that holds its token.
fn blocked_place(b: Blocked) -> PlaceId {
    match b {
        Blocked::Full(i) => PlaceId::producer_ready(i),
        Blocked::Empty(j) => PlaceId::consumer_idle(j),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a transition consumed and produced, and what its firing enabled and disabled. This is what is said aloud after
/// a manual firing.
fn fired_sentence(net: &Net, t: TransitionId, step: &Step) -> String {
    let (consumed, produced) = arcs_of(net, t);
    let mut text = format!("{} consumed {consumed}; produced {produced}", t.name());
    if !step.newly_enabled.is_empty() {
        text.push_str(&format!("; {} became enabled", set_names(step.newly_enabled)));
    }
    if !step.newly_disabled.is_empty() {
        text.push_str(&format!("; {} became disabled", set_names(step.newly_disabled)));
    }
    text.push('.');
    text
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn buffer_sentence(s: &Snapshot) -> String {
    format!("Used {} of {CAPACITY}, Free {}", s.marking.used(), s.marking.free())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn history_sentence(source: Source, trace: &Trace) -> String {
    match source {
        Source::Script(s) => format!("{}: immutable script of {} events", s.label(), trace.last()),
        Source::Manual { origin, fork } => format!(
            "Manual branch: forked from {} at its snapshot {fork}, {} events of its own",
            origin.label(),
            trace.last()
        ),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which arcs a selection puts forward in the focused wiring: those of the selected gate, or those at the selected
/// place.
fn emphasised(selected: Option<Target>, arc: &Arc) -> Option<bool> {
    match selected? {
        Target::Transition(t) => Some(arc.transition == t),
        Target::Place(p) => Some(arc.place == p),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The presentation of the snapshot at `index` of `trace`, or `None` if there is none.
pub(crate) fn present(net: &Net, trace: &Trace, index: usize, source: Source, view: &View) -> Option<Presentation> {
    let snapshot = trace.snapshot(index)?;
    let step = step(net, trace, index)?;
    let fired = snapshot.last.map(|e| e.transition);
    let mut slots = vec![SlotState::default(); SLOTS];

    for p in PlaceId::all() {
        let n = snapshot.marking.get(p);
        let waits = step.blocked.iter().find(|b| blocked_place(**b) == p).copied();
        let hub = matches!(p.kind(), PlaceKind::Free | PlaceKind::Used);
        let mut class = format!("pn-place pn-tok-{n}");
        if hub {
            class.push_str(" pn-hub");
        }
        if waits.is_some() {
            class.push_str(" pn-waiting");
        }
        let mut label = match p.kind() {
            PlaceKind::Free => format!("Free: {n} of {CAPACITY} slots"),
            PlaceKind::Used => format!("Used: {n} of {CAPACITY} slots"),
            _ => format!("{}: {}", p.name(), tokens_word(n)),
        };
        match waits {
            Some(Blocked::Full(_)) => label.push_str(". Waiting: buffer full"),
            Some(Blocked::Empty(_)) => label.push_str(". Waiting: buffer empty"),
            None => {},
        }
        let selected = view.selected == Some(Target::Place(p));
        slots[place_slot(p)] = SlotState::new(
            vec![
                ("class", class),
                ("aria-label", label),
                ("data-pn-tokens", n.to_string()),
                ("data-pn-selected", selected.to_string()),
                ("aria-pressed", selected.to_string()),
                ("tabindex", if view.roving == Target::Place(p) { "0" } else { "-1" }.to_owned()),
            ],
            hub.then(|| n.to_string()),
        );
    }

    for t in TransitionId::all() {
        let enabled = step.enabled.contains(t);
        let newly = step.newly_enabled.contains(t);
        let is_fired = fired == Some(t);
        let mut class = format!("pn-gate {}", if enabled { "pn-enabled" } else { "pn-disabled" });
        if newly {
            class.push_str(" pn-new");
        }
        if is_fired {
            class.push_str(" pn-fired");
        }
        let status = if is_fired {
            "fired"
        } else if newly {
            "new"
        } else if enabled {
            "enabled"
        } else {
            "disabled"
        };
        let (inputs, outputs) = arcs_of(net, t);
        let mut label = format!("{}, {}.", t.name(), if enabled { "enabled" } else { "disabled" });
        if newly {
            label.push_str(" Newly enabled.");
        }
        if is_fired {
            label.push_str(" Just fired.");
        }
        label.push_str(&format!(" Inputs {inputs}. Outputs {outputs}."));
        if !enabled {
            label.push_str(&format!(" Missing {}.", names(&net.missing(&snapshot.marking, t))));
        }
        let selected = view.selected == Some(Target::Transition(t));
        slots[gate_slot(t)] = SlotState::new(
            vec![
                ("class", class),
                ("aria-label", label),
                ("aria-disabled", (!enabled).to_string()),
                ("data-pn-status", status.to_owned()),
                ("data-pn-selected", selected.to_string()),
                ("aria-pressed", selected.to_string()),
                (
                    "tabindex",
                    if view.roving == Target::Transition(t) { "0" } else { "-1" }.to_owned(),
                ),
            ],
            Some(status.to_owned()),
        );
    }

    let arcs = net.arcs();
    for (index, arc) in arcs.iter().enumerate() {
        let emphasis = match (view.wiring, emphasised(view.selected, arc)) {
            (Wiring::Focused, Some(true)) => "on",
            (Wiring::Focused, Some(false)) => "dim",
            _ => "normal",
        };
        slots[arc_slot(index)] = SlotState::new(vec![("data-pn-emphasis", emphasis.to_owned())], None);
    }

    for cell in 0..usize::from(CAPACITY) {
        let full = cell < usize::from(snapshot.marking.used());
        slots[cell_slot(cell)] = SlotState::new(vec![("data-pn-full", full.to_string())], None);
    }
    slots[CAPTION_SLOT] = SlotState::new(
        vec![("data-pn-used", snapshot.marking.used().to_string())],
        Some(buffer_sentence(snapshot)),
    );

    let mut panel = vec![String::new(); PANEL.len()];
    let last = trace.last();
    panel[0] = format!("{index} of {last}");
    panel[1] = history_sentence(source, trace);
    panel[2] = fired.map_or("Initial marking".to_owned(), |t| t.name());
    panel[3] = match fired {
        Some(t) => {
            let (consumed, produced) = arcs_of(net, t);
            format!("Removed {consumed}. Added {produced}.")
        },
        None => "None yet.".to_owned(),
    };
    panel[4] = if fired.is_some() {
        format!(
            "Newly enabled: {}. Newly disabled: {}.",
            set_names(step.newly_enabled),
            set_names(step.newly_disabled)
        )
    } else {
        "None yet.".to_owned()
    };
    panel[5] = set_names(step.enabled);
    panel[6] = if step.blocked.is_empty() {
        "No worker waits on the buffer.".to_owned()
    } else {
        step.blocked.iter().map(|b| waiting(*b)).collect::<Vec<_>>().join("; ") + "."
    };
    panel[7] = format!("{}. {} in process.", buffer_sentence(snapshot), snapshot.marking.processing());
    panel[8] = format!(
        "Puts {}, Takes {}, Completes {}",
        snapshot.totals.puts, snapshot.totals.takes, snapshot.totals.completes
    );
    panel[9] = inspector(net, snapshot, &step, view.selected);

    let status = match fired {
        Some(t) => fired_sentence(net, t, &step),
        None => "Initial marking.".to_owned(),
    };
    Some(Presentation { slots, panel, status })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which buttons have nowhere to go, as dimmed controls do. A dimmed control is still focusable and still heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub(crate) restart: bool,
    pub(crate) previous: bool,
    pub(crate) next: bool,
    pub(crate) play: bool,
    pub(crate) fire: bool,
    pub(crate) checkpoints: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) fn bounds(net: &Net, trace: &Trace, index: usize, source: Source, view: &View) -> Bounds {
    let manual = source.is_manual();
    let last = trace.last();
    let can_fire = match (manual, view.selected, trace.snapshot(index)) {
        (true, Some(Target::Transition(t)), Some(s)) => net.is_enabled(&s.marking, t),
        _ => false,
    };
    Bounds {
        restart: index == 0,
        previous: index == 0,
        next: index >= last,
        play: manual || index >= last,
        fire: !can_fire,
        checkpoints: manual,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is said aloud for the snapshot at `index`: the event, or the starting marking.
pub(crate) fn summary(net: &Net, trace: &Trace, index: usize) -> String {
    match (trace.snapshot(index), step(net, trace, index)) {
        (Some(s), Some(step)) => match s.last {
            Some(e) => format!("{} {}.", fired_sentence(net, e.transition, &step), buffer_sentence(s)),
            None => format!("Initial marking. {}.", buffer_sentence(s)),
        },
        _ => String::new(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is said aloud when playback pauses.
pub(crate) fn pause_summary(net: &Net, trace: &Trace, index: usize) -> String {
    let buffer = trace.snapshot(index).map(buffer_sentence).unwrap_or_default();
    let waiting = step(net, trace, index).map(|s| s.blocked.len()).unwrap_or(0);
    format!(
        "Paused at snapshot {index} of {}. {buffer}. {waiting} workers wait on the buffer.",
        trace.last()
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether playback says anything aloud for the move to `index`. Only a change in who waits on the buffer is a
/// milestone, and so is reaching the end.
pub(crate) fn announced_while_playing(trace: &Trace, index: usize) -> bool {
    if index >= trace.last() {
        return true;
    }
    match (trace.before(index), trace.snapshot(index)) {
        (Some(before), Some(now)) => blocked(&before.marking) != blocked(&now.marking),
        _ => false,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sentence for a transition that is not enabled, which says each input that holds no token.
pub(crate) fn disabled_sentence(net: &Net, snapshot: &Snapshot, t: TransitionId) -> String {
    format!(
        "{} is not enabled: {} has no token. Nothing changed.",
        t.name(),
        names(&net.missing(&snapshot.marking, t))
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
