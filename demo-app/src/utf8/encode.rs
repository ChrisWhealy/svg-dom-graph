//! UTF-8 encoding as RFC 3629 defines it, worked out for one character at a time so that the demo can show every step.
//! Nothing here touches the DOM or the library.

/// The characters the demo can show, one for each length of sequence, from one byte to four.
pub(super) const SAMPLES: [char; 4] = ['A', '£', '€', '😀'];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many bytes the code point `code_point` takes: one up to U+007F, two up to U+07FF, three up to U+FFFF, and four
/// above that.
pub(super) fn length_of(code_point: u32) -> usize {
    match code_point {
        0..=0x7F => 1,
        0x80..=0x7FF => 2,
        0x800..=0xFFFF => 3,
        _ => 4,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many bits of the code point a sequence of `len` bytes holds: the bits of the bytes that are not the template.
pub(super) fn payload_len(len: usize) -> usize {
    match len {
        1 => 7,
        2 => 11,
        3 => 16,
        _ => 21,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The template of a sequence of `len` bytes, where `x` is a bit of the code point.
pub(super) fn template(len: usize) -> &'static str {
    match len {
        1 => "0xxxxxxx",
        2 => "110xxxxx 10xxxxxx",
        3 => "1110xxxx 10xxxxxx 10xxxxxx",
        _ => "11110xxx 10xxxxxx 10xxxxxx 10xxxxxx",
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The range of code points that take `len` bytes, written as the standard does.
pub(super) fn range(len: usize) -> &'static str {
    match len {
        1 => "U+0000 to U+007F",
        2 => "U+0080 to U+07FF",
        3 => "U+0800 to U+FFFF",
        _ => "U+10000 to U+10FFFF",
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the range of `len` bytes leaves out, if it leaves anything out. The range of three bytes holds U+D800 to
/// U+DFFF, the surrogates, which UTF-16 uses in pairs to write the characters above U+FFFF. They are not characters,
/// and RFC 3629 forbids encoding them. A Rust `char` cannot hold one, so [`encode`] never meets one.
pub(super) fn range_exception(len: usize) -> Option<&'static str> {
    (len == 3).then_some("U+D800 to U+DFFF, which are reserved for UTF-16 surrogates and cannot be encoded in UTF-8")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which bits of byte `byte` of a sequence of `len` bytes are the template, as indices into that byte's eight bits from
/// the left: in the first byte the leading `1`s and the `0` after them, and in every other byte the `10` that starts
/// it.
pub(super) fn byte_template_bits(len: usize, byte: usize) -> Vec<usize> {
    match (len, byte) {
        (1, _) => vec![0],
        (_, 0) => (0..=len).collect(),
        _ => vec![0, 1],
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which bits of a sequence of `len` bytes are the template, as indices into all its bits written out from the left.
#[cfg(test)]
pub(super) fn template_bits(len: usize) -> Vec<usize> {
    (0..len)
        .flat_map(|byte| byte_template_bits(len, byte).into_iter().map(move |bit| 8 * byte + bit))
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many bits of the code point go into each byte of a sequence of `len` bytes, from the first byte: the bits the
/// template leaves in it. They add up to [`payload_len`], and the demo lines each group up over its byte.
pub(super) fn payload_groups(len: usize) -> Vec<usize> {
    (0..len).map(|byte| 8 - byte_template_bits(len, byte).len()).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the demo shows about encoding one character.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Encoded {
    pub(super) code_point: u32,
    /// The bits of the code point that go into the bytes, from the most significant. Leading zeros are kept, so there
    /// are exactly [`payload_len`] of them.
    pub(super) payload: Vec<u8>,
    /// The bytes of the encoding.
    pub(super) bytes: Vec<u8>,
    /// Every bit of `bytes`, from the left, eight to a byte.
    pub(super) bits: Vec<u8>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The bits of `value` from bit `count - 1` down to bit 0.
fn bits_of(value: u32, count: usize) -> Vec<u8> {
    (0..count).rev().map(|shift| u8::from((value >> shift) & 1 == 1)).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Encodes `c` by filling the template for its length, not by calling the standard library, so that each step can be
/// shown. The unit tests check it against the standard library for every character.
pub(super) fn encode(c: char) -> Encoded {
    let code_point = u32::from(c);
    let len = length_of(code_point);
    // The low six bits of the code point shifted right by `shift`, as a continuation byte `10xxxxxx`.
    let continuation = |shift: u32| 0x80 | ((code_point >> shift) & 0x3F) as u8;
    let bytes = match len {
        1 => vec![code_point as u8],
        2 => vec![0xC0 | (code_point >> 6) as u8, continuation(0)],
        3 => vec![0xE0 | (code_point >> 12) as u8, continuation(6), continuation(0)],
        _ => vec![
            0xF0 | (code_point >> 18) as u8,
            continuation(12),
            continuation(6),
            continuation(0),
        ],
    };
    let bits = bytes.iter().flat_map(|&byte| bits_of(u32::from(byte), 8)).collect();
    Encoded {
        code_point,
        payload: bits_of(code_point, payload_len(len)),
        bytes,
        bits,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `c` as a Markdown code run, if it is a character that is safe and worth showing: not a control character, not
/// whitespace, which would show as nothing, and not one that ends the code run.
pub(super) fn glyph(c: char) -> Option<String> {
    (!c.is_control() && !c.is_whitespace() && c != '`' && c != '\\').then(|| format!("`{c}`"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The note under one character, in Markdown: its code point, how many bytes it needs and why, and how the bits fit.
pub(super) fn explain(c: char, encoded: &Encoded) -> String {
    let len = encoded.bytes.len();
    let shown = glyph(c).map_or_else(String::new, |g| format!(" {g}"));
    let bits: String = encoded.payload.iter().map(|&b| char::from(b'0' + b)).collect();
    let rule = if len == 1 {
        "It is ASCII: one byte, whose top bit is `0`.".to_owned()
    } else {
        format!(
            "The first byte starts with {len} ones and a zero, which says how long the sequence is. Every other byte starts \
             with `10`, which says it continues one."
        )
    };
    let except = range_exception(len).map_or_else(String::new, |what| format!(" (not counting {what})"));
    format!(
        "### U+{:04X}{shown}\nThe code point is in the range {}{except}, so it takes {len} byte{}. {rule} The template is `{}`, \
         where each `x` is a bit of the code point. Its {} bits are `{bits}`, and they fill the `x`s from the left. The \
         highlighted bits are the template. Each group of code point bits is over the byte it fills.",
        encoded.code_point,
        range(len),
        if len == 1 { "" } else { "s" },
        template(len),
        payload_len(len),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The bytes of `c` as hexadecimal pairs separated by spaces.
pub(super) fn hex_of(c: char) -> String {
    encode(c)
        .bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a button for `c` says, and what a screen reader reads for it: the character, its code point, and its length.
pub(super) fn button_label(c: char) -> String {
    let len = encode(c).bytes.len();
    format!("{c} U+{:04X}, {len} byte{}", u32::from(c), if len == 1 { "" } else { "s" })
}
