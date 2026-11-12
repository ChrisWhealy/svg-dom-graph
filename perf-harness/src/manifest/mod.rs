//! A workload manifest: what to run, with what seed, and how many times.
//!
//! The manifest is read from TOML, checked against `perf/schemas/manifest.schema.json` when the `schema` feature is on,
//! and then checked again for things a schema cannot say, such as whether the requested edges can fit under the degree
//! limit. Its canonical form is its typed fields, written as JSON in a fixed order. Its hash therefore does not depend
//! on how the TOML was laid out or commented.

use crate::ops::hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(feature = "schema")]
const MANIFEST_SCHEMA: &str = include_str!("../../../perf/schemas/manifest.schema.json");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Workload {
    /// W1: build the same scene again and again.
    SceneConstruction,
    /// W2: build a scene once, then move nodes so that their connectors are routed again.
    ConnectorRerouting,
    /// W3: build, drop and remove a scene again and again, with a different graph each time.
    SceneLifecycle,
    /// W4: build a scene once and leave it for a browser to be driven through with real input. See [`Interaction`].
    Interaction,
    /// W5: build a scene once and break it on purpose, one fault point at a time. See [`Recovery`].
    Recovery,
    /// W6: run the discrete-event service simulator on a fixed trace, with no fault injected. See [`Simulation`].
    ServiceSimulation,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrngName {
    Splitmix64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Correctness,
    Performance,
    Recovery,
    Endurance,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Sequential,
    Interleaved,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewPolicy {
    Unchanged,
    Expected,
    Ignored,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GestureKind {
    /// Press on a node, move, and release.
    Drag,
    /// Press on the background, move, and release.
    Pan,
    /// One wheel event with the control key held.
    WheelZoom,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a browser is asked to do to the scene of workload W4, and at what scale of the view.
///
/// A scale is in percent, since a manifest holds no floating-point number that would have to be compared for equality.
/// Every gesture is made once for each of the measured and warm-up iterations at each scale, so the cases are the
/// scales times the gestures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interaction {
    pub scales_percent: Vec<u32>,
    pub gestures: Vec<GestureKind>,
    /// How many pointer moves a drag or a pan is made of.
    pub steps: u32,
    /// How far the pointer moves at each step at most, in pixels.
    pub step_pixels: u32,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The generator that workload W6 runs, and how its observations are batched. Every figure is a parameter of the frozen
/// generator revision `mixed_outage/7`, so a manifest that names another is refusing the revision and not choosing a
/// setting: changing a figure is a new revision, and the whole seed panel is run again
/// (`dess/s0/docs/generator-and-qualification.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Simulation {
    /// The generator's name. Only `mixed_outage` exists.
    pub scenario: String,
    pub servers: u32,
    pub queue_capacity: u32,
    pub max_attempts: u32,
    pub retry_deferral_cap: u32,
    /// The fewest actual heap pops that the trace has to hold, stale ones included. It is independent of the slice
    /// minimum.
    pub min_pops: u64,
    /// How many consecutive nonempty slices one observation of Layer A or B covers.
    pub batch_slices: u32,
    /// How many arrivals the generator makes, when the trace is a scaled run of the frozen revision. Absent means the
    /// revision's own 4,000, and is not written back, so every manifest that has none keeps its canonical form. The
    /// draws of an arrival do not depend on how many there are, so a scaled trace is the same run for longer, and
    /// scaling is not a new revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrivals: Option<u64>,
    /// The layers that this manifest is meant to be run on, from `a`, `b` and `c`. Absent means all three, and is not
    /// written back. A scaled trace that is only a streaming check against the independent engine names `a`, so that it
    /// is never dispatched to a browser, and the orchestrator refuses a layer that is asked for and is not named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<String>>,
}

impl Simulation {
    /// The scenarios that have a frozen revision, with the parameters that revision fixes: servers, queue capacity,
    /// attempts, deferral cap.
    pub const FROZEN: [(&'static str, [u32; 4]); 1] = [("mixed_outage", [3, 12, 2, 4])];
    /// The slices of the fixed trace windows: priming, warm-up and measured.
    pub const WINDOW_SLICES: [u32; 3] = [256, 40, 240];
    /// What a trace has to hold before any timing, so that the windows are never wrapped, reused or shifted.
    pub const MIN_SLICES: u32 = 536;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn check(&self, execution: &Execution) -> Result<(), ManifestError> {
        let refuse = |message: String| Err(ManifestError::Semantic(message));
        let Some((_, frozen)) = Self::FROZEN.iter().find(|(name, _)| *name == self.scenario) else {
            return refuse(format!(
                "simulation.scenario {:?} has no frozen generator revision: only mixed_outage has",
                self.scenario
            ));
        };
        let given = [self.servers, self.queue_capacity, self.max_attempts, self.retry_deferral_cap];
        if &given != frozen {
            return refuse(format!(
                "simulation has servers, queue_capacity, max_attempts and retry_deferral_cap {given:?}, and the frozen revision of {} has {frozen:?}: another set is a new revision",
                self.scenario
            ));
        }
        if self.min_pops < 1 {
            return refuse("simulation.min_pops must be at least 1".to_string());
        }
        if self.arrivals.is_some_and(|n| !(100..=1_000_000).contains(&n)) {
            return refuse("simulation.arrivals must be from 100 to 1,000,000 when it is given".to_string());
        }
        if let Some(layers) = &self.layers {
            if layers.is_empty() || layers.iter().any(|l| !matches!(l.as_str(), "a" | "b" | "c")) {
                return refuse("simulation.layers must name at least one of a, b and c, and nothing else".to_string());
            }
            if layers.iter().enumerate().any(|(i, l)| layers[..i].contains(l)) {
                return refuse("simulation.layers must not repeat a layer".to_string());
            }
        }
        if self.batch_slices != 4 {
            return refuse(
                "simulation.batch_slices must be 4: an observation is four consecutive nonempty slices".to_string(),
            );
        }
        // The windows are fixed, and an observation is a whole batch of them. More iterations than a window holds would
        // wrap, reuse or shift it.
        let [_, warmup_slices, measured_slices] = Self::WINDOW_SLICES;
        if u64::from(execution.warmup_iterations) * u64::from(self.batch_slices) > u64::from(warmup_slices) {
            return refuse(format!(
                "execution.warmup_iterations of {} four-slice batches need more than the {warmup_slices} slices of the warm-up window",
                execution.warmup_iterations
            ));
        }
        if u64::from(execution.measurement_iterations) * u64::from(self.batch_slices) > u64::from(measured_slices) {
            return refuse(format!(
                "execution.measurement_iterations of {} four-slice batches need more than the {measured_slices} slices of the measured window",
                execution.measurement_iterations
            ));
        }
        if execution.modes.contains(&Mode::Recovery) {
            return refuse("the recovery mode of the simulator is workload W5's, with the service target".to_string());
        }
        Ok(())
    }
}

impl Recovery {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn check(&self) -> Result<(), ManifestError> {
        let refuse = |message: String| Err(ManifestError::Semantic(message));
        match (self.target, &self.scenario) {
            (RecoveryTarget::ServiceSimulation, None) => {
                return refuse("recovery.target = \"service_simulation\" needs recovery.scenario".to_string());
            },
            (RecoveryTarget::ServiceSimulation, Some(name)) if name.is_empty() => {
                return refuse("recovery.scenario must not be empty".to_string());
            },
            (RecoveryTarget::Graph, Some(_)) => {
                return refuse("recovery.scenario is for the service target only".to_string());
            },
            _ => {},
        }
        for entry in &self.points {
            if entry.point.target() != self.target {
                return refuse(format!(
                    "recovery.points names {:?}, which belongs to the {:?} target and not the {:?} one",
                    entry.point,
                    entry.point.target(),
                    self.target
                ));
            }
        }
        if self.points.is_empty() {
            return Err(ManifestError::Semantic(
                "recovery.points must name at least one fault point".to_string(),
            ));
        }
        for (index, entry) in self.points.iter().enumerate() {
            if self.points[..index].iter().any(|earlier| earlier.point == entry.point) {
                return Err(ManifestError::Semantic(
                    "recovery.points must not repeat a fault point".to_string(),
                ));
            }
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Interaction {
    /// The scales that the library's view accepts, in percent: a quarter to four times.
    pub const SCALE_RANGE_PERCENT: std::ops::RangeInclusive<u32> = 25..=400;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn check(&self) -> Result<(), ManifestError> {
        let refuse = |message: &str| Err(ManifestError::Semantic(message.to_string()));
        if self.scales_percent.is_empty() || self.gestures.is_empty() {
            return refuse("interaction needs at least one scale and one gesture");
        }
        for (index, scale) in self.scales_percent.iter().enumerate() {
            if !Self::SCALE_RANGE_PERCENT.contains(scale) {
                return refuse(
                    "interaction.scales_percent must each be from 25 to 400, which is what the view accepts",
                );
            }
            if self.scales_percent[..index].contains(scale) {
                return refuse("interaction.scales_percent must not repeat a scale");
            }
        }
        for (index, gesture) in self.gestures.iter().enumerate() {
            if self.gestures[..index].contains(gesture) {
                return refuse("interaction.gestures must not repeat a gesture");
            }
        }
        if !(1..=100).contains(&self.steps) || !(1..=50).contains(&self.step_pixels) {
            return refuse("interaction.steps must be from 1 to 100 and interaction.step_pixels from 1 to 50");
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A place at which the library can fail part way, and the recovery runner breaks it. The contract of each is written
/// down in `perf/fault-contracts.json`, from the library's code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultPoint {
    NodeConstruction,
    DomTransform,
    ConnectorUpdate,
    LabelUpdate,
    DragInstall,
    EdgeConstruction,
    EdgeLabelDraw,
    EdgeLabelPlacement,
    ElbowEdgeConstruction,
    ConnectorTypeRedraw,
    ConnectorTypeLabel,
    DataNodeConstruction,
    DataValuesUpdate,
    OperatorConstruction,
    ContainerConstruction,
    RollbackNodeConstruction,
    RollbackEdgeConstruction,
    RollbackEdgeLabelDraw,
    RollbackDataNodeConstruction,
    RollbackOperatorConstruction,
    RollbackContainerConstruction,
    ScheduledFrame,
    Teardown,
    MidDragTeardown,
    RemovedHost,
    // The points of the service-simulation target. Each is valid only for that target, and the graph points only for the graph target.
    ServiceDataNodeConstruction,
    ServiceDataSetter,
    ServiceOverlayWrite,
    ServiceControlAria,
    ServiceTimerRequest,
    ServiceFrameRequest,
    ServiceGalleryHidden,
    ServiceDetachedBeforeDraw,
    ServiceDetachedDuringDraw,
    ServiceTeardownLateTick,
}

impl Graph {
    /// The graph of a manifest that has none.
    fn absent() -> Self {
        Self {
            nodes: 0,
            edges: None,
            maximum_degree: None,
        }
    }

    fn is_absent(&self) -> bool {
        self.nodes == 0 && self.edges.is_none() && self.maximum_degree.is_none()
    }
}

impl FaultPoint {
    /// The target that this point belongs to.
    #[must_use]
    pub fn target(self) -> RecoveryTarget {
        match self {
            Self::ServiceDataNodeConstruction
            | Self::ServiceDataSetter
            | Self::ServiceOverlayWrite
            | Self::ServiceControlAria
            | Self::ServiceTimerRequest
            | Self::ServiceFrameRequest
            | Self::ServiceGalleryHidden
            | Self::ServiceDetachedBeforeDraw
            | Self::ServiceDetachedDuringDraw
            | Self::ServiceTeardownLateTick => RecoveryTarget::ServiceSimulation,
            _ => RecoveryTarget::Graph,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the recovery workload breaks. A manifest that does not say breaks the graph scene, as every manifest before this key did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryTarget {
    /// The scene of the manifest's `[graph]`, with its own twin executor and reference model.
    #[default]
    Graph,
    /// The discrete-event service simulator's controller and surface, with an executor of its own. It has no graph and no connector.
    ServiceSimulation,
}

impl RecoveryTarget {
    fn is_graph(&self) -> bool {
        *self == Self::Graph
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a fault point is expected to leave behind, taken from the library's documented behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expectation {
    /// The state afterwards is the state before.
    NothingCommitted,
    /// A documented part of the change is kept, and a defined retry puts the rest right.
    PartlyCommitted,
    /// Nothing documents it. The runner records what it sees, and reports it as a finding and not as a pass or a
    /// failure.
    Undocumented,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultExpectation {
    pub point: FaultPoint,
    pub expect: Expectation,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The fault points of workload W5 and what each is expected to leave behind. The scene they are broken in is the one
/// of the manifest's `[graph]`, built once. `execution.warmup_iterations` and `measurement_iterations` count the cycles
/// of an endurance run, in each of which every fault point is broken and recovered from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    /// What is broken. Absent, and not written back, means the graph, so every manifest that exists keeps its canonical form.
    #[serde(default, skip_serializing_if = "RecoveryTarget::is_graph")]
    pub target: RecoveryTarget,
    /// The scenario of the simulator that is broken: a fixture or the trace of a seed. Required for the service target, and refused for
    /// the graph one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    pub points: Vec<FaultExpectation>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    /// At least 1 for every workload that has a graph.
    pub nodes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edges: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_degree: Option<u32>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub modes: Vec<Mode>,
    pub warmup_iterations: u32,
    pub measurement_iterations: u32,
    pub repetitions: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Order>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub capture_dom_counts: bool,
    pub capture_listener_tally: bool,
    pub capture_heap: bool,
    pub capture_wasm_capacity: bool,
    pub capture_wasm_live_allocation: bool,
    pub capture_frame_timing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_every: Option<u32>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointPolicy {
    pub view: ViewPolicy,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dom_nodes: Option<u64>,
    pub wall_ms: u64,
    /// How many bytes live memory may grow by over the measured part of an endurance run before the run fails its gate.
    /// Absent means none, which suits a workload whose live bytes are exact. A structure that grows legitimately would
    /// state the most it needs here, so that the allowance is a decision in the manifest and not a setting of the tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_growth_allowance_bytes: Option<u64>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub workload: Workload,
    pub seed: u64,
    pub prng: PrngName,
    /// The graph of the workload. The recovery workload with the service target, and the simulator workload, have none,
    /// and then this is the empty graph that [`Graph::absent`] gives, which is not written back.
    #[serde(default = "Graph::absent", skip_serializing_if = "Graph::is_absent")]
    pub graph: Graph,
    pub execution: Execution,
    pub measurement: Measurement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<CheckpointPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<Interaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<Recovery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulation: Option<Simulation>,
    pub limits: Limits,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// The text is not TOML, or not a manifest.
    Parse(String),
    /// The JSON schema rejected the manifest. Each entry is one violation.
    Schema(Vec<String>),
    /// The manifest is well formed, but asks for something that cannot be built.
    Semantic(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "the manifest could not be read: {message}"),
            Self::Schema(violations) => write!(f, "the manifest breaks its schema: {}", violations.join("; ")),
            Self::Semantic(message) => write!(f, "the manifest cannot be built: {message}"),
        }
    }
}

impl std::error::Error for ManifestError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Manifest {
    /// Reads, validates and checks a manifest.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] if the text is not TOML, breaks the schema (with the `schema` feature), or asks for
    /// more edges than the nodes and the degree limit can hold.
    pub fn from_toml(text: &str) -> Result<Self, ManifestError> {
        let value: serde_json::Value = toml::from_str(text).map_err(|e| ManifestError::Parse(e.to_string()))?;
        Self::from_json(value)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The same as [`from_toml`](Self::from_toml), from a manifest already converted to JSON.
    ///
    /// # Errors
    ///
    /// See [`from_toml`](Self::from_toml).
    pub fn from_json(value: serde_json::Value) -> Result<Self, ManifestError> {
        #[cfg(feature = "schema")]
        validate_against_schema(&value)?;
        let manifest: Self = serde_json::from_value(value).map_err(|e| ManifestError::Parse(e.to_string()))?;
        manifest.check_semantics()?;
        Ok(manifest)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The canonical form: the typed fields as JSON, in the order they are declared.
    pub fn canonical_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a manifest is always serialisable")
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn canonical_sha256(&self) -> String {
        hex(&Sha256::digest(self.canonical_json()))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether this is the recovery workload aimed at the service simulator, which has no graph.
    #[must_use]
    pub fn is_service_recovery(&self) -> bool {
        self.recovery
            .as_ref()
            .is_some_and(|r| r.target == RecoveryTarget::ServiceSimulation)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether this manifest has no graph: the simulator is not a graph, whether it is run cleanly or broken on
    /// purpose.
    #[must_use]
    pub fn has_no_graph(&self) -> bool {
        self.is_service_recovery() || self.workload == Workload::ServiceSimulation
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many cycles or moves one run makes: the warm-up and the measured iterations together.
    pub fn total_iterations(&self) -> u64 {
        u64::from(self.execution.warmup_iterations) + u64::from(self.execution.measurement_iterations)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn check_semantics(&self) -> Result<(), ManifestError> {
        self.check_ranges()?;
        let nodes = u64::from(self.graph.nodes);
        let edges = u64::from(self.graph.edges.unwrap_or(0));
        let ordered_pairs = nodes * nodes.saturating_sub(1);
        if edges > ordered_pairs {
            return Err(ManifestError::Semantic(format!(
                "{edges} edges cannot join {nodes} nodes without a repeat or a self-loop: at most {ordered_pairs}"
            )));
        }
        if let Some(limit) = self.graph.maximum_degree {
            // Each edge adds one to the degree of both its ends, so the degrees add up to twice the edges.
            if edges * 2 > nodes * u64::from(limit) {
                return Err(ManifestError::Semantic(format!(
                    "{edges} edges need a degree total of {}, but {nodes} nodes of degree at most {limit} give {}",
                    edges * 2,
                    nodes * u64::from(limit)
                )));
            }
            if edges > 0 && limit == 0 {
                return Err(ManifestError::Semantic("a maximum degree of 0 allows no edge".to_string()));
            }
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The rules that the schema states as minimums and conditions, checked again here.
    ///
    /// A build without the `schema` feature, such as one for a browser, then refuses the same manifests that a native
    /// one does. With the feature on, the schema's own message comes first and these never fire.
    fn check_ranges(&self) -> Result<(), ManifestError> {
        let refuse = |message: &str| Err(ManifestError::Semantic(message.to_string()));
        if self.schema_version != 1 {
            return refuse("schema_version must be 1");
        }
        if self.graph.nodes == 0 && !self.has_no_graph() {
            return refuse("graph.nodes must be at least 1");
        }
        if self.graph.nodes > 0 && self.has_no_graph() {
            return refuse(
                "the simulator, which the service target of the recovery workload and workload W6 both are, has no [graph]",
            );
        }
        if self.execution.modes.is_empty() {
            return refuse("execution.modes must name at least one mode");
        }
        for (index, mode) in self.execution.modes.iter().enumerate() {
            if self.execution.modes[..index].contains(mode) {
                return refuse("execution.modes must not repeat a mode");
            }
        }
        if self.execution.measurement_iterations == 0 || self.execution.repetitions == 0 || self.limits.wall_ms == 0 {
            return refuse("measurement_iterations, repetitions and wall_ms must each be at least 1");
        }
        if self.measurement.checkpoint_every == Some(0) || self.limits.dom_nodes == Some(0) {
            return refuse("checkpoint_every and dom_nodes must be at least 1 when they are given");
        }
        // The simulator's correctness is the engine's own hash of its state at every slice, so it has no checkpoint
        // policy to choose.
        if self.execution.modes.contains(&Mode::Correctness)
            && self.workload != Workload::ServiceSimulation
            && (self.checkpoint.is_none() || self.measurement.checkpoint_every.is_none())
        {
            return refuse("the correctness mode needs a [checkpoint] policy and measurement.checkpoint_every");
        }
        if self.workload == Workload::ConnectorRerouting && self.graph.edges.is_none() {
            return refuse("connector_rerouting needs graph.edges");
        }
        match (&self.interaction, self.workload) {
            (None, Workload::Interaction) => return refuse("the interaction workload needs an [interaction] table"),
            (Some(_), workload) if workload != Workload::Interaction => {
                return refuse("only the interaction workload takes an [interaction] table");
            },
            (Some(interaction), _) => interaction.check()?,
            (None, _) => {},
        }
        match (&self.recovery, self.workload) {
            (None, Workload::Recovery) => return refuse("the recovery workload needs a [recovery] table"),
            (Some(_), workload) if workload != Workload::Recovery => {
                return refuse("only the recovery workload takes a [recovery] table");
            },
            (Some(recovery), _) => recovery.check()?,
            (None, _) => {},
        }
        match (&self.simulation, self.workload) {
            (None, Workload::ServiceSimulation) => return refuse("workload W6 needs a [simulation] table"),
            (Some(_), workload) if workload != Workload::ServiceSimulation => {
                return refuse("only the service_simulation workload takes a [simulation] table");
            },
            (Some(simulation), _) => {
                simulation.check(&self.execution)?;
                if self.checkpoint.is_some() || self.measurement.checkpoint_every.is_some() {
                    return refuse(
                        "the simulator workload takes no [checkpoint] policy and no checkpoint_every: it is checked at every slice",
                    );
                }
            },
            (None, _) => {},
        }
        if self.workload == Workload::Recovery
            && !self.is_service_recovery()
            && self.graph.edges.is_none_or(|edges| edges == 0)
        {
            return refuse("the recovery workload needs graph.edges, since a connector has to exist to be broken");
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(feature = "schema")]
fn validate_against_schema(value: &serde_json::Value) -> Result<(), ManifestError> {
    let schema: serde_json::Value = serde_json::from_str(MANIFEST_SCHEMA).expect("the manifest schema is valid JSON");
    let validator = jsonschema::validator_for(&schema).expect("the manifest schema is a valid schema");
    let violations: Vec<String> = validator
        .iter_errors(value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(ManifestError::Schema(violations))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
