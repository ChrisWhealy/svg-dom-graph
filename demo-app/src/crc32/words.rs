//! The words of the diagram.

use super::{
    INPUT_LEN,
    algorithm::{Computation, FINAL_MASK, Finding, INITIAL, POLYNOMIAL},
    transmission::Transmission,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The eight shifts that make the table's entry for `index`, as a numbered list. Each shifts the register right by one
/// place, and XORs in the polynomial if the bit that fell off the end was a one.
pub(super) fn shifts(index: u8) -> String {
    let mut register = u32::from(index);
    let mut lines = Vec::new();
    for n in 1..=8 {
        let before = register;
        let fell = before & 1;
        let shifted = before >> 1;
        register = if fell == 1 { shifted ^ POLYNOMIAL } else { shifted };
        lines.push(if fell == 1 {
            format!(
                "{n}. `{before:08X}` loses a **1**. Shifted: `{shifted:08X}`. XOR `{POLYNOMIAL:08X}` = `{register:08X}`"
            )
        } else {
            format!("{n}. `{before:08X}` loses a **0**. Shifted: `{shifted:08X}`")
        });
    }
    lines.join("\n")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The words of the diagram: what each box is, and how its values are worked out. Every number in them is a real one.
/// The sender's are worked out for the first byte of the text, so that a reader can follow one byte all the way through
/// by hand.
pub(super) struct Words {
    pub(super) input: String,
    pub(super) bytes: String,
    pub(super) index: String,
    pub(super) entry: String,
    pub(super) register: String,
    pub(super) last: String,
    pub(super) result: String,
    pub(super) no_bytes: String,
    pub(super) received: String,
    pub(super) received_register: String,
    pub(super) received_crc: String,
    pub(super) syndrome: String,
    pub(super) position: String,
    pub(super) corrected: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The words for `bytes`, what was worked out for them, and what became of them in transmission.
pub(super) fn words(bytes: &[u8], computed: &Computation, transmission: &Transmission) -> Words {
    let crc = computed.crc;
    let register = computed.register;
    let first = bytes.first().zip(computed.steps.first());
    let (index, entry, register_step) = match first {
        Some((&byte, step)) => (
            format!(
                "For the first byte, `{byte:02X}`, the register is `{INITIAL:08X}`, so its lowest byte is `{low:02X}`. \
                 `{byte:02X}` XOR `{low:02X}` = `{index:02X}`.\n\nFor each later byte, the register is the one that the \
                 byte before it made, in the box below.",
                low = INITIAL & 0xFF,
                index = step.index
            ),
            format!(
                "The first index is `{index:02X}`. Its entry starts as the number `{index:08X}`, and is shifted eight \
                 times:\n\n{shifts}\n\nThe last line, `{entry:08X}`, is the table's entry for `{index:02X}`. A program \
                 makes all 256 entries once, then looks them up.",
                index = step.index,
                shifts = shifts(step.index),
                entry = step.entry
            ),
            format!(
                "For the first byte, `{INITIAL:08X}` shifted right by eight places is `{shifted:08X}`. XOR the entry, \
                 `{entry:08X}` = `{new:08X}`.\n\nThe new register goes back to the top to make the next byte's index: \
                 its lowest byte is XORed with the next input byte.",
                shifted = INITIAL >> 8,
                entry = step.entry,
                new = step.register
            ),
        ),
        None => (String::new(), String::new(), String::new()),
    };

    let t = transmission;
    let n = bytes.len();
    let what_happened = if !t.flips.data.is_empty() {
        let changed: Vec<String> = t
            .flips
            .bytes()
            .into_iter()
            .map(|byte| format!("byte {byte}: `{:02X}` became `{:02X}`", t.sent[byte], t.received[byte]))
            .collect();
        format!(
            "{} flipped on the way, so each changed byte is marked. {}.",
            if t.flips.data.len() == 1 {
                "A bit of the data was".to_owned()
            } else {
                format!("{} bits of the data were", t.flips.data.len())
            },
            changed.join("; ")
        )
    } else if t.flips.crc.is_empty() {
        "No error has been introduced, so these are the bytes that were sent. Use the button at the top of \
         the diagram to flip one bit at random."
            .to_owned()
    } else {
        "No bit of the data was flipped, so these are the bytes that were sent. Only the CRC was changed on the way."
            .to_owned()
    };
    let differing = match t.flips.bytes().first() {
        Some(byte) => format!(
            "The registers from byte {byte} on are marked, because they differ from the sender's. An error in one byte \
             changes the register for every byte after it."
        ),
        None => "They are the same as the sender's, because the bytes are the same.".to_owned(),
    };
    let found = match t.finding {
        Finding::NoError => "The syndrome is zero, so there is nothing to find.".to_owned(),
        Finding::Data { byte, bit } => format!(
            "Bit {bit} of byte {byte} gives `{:08X}`. That is the bit that was flipped.",
            t.syndrome
        ),
        Finding::Crc { bit } => format!(
            "A syndrome with a single 1, here bit {bit}, means that a bit of the CRC itself changed. The data is right."
        ),
        Finding::Uncorrectable => format!(
            "No single bit gives `{:08X}`, so more than one bit changed. CRC-32 **detects** that, but it cannot say which \
             bits to correct. With data this short, up to three flipped bits are never mistaken for one, and always give a \
             syndrome that is not zero. With four or more, the syndrome could match a single bit by chance, and then the \
             receiver would correct the wrong bit.",
            t.syndrome
        ),
    };
    let corrected = match t.finding {
        Finding::NoError => "There was nothing to correct, so these are the bytes that arrived.".to_owned(),
        Finding::Data { byte, bit } => format!(
            "Flipping bit {bit} of byte {byte} back makes `{:02X}` again. The CRC of the corrected bytes is `{:08X}`, the \
             CRC that arrived. They are the bytes that were sent.",
            t.corrected[byte], t.corrected_crc
        ),
        Finding::Crc { .. } => {
            "The receiver believes that only the CRC changed, so it leaves the data as it arrived.".to_owned()
        },
        Finding::Uncorrectable => "The receiver cannot tell which bits to flip, so these are the bytes that arrived. \
                                   The error is detected but not corrected. A real system would ask for the data again."
            .to_owned(),
    };
    let arrival = if t.flips.crc.is_empty() {
        String::new()
    } else {
        format!(
            "{} flipped on the way, so the CRC that was `{:08X}` arrived as `{:08X}`. ",
            if t.flips.crc.len() == 1 {
                "A bit of it was".to_owned()
            } else {
                format!("{} bits of it were", t.flips.crc.len())
            },
            t.sent_crc,
            t.arrived_crc
        )
    };
    Words {
        input: format!(
            "### Text to checksum\nType any text. CRC-32 works on its **bytes**. It covers the first {INPUT_LEN} bytes, \
             and the note on the left says how many it has."
        ),
        bytes: "### Input bytes\nThe bytes of the text, as a hex dump. A UTF-8 character such as `\u{20AC}` occupies \
                more than one byte, but CRC-32 ignores this and looks at byte separately.\n\nThis is the **sender**."
            .to_owned(),
        index: format!(
            "### Table index\nThe calculation takes one byte at a time. It **XORs** the byte with the **lowest byte \
             of the register**. The result is a number from 0 to 255 that picks an entry in a table of 256.\n\n{index}"
        ),
        entry: format!(
            "### Table entry\nEach index has a ready-made entry. Shifting the index through a 32-bit register **one bit \
             at a time** makes it. Shift right by one place. If the bit that fell off the end was a **1**, XOR the \
             register with the polynomial `{POLYNOMIAL:08X}`. Do this eight times, once for each bit of the byte.\n\n{entry}"
        ),
        register: format!(
            "### Register after each byte\nThe new register is the old register **shifted right by a whole byte**, \
             eight places, XORed with the table's entry.\n\n{register_step}\n\nThe box on the right is the register \
             before the first byte. It is all ones, so that leading zero bytes still change the checksum."
        ),
        last: "### Register after the last byte\nWhen every byte has been shifted through, the register holds the \
               checksum **before** its last step. It is not the answer yet."
            .to_owned(),
        result: format!(
            "### CRC-32\nThe last step flips every bit of the register, by XORing it with the mask `{FINAL_MASK:08X}`. \
             `{register:08X}` XOR `{FINAL_MASK:08X}` = `{crc:08X}`.\n\nThat is the **CRC-32**. The sender sends it along \
             with the data, and the receiver checks it."
        ),
        no_bytes: format!(
            "### Initial register\nThere are no bytes, so nothing is shifted through the register. It stays `{INITIAL:08X}`, \
             all ones, and the final XOR turns it to `{crc:08X}`. Type some text to see the steps, and to send them."
        ),
        received: format!(
            "### Received bytes\nThe bytes that arrived. The channel can flip a bit on the way.\n\n{what_happened}"
        ),
        received_register: format!(
            "### Received register after each byte\nThe receiver does not trust the data. It works out the CRC of what \
             arrived, by the same calculation as the sender, one byte at a time.\n\n{differing}"
        ),
        received_crc: format!(
            "### CRC of the received bytes\nThe last register XOR `{FINAL_MASK:08X}` is `{:08X}`. The CRC that the \
             sender sent was `{:08X}`.",
            t.received_computed.crc, t.sent_crc
        ),
        syndrome: format!(
            "### Syndrome\nThe CRC travels with the data, so the receiver has its own copy of the CRC, on the left. {arrival}It \
             XORs that with the CRC it worked out: `{:08X}` XOR `{:08X}` = `{:08X}`.\n\nIf the data and the CRC arrived \
             unchanged, this is **zero**. Anything else means that something changed, and the error is **detected**. The \
             syndrome is more than a warning. It depends only on **where** the flipped bits are, and not on what the data \
             holds, so it can say **which** bit.",
            t.arrived_crc, t.received_computed.crc, t.syndrome
        ),
        position: format!(
            "### Error position\nA polynomial this good gives each of the {bits} bits of {n} bytes a different syndrome, \
             and none is a single 1. So the receiver works out the syndrome of flipping each bit and looks for the one \
             that matches `{:08X}`.\n\n{found}\n\nBits are counted from 0 at the least significant bit of a byte.",
            t.syndrome,
            bits = n * 8
        ),
        corrected: format!("### Corrected bytes\n{corrected}"),
    }
}
