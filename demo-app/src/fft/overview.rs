//! The overview scene: one node for each lane at each boundary `S0 … SL`, and a connector for each dependency between
//! two boundaries. It is drawn once for a size and a signal. Stepping changes only values, marks and emphasis.
//!
//! Every lane is a node of its own, so a connector is an ordinary edge between two nodes. It shares the scene's zoom
//! and pan, and needs nothing from the library's private drawing layers.

use super::{
    algorithm::{FftTrace, pair_lanes},
    cursor::{selection_of, shown_stage},
    layout::{Grid, dependencies, heading, lane_name},
    sizing::{content, only, widest_value},
};
use demo_common::{
    complex::Complex,
    util::{add_step_driver, document, required_element, resize_svg, stringify},
};
use svg_dom::{SvgRoot, root::utils::Size};
use svg_dom_graph::{
    EdgeId, NodeId,
    scene::{
        ConnectorOptions, ConnectorType, Scene, Selection, SelectionToolbarOptions, SelectionTransition, Side,
        ToolbarOptions, ViewTransform,
    },
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How tall a column heading is, and the least width it is given.
const HEADING_HEIGHT: f64 = 28.0;
const HEADING_MIN_WIDTH: f64 = 176.0;

/// The four connectors of one butterfly: `a → sum`, `a → difference`, `b → sum`, `b → difference`.
type PairEdges = [EdgeId; 4];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The overview scene and the handles to everything a step changes.
pub(super) struct Overview {
    scene: Scene,
    /// The node of each lane, by boundary and then lane.
    nodes: Vec<Vec<NodeId>>,
    /// The connectors of each butterfly, by stage (from 1, so index `stage − 1`) and then pair.
    edges: Vec<Vec<PairEdges>>,
    driver: NodeId,
}

impl Overview {
    /// Draws the overview of `trace` into `#svg_id`, and wires its stepping toolbar to `on_step`.
    ///
    /// Every node holds the placeholder zero, and is marked as not computed. [`render`](Self::render) writes the
    /// values.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page has no `#svg_id`, or if any library call fails.
    pub(super) fn build(
        svg_id: &str,
        trace: &FftTrace,
        view: Option<ViewTransform>,
        on_step: impl Fn(&Scene, NodeId, SelectionTransition) + 'static,
    ) -> Result<Self, String> {
        let lanes = trace.lanes();
        let stages = trace.stages();
        let document = document()?;
        required_element(&document, svg_id)?;
        let scene = Scene::new(SvgRoot::attach(svg_id).map_err(stringify)?).map_err(stringify)?;

        // The widest number and the longest name, measured together, give the size of every node.
        let widest = widest_value(
            &scene,
            "S0[0]=x[0]",
            trace.states.iter().flatten().copied().chain(std::iter::once(Complex::ZERO)),
        )?;
        let longest = (0..=stages)
            .flat_map(|b| (0..lanes).map(move |lane| (b, lane)))
            .map(|(b, lane)| lane_name(b, lane, trace.bit_reversed[lane]))
            .max_by_key(|name| name.chars().count())
            .unwrap_or_default();
        let node_size = scene.measure_named_data_node(&longest, &content(widest)).map_err(stringify)?;
        let grid = Grid::new(lanes, stages + 1, node_size, HEADING_HEIGHT);

        for boundary in 0..=stages {
            let size = Size::new(node_size.width.max(HEADING_MIN_WIDTH), HEADING_HEIGHT);
            scene
                .add_node(grid.heading_origin(boundary), size, heading(boundary))
                .map_err(stringify)?;
        }

        let mut nodes = Vec::with_capacity(stages + 1);
        for boundary in 0..=stages {
            let mut column = Vec::with_capacity(lanes);
            for lane in 0..lanes {
                let name = lane_name(boundary, lane, trace.bit_reversed[lane]);
                let id = scene
                    .add_named_data_node(grid.node_origin(boundary, lane), &name, content(widest))
                    .map_err(stringify)?;
                column.push(id);
            }
            nodes.push(column);
        }

        // Straight connectors, so that the four of a butterfly cross in the middle as the wings of a butterfly do.
        let mut edges = Vec::with_capacity(stages);
        for stage in 1..=stages {
            let mut stage_edges = Vec::with_capacity(lanes / 2);
            for (upper, lower, _, _) in pair_lanes(lanes, stage) {
                let mut four = Vec::with_capacity(4);
                for dependency in dependencies(upper, lower) {
                    let options = ConnectorOptions::default()
                        .with_connector_type(ConnectorType::Straight)
                        .with_from_side(Some(Side::East))
                        .with_to_side(Some(Side::West))
                        .with_from_position(Some(dependency.from_position))
                        .with_to_position(Some(dependency.to_position));
                    four.push(
                        scene
                            .add_edge_with(
                                nodes[stage - 1][dependency.source],
                                nodes[stage][dependency.target],
                                options,
                            )
                            .map_err(stringify)?,
                    );
                }
                stage_edges
                    .push(<PairEdges>::try_from(four).map_err(|_| "a butterfly has four connectors".to_owned())?);
            }
            edges.push(stage_edges);
        }

        // The size comes from the visible nodes alone. The step driver is far off the canvas and is never counted.
        resize_svg(&document, svg_id, grid.canvas())?;
        scene.refresh_layout().map_err(stringify)?;
        if let Some(view) = view {
            scene.set_view(view).map_err(stringify)?;
        }
        let driver = add_step_driver(&scene, svg_id, stages)?;
        scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
        scene
            .show_selection_toolbar(driver, SelectionToolbarOptions::new(Side::North), on_step)
            .map_err(stringify)?;

        Ok(Self { scene, nodes, edges, driver })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The scene, for its view.
    pub(super) const fn scene(&self) -> &Scene {
        &self.scene
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the state with `completed` stages done and butterfly `pair` selected.
    ///
    /// A boundary that is done shows its stored values. One that is not holds the placeholder zero and is marked as not
    /// computed, so that no result is shown before it exists. The selected butterfly's four lanes and four connectors
    /// are emphasised, and every other is not. Writing the same state again is safe, and repairs a partly written one.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `pair` is not a butterfly of the stage being shown, or if any library call fails.
    pub(super) fn render(&self, trace: &FftTrace, completed: usize, pair: usize) -> Result<(), String> {
        let stage = shown_stage(completed);
        let selected = trace
            .stage_pairs(stage)
            .get(pair)
            .ok_or_else(|| format!("stage {stage} has no pair {pair}"))?;

        for (boundary, column) in self.nodes.iter().enumerate() {
            let done = boundary <= completed;
            for (lane, &id) in column.iter().enumerate() {
                let value = if done { trace.states[boundary][lane] } else { Complex::ZERO };
                self.scene.set_data_values(id, only(value)).map_err(stringify)?;
                self.scene
                    .set_unreached_cells(id, if done { &[] } else { &[0] })
                    .map_err(stringify)?;
            }
        }

        let emphasised = |boundary: usize, lane: usize| {
            (boundary == stage - 1 || boundary == stage) && (lane == selected.upper || lane == selected.lower)
        };
        for (boundary, column) in self.nodes.iter().enumerate() {
            for (lane, &id) in column.iter().enumerate() {
                self.scene.set_focus(id, emphasised(boundary, lane)).map_err(stringify)?;
            }
        }
        for (index, stage_edges) in self.edges.iter().enumerate() {
            for (number, four) in stage_edges.iter().enumerate() {
                let on = index + 1 == stage && number == pair;
                for &edge in four {
                    self.scene.set_edge_focus(edge, on).map_err(stringify)?;
                }
            }
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts the stepping toolbar's selection where `completed` stages means.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the library cannot write the selection.
    pub(super) fn set_driver(&self, completed: usize) -> Result<(), String> {
        let selection = selection_of(completed).map_or(Selection::None, Selection::Cell);
        self.scene.set_selection(self.driver, selection).map_err(stringify)
    }
}
