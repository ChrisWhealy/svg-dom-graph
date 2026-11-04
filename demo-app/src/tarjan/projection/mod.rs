//! What a snapshot looks like, in every view at once, and what is said about it. Pure: nothing here touches the DOM.
//!
//! One function makes a [`Presentation`] from one snapshot, a choice of what to look at, and where the view is. The
//! source vertices and edges, the stack, the roster of components, the outlines, the condensation and the text are all
//! parts of that one value, so no view can show a different snapshot from another. The page writes all of it, or only
//! what differs from the presentation before, and the two must give the same page.

use super::{
    condensation::Condensation,
    fixture::name,
    layout::Layout,
    solver::{Event, Graph, Kind, Snapshot, Trace},
    view::{Camera, Visibility, visibility},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the page says when a button has nowhere to go.
pub(crate) const AT_FIRST: &str = "Already at the first step.";
pub(crate) const AT_LAST: &str = "Already at the last step.";
pub(crate) const NO_EARLIER_COMPONENT: &str = "No earlier component closed.";
pub(crate) const NO_LATER_COMPONENT: &str = "No later component closed.";

/// A vertex or an edge, which is what a person can select and what the keyboard moves between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Vertex(usize),
    /// An edge, by its place in the sorted list of edges.
    Edge(usize),
}

/// How fast Play goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Speed {
    Slow,
    Normal,
    Fast,
    Faster,
}

impl Speed {
    pub(crate) const ALL: [Self; 4] = [Self::Slow, Self::Normal, Self::Fast, Self::Faster];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Slow => "800",
            Self::Normal => "400",
            Self::Fast => "200",
            Self::Faster => "100",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Slow => "800 ms",
            Self::Normal => "400 ms",
            Self::Fast => "200 ms",
            Self::Faster => "100 ms",
        }
    }

    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }

    pub(crate) fn interval_ms(self) -> u32 {
        match self {
            Self::Slow => 800,
            Self::Normal => 400,
            Self::Fast => 200,
            Self::Faster => 100,
        }
    }
}

/// What a person has chosen to look at. None of it changes the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Choices {
    pub(crate) selected: Option<Target>,
    /// The one target that is a tab stop. It follows the focus.
    pub(crate) roving: Target,
    /// The component that is picked out in every view, by its C-ID counted from 1.
    pub(crate) component: Option<usize>,
    pub(crate) speed: Speed,
    pub(crate) follow: bool,
}

impl Choices {
    pub(crate) fn first() -> Self {
        Self {
            selected: None,
            roving: Target::Vertex(0),
            component: None,
            speed: Speed::Normal,
            follow: true,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The order that the keyboard visits the targets in: the vertices by ID, and then the edges by their place in the
/// sorted list.
pub(crate) fn target_order(vertices: usize, edges: usize) -> Vec<Target> {
    (0..vertices).map(Target::Vertex).chain((0..edges).map(Target::Edge)).collect()
}

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
/// Where a vertex is in the algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VertexState {
    Undiscovered,
    OnStack,
    Closed,
}

impl VertexState {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Undiscovered => "undiscovered",
            Self::OnStack => "on-stack",
            Self::Closed => "closed",
        }
    }
}

/// One vertex as the page draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VertexView {
    /// The two cells of the node: its index and its lowlink, or `--` before it is discovered.
    pub(crate) values: [String; 2],
    pub(crate) state: VertexState,
    /// Its component's C-ID, once it is emitted.
    pub(crate) component: Option<usize>,
    /// Whether the page's attention is on it.
    pub(crate) current: bool,
    pub(crate) label: String,
    pub(crate) tab_stop: bool,
    pub(crate) selected: bool,
}

/// What is shown of one edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EdgeView {
    /// What kind of edge it is, if it is the one being inspected.
    pub(crate) inspected: Option<Kind>,
    pub(crate) label: String,
    pub(crate) tab_stop: bool,
    pub(crate) selected: bool,
}

/// An outline round the vertices of one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutlineView {
    pub(crate) component: usize,
    pub(crate) members: Vec<usize>,
    pub(crate) selected: bool,
}

/// One entry of the stack of the algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StackRow {
    pub(crate) vertex: usize,
    pub(crate) text: String,
    /// What the cell of the stack in the scene says: the vertex, its index and its lowlink.
    pub(crate) cell: String,
}

/// One row of the roster of components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RosterRow {
    pub(crate) component: usize,
    pub(crate) text: String,
    /// The first line of the row, which names the component and says how big it is.
    pub(crate) title: String,
    /// The names of its vertices, which the scene wraps over as many lines as it needs.
    pub(crate) members: String,
    pub(crate) selected: bool,
}

/// The text panel's fields, in order: the ID of each in the page and what a person reads.
pub(crate) const PANEL: [(&str, &str); 9] = [
    ("tj-p-snapshot", "Step"),
    ("tj-p-event", "This event"),
    ("tj-p-attention", "Attention"),
    ("tj-p-view", "View"),
    ("tj-p-stack", "Tarjan SCC stack"),
    ("tj-p-chain", "Active DFS chain"),
    ("tj-p-components", "Components"),
    ("tj-p-dag", "Condensation"),
    ("tj-p-selected", "Selected"),
];

/// Everything that the page shows for one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Presentation {
    pub(crate) vertices: Vec<VertexView>,
    pub(crate) edges: Vec<EdgeView>,
    pub(crate) outlines: Vec<OutlineView>,
    /// The stack from bottom to top.
    pub(crate) stack: Vec<StackRow>,
    pub(crate) roster: Vec<RosterRow>,
    pub(crate) condensation: Condensation,
    /// The component that is picked out in the condensation, if it is there.
    pub(crate) dag_selected: Option<usize>,
    pub(crate) panel: Vec<String>,
    /// The sentence that a person is told when this snapshot is reached by hand.
    pub(crate) status: String,
}

/// What a presentation is made from, besides the snapshot.
pub(crate) struct Context<'a> {
    pub(crate) graph: &'a Graph,
    pub(crate) edges: &'a [(usize, usize)],
    pub(crate) trace: &'a Trace,
    pub(crate) layout: &'a Layout,
    pub(crate) camera: Camera,
    pub(crate) viewport: (f64, f64),
}

/// The ID of edge `e`: `e000` to `e113`.
pub(crate) fn edge_name(e: usize) -> String {
    format!("e{e:03}")
}

/// The place of the edge `from` to `to` in the sorted list, if it is an edge.
pub(crate) fn edge_index(edges: &[(usize, usize)], from: usize, to: usize) -> Option<usize> {
    edges.binary_search(&(from, to)).ok()
}

fn kind_word(kind: Kind) -> &'static str {
    match kind {
        Kind::Tree => "tree edge",
        Kind::Stack => "stack edge",
        Kind::Closed => "closed edge",
    }
}

fn two(n: Option<usize>) -> String {
    n.map_or("--".to_owned(), |n| format!("{n:02}"))
}

fn names(vertices: &[usize]) -> String {
    match vertices {
        [] => "none".to_owned(),
        [only] => name(*only),
        [init @ .., last] => format!(
            "{} and {}",
            init.iter().map(|&v| name(v)).collect::<Vec<_>>().join(", "),
            name(*last)
        ),
    }
}

/// What the edge `from` to `to` is, at this snapshot, from the vertex that it leaves.
fn classify(s: &Snapshot, from: usize, to: usize) -> Option<Kind> {
    s.index[from]?;
    Some(match (s.index[to], s.on_stack[to]) {
        (None, _) => Kind::Tree,
        (Some(_), true) => Kind::Stack,
        (Some(_), false) => Kind::Closed,
    })
}

/// A sentence for one event, with what it changed.
pub(crate) fn describe(event: &Option<Event>, s: &Snapshot) -> String {
    match event {
        None => "Before the search: nothing is discovered, the stack is empty and no component is known.".to_owned(),
        Some(Event::Enter { v }) => format!(
            "Enter {}: index and lowlink are {}, and it goes on the SCC stack.",
            name(*v),
            two(s.index[*v])
        ),
        Some(Event::Inspect { v, w, kind, before, after }) => {
            let outcome = match kind {
                Kind::Tree => format!("{} is undiscovered, so the search goes down to it.", name(*w)),
                Kind::Stack if after != before => format!(
                    "{} is on the stack, so lowlink of {} goes from {before:02} to {after:02}, using the index of {}.",
                    name(*w),
                    name(*v),
                    name(*w)
                ),
                Kind::Stack => format!("{} is on the stack, and lowlink of {} stays {after:02}.", name(*w), name(*v)),
                Kind::Closed => format!("{} is already in a component, so it is ignored.", name(*w)),
            };
            format!("Inspect {} to {}, a {}: {outcome}", name(*v), name(*w), kind_word(*kind))
        },
        Some(Event::Return { parent, child, before, after }) => {
            if after != before {
                format!(
                    "Return from {} to {}: lowlink of {} goes from {before:02} to {after:02}.",
                    name(*child),
                    name(*parent),
                    name(*parent)
                )
            } else {
                format!(
                    "Return from {} to {}: lowlink of {} stays {after:02}.",
                    name(*child),
                    name(*parent),
                    name(*parent)
                )
            }
        },
        Some(Event::Closed { root, members }) => format!(
            "Component C{} closes at {}, which has lowlink equal to its index: {} {} popped together.",
            s.components.len(),
            name(*root),
            members.len(),
            if members.len() == 1 { "vertex is" } else { "vertices are" }
        ),
        Some(Event::Complete) => format!("Complete: all {} components are found.", s.components.len()),
    }
}

/// How the whole condensation reads.
fn dag_text(c: &Condensation) -> String {
    format!("{} components and {} arrows between them", c.nodes.len(), c.arcs.len())
}

/// The sentence for the visibility of the attention vertex, which is empty while it is well in view.
pub(crate) fn view_notice(visibility: Option<Visibility>, vertex: Option<usize>) -> String {
    match (visibility, vertex) {
        (Some(Visibility::NearEdge), Some(v)) => {
            format!(
                "Active vertex {} is near the edge of the view; use Zoom to active vertex.",
                name(v)
            )
        },
        (Some(Visibility::OffScreen), Some(v)) => {
            format!("Active vertex {} is off-screen; use Zoom to active vertex.", name(v))
        },
        _ => String::new(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The presentation of the snapshot at `index`, or `None` if there is none.
pub(crate) fn present(ctx: &Context, index: usize, choices: &Choices) -> Option<Presentation> {
    let s = ctx.trace.snapshot(index)?;
    let attention = s.attention();
    let n = s.index.len();
    let selected_component = choices.component.filter(|&c| c >= 1 && c <= s.components.len());
    let inspected = match s.event {
        Some(Event::Inspect { v, w, kind, .. }) => edge_index(ctx.edges, v, w).map(|e| (e, kind)),
        _ => None,
    };

    let vertices = (0..n)
        .map(|v| {
            let state = match (s.index[v], s.on_stack[v]) {
                (None, _) => VertexState::Undiscovered,
                (Some(_), true) => VertexState::OnStack,
                (Some(_), false) => VertexState::Closed,
            };
            let component = s.membership[v].map(|c| c + 1);
            let mut label = format!(
                "{}: index {}, lowlink {}, {}",
                name(v),
                two(s.index[v]),
                two(s.lowlink[v]),
                match state {
                    VertexState::Undiscovered => "not discovered yet".to_owned(),
                    VertexState::OnStack => "on the SCC stack".to_owned(),
                    VertexState::Closed => format!("in component C{}", component.unwrap_or(0)),
                }
            );
            if attention == Some(v) {
                label.push_str(", the vertex in focus");
            }
            VertexView {
                values: [two(s.index[v]), two(s.lowlink[v])],
                state,
                component,
                current: attention == Some(v),
                label,
                tab_stop: choices.roving == Target::Vertex(v),
                selected: choices.selected == Some(Target::Vertex(v)),
            }
        })
        .collect();

    let edges = ctx
        .edges
        .iter()
        .enumerate()
        .map(|(e, &(a, b))| {
            let mut label = format!("Edge {} from {} to {}", edge_name(e), name(a), name(b));
            let now = inspected.filter(|&(i, _)| i == e).map(|(_, k)| k);
            match (now, classify(s, a, b)) {
                (Some(kind), _) => label.push_str(&format!(", being inspected now: a {}", kind_word(kind))),
                (None, Some(kind)) => {
                    label.push_str(&format!(", from a discovered vertex: it would be a {}", kind_word(kind)))
                },
                (None, None) => label.push_str(", not reached yet"),
            }
            EdgeView {
                inspected: now,
                label,
                tab_stop: choices.roving == Target::Edge(e),
                selected: choices.selected == Some(Target::Edge(e)),
            }
        })
        .collect();

    let outlines = s
        .components
        .iter()
        .enumerate()
        .map(|(c, members)| OutlineView {
            component: c + 1,
            members: members.clone(),
            selected: selected_component == Some(c + 1),
        })
        .collect();

    let stack = s
        .scc_stack
        .iter()
        .map(|&v| StackRow {
            vertex: v,
            text: format!("{}: index {}, lowlink {}", name(v), two(s.index[v]), two(s.lowlink[v])),
            cell: format!("{} · {} · {}", name(v), two(s.index[v]), two(s.lowlink[v])),
        })
        .collect();

    let roster = s
        .components
        .iter()
        .enumerate()
        .map(|(c, members)| {
            let mut sorted = members.clone();
            sorted.sort_unstable();
            RosterRow {
                component: c + 1,
                text: format!(
                    "C{} ({} {}): {}",
                    c + 1,
                    members.len(),
                    if members.len() == 1 { "vertex" } else { "vertices" },
                    names(&sorted)
                ),
                title: format!(
                    "C{} · {} {}",
                    c + 1,
                    members.len(),
                    if members.len() == 1 { "vertex" } else { "vertices" }
                ),
                members: names(&sorted),
                selected: selected_component == Some(c + 1),
            }
        })
        .collect();

    let condensation = Condensation::of(ctx.graph, s);
    let visible = attention.map(|v| visibility(&ctx.camera, ctx.viewport, ctx.layout, v));
    let mut event_text = describe(&s.event, s);
    if let Some(Event::Inspect { w, .. }) = &s.event {
        if visibility(&ctx.camera, ctx.viewport, ctx.layout, *w) == Visibility::OffScreen {
            event_text.push_str(&format!(" The target {} is off-screen.", name(*w)));
        }
    }

    let mut panel = vec![String::new(); PANEL.len()];
    panel[0] = format!("{index} of {}", ctx.trace.last());
    panel[1] = event_text.clone();
    panel[2] = match attention {
        Some(v) => name(v),
        None => "none".to_owned(),
    };
    panel[3] = view_notice(visible, attention);
    panel[4] = match s.scc_stack.last() {
        Some(&top) => format!("depth {}, top {}", s.scc_stack.len(), name(top)),
        None => "empty".to_owned(),
    };
    panel[5] = if s.chain.is_empty() {
        "empty".to_owned()
    } else {
        format!("{} calls: {}", s.chain.len(), names(&s.chain))
    };
    panel[6] = format!(
        "{} found{}",
        s.components.len(),
        s.components.last().map_or(String::new(), |c| format!(
            "; newest C{} has {} vertices",
            s.components.len(),
            c.len()
        ))
    );
    panel[7] = dag_text(&condensation);
    panel[8] = inspector(ctx, s, choices.selected);

    let status = match &s.event {
        None => format!("Step 0 of {}. {event_text}", ctx.trace.last()),
        Some(_) => format!("Step {index} of {}. {event_text}", ctx.trace.last()),
    };
    Some(Presentation {
        vertices,
        edges,
        outlines,
        stack,
        roster,
        dag_selected: selected_component.filter(|&c| condensation.nodes.iter().any(|n| n.id == c)),
        condensation,
        panel,
        status,
    })
}

/// What the inspector says of the selected vertex or edge.
fn inspector(ctx: &Context, s: &Snapshot, selected: Option<Target>) -> String {
    match selected {
        None => "Nothing selected. Select a vertex or an edge to see it described.".to_owned(),
        Some(Target::Vertex(v)) => {
            let out: Vec<usize> = ctx.graph.targets(v).to_vec();
            format!(
                "{}: index {}, lowlink {}, {}. Edges out: {}.",
                name(v),
                two(s.index[v]),
                two(s.lowlink[v]),
                match (s.index[v], s.on_stack[v], s.membership[v]) {
                    (None, ..) => "not discovered yet".to_owned(),
                    (_, true, _) => "on the SCC stack".to_owned(),
                    (_, false, Some(c)) => format!("in component C{}", c + 1),
                    _ => "closed".to_owned(),
                },
                names(&out)
            )
        },
        Some(Target::Edge(e)) => match ctx.edges.get(e) {
            Some(&(a, b)) => format!(
                "{}: from {} to {}. {}",
                edge_name(e),
                name(a),
                name(b),
                match classify(s, a, b) {
                    Some(kind) => format!("At this step it would be a {}.", kind_word(kind)),
                    None => format!("{} has not been discovered, so it has not been reached.", name(a)),
                }
            ),
            None => "That edge does not exist.".to_owned(),
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One change to make to a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Write {
    Vertex(usize, VertexView),
    Edge(usize, EdgeView),
    Outlines(Vec<OutlineView>),
    Stack(Vec<StackRow>),
    Roster(Vec<RosterRow>),
    /// The condensation and the component picked out in it. The page rebuilds its scene only if the nodes and arrows
    /// differ.
    Dag(Condensation, Option<usize>),
    Panel(usize, String),
}

/// The writes that draw all of `p`.
pub(crate) fn full(p: &Presentation) -> Vec<Write> {
    let mut writes: Vec<Write> = Vec::new();
    writes.extend(p.vertices.iter().cloned().enumerate().map(|(i, v)| Write::Vertex(i, v)));
    writes.extend(p.edges.iter().cloned().enumerate().map(|(i, e)| Write::Edge(i, e)));
    writes.push(Write::Outlines(p.outlines.clone()));
    writes.push(Write::Stack(p.stack.clone()));
    writes.push(Write::Roster(p.roster.clone()));
    writes.push(Write::Dag(p.condensation.clone(), p.dag_selected));
    writes.extend(p.panel.iter().cloned().enumerate().map(|(i, t)| Write::Panel(i, t)));
    writes
}

/// The writes that turn a page that shows `before` into one that shows `after`.
pub(crate) fn diff(before: &Presentation, after: &Presentation) -> Vec<Write> {
    let mut writes: Vec<Write> = Vec::new();
    for (i, (a, b)) in before.vertices.iter().zip(&after.vertices).enumerate() {
        if a != b {
            writes.push(Write::Vertex(i, b.clone()));
        }
    }
    for (i, (a, b)) in before.edges.iter().zip(&after.edges).enumerate() {
        if a != b {
            writes.push(Write::Edge(i, b.clone()));
        }
    }
    if before.outlines != after.outlines {
        writes.push(Write::Outlines(after.outlines.clone()));
    }
    if before.stack != after.stack {
        writes.push(Write::Stack(after.stack.clone()));
    }
    if before.roster != after.roster {
        writes.push(Write::Roster(after.roster.clone()));
    }
    if before.condensation != after.condensation || before.dag_selected != after.dag_selected {
        writes.push(Write::Dag(after.condensation.clone(), after.dag_selected));
    }
    for (i, (a, b)) in before.panel.iter().zip(&after.panel).enumerate() {
        if a != b {
            writes.push(Write::Panel(i, b.clone()));
        }
    }
    writes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which buttons have nowhere to go, as dimmed controls do. A dimmed control is still focusable and still heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub(crate) restart: bool,
    pub(crate) previous: bool,
    pub(crate) next: bool,
    pub(crate) play: bool,
    pub(crate) previous_component: bool,
    pub(crate) next_component: bool,
    pub(crate) last: bool,
}

pub(crate) fn bounds(trace: &Trace, index: usize) -> Bounds {
    let last = trace.last();
    Bounds {
        restart: index == 0,
        previous: index == 0,
        next: index >= last,
        play: index >= last,
        previous_component: trace.previous_closure(index).is_none(),
        next_component: trace.next_closure(index).is_none(),
        last: index >= last,
    }
}

/// Whether playback says anything aloud for the move to `index`: a component closing, or the end.
pub(crate) fn announced_while_playing(trace: &Trace, index: usize) -> bool {
    matches!(
        trace.snapshot(index).and_then(|s| s.event.as_ref()),
        Some(Event::Closed { .. }) | Some(Event::Complete)
    )
}

/// What is said aloud when playback pauses.
pub(crate) fn pause_summary(trace: &Trace, index: usize) -> String {
    let found = trace.snapshot(index).map_or(0, |s| s.components.len());
    format!("Paused at step {index} of {}. {found} components found.", trace.last())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
