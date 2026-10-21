//! The walk through the calculation: which step the diagram is at, and so which boxes have been reached. Pure, so none
//! of it needs a browser.
//!
//! One flat position counts every step, so a selection toolbar can step through all of them. With `n` bytes of text,
//! the positions are:
//!
//! | Positions | Step |
//! |---|---|
//! | `0` | The text has been entered. Nothing has been worked out. |
//! | `1..=n` | Byte `p - 1` goes through the register: its index, its table entry and the new register. |
//! | `n + 1` | The register is XORed with the final mask, which makes the CRC. |
//! | `n + 2` | The bytes and the CRC arrive at the receiver, with any bit that was flipped on the way. |
//! | `n + 3` | The receiver's registers are worked out for the bytes that arrived. |
//! | `n + 4` | The CRC of the bytes that arrived. |
//! | `n + 5` | The syndrome. |
//! | `n + 6` | The error position. |
//! | `n + 7` | The corrected bytes. |
//!
//! Without any text there is nothing to send, so the two positions are the initial register and the CRC.
//!
//! The scene is drawn again at every position, from [`Walk::progress`] alone. So stepping backwards is the same work as
//! stepping forwards, and nothing needs undoing.

/// How many positions the toolbar's `Prev 8` and `Next 8` buttons move.
pub(super) const STRIDE: usize = 8;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The one box, or row of boxes, that the latest step reached, which the diagram marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Newest {
    /// The walk has not started.
    Nothing,
    /// Byte `n` is going through the register.
    Byte(usize),
    /// The sender's register is the initial register, since there are no bytes.
    LastRegister,
    /// The CRC the sender sends.
    Crc,
    Arrival,
    ReceiverRegisters,
    ReceivedCrc,
    Syndrome,
    Position,
    Corrected,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What has been reached at one position: everything that is not yet reached is drawn faint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Progress {
    /// How many of the sender's bytes have been through the register. The first of the table index, the table entry and
    /// the register that are not yet reached is this one.
    pub(super) bytes_done: usize,
    /// The register after the last byte.
    pub(super) last_register: bool,
    /// The CRC, made by the final XOR.
    pub(super) crc: bool,
    /// The bytes and the CRC that arrived.
    pub(super) arrived: bool,
    pub(super) receiver_registers: bool,
    pub(super) received_crc: bool,
    pub(super) syndrome: bool,
    pub(super) position: bool,
    pub(super) corrected: bool,
    pub(super) newest: Newest,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The steps of a text of a given number of bytes.
#[derive(Clone, Copy, Debug)]
pub(super) struct Walk {
    bytes: usize,
}
impl Walk {
    /// The steps for a text of `bytes` bytes.
    pub(super) fn new(bytes: usize) -> Self {
        Self { bytes }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many positions there are.
    pub(super) fn positions(self) -> usize {
        if self.bytes == 0 { 2 } else { self.bytes + 8 }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The position at which the bytes arrive at the receiver, or the last position if there is no receiver.
    pub(super) fn arrival(self) -> usize {
        if self.bytes == 0 { self.positions() - 1 } else { self.bytes + 2 }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What has been reached at `position`. A position past the last is the last.
    pub(super) fn progress(self, position: usize) -> Progress {
        let n = self.bytes;
        let p = position.min(self.positions() - 1);
        if n == 0 {
            return Progress {
                bytes_done: 0,
                last_register: true,
                crc: p >= 1,
                arrived: false,
                receiver_registers: false,
                received_crc: false,
                syndrome: false,
                position: false,
                corrected: false,
                newest: if p == 0 { Newest::LastRegister } else { Newest::Crc },
            };
        }
        let newest = match p {
            0 => Newest::Nothing,
            p if p <= n => Newest::Byte(p - 1),
            p if p == n + 1 => Newest::Crc,
            p if p == n + 2 => Newest::Arrival,
            p if p == n + 3 => Newest::ReceiverRegisters,
            p if p == n + 4 => Newest::ReceivedCrc,
            p if p == n + 5 => Newest::Syndrome,
            p if p == n + 6 => Newest::Position,
            _ => Newest::Corrected,
        };
        Progress {
            bytes_done: p.min(n),
            last_register: p >= n,
            crc: p > n,
            arrived: p >= n + 2,
            receiver_registers: p >= n + 3,
            received_crc: p >= n + 4,
            syndrome: p >= n + 5,
            position: p >= n + 6,
            corrected: p >= n + 7,
            newest,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
