//! What happens to the data between the sender and the receiver.

use super::{
    algorithm::{Computation, Finding, compute, locate},
    flips::Flips,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What happens to the data between the sender and the receiver: the bytes that were sent, the bytes that arrived, and
/// what the receiver works out from them.
pub(super) struct Transmission {
    pub(super) sent: Vec<u8>,
    pub(super) sent_crc: u32,
    /// The bits that were flipped in transmission, if the user introduced any.
    pub(super) flips: Flips,
    pub(super) received: Vec<u8>,
    /// The CRC as it arrived: the CRC that was sent, with its flipped bits.
    pub(super) arrived_crc: u32,
    pub(super) received_computed: Computation,
    pub(super) syndrome: u32,
    pub(super) finding: Finding,
    pub(super) corrected: Vec<u8>,
    pub(super) corrected_crc: u32,
}

impl Transmission {
    /// What arrives when `sent` and its CRC go through a channel that flips the bits of `flips`, and what the receiver
    /// makes of it.
    pub(super) fn new(sent: &[u8], sent_crc: u32, flips: &Flips) -> Self {
        let mut received = sent.to_vec();
        for &(byte, bit) in &flips.data {
            received[byte] ^= 1 << bit;
        }
        let arrived_crc = sent_crc ^ flips.crc_mask();
        let received_computed = compute(&received);
        let syndrome = arrived_crc ^ received_computed.crc;
        let finding = locate(sent.len(), syndrome);
        let mut corrected = received.clone();
        if let Finding::Data { byte, bit } = finding {
            corrected[byte] ^= 1 << bit;
        }
        let corrected_crc = compute(&corrected).crc;
        Self {
            sent: sent.to_vec(),
            sent_crc,
            flips: flips.clone(),
            received,
            arrived_crc,
            received_computed,
            syndrome,
            finding,
            corrected,
            corrected_crc,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the error-position box says.
    pub(super) fn position(&self) -> String {
        match self.finding {
            Finding::NoError => "none".to_owned(),
            Finding::Data { byte, bit } => format!("byte {byte}, bit {bit}"),
            Finding::Crc { bit } => format!("bit {bit} of the CRC"),
            Finding::Uncorrectable => "more than one bit".to_owned(),
        }
    }
}
