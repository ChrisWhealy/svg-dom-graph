//! Deliberate faults and leaks, to prove that the checks which should notice them do.

use super::Run;
use perf_harness::model::Checkpoint;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A deliberate fault in what was observed, to prove that the comparison notices it.
///
/// It is applied to the observed checkpoint and never to the scene, so it tests the check and not the library. A run with a
/// fault must come out `invalid_incorrect` and name the section that the fault damaged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The first connector's start is a unit out.
    ConnectorOffset,
    /// The first node is three units lower than it is.
    NodeMoved,
    /// The first node, and everything about it, is gone.
    NodeRemoved,
    /// The first node's text is not what it was given.
    TextChanged,
    /// The first two nodes are painted in the other order.
    PaintOrderSwapped,
    /// One edge is missing from the relationships.
    EdgeDropped,
}

impl Fault {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "connector_offset" => Self::ConnectorOffset,
            "node_moved" => Self::NodeMoved,
            "node_removed" => Self::NodeRemoved,
            "text_changed" => Self::TextChanged,
            "paint_order_swapped" => Self::PaintOrderSwapped,
            "edge_dropped" => Self::EdgeDropped,
            _ => return None,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(super) fn apply(self, actual: &mut Checkpoint) {
        match self {
            Self::ConnectorOffset => {
                if let Some(connector) = actual.geometry.elements.iter_mut().find(|e| e.kind == "connector_path") {
                    connector.coordinates[0] += 1.0;
                }
            },
            Self::NodeMoved => {
                if let Some(node) = actual.geometry.elements.iter_mut().find(|e| e.kind == "node_box") {
                    node.coordinates[1] += 3.0;
                }
            },
            Self::NodeRemoved => {
                if let Some(key) = actual.topology.nodes.first().cloned() {
                    actual.topology.nodes.retain(|k| *k != key);
                    actual.content.remove(&key);
                    actual.paint_order.retain(|k| *k != key);
                    actual.geometry.elements.retain(|e| !(e.key == key && e.kind == "node_box"));
                }
            },
            Self::TextChanged => {
                if let Some(text) = actual.content.values_mut().next() {
                    text.push_str(" (changed)");
                }
            },
            Self::PaintOrderSwapped => {
                if actual.paint_order.len() >= 2 {
                    actual.paint_order.swap(0, 1);
                }
            },
            Self::EdgeDropped => {
                actual.topology.edges.pop();
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A leak put into every cycle on purpose, to prove that the endurance check notices a leak.
///
/// It is made after each teardown and before the resources are counted, and it is never undone. A check that cannot see these
/// cannot be trusted to see a real one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leak {
    /// This many bytes are allocated and never freed.
    Bytes(usize),
    /// A closure that captures this many bytes is given up with `Closure::forget`, as the library does.
    Closure(usize),
    /// The library's own `svg_buttons::watch` is called, which leaks two closures each time, as it does in a real page.
    LibraryWatch,
}

impl Leak {
    pub fn parse(kind: &str, bytes: usize) -> Option<Self> {
        Some(match kind {
            "bytes" => Self::Bytes(bytes),
            "closure" => Self::Closure(bytes),
            "library_watch" => Self::LibraryWatch,
            _ => return None,
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Run<'_> {
    /// Makes the leak that the self-test asked for, if any.
    pub(super) fn inject_leak(&self) {
        match self.leak {
            Some(Leak::Bytes(bytes)) => {
                let leaked: &'static mut [u8] = Box::leak(vec![1u8; bytes].into_boxed_slice());
                std::hint::black_box(leaked);
            },
            Some(Leak::Closure(bytes)) => {
                let payload = vec![1u8; bytes];
                let closure = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                    std::hint::black_box(&payload);
                });
                closure.forget();
            },
            Some(Leak::LibraryWatch) => {
                if self.document.get_element_by_id("perf-endurance-stage").is_none() {
                    let Ok(stage) = self.document.create_element("div") else { return };
                    stage.set_id("perf-endurance-stage");
                    if let Some(body) = self.document.body() {
                        let _ = body.append_child(&stage);
                    }
                }
                let _ = svg_dom_graph::controls::svg_buttons::watch(&self.document, "perf-endurance-stage", |_| {});
            },
            None => {},
        }
    }
}
