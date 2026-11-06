//! The queue of prepared operations, and the one place that makes library calls and reads a clock.

use super::phase;
use super::{Run, describe};
use crate::alloc;
use perf_harness::manifest::Workload;
use perf_harness::ops::Op;
use perf_harness::stats::batch_size;
use perf_harness::workload::field_for;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{ConnectorOptions, ConnectorType, Scene},
};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
/// Numbers the host elements of every run in a page, so that a later run never makes an id that an earlier one used.
static SVG_SEQUENCE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Create,
    Nodes,
    Edges,
    Moves,
    Drop,
    Remove,
}

impl Kind {
    pub(super) fn of(op: &Op) -> Option<Self> {
        match op {
            Op::CreateScene => Some(Self::Create),
            Op::AddNode { .. } => Some(Self::Nodes),
            Op::AddEdge { .. } => Some(Self::Edges),
            Op::MoveNode { .. } => Some(Self::Moves),
            Op::DropScene => Some(Self::Drop),
            Op::RemoveHost => Some(Self::Remove),
            Op::Phase { .. } | Op::Checkpoint { .. } => None,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An operation with everything it needs already worked out, so that running it is only the library call.
///
/// Finding a node's id, building a key, copying a label and making the host element are the harness's own work. They are done
/// when the operation is prepared, before any clock is read, so that a timed interval holds nothing but calls into the library.
pub(super) enum Prepared {
    Create {
        id: String,
    },
    AddNode {
        key: String,
        point: Point,
        size: Size,
        label: String,
    },
    AddEdge {
        key: String,
        from: NodeId,
        to: NodeId,
    },
    Move {
        id: NodeId,
        point: Point,
    },
    Drop,
    Remove,
}

/// What a run of waiting operations is: their kind, whether they are in the measured part, and whether they are timed.
pub(super) type Waiting = (Kind, bool, bool);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Run<'_> {
    /// Works out everything an operation needs, so that running it is only the library call.
    pub(super) fn prepare(&mut self, op: &Op) -> Result<Prepared, String> {
        Ok(match op {
            Op::CreateScene => {
                self.cycle += 1;
                self.checkpoint_this_cycle = false;
                self.building = true;
                let id = format!(
                    "perf-svg-{}",
                    SVG_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
                );
                // The host element is the page's, not the library's, so it is made here and the scene is made on it later.
                let (width, height) = field_for(self.manifest.graph.nodes);
                let svg = self
                    .document
                    .create_element_ns(Some(SVG_NS), "svg")
                    .map_err(|e| format!("{e:?}"))?;
                for (name, value) in [
                    ("id", id.clone()),
                    ("width", width.to_string()),
                    ("height", height.to_string()),
                    ("viewBox", format!("0 0 {width} {height}")),
                ] {
                    svg.set_attribute(name, &value).map_err(|e| format!("{e:?}"))?;
                }
                self.document
                    .body()
                    .ok_or("there is no body")?
                    .append_child(&svg)
                    .map_err(|e| format!("{e:?}"))?;
                self.svg = Some(svg);
                Prepared::Create { id }
            },
            Op::AddNode { key, x, y, w, h, label } => Prepared::AddNode {
                key: key.clone(),
                point: Point::new(*x as f64, *y as f64),
                size: Size::new(*w as f64, *h as f64),
                label: label.clone(),
            },
            Op::AddEdge { from, to } => Prepared::AddEdge {
                key: format!("{from}->{to}"),
                from: self.node(from)?,
                to: self.node(to)?,
            },
            Op::MoveNode { key, x, y } => Prepared::Move {
                id: self.node(key)?,
                point: Point::new(*x as f64, *y as f64),
            },
            Op::DropScene => Prepared::Drop,
            Op::RemoveHost => Prepared::Remove,
            Op::Phase { .. } | Op::Checkpoint { .. } => return Err("a marker reached prepare".to_string()),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Queues a prepared operation, and runs the queue if it is full.
    ///
    /// A queue that is not timed holds one operation and runs at once. A timed batch of moves is as long as the warm-up
    /// said it should be. Every other timed queue is a whole run of its kind, up to the capacity of the vector.
    pub(super) fn submit(&mut self, prepared: Prepared, waiting: Waiting) -> Result<(), String> {
        self.pending_meta = Some(waiting);
        self.pending.push(prepared);
        let (kind, recording, timed) = waiting;
        // A scene is made, dropped and removed one operation at a time, so those run at once. Waiting for the next operation would
        // leave the sample after a teardown to be taken while the generator had already begun planning the next cycle.
        let wanted = if !timed || matches!(kind, Kind::Create | Kind::Drop | Kind::Remove) {
            1
        } else if kind == Kind::Moves && recording {
            self.move_batch.unwrap_or(1) as usize
        } else {
            self.pending_capacity
        };
        if self.pending.len() >= wanted.min(self.pending_capacity) {
            self.flush()
        } else {
            Ok(())
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs the queue.
    pub(super) fn flush(&mut self) -> Result<(), String> {
        let Some((kind, recording, timed)) = self.pending_meta.take() else {
            return Ok(());
        };
        let count = self.pending.len();
        if count == 0 {
            return Ok(());
        }
        // The vector is taken out and put back, so that it keeps its capacity and no run allocates one.
        let mut items = std::mem::take(&mut self.pending);
        let elapsed = self.run_batch(kind, &mut items, timed);
        items.clear();
        self.pending = items;
        let elapsed_us = elapsed?;
        if timed {
            self.record(kind, recording, count as u32, elapsed_us);
        }
        if kind == Kind::Remove {
            self.on_teardown_complete();
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the library calls of one queue, one after the other, and gives the time they took.
    ///
    /// The clock is read once before the first call and once after the last. Nothing else is between them: not the
    /// model, not the hash, not the bookkeeping of what the calls returned, which is done after the second reading.
    fn run_batch(&mut self, kind: Kind, items: &mut [Prepared], timed: bool) -> Result<f64, String> {
        // Set before the clock is read, so that the store is not inside the interval.
        alloc::set_tag(phase::LIBRARY);
        let started = if timed { self.now_ms() } else { 0.0 };
        match kind {
            Kind::Create => {
                let Some(Prepared::Create { id }) = items.first() else {
                    return Err("a create had no id".to_string());
                };
                let scene = Scene::new(SvgRoot::attach(id).map_err(describe)?).map_err(describe)?;
                self.scene = Some(scene);
            },
            Kind::Nodes => {
                let scene = self.scene.as_ref().ok_or("there is no scene")?;
                self.scratch_ids.clear();
                for item in items.iter_mut() {
                    if let Prepared::AddNode { point, size, label, .. } = item {
                        // The label is moved in and not copied, so that no copy is made while the clock runs.
                        self.scratch_ids
                            .push(scene.add_node(*point, *size, std::mem::take(label)).map_err(describe)?);
                    }
                }
            },
            Kind::Edges => {
                let scene = self.scene.as_ref().ok_or("there is no scene")?;
                // The library's default connector is an elbow, and the reference model predicts a straight one.
                let options = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
                for item in items.iter() {
                    if let Prepared::AddEdge { from, to, .. } = item {
                        scene.add_edge_with(*from, *to, options).map_err(describe)?;
                    }
                }
            },
            Kind::Moves => {
                let scene = self.scene.as_ref().ok_or("there is no scene")?;
                for item in items.iter() {
                    if let Prepared::Move { id, point } = item {
                        scene.move_node(*id, *point).map_err(describe)?;
                    }
                }
            },
            Kind::Drop => drop(self.scene.take()),
            Kind::Remove => {
                if let Some(svg) = self.svg.take() {
                    svg.remove();
                }
            },
        }
        let elapsed_us = if timed { (self.now_ms() - started) * 1000.0 } else { 0.0 };

        // What the calls returned is kept now, with the clock stopped.
        alloc::set_tag(phase::BOOKKEEPING);
        match kind {
            Kind::Nodes => {
                for (item, id) in items.iter_mut().zip(self.scratch_ids.iter()) {
                    if let Prepared::AddNode { key, .. } = item {
                        self.nodes.insert(std::mem::take(key), *id);
                    }
                }
            },
            Kind::Edges => {
                for item in items.iter_mut() {
                    if let Prepared::AddEdge { key, .. } = item {
                        self.edge_keys.push(std::mem::take(key));
                    }
                }
            },
            Kind::Drop => self.nodes.clear(),
            Kind::Remove => self.edge_keys.clear(),
            Kind::Create | Kind::Moves => {},
        }
        Ok(elapsed_us)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Keeps the time of a queue that was run.
    fn record(&mut self, kind: Kind, recording: bool, count: u32, microseconds: f64) {
        match (kind, recording) {
            (Kind::Moves, false) => {
                self.warmup_move_us += microseconds;
                self.warmup_moves += u64::from(count);
            },
            (Kind::Moves, true) => self
                .samples
                .entry("mutation.move_node")
                .or_default()
                .push(microseconds / f64::from(count)),
            (kind, _) => {
                let name = match kind {
                    Kind::Create => "construction.create_scene",
                    Kind::Nodes => "construction.nodes",
                    Kind::Edges => "construction.edges",
                    Kind::Drop => "teardown.drop_scene",
                    Kind::Remove => "teardown.remove_host",
                    Kind::Moves => unreachable!("moves are handled above"),
                };
                self.samples.entry(name).or_default().push(microseconds);
                if matches!(kind, Kind::Create | Kind::Nodes | Kind::Edges) {
                    self.cycle_construction_us += microseconds;
                } else {
                    self.cycle_teardown_us += microseconds;
                }
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Works out from the warm-up how many moves to time together, so that an observation spans enough timer steps.
    pub(super) fn finish_warmup(&mut self) {
        if self.timing_on && self.manifest.workload == Workload::ConnectorRerouting {
            if self.warmup_moves > 0 {
                let average = self.warmup_move_us / self.warmup_moves as f64;
                let wanted = batch_size(self.resolution_us, average);
                let batch = wanted.min(self.pending_capacity as u32);
                if batch < wanted {
                    self.outcome.notes.push(format!(
                        "a batch of {wanted} moves was wanted, and {batch} is the most that can wait, so each observation is shorter than 50 timer steps"
                    ));
                }
                self.move_batch = Some(batch);
            } else {
                self.outcome
                    .notes
                    .push("there was no warm-up, so every move was timed on its own".to_string());
            }
        }
    }
}
