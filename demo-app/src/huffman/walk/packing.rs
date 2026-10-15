use super::{
    super::algorithm::{PackStep, Trace},
    shown::code_text,
    symbol_markdown,
};

/// Everything the scene shows at one step of pass two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::huffman) struct Packing {
    /// What happened at this step, and why, in up to three sentences. The first says what happened.
    pub(in crate::huffman) caption: Vec<String>,
    /// The byte whose code is being packed, so the table can ring its row. `None` at the flush.
    pub(in crate::huffman) symbol: Option<u8>,
    /// The bits the step works on, most significant first: the pending bits followed by the new code. At the flush, the
    /// last pending bits followed by the zeros that pad them to a whole byte.
    pub(in crate::huffman) appended: Vec<bool>,
    /// The cells of `appended` that are new at this step: the code, or the padding.
    pub(in crate::huffman) marked: Vec<usize>,
    /// The bits still pending once the step's whole bytes are written. Always fewer than 8, and none after the flush.
    pub(in crate::huffman) pending: Vec<bool>,
    /// Every byte written so far.
    pub(in crate::huffman) output: Vec<u8>,
    /// The cells of `output` that this step wrote.
    pub(in crate::huffman) new_bytes: Vec<usize>,
    /// At the flush only: the finished output's size against the input's, as Markdown.
    pub(in crate::huffman) summary: Option<String>,
}

/// How many positions pass two has: one for each input byte, then the flush that pads the last byte.
pub(in crate::huffman) fn pack_steps(trace: &Trace) -> usize {
    trace.packed.steps.len() + 1
}

/// The low `count` bits of `value`, most significant first.
fn bits_of(value: u64, count: u8) -> Vec<bool> {
    (0..count).rev().map(|i| value >> i & 1 == 1).collect()
}

/// A run of bytes as binary, comma-separated.
fn binary_list(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:08b}")).collect::<Vec<_>>().join(", ")
}

/// What the scene shows at step `index` of pass two, counting from 0. An index past the last is the last.
///
/// Each input byte is one step, and the flush that pads the last byte is the step after them. The flush is not a
/// [`PackStep`] of its own: it is worked out from the last step and the packed output's padding.
pub(in crate::huffman) fn packing(trace: &Trace, index: usize) -> Packing {
    let steps = &trace.packed.steps;
    let index = index.min(steps.len());
    match steps.get(index) {
        Some(step) => packing_step(trace, index, step),
        None => flush(trace),
    }
}

/// The step that appends the code of input byte `index`.
fn packing_step(trace: &Trace, index: usize, step: &PackStep) -> Packing {
    let steps = &trace.packed.steps;
    let output: Vec<u8> = steps[..=index].iter().flat_map(|s| s.emitted.iter().copied()).collect();
    let new_bytes = (output.len() - step.emitted.len()..output.len()).collect();

    let mut appended = bits_of(step.acc_before, step.nbits_before);
    let before = appended.len();
    appended.extend(bits_of(u64::from(step.code.bits), step.code.len));
    let marked = (before..appended.len()).collect();

    let bits_so_far: usize = steps[..=index].iter().map(|s| usize::from(s.code.len)).sum();
    Packing {
        caption: step_caption(index, steps.len(), step, bits_so_far, output.len()),
        symbol: Some(step.symbol),
        appended,
        marked,
        pending: bits_of(step.acc_after, step.nbits_after),
        output,
        new_bytes,
        summary: None,
    }
}

/// The sentences for appending the code of input byte `index`.
fn step_caption(index: usize, total: usize, step: &PackStep, bits_so_far: usize, written: usize) -> Vec<String> {
    let name = symbol_markdown(step.symbol);
    let code = code_text(step.code);
    let pending = match step.nbits_before {
        0 => "There are no pending bits yet.".to_owned(),
        1 => "One bit is pending.".to_owned(),
        n => format!("{n} bits are pending."),
    };
    let first = format!(
        "**Byte {} of {total}**: '{name}' has the code {code}, which is {} bits. {pending}",
        index + 1,
        step.code.len
    );

    let held = step.nbits_before + step.code.len;
    let second = match step.emitted.len() {
        0 => format!("Together they make {held} bits, which is fewer than 8, so no byte is complete yet."),
        n => format!(
            "Together they make {held} bits, enough for {} to write: {}. {} stay pending.",
            if n == 1 { "one whole byte".to_owned() } else { format!("{n} whole bytes") },
            binary_list(&step.emitted),
            match step.nbits_after {
                0 => "No bits".to_owned(),
                1 => "One bit".to_owned(),
                n => format!("{n} bits"),
            }
        ),
    };
    let third = if index == 0 {
        "Appending is one shift and one OR, not a loop over the bits: the pending bits shift left by the code's length, then the code is OR-ed in."
            .to_owned()
    } else {
        format!("So far {bits_so_far} bits of code have been packed, and {written} whole bytes written.")
    };
    vec![first, second, third]
}

/// The step after the last input byte, which pads what is pending out to a whole byte.
fn flush(trace: &Trace) -> Packing {
    let packed = &trace.packed;
    let padding = packed.padding();
    let (pending, input_len) = match packed.steps.last() {
        Some(last) => (bits_of(last.acc_after, last.nbits_after), packed.steps.len()),
        None => (Vec::new(), 0),
    };

    let mut appended = pending.clone();
    let before = appended.len();
    appended.extend(std::iter::repeat_n(false, usize::from(padding)));
    let marked = (before..appended.len()).collect();
    let new_bytes = if padding > 0 { vec![packed.bytes.len() - 1] } else { Vec::new() };

    let first = if padding == 0 {
        "**Flush**: the last code ended exactly on a byte boundary, so nothing is pending and no padding is needed."
            .to_owned()
    } else {
        format!(
            "**Flush**: {} pending {} cannot fill a byte, so {padding} zero {} pad it out: {:08b}.",
            pending.len(),
            if pending.len() == 1 { "bit" } else { "bits" },
            if padding == 1 { "bit" } else { "bits" },
            packed.bytes.last().copied().unwrap_or(0),
        )
    };
    let second = format!(
        "The {input_len} input bytes ({} bits) are now {} bits in {} bytes. The result is shown below in hexadecimal.",
        input_len * 8,
        packed.bit_len,
        packed.bytes.len(),
    );
    let third = "A decoder is given the code lengths, to rebuild the codes, and the bit count, to ignore the padding."
        .to_owned();

    Packing {
        caption: vec![first, second, third],
        symbol: None,
        appended,
        marked,
        pending: Vec::new(),
        output: packed.bytes.clone(),
        new_bytes,
        summary: Some(summary(trace)),
    }
}

/// The finished output's size against the input's, and the compression ratio that makes of them. Two lines, each a
/// Markdown paragraph.
///
/// The code lengths a decoder needs to rebuild the codes are not counted, since this demo does not write them out.
fn summary(trace: &Trace) -> String {
    let packed = &trace.packed;
    let original = packed.steps.len().max(1);
    let encoded = packed.bytes.len();
    let ratio = original as f64 / encoded.max(1) as f64;
    // Tenths of a percent, rounded to the nearest, so that the text has no floating point noise. The saving is taken from
    // this figure, so the two always add up to 100%.
    let permille = (encoded * 1000 + original / 2) / original;
    let change = if encoded <= original {
        format!("a saving of {}.{}%", (1000 - permille) / 10, (1000 - permille) % 10)
    } else {
        format!("{}.{}% larger", (permille - 1000) / 10, (permille - 1000) % 10)
    };
    format!(
        "**Original**: {original} bytes ({} bits)\n\n**Encoded**: {encoded} bytes, which is {} bits of code and {} of \
         padding\n\n**Compression ratio**: {ratio:.2} to 1. The output is {}.{}% of the original size, {change}. The \
         code lengths a decoder needs are not counted.",
        original * 8,
        packed.bit_len,
        packed.padding(),
        permille / 10,
        permille % 10,
    )
}
