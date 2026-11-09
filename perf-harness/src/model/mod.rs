//! The reference model: what a scene should look like after a stream of operations, worked out without a DOM.
//!
//! The model owns nothing of the library's. It follows the operations, keeps the boxes and edges they describe, and
//! answers with a checkpoint in the shape of `perf/schemas/checkpoint.schema.json`. A runner compares its own
//! checkpoint with this one, so an optimisation that skips work shows up as a difference.
//!
//! What it predicts, and how sure that is:
//!
//! * Topology and content are exact. They follow from the operations alone.
//! * Node boxes are exact. A plain node is drawn at the size it was given.
//! * A connector's ends follow the library's documented rule for a straight connector: each end is where the ray
//!   between the two boxes' centres crosses that box's boundary. Phase 2 confirmed it against the DOM. Where the two
//!   ends coincide the library draws no path, and the model records a path with no coordinates.
//! * Paint order is node creation order. Phase 2 confirmed in Chrome that a move does not raise a node.
//! * DOM ids and the accessibility tree are not predicted. They are left empty and listed as not modelled.

use crate::manifest::ViewPolicy;
use crate::ops::{Op, hex};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneState {
    /// No scene yet, or the host has been removed again.
    Absent,
    Live,
    /// The last handle is gone, but the host and its DOM are still in the document.
    Dropped,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
struct ModelNode {
    key: String,
    label: String,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy)]
struct ModelEdge {
    from: usize,
    to: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct Model {
    state: SceneState,
    nodes: Vec<ModelNode>,
    by_key: BTreeMap<String, usize>,
    edges: Vec<ModelEdge>,
    pairs: BTreeSet<(usize, usize)>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EdgeOut {
    pub from: String,
    pub to: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Topology {
    pub nodes: Vec<String>,
    pub edges: Vec<EdgeOut>,
    pub dangling: Vec<String>,
    pub duplicate_keys: Vec<String>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DomIdentity {
    pub ids: Vec<String>,
    pub duplicates: Vec<String>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeometryElement {
    pub key: String,
    pub kind: &'static str,
    pub coordinates: Vec<f64>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Geometry {
    pub tolerance: f64,
    pub elements: Vec<GeometryElement>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Interaction {
    pub selection: Vec<String>,
    pub focused: Option<String>,
    pub view: Option<View>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AccessibleElement {
    pub key: String,
    pub role: String,
    pub name: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Accessibility {
    pub elements: Vec<AccessibleElement>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Checkpoint {
    pub schema_version: u32,
    pub topology: Topology,
    pub dom_identity: DomIdentity,
    pub geometry: Geometry,
    pub content: BTreeMap<String, String>,
    pub interaction: Interaction,
    pub accessibility: Accessibility,
    pub paint_order: Vec<String>,
}

impl Checkpoint {
    /// The SHA-256 of each section that is compared exactly. Geometry is compared within a tolerance, so it has none.
    pub fn section_hashes(&self) -> BTreeMap<&'static str, String> {
        fn digest<T: Serialize>(section: &T) -> String {
            hex(&Sha256::digest(
                serde_json::to_vec(section).expect("a section is always serialisable"),
            ))
        }
        BTreeMap::from([
            ("topology", digest(&self.topology)),
            ("dom_identity", digest(&self.dom_identity)),
            ("content", digest(&self.content)),
            ("interaction", digest(&self.interaction)),
            ("accessibility", digest(&self.accessibility)),
            ("paint_order", digest(&self.paint_order)),
        ])
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The SHA-256 of the whole checkpoint, geometry included.
    ///
    /// It says that two models agree to the last bit. It is not how a runner's DOM is compared with the model, which
    /// uses the tolerance. Every coordinate comes from basic floating-point arithmetic, which is exact to the last bit
    /// on every platform, so the digest does not depend on the machine.
    pub fn model_sha256(&self) -> String {
        hex(&Sha256::digest(
            serde_json::to_vec(self).expect("a checkpoint is always serialisable"),
        ))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Model {
    pub fn new() -> Self {
        Self {
            state: SceneState::Absent,
            nodes: Vec::new(),
            by_key: BTreeMap::new(),
            edges: Vec::new(),
            pairs: BTreeSet::new(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Applies one operation.
    ///
    /// # Errors
    ///
    /// Returns a message if the operation is not allowed in the model's present state: one that needs a scene when
    /// there is none, a key that is already used, an unknown node, a self-loop or a repeated edge. A stream that does
    /// that is a bug in the generator, so it stops the run.
    pub fn apply(&mut self, op: &Op) -> Result<(), String> {
        match op {
            Op::Phase { .. } | Op::Checkpoint { .. } => Ok(()),
            Op::CreateScene => {
                if self.state != SceneState::Absent {
                    return Err("a scene was created while another was still there".to_string());
                }
                self.state = SceneState::Live;
                Ok(())
            },
            Op::DropScene => {
                self.require_live("drop the scene")?;
                self.state = SceneState::Dropped;
                Ok(())
            },
            Op::RemoveHost => {
                if self.state != SceneState::Dropped {
                    return Err("the host was removed before the scene was dropped".to_string());
                }
                *self = Self::new();
                Ok(())
            },
            Op::AddNode { key, x, y, w, h, label } => {
                self.require_live("add a node")?;
                if self.by_key.contains_key(key) {
                    return Err(format!("the node key {key} is already used"));
                }
                if *w <= 0 || *h <= 0 {
                    return Err(format!("the node {key} has no area"));
                }
                self.by_key.insert(key.clone(), self.nodes.len());
                self.nodes.push(ModelNode {
                    key: key.clone(),
                    label: label.clone(),
                    x: *x,
                    y: *y,
                    w: *w,
                    h: *h,
                });
                Ok(())
            },
            Op::AddEdge { from, to } => {
                self.require_live("add an edge")?;
                let (from, to) = (self.index(from)?, self.index(to)?);
                if from == to {
                    return Err("a node was joined to itself".to_string());
                }
                if !self.pairs.insert((from, to)) {
                    return Err(format!(
                        "the edge {}->{} was added twice",
                        self.nodes[from].key, self.nodes[to].key
                    ));
                }
                self.edges.push(ModelEdge { from, to });
                Ok(())
            },
            Op::MoveNode { key, x, y } => {
                self.require_live("move a node")?;
                let index = self.index(key)?;
                self.nodes[index].x = *x;
                self.nodes[index].y = *y;
                Ok(())
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the scene should look like now.
    ///
    /// # Errors
    ///
    /// Returns a message if the scene has been dropped but its host is still in the document, because the model does
    /// not predict what is left there.
    pub fn checkpoint(&self, view: ViewPolicy) -> Result<Checkpoint, String> {
        if self.state == SceneState::Dropped {
            return Err("a checkpoint was taken between dropping the scene and removing its host".to_string());
        }
        let mut sorted: Vec<&ModelNode> = self.nodes.iter().collect();
        sorted.sort_by(|a, b| a.key.cmp(&b.key));

        let mut elements: Vec<GeometryElement> = self
            .nodes
            .iter()
            .map(|node| GeometryElement {
                key: node.key.clone(),
                kind: "node_box",
                coordinates: vec![node.x as f64, node.y as f64, node.w as f64, node.h as f64],
            })
            .collect();
        for edge in &self.edges {
            let (from, to) = (&self.nodes[edge.from], &self.nodes[edge.to]);
            let start = boundary_point(from, centre(to));
            let end = boundary_point(to, centre(from));
            // The library drops a point that exactly repeats the one before it, so a connector whose two ends are the
            // same point, such as one between two boxes that touch along a side, is left with a single point and is
            // drawn as an empty path. It is recorded here as a path with no coordinates.
            let coordinates = if start == end { Vec::new() } else { vec![start.0, start.1, end.0, end.1] };
            elements.push(GeometryElement {
                key: edge_key(from, to),
                kind: "connector_path",
                coordinates,
            });
        }

        Ok(Checkpoint {
            schema_version: 1,
            topology: Topology {
                nodes: sorted.iter().map(|node| node.key.clone()).collect(),
                edges: self
                    .edges
                    .iter()
                    .map(|edge| EdgeOut {
                        from: self.nodes[edge.from].key.clone(),
                        to: self.nodes[edge.to].key.clone(),
                    })
                    .collect(),
                dangling: Vec::new(),
                duplicate_keys: Vec::new(),
            },
            dom_identity: DomIdentity {
                ids: Vec::new(),
                duplicates: Vec::new(),
            },
            geometry: Geometry { tolerance: 0.01, elements },
            content: self.nodes.iter().map(|node| (node.key.clone(), node.label.clone())).collect(),
            interaction: Interaction {
                selection: Vec::new(),
                focused: None,
                view: match view {
                    ViewPolicy::Ignored => None,
                    ViewPolicy::Unchanged | ViewPolicy::Expected => Some(View { x: 0.0, y: 0.0, scale: 1.0 }),
                },
            },
            accessibility: Accessibility { elements: Vec::new() },
            paint_order: self.nodes.iter().map(|node| node.key.clone()).collect(),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn require_live(&self, what: &str) -> Result<(), String> {
        if self.state == SceneState::Live {
            Ok(())
        } else {
            Err(format!("tried to {what} with no live scene"))
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn index(&self, key: &str) -> Result<usize, String> {
        self.by_key
            .get(key)
            .copied()
            .ok_or_else(|| format!("the node {key} does not exist"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn edge_key(from: &ModelNode, to: &ModelNode) -> String {
    format!("{}->{}", from.key, to.key)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn centre(node: &ModelNode) -> (f64, f64) {
    (node.x as f64 + node.w as f64 / 2.0, node.y as f64 + node.h as f64 / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the ray from `node`'s centre towards `towards` crosses the boundary of `node`.
///
/// The ray is scaled by whichever of two ratios is smaller: the half-width over the horizontal distance, or the
/// half-height over the vertical one. That reaches an edge before it would pass a corner. When `towards` is the centre
/// itself the direction is undefined, and the centre is returned.
fn boundary_point(node: &ModelNode, towards: (f64, f64)) -> (f64, f64) {
    let (cx, cy) = centre(node);
    let (dx, dy) = (towards.0 - cx, towards.1 - cy);
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let ratio = |half: f64, distance: f64| if distance == 0.0 { f64::INFINITY } else { half / distance.abs() };
    let scale = ratio(node.w as f64 / 2.0, dx).min(ratio(node.h as f64 / 2.0, dy));
    (cx + dx * scale, cy + dy * scale)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
