//! Showing how far the walk has got: the boxes it has not reached are drawn faint, and the box it has just reached is
//! marked.

use super::walk::{Newest, Progress};
use crate::util::{all_if, mark_unreached, stringify};
use svg_dom_graph::{
    NodeId,
    scene::{Scene, Selection},
};

/// The receiver's boxes, which exist only when there is text to send.
pub(super) struct ReceiverNodes {
    pub(super) bytes: NodeId,
    pub(super) registers: NodeId,
    pub(super) crc: NodeId,
    pub(super) arrived_crc: NodeId,
    pub(super) syndrome: NodeId,
    pub(super) position: NodeId,
    pub(super) corrected: NodeId,
}

/// The boxes whose progress the walk shows. The others are always there to read.
pub(super) struct Nodes {
    /// How many bytes the text has.
    pub(super) bytes: usize,
    /// The sender's boxes that have a value for each byte: the bytes themselves, the table index, the table entry and
    /// the register. There are none without text.
    pub(super) dump: Option<NodeId>,
    pub(super) per_byte: Vec<NodeId>,
    pub(super) last_register: NodeId,
    pub(super) sent_crc: NodeId,
    pub(super) receiver: Option<ReceiverNodes>,
    /// Each box that has an explanation, paired with the node of that explanation.
    pub(super) notes: Vec<(NodeId, NodeId)>,
}

impl Nodes {
    /// The explanation of `id`, if it has one.
    fn note_of(&self, id: NodeId) -> Option<NodeId> {
        self.notes.iter().find(|(node, _)| *node == id).map(|&(_, note)| note)
    }

    /// Draws the explanation of `id` faint if `unreached`, so that it can be read ahead of the walk, and at full
    /// strength once the walk arrives at the box it explains. A box with no explanation is left alone.
    fn dim_note(&self, scene: &Scene, id: NodeId, unreached: bool) -> Result<(), String> {
        match self.note_of(id) {
            Some(note) => mark_unreached(scene, note, &all_if(unreached, 1)),
            None => Ok(()),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws what `progress` has not reached faint, and marks what it has just reached.
///
/// A box keeps its text while it is faint, so a reader can see what is coming. The library marks the box as "not yet
/// computed" for assistive technology too. So is the explanation beside a box, which is faint until the walk arrives at
/// the box it explains. A box with values for each byte is reached by the first byte.
///
/// # Errors
///
/// Returns `Err` if a library call fails.
pub(super) fn show(scene: &Scene, nodes: &Nodes, progress: &Progress) -> Result<(), String> {
    let pending: Vec<usize> = (progress.bytes_done..nodes.bytes).collect();
    let current = match progress.newest {
        Newest::Byte(byte) => Selection::Cell(byte),
        _ => Selection::None,
    };
    // The same byte is marked in the bytes and in everything worked out from it.
    for &id in nodes.dump.iter().chain(&nodes.per_byte) {
        scene.set_selection(id, current).map_err(stringify)?;
    }
    for &id in &nodes.per_byte {
        mark_unreached(scene, id, &pending)?;
        nodes.dim_note(scene, id, progress.bytes_done == 0)?;
    }
    nodes.dim_note(scene, nodes.last_register, !progress.last_register)?;
    single(scene, nodes.last_register, progress.last_register, false)?;
    single(scene, nodes.sent_crc, progress.crc, progress.newest == Newest::Crc)?;

    if let Some(receiver) = &nodes.receiver {
        let all: Vec<usize> = (0..nodes.bytes).collect();
        let arrived = if progress.arrived { Vec::new() } else { all.clone() };
        mark_unreached(scene, receiver.bytes, &arrived)?;
        nodes.dim_note(scene, receiver.bytes, !progress.arrived)?;
        mark_unreached(scene, receiver.arrived_crc, &all_if(!progress.arrived, 1))?;
        mark_unreached(
            scene,
            receiver.registers,
            &if progress.receiver_registers { Vec::new() } else { all.clone() },
        )?;
        nodes.dim_note(scene, receiver.registers, !progress.receiver_registers)?;
        nodes.dim_note(scene, receiver.crc, !progress.received_crc)?;
        nodes.dim_note(scene, receiver.syndrome, !progress.syndrome)?;
        nodes.dim_note(scene, receiver.position, !progress.position)?;
        nodes.dim_note(scene, receiver.corrected, !progress.corrected)?;
        single(
            scene,
            receiver.crc,
            progress.received_crc,
            progress.newest == Newest::ReceivedCrc,
        )?;
        single(scene, receiver.syndrome, progress.syndrome, progress.newest == Newest::Syndrome)?;
        mark_unreached(scene, receiver.position, &all_if(!progress.position, 1))?;
        mark_unreached(scene, receiver.corrected, &if progress.corrected { Vec::new() } else { all })?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the one-value box `id` faint if it is not `reached`, and marks it if it is the `newest`.
fn single(scene: &Scene, id: NodeId, reached: bool, newest: bool) -> Result<(), String> {
    mark_unreached(scene, id, &all_if(!reached, 1))?;
    let selection = if newest { Selection::Cell(0) } else { Selection::None };
    scene.set_selection(id, selection).map_err(stringify)
}
