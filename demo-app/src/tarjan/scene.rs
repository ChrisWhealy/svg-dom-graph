//! The scene: everything on the page that is drawn is in this one Scene, so it zooms and pans as one.
//!
//! - The controls: a layer on the root of the `<svg>`, above the content, so that it never zooms or pans.
//! - The graph: a named data node for each vertex, with two cells, its index and its lowlink, and a straight connector
//!   for each edge. An edge whose reverse is also an edge is pinned to a lane so that the two run side by side.
//! - The stack: one data node of 27 text cells, which is as deep as the stack gets. The cells are rewritten in place,
//!   the newest vertex in the first, and the cells that are not in use are empty.
//! - The roster of components and the condensation: rows and nodes that are made when a component is first emitted, and
//!   hidden again when a step goes back before it. The library has no way to take a node away, so a node that has been
//!   shown stays in the scene, and a node for a component that is not yet emitted has not been made.
//!
//! Above them, in the same transformed group, is the overlay, painted in this order: the outlines of the components,
//! one transparent hit path for each of the 114 edges, and last one transparent hit rectangle for each of the 64
//! vertices. A click on a vertex is therefore a click on that vertex, even where an edge passes over it. The library's
//! own nodes and connectors for the graph are hidden from assistive technology, since the hit targets say what they
//! are.

use super::{
    condensation::{BOX, Condensation},
    dom::{add, make, set},
    fixture::name,
    layout::{End, Layout, ROSTER_AT, ROSTER_ROW, STACK_AT, STACK_ROWS, ends},
    projection::{EdgeView, OutlineView, RosterRow, StackRow, VertexView, edge_name},
    solver::Kind,
    ui_layout::CONTROLS,
    view::Camera,
};
use demo_common::util::stringify;
use std::{cell::RefCell, collections::BTreeMap};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, LabellingStyle, NodeOptions,
        NodeValues, Scene, Side, ToolbarOptions, ViewTransform,
    },
};
use wasm_bindgen::JsCast;

/// The labels of the rows of the stack, which count from the top.
const STACK_LABELS: &[&str] = &[
    "top", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
    "21", "22", "23", "24", "25", "26", "27",
];

/// The fill of a node of the condensation.
const DAG_FILL: &str = "#e0f2fe";

/// A node's content: two text cells side by side, the index on the left and the lowlink on the right, as two digits or
/// two dashes, which never change in number or in kind, so that they can be rewritten in place without the node being
/// resized.
fn vertex_content(index: &str, lowlink: &str) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Text(vec![index.to_owned(), lowlink.to_owned()]), DataFormat::Text)
        .with_layout(GridLayout::Columns(2))
        .with_cell_width_hint("63")
}

/// The content of the stack's node: a cell for each place in the stack, all empty.
fn stack_content() -> DataNodeContent {
    DataNodeContent::new(NodeValues::Text(vec![String::new(); STACK_ROWS]), DataFormat::Text)
        .with_layout(GridLayout::Columns(1))
        .with_labelling_style(LabellingStyle::Text(STACK_LABELS))
        .with_cell_width_hint("v00 · 00 · 00")
}

fn n(x: f64) -> String {
    format!("{x:.1}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One row of the roster.
struct Row {
    x: f64,
    y: f64,
    group: web_sys::Element,
    /// The text of the row, in which a line for the title and as many lines as the members need are written.
    text: web_sys::Element,
}

/// The size of the writing in a row of the roster.
const ROSTER_FONT: f64 = 15.0;

/// The nodes and arrows of the condensation that have been made so far.
#[derive(Default)]
pub(super) struct DagParts {
    /// The node of each component, by its C-ID less one: the library's id of it, and its group.
    pub(super) nodes: Vec<Option<(NodeId, web_sys::Element)>>,
    /// The connector of each arrow that has been made.
    edges: BTreeMap<(usize, usize), web_sys::Element>,
    /// The top left of each node as the table placed it, which is where it goes back to after the springs.
    pub(super) home: Vec<Option<(f64, f64)>>,
    /// The arrows that are shown now, as pairs of C-IDs.
    pub(super) shown: Vec<(usize, usize)>,
}

/// The scene, built and in the page.
pub(super) struct Board {
    /// Kept so that the scene, and the listeners it put on the page, live as long as the page does.
    pub(super) scene: Scene,
    svg: web_sys::Element,
    content: web_sys::Element,
    pub(super) nodes: Vec<NodeId>,
    stack: NodeId,
    pub(super) layout: Layout,
    outlines: web_sys::Element,
    pub(super) native: Vec<web_sys::Element>,
    pub(super) edge_hits: Vec<web_sys::Element>,
    pub(super) vertex_hits: Vec<web_sys::Element>,
    roster: Vec<Row>,
    pub(super) dag: RefCell<DagParts>,
    document: web_sys::Document,
    /// What the springs are doing.
    pub(super) motion: RefCell<super::motion::Motion>,
}

/// Builds the scene in a new `<svg>` in `host`, with the graph, the stack, the roster and the overlay.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added, or the scene did not draw what the graph has.
pub(super) fn build_board(
    document: &web_sys::Document,
    host: &web_sys::Element,
    edges: &[(usize, usize)],
    instance: u32,
    vertices: usize,
) -> Result<Board, String> {
    let scene_id = format!("tj-{instance}-scene");
    let svg = make(
        document,
        "svg",
        &[
            ("id", &scene_id),
            ("class", "tj-scene-svg"),
            ("style", "display: block; width: 100%; height: 100%;"),
            ("role", "group"),
            (
                "aria-label",
                "Tarjan scene: the graph of 64 vertices and 114 edges, the stack, the components and the condensation",
            ),
        ],
    )?;
    add(host, &svg)?;
    let scene = Scene::new(SvgRoot::attach(&scene_id).map_err(stringify)?).map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    scene.refresh_toolbar_layout().map_err(stringify)?;

    // The layout is worked out from the size that a vertex's node really has.
    let size = scene
        .measure_named_data_node(&name(0), &vertex_content("--", "--"))
        .map_err(stringify)?;
    let layout = Layout::build(size.width, size.height);
    let mut nodes = Vec::with_capacity(vertices);
    for v in 0..vertices {
        let r = layout.rect(v);
        nodes.push(
            scene
                .add_named_data_node_with(
                    Point::new(r.x, r.y),
                    &name(v),
                    vertex_content("--", "--"),
                    NodeOptions::default(),
                )
                .map_err(stringify)?,
        );
    }
    let stack = scene
        .add_named_data_node_with(
            Point::new(STACK_AT.0, STACK_AT.1),
            "Tarjan SCC stack",
            stack_content(),
            NodeOptions::default(),
        )
        .map_err(stringify)?;
    for &(a, b) in edges {
        let lanes = ends(&layout, a, b, edges.binary_search(&(b, a)).is_ok());
        scene
            .add_edge_with(nodes[a], nodes[b], connector(lanes.from, lanes.to))
            .map_err(stringify)?;
    }

    // The graph's own nodes and connectors are the picture. The overlay is what is read. The stack's node is read as it
    // is.
    let content = svg
        .query_selector(".svg-dom-graph-content")
        .map_err(|e| format!("could not look for the content group: {e:?}"))?
        .ok_or_else(|| "the scene has no content group".to_owned())?;
    let mut native = Vec::with_capacity(edges.len());
    let mut groups = 0;
    let mut child = content.first_element_child();
    while let Some(element) = child {
        child = element.next_element_sibling();
        if element.tag_name().eq_ignore_ascii_case("path") {
            set(&element, "aria-hidden", "true")?;
            native.push(element);
        } else if groups < vertices {
            set(&element, "aria-hidden", "true")?;
            groups += 1;
        } else {
            set(&element, "data-tj-stack", "true")?;
        }
    }
    if native.len() != edges.len() {
        return Err(format!("the scene drew {} connectors, not {}", native.len(), edges.len()));
    }
    for (e, path) in native.iter().enumerate() {
        set(path, "data-tj-edge", &edge_name(e))?;
        set(path, "id", &format!("tj-{instance}-edge-{e}"))?;
    }

    let overlay = make(document, "g", &[("class", "tj-overlay")])?;
    let headings = make(document, "g", &[("class", "tj-headings"), ("aria-hidden", "true")])?;
    add(&overlay, &headings)?;
    for (x, y, text) in [
        (ROSTER_AT.0, ROSTER_AT.1 - 12.0, "Components found"),
        (
            layout.dag_origin.0,
            layout.dag_origin.1 - 40.0,
            "Condensation: arrows point right to left, toward the sink",
        ),
    ] {
        let heading = make(document, "text", &[("class", "tj-heading"), ("x", &n(x)), ("y", &n(y))])?;
        heading.set_text_content(Some(text));
        add(&headings, &heading)?;
    }
    add(&content, &overlay)?;
    let outlines = make(document, "g", &[("class", "tj-outlines"), ("aria-hidden", "true")])?;
    add(&overlay, &outlines)?;
    let roster_layer = make(
        document,
        "g",
        &[("class", "tj-roster"), ("role", "group"), ("aria-label", "Components found")],
    )?;
    add(&overlay, &roster_layer)?;
    let edge_layer = make(document, "g", &[("class", "tj-edge-hits")])?;
    add(&overlay, &edge_layer)?;
    let vertex_layer = make(document, "g", &[("class", "tj-vertex-hits")])?;
    add(&overlay, &vertex_layer)?;

    let mut roster = Vec::with_capacity(10);
    for i in 0..10 {
        let r = Layout::roster_row(i);
        let group = make(
            document,
            "g",
            &[
                ("class", "tj-roster-row"),
                ("data-tj-roster", &(i + 1).to_string()),
                ("data-tj-component", &(i + 1).to_string()),
                ("data-tj-hidden", "true"),
                ("role", "button"),
                ("tabindex", "-1"),
            ],
        )?;
        add(
            &group,
            &make(
                document,
                "rect",
                &[
                    ("x", &n(r.x)),
                    ("y", &n(r.y)),
                    ("width", &n(r.width)),
                    ("height", &n(r.height)),
                    ("rx", "6"),
                ],
            )?,
        )?;
        let text = make(
            document,
            "text",
            &[
                ("x", &n(r.x + 10.0)),
                ("y", &n(r.y + r.height / 2.0)),
                ("dominant-baseline", "central"),
            ],
        )?;
        add(&group, &text)?;
        add(&roster_layer, &group)?;
        roster.push(Row { x: r.x, y: r.y, group, text });
    }

    let mut edge_hits = Vec::with_capacity(edges.len());
    for (e, path) in native.iter().enumerate() {
        let hit = make(
            document,
            "path",
            &[
                ("class", "tj-edge-hit"),
                ("id", &format!("tj-{instance}-edge-hit-{e}")),
                ("data-tj-edge-hit", &edge_name(e)),
                ("d", &path.get_attribute("d").unwrap_or_default()),
                ("role", "button"),
                ("tabindex", "-1"),
            ],
        )?;
        add(&edge_layer, &hit)?;
        edge_hits.push(hit);
    }
    let mut vertex_hits = Vec::with_capacity(vertices);
    for v in 0..vertices {
        let r = layout.rect(v);
        let hit = make(
            document,
            "rect",
            &[
                ("class", "tj-vertex"),
                ("id", &format!("tj-{instance}-vertex-{v}")),
                ("data-tj-vertex", &name(v)),
                ("x", &n(r.x)),
                ("y", &n(r.y)),
                ("width", &n(r.width)),
                ("height", &n(r.height)),
                ("rx", "6"),
                ("role", "button"),
                ("tabindex", "-1"),
            ],
        )?;
        add(&vertex_layer, &hit)?;
        vertex_hits.push(hit);
    }
    let mut known = content.first_element_child();
    while let Some(element) = known {
        set(&element, "data-tj-known", "true")?;
        known = element.next_element_sibling();
    }
    set(&svg, "data-tj-springs", "idle")?;
    Ok(Board {
        scene,
        svg,
        content,
        nodes,
        stack,
        layout,
        outlines,
        native,
        edge_hits,
        vertex_hits,
        roster,
        dag: RefCell::new(DagParts::default()),
        document: document.clone(),
        motion: RefCell::new(super::motion::Motion::Idle),
    })
}

/// The options of a straight connector whose ends are fixed as `from` and `to` say.
fn connector(from: End, to: End) -> ConnectorOptions {
    let mut options = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    if let End::Pinned(side, at) = from {
        options = options.with_from_side(Some(side)).with_from_position(Some(at));
    }
    if let End::Pinned(side, at) = to {
        options = options.with_to_side(Some(side)).with_to_position(Some(at));
    }
    options
}

impl Board {
    /// The element that the library has just added to the content group, which is the one that is not marked as known.
    /// It is marked, so that the next call finds the next one.
    fn fresh(&self) -> Result<web_sys::Element, String> {
        let mut child = self.content.first_element_child();
        while let Some(element) = child {
            if !element.has_attribute("data-tj-known") {
                set(&element, "data-tj-known", "true")?;
                return Ok(element);
            }
            child = element.next_element_sibling();
        }
        Err("the library added nothing to the scene".to_owned())
    }

    /// The view that the scene has now.
    ///
    /// The controller's origin is the top left of what the controls leave uncovered, so the library's own `ty` is that
    /// much larger.
    pub(super) fn camera(&self) -> Camera {
        let v = self.scene.view();
        Camera {
            scale: v.scale,
            tx: v.tx,
            ty: v.ty - CONTROLS.height,
        }
    }

    /// Writes the view.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the view is outside what the library allows, or could not be written.
    pub(super) fn set_camera(&self, camera: Camera) -> Result<(), String> {
        self.scene
            .set_view(ViewTransform {
                scale: camera.scale,
                tx: camera.tx,
                ty: camera.ty + CONTROLS.height,
            })
            .map_err(stringify)
    }

    pub(super) fn svg(&self) -> &web_sys::Element {
        &self.svg
    }

    /// The size of the viewport, in screen pixels, which is what the scene's `<svg>` fills of its window, less the
    /// strip of controls that covers its top.
    pub(super) fn viewport(&self) -> (f64, f64) {
        (
            f64::from(self.svg.client_width()),
            (f64::from(self.svg.client_height()) - CONTROLS.height).max(1.0),
        )
    }

    /// Puts the controls in the scene: a layer on the root of the `<svg>`, above the content, so that it never zooms or
    /// pans, with a background so that the graph does not show through. Returns the layer for the controls to be drawn
    /// in.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an element could not be made or added.
    pub(super) fn controls_layer(&self) -> Result<web_sys::Element, String> {
        let layer = make(
            &self.document,
            "g",
            &[("id", "tj-controls-svg"), ("role", "group"), ("aria-label", "Tarjan controls")],
        )?;
        add(
            &layer,
            &make(
                &self.document,
                "rect",
                &[
                    ("class", "tj-controls-background"),
                    ("width", &n(CONTROLS.width)),
                    ("height", &n(CONTROLS.height)),
                    ("fill", "#ffffee"),
                    ("stroke", "#64748b"),
                ],
            )?,
        )?;
        add(&self.svg, &layer)?;
        Ok(layer)
    }

    /// Puts the toolbar and the surface for panning back where the `<svg>` is now, which the scene cannot see for
    /// itself.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the layout could not be written.
    pub(super) fn refresh_layout(&self) -> Result<(), String> {
        self.scene.refresh_toolbar_layout().map_err(stringify)
    }

    /// The hit rectangle of a vertex, which is what the keyboard focus goes to.
    pub(super) fn vertex_element(&self, v: usize) -> Option<web_sys::Element> {
        self.vertex_hits.get(v).cloned()
    }

    /// The hit path of an edge.
    pub(super) fn edge_element(&self, e: usize) -> Option<web_sys::Element> {
        self.edge_hits.get(e).cloned()
    }

    /// Writes one vertex: the two values of its node, and the class and name of its hit rectangle.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the values or an attribute could not be written.
    pub(super) fn write_vertex(&self, v: usize, view: &VertexView) -> Result<(), String> {
        self.scene
            .set_data_values(self.nodes[v], NodeValues::Text(view.values.to_vec()))
            .map_err(stringify)?;
        let hit = &self.vertex_hits[v];
        let mut class = format!("tj-vertex tj-{}", view.state.word());
        if let Some(c) = view.component {
            class.push_str(&format!(" tj-comp-{}", (c - 1) % 10 + 1));
        }
        if view.current {
            class.push_str(" tj-current");
        }
        if view.selected {
            class.push_str(" tj-selected");
        }
        set(hit, "class", &class)?;
        set(hit, "aria-label", &view.label)?;
        set(hit, "aria-pressed", &view.selected.to_string())?;
        set(hit, "tabindex", if view.tab_stop { "0" } else { "-1" })?;
        set(hit, "data-tj-state", view.state.word())
    }

    /// Writes one edge: how the connector is marked, and the name of its hit path.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an attribute could not be written.
    pub(super) fn write_edge(&self, e: usize, view: &EdgeView) -> Result<(), String> {
        let kind = match view.inspected {
            None => "none",
            Some(Kind::Tree) => "tree",
            Some(Kind::Stack) => "stack",
            Some(Kind::Closed) => "closed",
        };
        set(&self.native[e], "data-tj-inspected", kind)?;
        let hit = &self.edge_hits[e];
        set(
            hit,
            "class",
            if view.selected { "tj-edge-hit tj-selected" } else { "tj-edge-hit" },
        )?;
        set(hit, "aria-label", &view.label)?;
        set(hit, "aria-pressed", &view.selected.to_string())?;
        set(hit, "tabindex", if view.tab_stop { "0" } else { "-1" })
    }

    /// Draws the marks of the components again: a dashed outline and a label round each vertex of each component.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an element could not be made or added.
    pub(super) fn write_outlines(&self, outlines: &[OutlineView]) -> Result<(), String> {
        self.outlines.set_text_content(None);
        for o in outlines {
            let class = format!(
                "tj-outline tj-comp-{}{}",
                (o.component - 1) % 10 + 1,
                if o.selected { " tj-selected" } else { "" }
            );
            let group = make(
                &self.document,
                "g",
                &[("class", &class), ("data-tj-component", &o.component.to_string())],
            )?;
            // The vertices of a component are not next to each other, so each has its own dashed outline and its own
            // label.
            for &v in &o.members {
                let r = self.layout.rect(v);
                let (x, y) = (r.x - 6.0, r.y - 6.0);
                let rect = make(
                    &self.document,
                    "rect",
                    &[
                        ("x", &n(x)),
                        ("y", &n(y)),
                        ("width", &n(self.layout.box_size.0 + 12.0)),
                        ("height", &n(self.layout.box_size.1 + 12.0)),
                        ("rx", "10"),
                    ],
                )?;
                add(&group, &rect)?;
                let label = make(
                    &self.document,
                    "text",
                    &[("x", &n(x + 4.0)), ("y", &n(y - 5.0)), ("class", "tj-outline-label")],
                )?;
                label.set_text_content(Some(&format!("C{}", o.component)));
                add(&group, &label)?;
            }
            add(&self.outlines, &group)?;
        }
        Ok(())
    }

    /// Writes the stack into the cells of its node: the newest vertex first, and the cells that are not in use empty.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the stack is deeper than the node has cells, or the values could not be written.
    pub(super) fn write_stack(&self, rows: &[StackRow]) -> Result<(), String> {
        if rows.len() > STACK_ROWS {
            return Err(format!("the stack is {} deep, and the node has {STACK_ROWS} cells", rows.len()));
        }
        let mut cells: Vec<String> = rows.iter().rev().map(|r| r.cell.clone()).collect();
        cells.resize(STACK_ROWS, String::new());
        self.scene
            .set_data_values(self.stack, NodeValues::Text(cells))
            .map_err(stringify)
    }

    /// Writes the roster: a row for each component that has been emitted, and the rows after them hidden.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an attribute could not be written.
    pub(super) fn write_roster(&self, rows: &[RosterRow]) -> Result<(), String> {
        for (i, row) in self.roster.iter().enumerate() {
            match rows.get(i) {
                Some(r) => {
                    self.write_row_text(row, r)?;
                    set(&row.group, "aria-label", &r.text)?;
                    set(&row.group, "aria-pressed", &r.selected.to_string())?;
                    set(
                        &row.group,
                        "class",
                        if r.selected { "tj-roster-row tj-selected" } else { "tj-roster-row" },
                    )?;
                    set(&row.group, "tabindex", "0")?;
                    set(&row.group, "data-tj-hidden", "false")?;
                },
                None => {
                    set(&row.group, "data-tj-hidden", "true")?;
                    set(&row.group, "aria-pressed", "false")?;
                    set(&row.group, "tabindex", "-1")?;
                },
            }
        }
        Ok(())
    }

    /// Writes the lines of a row: its title, and its members wrapped to fit the row. A line that is still too long,
    /// which a font that is wider than was assumed would give, is squeezed to fit, so that no text is ever outside its
    /// row.
    fn write_row_text(&self, row: &Row, r: &RosterRow) -> Result<(), String> {
        row.text.set_text_content(None);
        let width = ROSTER_ROW.0 - 20.0;
        let per_line = (width / (ROSTER_FONT * 0.62)) as usize;
        let mut lines = vec![r.title.clone()];
        let mut line = String::new();
        for word in r.members.split(", ") {
            let next = if line.is_empty() { word.to_owned() } else { format!("{line}, {word}") };
            if next.len() > per_line && !line.is_empty() {
                lines.push(format!("{line},"));
                line = word.to_owned();
            } else {
                line = next;
            }
        }
        lines.push(line);
        let step = ROSTER_FONT + 4.0;
        let top = ROSTER_ROW.1 / 2.0 - step * (lines.len() as f64 - 1.0) / 2.0;
        for (i, text) in lines.iter().enumerate() {
            let tspan = make(
                &self.document,
                "tspan",
                &[
                    ("x", &n(row.x + 10.0)),
                    ("y", &n(row.y + top + step * i as f64)),
                    ("dominant-baseline", "central"),
                ],
            )?;
            tspan.set_text_content(Some(text));
            add(&row.text, &tspan)?;
            // Measured once it is in the page, which is the only time that its real width is known.
            if let Some(shape) = tspan.dyn_ref::<web_sys::SvgTextContentElement>() {
                if f64::from(shape.get_computed_text_length()) > width {
                    set(&tspan, "textLength", &n(width))?;
                    set(&tspan, "lengthAdjust", "spacingAndGlyphs")?;
                }
            }
        }
        Ok(())
    }

    /// Shows the condensation: the node of each component that is emitted and the arrow of each pair that is known,
    /// made if they have not been made yet, and every other node and arrow hidden. Placing is by the fixed table, so a
    /// node is made in the place that it will always have. A write that fails part way leaves what was made, and the
    /// next call finds it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the condensation cannot be placed or a node or arrow could not be made or written.
    pub(super) fn write_dag(&self, c: &Condensation, selected: Option<usize>) -> Result<(), String> {
        let placed = c.placements().map_err(|e| e.to_string())?;
        let mut dag = self.dag.borrow_mut();
        if dag.nodes.len() < 10 {
            dag.nodes.resize_with(10, || None);
        }
        for (node, place) in c.nodes.iter().zip(&placed) {
            if dag.nodes[node.id - 1].is_some() {
                continue;
            }
            let (x, y) = self.layout.dag_centre(place.centre);
            let label = format!("{} · {}", node.name(), node.members.len());
            let id = self
                .scene
                .add_node_with(
                    Point::new(x - BOX.0 / 2.0, y - BOX.1 / 2.0),
                    Size::new(BOX.0, BOX.1),
                    label,
                    NodeOptions::default().with_fill(Some(DAG_FILL)),
                )
                .map_err(stringify)?;
            // Kept at once, so that a write that fails after this finds the node on the next call.
            dag.nodes[node.id - 1] = Some((id, self.fresh()?));
            if dag.home.len() < 10 {
                dag.home.resize(10, None);
            }
            dag.home[node.id - 1] = Some((x - BOX.0 / 2.0, y - BOX.1 / 2.0));
        }
        for &(from, to) in &c.arcs {
            if dag.edges.contains_key(&(from, to)) {
                continue;
            }
            let (Some((a, _)), Some((b, _))) = (dag.nodes[from - 1].as_ref(), dag.nodes[to - 1].as_ref()) else {
                return Err(format!("the arrow C{from} to C{to} has no node"));
            };
            self.scene
                .add_edge_with(*a, *b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
                .map_err(stringify)?;
            dag.edges.insert((from, to), self.fresh()?);
        }
        for (i, slot) in dag.nodes.iter().enumerate() {
            if let Some((_, group)) = slot {
                let id = i + 1;
                set(group, "data-tj-dag-node", &id.to_string())?;
                set(group, "data-tj-component", &id.to_string())?;
                set(group, "role", "button")?;
                let shown = c.nodes.iter().find(|n| n.id == id);
                set(group, "data-tj-hidden", &shown.is_none().to_string())?;
                set(group, "tabindex", if shown.is_some() { "0" } else { "-1" })?;
                set(group, "aria-pressed", &(selected == Some(id)).to_string())?;
                set(group, "data-tj-selected", &(selected == Some(id)).to_string())?;
                if let Some(node) = shown {
                    set(
                        group,
                        "aria-label",
                        &format!("{}: {} vertices", node.name(), node.members.len()),
                    )?;
                }
            }
        }
        dag.shown.clone_from(&c.arcs);
        for (pair, path) in &dag.edges {
            set(path, "data-tj-dag-edge", &format!("{}-{}", pair.0, pair.1))?;
            set(path, "aria-hidden", "true")?;
            set(path, "data-tj-hidden", &(!c.arcs.contains(pair)).to_string())?;
        }
        Ok(())
    }
}
