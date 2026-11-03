//! What the page shows for one snapshot, worked out from the snapshot and the preferences and nothing else. Pure:
//! nothing here touches the DOM.
//!
//! The page keeps a fixed list of elements, one for each vertex, edge, residual lane and queue mark. [`present`] says
//! what every one of them should be, as a [`Presentation`]. The page then writes only what differs from the
//! presentation that it drew last, which [`diff`] works out. A full redraw is the same presentation written in full.

use super::{
    fixtures::NAMES,
    solver::{Direction, Event, Hop, Snapshot, Trace},
};
use std::collections::BTreeSet;

/// The radius of a vertex, in the units of the scene.
pub(crate) const NODE_RADIUS: f64 = 22.0;

/// What is written where a field has nothing to say.
pub(crate) const NOTHING: &str = "\u{2014}";

/// What a person is told when a press of a button has nowhere to go.
pub(crate) const AT_FIRST: &str = "Already at the first step";
pub(crate) const AT_LAST: &str = "Already at the last step";
pub(crate) const NO_EARLIER_AUGMENTATION: &str = "No earlier augmentation";
pub(crate) const NO_LATER_AUGMENTATION: &str = "No later augmentation";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Something in the graph that can have the focus: a vertex, or an original edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Vertex(usize),
    Edge(usize),
}

/// The order of the targets, which is also the order that the keys go through them: the vertices, then the edges by ID.
pub(crate) fn target_order(trace: &Trace) -> Vec<Target> {
    (0..trace.vertex_count())
        .map(Target::Vertex)
        .chain((0..trace.edges().len()).map(Target::Edge))
        .collect()
}

/// Where the roving focus goes when `key` is pressed with it on `from`. Left and Up go to the one before, Right and
/// Down to the one after, and both wrap round. Home and End go to the first and the last. The order is the list's, and
/// not where the targets are in the picture. Any other key gives `None`.
pub(crate) fn move_roving(order: &[Target], from: Target, key: &str) -> Option<Target> {
    let at = order.iter().position(|&t| t == from).unwrap_or(0);
    let last = order.len().checked_sub(1)?;
    let to = match key {
        "ArrowLeft" | "ArrowUp" => (at + last) % order.len(),
        "ArrowRight" | "ArrowDown" => (at + 1) % order.len(),
        "Home" => 0,
        "End" => last,
        _ => return None,
    };
    order.get(to).copied()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How fast playback goes. The page's default is the third.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Speed {
    Half,
    One,
    Two,
    Four,
}

impl Speed {
    pub(crate) const ALL: [Speed; 4] = [Self::Half, Self::One, Self::Two, Self::Four];

    /// How long between one step and the next, in milliseconds.
    pub(crate) fn interval_ms(self) -> u32 {
        match self {
            Self::Half => 800,
            Self::One => 400,
            Self::Two => 200,
            Self::Four => 100,
        }
    }

    /// What the selector says, and what its option's value is.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Half => "0.5\u{d7}",
            Self::One => "1\u{d7}",
            Self::Two => "2\u{d7}",
            Self::Four => "4\u{d7}",
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Half => "0.5",
            Self::One => "1",
            Self::Two => "2",
            Self::Four => "4",
        }
    }

    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|speed| speed.key() == key)
    }
}

/// What a person has chosen to see. None of it changes the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Prefs {
    /// Whether the residual transitions are drawn.
    pub(crate) show_residual: bool,
    /// Whether the queue is marked in the graph. The text of the queue is always there.
    pub(crate) show_queue: bool,
    /// Whether the transition that is being looked at is marked, and the ones before it are listed.
    pub(crate) detailed: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            show_residual: true,
            show_queue: true,
            detailed: true,
        }
    }
}

/// Which of the preferences is meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pref {
    Residual,
    Queue,
    Detail,
}

impl Prefs {
    #[cfg(test)]
    pub(crate) fn get(&self, pref: Pref) -> bool {
        match pref {
            Pref::Residual => self.show_residual,
            Pref::Queue => self.show_queue,
            Pref::Detail => self.detailed,
        }
    }

    pub(crate) fn toggled(mut self, pref: Pref) -> Self {
        match pref {
            Pref::Residual => self.show_residual = !self.show_residual,
            Pref::Queue => self.show_queue = !self.show_queue,
            Pref::Detail => self.detailed = !self.detailed,
        }
        self
    }
}

/// Everything about what is on the page that is not in the trace: the one that has the roving focus, the one that was
/// focused, the preferences, and the speed. The index is kept by the controller, with the snapshot it picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct View {
    pub(crate) focused: Option<Target>,
    pub(crate) roving: Target,
    pub(crate) prefs: Prefs,
    pub(crate) speed: Speed,
}

impl View {
    /// The view of a graph that has just been shown: the roving focus on the source, nothing focused.
    pub(crate) fn first(prefs: Prefs, speed: Speed) -> Self {
        Self {
            focused: None,
            roving: Target::Vertex(0),
            prefs,
            speed,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One element of the picture, as it should be: its class, its other attributes, and the text in it if it has any.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Slot {
    pub(crate) class: String,
    pub(crate) attrs: Vec<(&'static str, String)>,
    pub(crate) text: Option<String>,
}

/// Where each element is in the list of [`Slot`]s, for a graph of `vertices` and `edges`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Slots {
    vertices: usize,
    edges: usize,
}

impl Slots {
    pub(crate) fn new(vertices: usize, edges: usize) -> Self {
        Self { vertices, edges }
    }

    /// The group of the vertex `v`.
    pub(crate) fn vertex(self, v: usize) -> usize {
        v
    }

    /// The group of the original edge `e`, and its label.
    pub(crate) fn edge(self, e: usize) -> usize {
        self.vertices + e
    }

    pub(crate) fn edge_label(self, e: usize) -> usize {
        self.vertices + self.edges + e
    }

    /// The group of the residual lane of edge `e` going `direction`, and its label.
    pub(crate) fn lane(self, e: usize, direction: Direction) -> usize {
        self.vertices + 2 * self.edges + 2 * e + usize::from(direction == Direction::Reverse)
    }

    pub(crate) fn lane_label(self, e: usize, direction: Direction) -> usize {
        self.vertices + 4 * self.edges + 2 * e + usize::from(direction == Direction::Reverse)
    }

    /// The mark that says where vertex `v` is in the queue.
    pub(crate) fn badge(self, v: usize) -> usize {
        self.vertices + 6 * self.edges + v
    }

    pub(crate) fn count(self) -> usize {
        2 * self.vertices + 6 * self.edges
    }
}

/// The fields of the text panel, in order: the id that the page gives each, and what it is called.
pub(crate) const PANEL: [(&str, &str); 12] = [
    ("ek-p-step", "Step"),
    ("ek-p-phase", "Phase"),
    ("ek-p-event", "This event"),
    ("ek-p-queue", "Queue"),
    ("ek-p-discovered", "Discovered"),
    ("ek-p-predecessors", "Reached by"),
    ("ek-p-history", "Looked at"),
    ("ek-p-path", "Path"),
    ("ek-p-bottleneck", "Bottleneck"),
    ("ek-p-augmentation", "Augmentation"),
    ("ek-p-total", "Flow so far"),
    ("ek-p-cut", "Cut"),
];

/// What every element of the picture and every field of the text panel should be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Presentation {
    pub(crate) slots: Vec<Slot>,
    pub(crate) panel: Vec<String>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One write that makes the page match a presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Write {
    Class(usize, String),
    Attr(usize, &'static str, String),
    RemoveAttr(usize, &'static str),
    Text(usize, String),
    Panel(usize, String),
}

/// The writes that bring a page that shows `prev` to one that shows `next`. Only what differs is written.
pub(crate) fn diff(prev: &Presentation, next: &Presentation) -> Vec<Write> {
    let mut writes = Vec::new();
    for (at, (old, new)) in prev.slots.iter().zip(&next.slots).enumerate() {
        if old.class != new.class {
            writes.push(Write::Class(at, new.class.clone()));
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
        if old.text != new.text {
            if let Some(text) = &new.text {
                writes.push(Write::Text(at, text.clone()));
            }
        }
    }
    for (at, (old, new)) in prev.panel.iter().zip(&next.panel).enumerate() {
        if old != new {
            writes.push(Write::Panel(at, new.clone()));
        }
    }
    writes
}

/// The attributes that a presentation manages. An element that a presentation gives none of one of them must not have
/// it.
const MANAGED: [&str; 3] = ["aria-label", "aria-hidden", "tabindex"];

/// Every write of `next`, as a page that shows nothing would need them, and as a page that shows something else would.
/// It is how a page is drawn from nothing, and how one that may be part written is put right, so it removes what `next`
/// does not have as well as writing what it has. Otherwise a lane that is hidden now would keep the name that it had
/// when it was shown.
pub(crate) fn full(next: &Presentation) -> Vec<Write> {
    let mut writes = Vec::new();
    for (at, slot) in next.slots.iter().enumerate() {
        writes.push(Write::Class(at, slot.class.clone()));
        for (name, value) in &slot.attrs {
            writes.push(Write::Attr(at, name, value.clone()));
        }
        for name in MANAGED {
            if !slot.attrs.iter().any(|(n, _)| *n == name) {
                writes.push(Write::RemoveAttr(at, name));
            }
        }
        if let Some(text) = &slot.text {
            writes.push(Write::Text(at, text.clone()));
        }
    }
    for (at, text) in next.panel.iter().enumerate() {
        writes.push(Write::Panel(at, text.clone()));
    }
    writes
}

/// A page, as far as a test can tell: what each element is. Writes can be applied to it, to check that the writes of a
/// [`diff`] leave it the same as the writes of a [`full`].
#[cfg(test)]
pub(crate) fn apply(model: &mut Presentation, writes: &[Write]) {
    for write in writes {
        match write {
            Write::Class(at, class) => model.slots[*at].class = class.clone(),
            Write::Attr(at, name, value) => {
                let attrs = &mut model.slots[*at].attrs;
                match attrs.iter_mut().find(|(n, _)| n == name) {
                    Some(existing) => existing.1 = value.clone(),
                    None => attrs.push((name, value.clone())),
                }
            },
            Write::RemoveAttr(at, name) => model.slots[*at].attrs.retain(|(n, _)| n != name),
            Write::Text(at, text) => model.slots[*at].text = Some(text.clone()),
            Write::Panel(at, text) => model.panel[*at] = text.clone(),
        }
        model.slots.iter_mut().for_each(|slot| slot.attrs.sort());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is left of an edge for `direction`: the room to add flow going forward, and the flow that could be taken back.
pub(crate) fn residual(capacity: u32, flow: u64, direction: Direction) -> u64 {
    match direction {
        Direction::Forward => u64::from(capacity).saturating_sub(flow),
        Direction::Reverse => flow,
    }
}

/// The text for a vertex: its name.
fn vertex_name(v: usize) -> &'static str {
    NAMES.get(v).copied().unwrap_or("?")
}

/// The words for a hop: its edge, which way, and between which vertices.
fn hop_text(hop: &Hop) -> String {
    format!(
        "e{} {} {}\u{2192}{}",
        hop.edge,
        hop.direction.letter(),
        vertex_name(hop.from),
        vertex_name(hop.to)
    )
}

/// The names of the vertices of `set`, in the order of the list, as the text writes them.
fn names_of(set: impl IntoIterator<Item = usize>) -> String {
    let names: Vec<&str> = set.into_iter().map(vertex_name).collect();
    if names.is_empty() { "(none)".to_owned() } else { names.join(", ") }
}

/// The accessible name of the original edge `e`: its ID, its ends, its flow and capacity, and both residuals.
pub(crate) fn edge_name(trace: &Trace, e: usize, flow: u64) -> String {
    let edge = &trace.edges()[e];
    format!(
        "e{e} {} to {}; flow {flow} of {}; forward residual {}; reverse residual {flow}",
        vertex_name(edge.from),
        vertex_name(edge.to),
        edge.capacity,
        residual(edge.capacity, flow, Direction::Forward)
    )
}

/// The accessible name of a residual transition. It says which original edge it belongs to, and which way it goes.
pub(crate) fn lane_name(trace: &Trace, e: usize, direction: Direction, capacity: u64) -> String {
    let edge = &trace.edges()[e];
    let (from, to, word) = match direction {
        Direction::Forward => (edge.from, edge.to, "forward"),
        Direction::Reverse => (edge.to, edge.from, "reverse"),
    };
    format!(
        "{word} of e{e} {} to {}, capacity {capacity}",
        vertex_name(from),
        vertex_name(to)
    )
}

/// The accessible name of a vertex.
fn node_name(trace: &Trace, v: usize, snapshot: &Snapshot) -> String {
    let mut name = format!("Vertex {}", vertex_name(v));
    if v == trace.source() {
        name.push_str(", the source");
    }
    if v == trace.sink() {
        name.push_str(", the sink");
    }
    if snapshot.bfs.current == Some(v) {
        name.push_str("; being searched from");
    } else if let Some(at) = snapshot.bfs.queue.iter().position(|&q| q == v) {
        name.push_str(&format!("; queue position {}", at + 1));
    } else if snapshot.bfs.discovered.contains(&v) && snapshot.bfs.search > 0 {
        name.push_str("; discovered");
    }
    if let Some(cut) = &snapshot.final_cut {
        name.push_str(if cut.reachable.contains(&v) { "; in S" } else { "; in T" });
    }
    name
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The edges that the predecessors of this search use, which are the tree that the search has grown.
fn tree_edges(snapshot: &Snapshot) -> BTreeSet<usize> {
    snapshot.bfs.predecessor.iter().flatten().map(|p| p.edge).collect()
}

/// The words for the event of `snapshot`, which always say which event it is and its index, so that every step changes
/// the text, even one that changes nothing in the picture.
pub(crate) fn event_line(trace: &Trace, snapshot: &Snapshot) -> String {
    let at = snapshot.index;
    let detail = match snapshot.event {
        Event::Initial => "the network has no flow yet".to_owned(),
        Event::SearchStarted => format!(
            "search {} starts at {}; queue: {}",
            snapshot.bfs.search,
            vertex_name(trace.source()),
            vertex_name(trace.source())
        ),
        Event::VertexDequeued { vertex } => format!("take {} off the queue", vertex_name(vertex)),
        Event::ArcExamined { hop, already_discovered } => format!(
            "look at {}: {} is {}",
            hop_text(&hop),
            vertex_name(hop.to),
            if already_discovered { "already discovered, so it is ignored" } else { "new" }
        ),
        Event::VertexDiscovered { vertex, edge, direction } => {
            format!("discover {} by e{edge} {}", vertex_name(vertex), direction.letter())
        },
        Event::PathFound => "the sink is reached, and the path is fixed".to_owned(),
        Event::BottleneckDetermined { bottleneck } => format!("the bottleneck of the path is {bottleneck}"),
        Event::FlowAugmented => match &snapshot.augmentation {
            Some(a) => format!("add {} along the path, so the flow is {}", a.delta, a.total_after),
            None => "the flow is augmented".to_owned(),
        },
        Event::SearchExhausted => "the sink cannot be reached, so the run is over".to_owned(),
        Event::MinCutShown => match &snapshot.final_cut {
            Some(cut) => format!("the cut is {}: max flow = min cut = {}", cut.capacity, snapshot.total_flow),
            None => "the cut".to_owned(),
        },
    };
    format!("#{at} {}: {detail}", snapshot.event.name())
}

/// The path of `snapshot` as text: the vertices it visits, and the transitions it uses.
fn path_text(snapshot: &Snapshot) -> String {
    let Some(first) = snapshot.path.first() else {
        return NOTHING.to_owned();
    };
    let mut vertices = vec![vertex_name(first.from)];
    vertices.extend(snapshot.path.iter().map(|hop| vertex_name(hop.to)));
    let edges: Vec<String> = snapshot
        .path
        .iter()
        .map(|hop| format!("e{} {}", hop.edge, hop.direction.letter()))
        .collect();
    format!("{} using {}", vertices.join(" \u{2192} "), edges.join(", "))
}

/// The text of how each vertex was reached.
fn predecessor_text(snapshot: &Snapshot) -> String {
    let reached: Vec<String> = snapshot
        .bfs
        .predecessor
        .iter()
        .enumerate()
        .filter_map(|(v, pred)| {
            pred.map(|p| {
                format!(
                    "{} from {} by e{} {}",
                    vertex_name(v),
                    vertex_name(p.from),
                    p.edge,
                    p.direction.letter()
                )
            })
        })
        .collect();
    if reached.is_empty() { NOTHING.to_owned() } else { reached.join("; ") }
}

/// The transitions that this search has looked at so far, which the detail mode lists.
fn history_text(trace: &Trace, snapshot: &Snapshot) -> String {
    let mut looked: Vec<String> = Vec::new();
    for earlier in &trace.snapshots()[..=snapshot.index] {
        if earlier.bfs.search != snapshot.bfs.search {
            continue;
        }
        if let Event::ArcExamined { hop, already_discovered } = earlier.event {
            looked.push(format!(
                "{}{}",
                hop_text(&hop),
                if already_discovered { " (ignored)" } else { "" }
            ));
        }
    }
    if looked.is_empty() { NOTHING.to_owned() } else { looked.join("; ") }
}

/// The text of what the last augmentation changed, edge by edge, and the total.
fn augmentation_text(snapshot: &Snapshot) -> String {
    let Some(a) = &snapshot.augmentation else {
        return NOTHING.to_owned();
    };
    let changes: Vec<String> = a
        .path
        .iter()
        .map(|hop| {
            let word = if hop.direction == Direction::Reverse { "takes back" } else { "adds" };
            format!(
                "e{} {} {word} {}: {} to {}",
                hop.edge,
                hop.direction.letter(),
                a.delta,
                a.flows_before[hop.edge],
                a.flows_after[hop.edge]
            )
        })
        .collect();
    format!("{}; total {} to {}", changes.join("; "), a.total_before, a.total_after)
}

/// The fields of the text panel for `snapshot`, in the order of [`PANEL`].
pub(crate) fn panel_text(trace: &Trace, snapshot: &Snapshot, prefs: Prefs) -> Vec<String> {
    let queue = if snapshot.bfs.search == 0 {
        NOTHING.to_owned()
    } else {
        names_of(snapshot.bfs.queue.iter().copied())
    };
    let discovered = if snapshot.bfs.search == 0 {
        NOTHING.to_owned()
    } else {
        names_of(snapshot.bfs.discovered.iter().copied())
    };
    vec![
        format!("Step {} of {}", snapshot.index, trace.last()),
        snapshot.event.name().to_owned(),
        event_line(trace, snapshot),
        queue,
        discovered,
        predecessor_text(snapshot),
        if prefs.detailed { history_text(trace, snapshot) } else { NOTHING.to_owned() },
        path_text(snapshot),
        snapshot.bottleneck.map_or_else(|| NOTHING.to_owned(), |b| b.to_string()),
        augmentation_text(snapshot),
        snapshot.total_flow.to_string(),
        match &snapshot.final_cut {
            Some(cut) => format!(
                "S = {}; T = {}; crossing {}; max flow = min cut = {}",
                names_of(cut.reachable.iter().copied()),
                names_of((0..trace.vertex_count()).filter(|v| !cut.reachable.contains(v))),
                if cut.crossing.is_empty() {
                    "(none)".to_owned()
                } else {
                    cut.crossing.iter().map(|e| format!("e{e}")).collect::<Vec<_>>().join(", ")
                },
                cut.capacity
            ),
            None => NOTHING.to_owned(),
        },
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn class_of(base: &str, flags: &[(bool, &str)]) -> String {
    let mut class = base.to_owned();
    for (on, name) in flags {
        if *on {
            class.push(' ');
            class.push_str(name);
        }
    }
    class
}

fn tabindex(on: bool) -> String {
    if on { "0".to_owned() } else { "-1".to_owned() }
}

/// What every element of the picture and every field of the panel should be for `snapshot` and `view`.
///
/// The snapshot need not be one of the trace's. A test draws a state that no run produces, to see how lanes lie.
pub(crate) fn present(trace: &Trace, snapshot: &Snapshot, view: &View) -> Presentation {
    let slots = Slots::new(trace.vertex_count(), trace.edges().len());
    let mut list = vec![Slot::default(); slots.count()];
    let prefs = view.prefs;
    let examined = snapshot.bfs.examined.filter(|_| prefs.detailed);
    let tree = tree_edges(snapshot);
    let on_path: BTreeSet<usize> = snapshot.path.iter().map(|h| h.edge).collect();
    let augmented = snapshot.augmentation.as_ref();
    let augmented_edges: BTreeSet<usize> =
        augmented.map(|a| a.path.iter().map(|h| h.edge).collect()).unwrap_or_default();
    let cut = snapshot.final_cut.as_ref();
    for v in 0..trace.vertex_count() {
        let in_queue = snapshot.bfs.queue.iter().position(|&q| q == v);
        let class = class_of(
            "ek-node",
            &[
                (v == trace.source(), "ek-source"),
                (v == trace.sink(), "ek-sink"),
                (snapshot.bfs.current == Some(v), "ek-current"),
                (snapshot.bfs.search > 0 && snapshot.bfs.discovered.contains(&v), "ek-found"),
                (cut.is_some_and(|c| c.reachable.contains(&v)), "ek-s"),
                (cut.is_some_and(|c| !c.reachable.contains(&v)), "ek-t"),
                (view.focused == Some(Target::Vertex(v)), "ek-focused"),
            ],
        );
        list[slots.vertex(v)] = Slot {
            class,
            attrs: vec![
                ("aria-label", node_name(trace, v, snapshot)),
                ("tabindex", tabindex(view.roving == Target::Vertex(v))),
            ],
            text: None,
        };
        let shown = prefs.show_queue && in_queue.is_some();
        list[slots.badge(v)] = Slot {
            class: class_of("ek-badge", &[(!shown, "ek-off")]),
            attrs: vec![],
            text: Some(in_queue.map_or_else(String::new, |at| (at + 1).to_string())),
        };
    }
    for e in 0..trace.edges().len() {
        let edge = trace.edges()[e];
        let flow = snapshot.flows[e];
        let this_arc = examined.filter(|x| x.hop.edge == e);
        let crossing = cut.is_some_and(|c| c.crossing.contains(&e));
        let class = class_of(
            "ek-edge",
            &[
                (edge.capacity == 0, "ek-zero"),
                (this_arc.is_some(), "ek-examined"),
                (tree.contains(&e), "ek-pred"),
                (on_path.contains(&e), "ek-path"),
                (augmented_edges.contains(&e), "ek-aug"),
                (crossing, "ek-cut"),
                (cut.is_some() && !crossing, "ek-dim"),
                (view.focused == Some(Target::Edge(e)), "ek-focused"),
            ],
        );
        list[slots.edge(e)] = Slot {
            class,
            attrs: vec![
                ("aria-label", edge_name(trace, e, flow)),
                ("tabindex", tabindex(view.roving == Target::Edge(e))),
            ],
            text: None,
        };
        let label = match augmented.filter(|_| augmented_edges.contains(&e)) {
            Some(a) => format!("{}\u{2192}{}/{}", a.flows_before[e], a.flows_after[e], edge.capacity),
            None => format!("{flow}/{}", edge.capacity),
        };
        list[slots.edge_label(e)] = Slot {
            class: class_of("ek-label", &[(augmented_edges.contains(&e), "ek-aug")]),
            attrs: vec![],
            text: Some(label),
        };
        for direction in [Direction::Forward, Direction::Reverse] {
            let room = residual(edge.capacity, flow, direction);
            let shown = prefs.show_residual && room > 0;
            let here = examined.is_some_and(|x| x.hop.edge == e && x.hop.direction == direction);
            let used = snapshot.path.iter().any(|h| h.edge == e && h.direction == direction);
            let base = if direction == Direction::Forward {
                "ek-res ek-res-f"
            } else {
                "ek-res ek-res-r"
            };
            list[slots.lane(e, direction)] = Slot {
                class: class_of(base, &[(!shown, "ek-off"), (here, "ek-examined"), (used, "ek-path")]),
                attrs: if shown {
                    vec![("aria-label", lane_name(trace, e, direction, room))]
                } else {
                    vec![("aria-hidden", "true".to_owned())]
                },
                text: None,
            };
            list[slots.lane_label(e, direction)] = Slot {
                class: class_of("ek-res-label", &[(!shown, "ek-off")]),
                attrs: vec![],
                text: Some(match (shown, direction) {
                    (false, _) => String::new(),
                    (true, Direction::Forward) => room.to_string(),
                    (true, Direction::Reverse) => format!("rev {room}"),
                }),
            };
        }
    }
    for slot in &mut list {
        slot.attrs.sort();
    }
    Presentation {
        slots: list,
        panel: panel_text(trace, snapshot, prefs),
    }
}

/// The presentation of the snapshot at `index` of `trace`.
pub(crate) fn present_at(trace: &Trace, index: usize, view: &View) -> Option<Presentation> {
    trace.snapshot(index).map(|snapshot| present(trace, snapshot, view))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which buttons are at the end of what they can do. A button that is is `aria-disabled`, and still takes a press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub(crate) restart: bool,
    pub(crate) previous: bool,
    pub(crate) next: bool,
    pub(crate) play: bool,
    pub(crate) previous_augmentation: bool,
    pub(crate) next_augmentation: bool,
    pub(crate) jump_to_cut: bool,
}

/// The buttons that are at an end at the snapshot `index` of `trace`.
pub(crate) fn bounds(trace: &Trace, index: usize) -> Bounds {
    let last = trace.last();
    Bounds {
        restart: index == 0,
        previous: index == 0,
        next: index >= last,
        play: index >= last,
        previous_augmentation: trace.previous_augmentation(index).is_none(),
        next_augmentation: trace.next_augmentation(index).is_none(),
        jump_to_cut: index >= last,
    }
}

/// The one sentence that is said when a person moves by hand to the snapshot at `index`.
pub(crate) fn summary(trace: &Trace, snapshot: &Snapshot) -> String {
    format!("Step {} of {}: {}", snapshot.index, trace.last(), snapshot.event.name())
}

/// Whether a person who is listening is told of a step while the run is playing. Only three steps are.
pub(crate) fn announced_while_playing(event: &Event) -> bool {
    matches!(event, Event::PathFound | Event::FlowAugmented | Event::MinCutShown)
}

/// What a person hears when playback stops.
pub(crate) fn pause_summary(trace: &Trace, snapshot: &Snapshot) -> String {
    format!(
        "Paused at step {} of {}: {}, flow {}",
        snapshot.index,
        trace.last(),
        snapshot.event.name(),
        snapshot.total_flow
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
