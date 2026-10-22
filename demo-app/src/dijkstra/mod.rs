//! The Dijkstra panel:
//!
//! - `panel-dijkstra` / `#dijkstra-diagram` — [`build_dijkstra_demo`]: shortest paths in a weighted directed graph,
//!   found one step at a time.

mod algorithm;
mod layout;

use crate::source_file;
use algorithm::{Graph, RawEdge, Relaxation, Snapshot, random_weights, steps};
use demo_common::{
    focus,
    util::{
        MARGIN, add_step_driver, announce, clear_error, document, ensure_svg_in, report_error, required_element,
        resize_svg, stringify,
    },
};
use layout::{NAMES, Placement, Texts, vertex_origins, vertex_size};
use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    EdgeId, NodeId,
    scene::{
        BoolFormat, ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, LabellingStyle,
        NodeOptions, NodeValues, Scene, Selection, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the algorithm, then where everything goes and what each cell says.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("dijkstra/algorithm/mod.rs", "algorithm/mod.rs"),
    source_file!("dijkstra/layout/mod.rs", "layout/mod.rs"),
    source_file!("dijkstra/mod.rs", "mod.rs"),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ids of the panel's own elements.
const STAGE_ID: &str = "dijkstra-diagram-stage";
const DIAGRAM_ID: &str = "dijkstra-diagram";
const ERROR_ID: &str = "dijkstra-error";
const ANNOUNCEMENT_ID: &str = "dijkstra-announcement";

/// The graph: the weights of its edges, in the order they are drawn, which is the order their labels are ranked in. The
/// vertices are [`NAMES`], and A is where the run starts. The run stops when it settles F.
const EDGES: [RawEdge; 9] = [
    (0, 1, 4),
    (0, 2, 2),
    (2, 1, 1),
    (1, 3, 5),
    (2, 3, 8),
    (2, 4, 10),
    (3, 4, 2),
    (3, 5, 6),
    (4, 5, 3),
];
const SOURCE_VERTEX: usize = 0;
const DESTINATION: usize = 5;

/// The names of the three arrays and the queue, which are the names of their nodes.
const DISTANCE: &str = "Distance";
const PREVIOUS: &str = "Previous";
const SETTLED: &str = "Settled";
const QUEUE: &str = "Queue, nearest first";

/// The fill of a vertex that is settled: the lilac the sorting demos use for an element in its final place.
const SETTLED_FILL: &str = "#d9c9f3";

/// How much room the note is given before it makes the scene taller.
const NOTE_ROOM: f64 = 150.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The graph, and every step of the run on it. Worked out once, in plain Rust.
struct Model {
    graph: Graph,
    steps: Vec<Snapshot>,
    texts: Vec<Texts>,
}

impl Model {
    /// The model of the graph with these edges.
    fn new(edges: &[RawEdge]) -> Result<Self, String> {
        let graph = Graph::new(&NAMES, edges).map_err(|e| e.to_string())?;
        let steps = steps(&graph, SOURCE_VERTEX, Some(DESTINATION)).map_err(|e| e.to_string())?;
        let texts = steps.iter().map(|step| Texts::of(&graph, step)).collect();
        Ok(Self { graph, steps, texts })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What does not change from step to step: the widest string of each column, which keeps the columns the same width in
/// every step, and so where everything goes.
struct Fixed {
    distance_hint: String,
    previous_hint: String,
    queue_hint: String,
    placement: Placement,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the demo finds again between steps.
#[derive(Default)]
struct Runtime {
    model: Option<Rc<Model>>,
    fixed: Option<Rc<Fixed>>,
    /// The scene on the page. Every step draws a new one, so this keeps the last one alive.
    scene: Option<Scene>,
    /// The vertices of that scene, in the order of [`NAMES`].
    vertices: Vec<NodeId>,
    /// Where each vertex's box is, as the last scene had it: where the person dragged it to, or where it started.
    origins: Vec<Point>,
    /// The step the scene on the page is at.
    position: Option<usize>,
}

thread_local! {
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::default());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The model, worked out on first use.
fn model() -> Result<Rc<Model>, String> {
    if let Some(model) = RUNTIME.with_borrow(|runtime| runtime.model.clone()) {
        return Ok(model);
    }
    let model = Rc::new(Model::new(&EDGES)?);
    RUNTIME.with_borrow_mut(|runtime| runtime.model = Some(model.clone()));
    Ok(model)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives the same graph new random weights, and works out the run on them. The sizes of the arrays and the queue were
/// measured from the old run's texts, so they are measured again, and everything else is kept where it is: the view,
/// and where each vertex was dragged to.
fn use_random_weights() -> Result<(), String> {
    let weights = random_weights(EDGES.len(), js_sys::Math::random);
    let edges: Vec<RawEdge> = EDGES.iter().zip(&weights).map(|(&(from, to, _), &w)| (from, to, w)).collect();
    let model = Rc::new(Model::new(&edges)?);
    RUNTIME.with_borrow_mut(|runtime| {
        runtime.model = Some(model);
        runtime.fixed = None;
    });
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The content of one of the two text arrays, one row for each vertex, each row labelled with its vertex.
fn text_array(values: &[String], hint: &str) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Text(values.to_vec()), DataFormat::Text)
        .with_layout(GridLayout::Columns(1))
        .with_labelling_style(LabellingStyle::Text(&NAMES))
        .with_cell_width_hint(hint)
}

/// The content of the array of which vertices are settled, one row for each vertex, each row labelled with its vertex.
fn settled_array(flags: &[bool]) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Bool(flags.to_vec()), DataFormat::Bool(BoolFormat::TrueFalse))
        .with_layout(GridLayout::Columns(1))
        .with_labelling_style(LabellingStyle::Text(&NAMES))
}

/// The content of the queue: always the same number of cells, however many entries there are.
fn queue_array(cells: &[String], hint: &str) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Text(cells.to_vec()), DataFormat::Text)
        .with_layout(GridLayout::Columns(1))
        .with_cell_width_hint(hint)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The widest string of each column across every step, and from them the sizes of the arrays and the queue, and where
/// everything goes. Measured in the browser, since a character such as `∞` can come from a font whose width is not that
/// of the others. Done once, and then the same for every step.
fn fixed(scene: &Scene, model: &Model) -> Result<Rc<Fixed>, String> {
    if let Some(fixed) = RUNTIME.with_borrow(|runtime| runtime.fixed.clone()) {
        return Ok(fixed);
    }
    let width_of = |text: &str| -> Result<f64, String> {
        let content = DataNodeContent::new(NodeValues::Text(vec![text.to_owned()]), DataFormat::Text);
        scene.measure_data_node(&content).map(|size| size.width).map_err(stringify)
    };
    let widest = |column: fn(&Texts) -> &Vec<String>| -> Result<String, String> {
        let strings = model.texts.iter().flat_map(|texts| column(texts).clone());
        Ok(layout::widest_by(strings, width_of)?.unwrap_or_default())
    };
    let distance_hint = widest(|texts| &texts.distance)?;
    let previous_hint = widest(|texts| &texts.previous)?;
    let queue_hint = widest(|texts| &texts.queue)?;

    let first = &model.texts[0];
    let flags = vec![false; NAMES.len()];
    let measure =
        |name: &str, content: DataNodeContent| scene.measure_named_data_node(name, &content).map_err(stringify);
    let arrays = [
        measure(DISTANCE, text_array(&first.distance, &distance_hint))?,
        measure(PREVIOUS, text_array(&first.previous, &previous_hint))?,
        measure(SETTLED, settled_array(&flags))?,
    ];
    let queue = measure(QUEUE, queue_array(&first.queue, &queue_hint))?;
    let fixed = Rc::new(Fixed {
        distance_hint,
        previous_hint,
        queue_hint,
        placement: layout::place(arrays, queue, NOTE_ROOM),
    });
    RUNTIME.with_borrow_mut(|runtime| runtime.fixed = Some(fixed.clone()));
    Ok(fixed)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the graph for `snapshot`: a draggable box for each vertex at its origin, a connector for each edge with its
/// weight on it, and the emphasis for the step.
///
/// A settled vertex has a fill. The vertex being worked on has a focus ring, and so does the end of the edge being
/// examined. The edge being examined is emphasised, and so is every edge of the route once the run has found it. The
/// vertices are plain nodes, so they take focus. The arrays are data nodes, so they take a selection, and no node ever
/// has both.
fn draw_graph(scene: &Scene, model: &Model, snapshot: &Snapshot, origins: &[Point]) -> Result<Vec<NodeId>, String> {
    let mut vertices = Vec::with_capacity(NAMES.len());
    for (index, name) in NAMES.iter().enumerate() {
        let fill = snapshot.settled[index].then_some(SETTLED_FILL);
        let node = scene
            .add_node_with(origins[index], vertex_size(), *name, NodeOptions::default().with_fill(fill))
            .map_err(stringify)?;
        scene.make_draggable(node).map_err(stringify)?;
        vertices.push(node);
    }

    // Straight connectors, so that each runs from box to box in one line and its label has a long run to sit beside.
    let options = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    let mut edges: Vec<EdgeId> = Vec::with_capacity(model.graph.edges().len());
    for edge in model.graph.edges() {
        let (from, to, weight) = (edge.from, edge.to, edge.weight);
        let edge = scene.add_edge_with(vertices[from], vertices[to], options).map_err(stringify)?;
        scene.set_edge_label(edge, Some(&weight.to_string())).map_err(stringify)?;
        edges.push(edge);
    }

    if let Some(vertex) = snapshot.active {
        scene.set_focus(vertices[vertex], true).map_err(stringify)?;
    }
    if let Some((edge, _)) = snapshot.examined {
        scene.set_edge_focus(edges[edge], true).map_err(stringify)?;
        let to = model.graph.edges()[edge].to;
        if snapshot.active != Some(to) {
            scene.set_focus(vertices[to], true).map_err(stringify)?;
        }
    }
    for &edge in &snapshot.path {
        scene.set_edge_focus(edges[edge], true).map_err(stringify)?;
    }
    Ok(vertices)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the three arrays and the queue for `snapshot`, at the places they always have, and selects the rows that the
/// step changed: the vertex whose distance and predecessor were just improved, or the vertex just settled.
fn draw_arrays(scene: &Scene, model: &Model, fixed: &Fixed, snapshot: &Snapshot, texts: &Texts) -> Result<(), String> {
    let placement = &fixed.placement;
    let distance = scene
        .add_named_data_node(placement.distance, DISTANCE, text_array(&texts.distance, &fixed.distance_hint))
        .map_err(stringify)?;
    let previous = scene
        .add_named_data_node(placement.previous, PREVIOUS, text_array(&texts.previous, &fixed.previous_hint))
        .map_err(stringify)?;
    let settled = scene
        .add_named_data_node(placement.settled, SETTLED, settled_array(&snapshot.settled))
        .map_err(stringify)?;
    scene
        .add_named_data_node(placement.queue, QUEUE, queue_array(&texts.queue, &fixed.queue_hint))
        .map_err(stringify)?;

    match (snapshot.examined, snapshot.active) {
        (Some((_, Relaxation::Improved)), _) => {
            // The end of the edge got a shorter distance and a new predecessor, so those rows are what changed.
            let (edge, _) = snapshot.examined.ok_or("the step examined no edge")?;
            let to = model.graph.edges()[edge].to;
            scene.set_selection(distance, Selection::Cell(to)).map_err(stringify)?;
            scene.set_selection(previous, Selection::Cell(to)).map_err(stringify)?;
        },
        (None, Some(vertex)) => {
            scene.set_selection(distance, Selection::Cell(vertex)).map_err(stringify)?;
            scene.set_selection(settled, Selection::Cell(vertex)).map_err(stringify)?;
        },
        _ => {},
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a screen reader is told when the walk is at `position` of `count`: how far along it is, and what the step did.
fn announcement(position: usize, count: usize, note: &str) -> String {
    format!("Step {} of {count}. {note}", position + 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Remembers where each vertex of the scene on the page is, before it is replaced. A person may have dragged one, and
/// the next scene puts it back where they left it.
fn remember_origins() -> Result<(), String> {
    let (scene, vertices) = RUNTIME.with_borrow(|runtime| (runtime.scene.clone(), runtime.vertices.clone()));
    let Some(scene) = scene else { return Ok(()) };
    let origins = vertices
        .iter()
        .map(|&vertex| scene.node_rect(vertex).map(|rect| rect.origin).map_err(stringify))
        .collect::<Result<Vec<_>, _>>()?;
    if origins.len() == NAMES.len() {
        RUNTIME.with_borrow_mut(|runtime| runtime.origins = origins);
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene at step `position`, wires its stepping toolbar, and replaces whatever scene was there. It keeps the
/// zoom and pan, where each vertex was dragged to, and the keyboard focus on a button that the old scene had. Returns
/// what a screen reader should be told.
///
/// This is the way the Huffman, Hamming and CRC-32 demos draw a step again: note the view, the logical control that had
/// focus and where the vertices are before clearing the stage, draw the new scene at those places, give it the view,
/// and focus the control that matches. It is not a swap that can be undone: if drawing fails after the old scene is
/// cleared, the error is reported in the panel and the next step draws the scene again.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the panel, or if any library call fails.
fn build(position: usize) -> Result<String, String> {
    let model = model()?;
    let position = position.min(model.steps.len().saturating_sub(1));
    let snapshot = &model.steps[position];
    let view = RUNTIME.with_borrow(|runtime| runtime.scene.as_ref().map(Scene::view));
    remember_origins()?;
    let focused = focus::focused_selector(DIAGRAM_ID);
    let origins = RUNTIME.with_borrow(|runtime| runtime.origins.clone());
    let origins = if origins.len() == NAMES.len() { origins } else { vertex_origins().to_vec() };

    let document = document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 1000.0))?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let fixed = fixed(&scene, &model)?;
    let vertices = draw_graph(&scene, &model, snapshot, &origins)?;
    draw_arrays(&scene, &model, &fixed, snapshot, &model.texts[position])?;

    // The note on the step, under the graph. It is as tall as its text needs. Everything else is where it always is.
    let note_text = format!("### Step {} of {}\n{}", position + 1, model.steps.len(), snapshot.note);
    let note = scene
        .add_markdown_node(fixed.placement.note, fixed.placement.note_width, &note_text)
        .map_err(stringify)?;
    let note_rect = scene.node_rect(note).map_err(stringify)?;
    let canvas = Size::new(
        fixed.placement.canvas.width,
        fixed
            .placement
            .canvas
            .height
            .max(note_rect.origin.y + note_rect.size.height + MARGIN),
    );
    resize_svg(&document, DIAGRAM_ID, canvas)?;
    scene.refresh_layout().map_err(stringify)?;

    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }
    let step = add_step_driver(&scene, DIAGRAM_ID, model.steps.len())?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    scene
        .show_selection_toolbar(
            step,
            SelectionToolbarOptions::new(Side::North),
            move |_scene, _node, transition| {
                match transition.to {
                    Some(position) => go(position, false),
                    // "Restart" and "Previous" from the first position are the same transition, so both land here. Stepping
                    // back from the start is therefore a restart, with new random weights.
                    None => go(0, true),
                }
            },
        )
        .map_err(stringify)?;
    // `show_selection_toolbar` always resets "Step" to unstarted as its first committed act. Reapply the position.
    scene.set_selection(step, Selection::Cell(position)).map_err(stringify)?;

    RUNTIME.with_borrow_mut(|runtime| {
        runtime.scene = Some(scene);
        runtime.vertices = vertices;
        runtime.origins = origins;
        runtime.position = Some(position);
    });
    if let Some(selector) = focused {
        focus::focus(&selector);
    }
    Ok(announcement(position, model.steps.len(), &snapshot.note))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the diagram to step `position`, and announces it. `restart` is whether the person pressed "Restart": the graph
/// is then given new random weights, and the announcement says so first. A failure is shown in the panel's own error
/// banner, since the toolbar's own callback has nowhere to return it to. The banner clears again at the next step that
/// works.
fn go(position: usize, restart: bool) {
    let result = if restart {
        use_random_weights().and_then(|()| build(position))
    } else {
        build(position)
    };
    match result {
        Ok(said) => {
            clear_error(ERROR_ID);
            let said = if restart { format!("The edge weights are new. {said}") } else { said };
            announce(ANNOUNCEMENT_ID, &said);
        },
        Err(e) => report_error(ERROR_ID, &format!("This step failed (position {position}): {e}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Dijkstra demo: a weighted directed graph of six vertices, and the run of Dijkstra's algorithm from A to F
/// on it, one step at a time.
///
/// - The **graph** is on the left. A box is a vertex, and can be dragged. A connector is a directed edge, and its
///   weight is on a label beside it, which follows the connector as a box is dragged. A settled vertex is filled lilac.
///   The vertex being worked on has a focus ring, and so does the end of the edge being looked at. That edge is
///   emphasised, and so is every edge of the shortest route when the run has found it.
/// - The three **arrays** are the distance to each vertex, where it was reached from, and whether it is settled. An
///   unreached vertex has the distance `∞` and no predecessor, which is `—`. The rows that a step changed are selected.
/// - The **queue** shows what is waiting, nearest first, which is the order it would be taken in and not the order a
///   binary heap keeps it in. An entry that is out of date says `stale` and is ignored when it is taken out. The queue
///   has a fixed size, so no step moves anything: it shows eight entries and says how many more there are.
/// - A **note** under the graph says what the step did, and a selection toolbar at the top steps through the run. Every
///   step is announced in a live region outside the scene.
///
/// The graph opens with fixed weights, so that every visit starts the same. "Restart" gives its edges new random weights
/// from 1 to 10, and the run is worked out again on them.
///
/// Every value is worked out in plain Rust by [`algorithm::steps`]. The library does no graph algorithm: it draws.
///
/// # Errors
///
/// Returns `Err` if the panel is missing a part, or if any library call fails.
pub(crate) fn build_dijkstra_demo() -> Result<(), String> {
    let said = build(0)?;
    announce(ANNOUNCEMENT_ID, &said);
    Ok(())
}
