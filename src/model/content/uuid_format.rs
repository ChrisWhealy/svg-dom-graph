//! `NodeValues::Uuid`: a 128-bit value shown as a UUID, and the [`UuidFormat`] that says how its text is written.

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A 128-bit value to be shown as a UUID: sixteen bytes that [`DataFormat::Uuid`](super::DataFormat::Uuid) writes as
/// grouped hexadecimal digits.
///
/// This is a plain 128-bit value and nothing more. It is not the `uuid` crate's type, it does not generate or parse
/// anything, and it does not check that its bits describe a valid UUID. Any 128-bit value can be shown as one. The
/// value is held as a `u128` whose big-endian bytes are the sixteen bytes in order, so `0x0123..` has `01` as its
/// first byte, whatever the host's own byte order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Uuid(u128);

impl Uuid {
    /// The UUID whose sixteen bytes are those of `value`, most significant first.
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }

    /// The UUID of these sixteen `bytes`, in order.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(u128::from_be_bytes(bytes))
    }

    /// The same 128 bits as a `u128`.
    pub const fn as_u128(self) -> u128 {
        self.0
    }

    /// The sixteen bytes, in order.
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
}

impl From<u128> for Uuid {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl From<[u8; 16]> for Uuid {
    fn from(bytes: [u8; 16]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<Uuid> for u128 {
    fn from(uuid: Uuid) -> Self {
        uuid.0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The punctuation around a UUID's digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum UuidStyle {
    /// The canonical form of RFC 9562, in groups of 8, 4, 4, 4 and 12 digits joined by hyphens:
    /// `123e4567-e89b-12d3-a456-426614174000`.
    #[default]
    Hyphenated,
    /// The same 32 digits with nothing between them: `123e4567e89b12d3a456426614174000`.
    Simple,
    /// The canonical form inside braces, as the Microsoft registry and many Windows tools write a GUID:
    /// `{123e4567-e89b-12d3-a456-426614174000}`.
    Braced,
    /// The canonical form as a URN, as RFC 9562 defines it: `urn:uuid:123e4567-e89b-12d3-a456-426614174000`. The
    /// `urn:uuid:` prefix is always lowercase, whatever the [`UuidCase`].
    Urn,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether a UUID's hexadecimal digits are written in lowercase or uppercase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum UuidCase {
    /// `a` to `f`. RFC 9562 says to write a UUID this way.
    #[default]
    Lower,
    /// `A` to `F`, as Windows tools usually write a GUID.
    Upper,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How the sixteen bytes held in a [`Uuid`] map to the fields that its text is written from.
///
/// A UUID has five fields: a 32-bit one, two 16-bit ones, and then eight bytes. The two layouts differ in how the first
/// three are stored, and so in what the sixteen bytes are when they are read out of memory or off the wire. The text of
/// a UUID is the same in both, since it always writes the fields themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum UuidLayout {
    /// RFC 9562: every field is stored most significant byte first, so the sixteen bytes are the digits of the text in
    /// order. This is the order a UUID has on the network, and in a database column or a file that stores UUIDs as
    /// bytes.
    #[default]
    Rfc,
    /// A Microsoft GUID as it sits in memory: the first three fields are stored least significant byte first, and the
    /// last eight bytes are as they are written. So the bytes `33 22 11 00 55 44 77 66 88 99 AA BB CC DD EE FF` are the
    /// UUID `00112233-4455-6677-8899-aabbccddeeff`.
    Guid,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a [`Uuid`] is written: the punctuation around it, the case of its digits, and the byte layout that it is read
/// from.
/// Used by [`DataFormat::Uuid`](super::DataFormat::Uuid).
///
/// The default is the form RFC 9562 gives, `123e4567-e89b-12d3-a456-426614174000`: hyphenated, lowercase, and read in
/// RFC byte order. Every format writes a UUID with the same number of characters, whatever its value, so a node is as
/// wide for one UUID as for any other. That number is 32 for [`UuidStyle::Simple`], 36 for [`UuidStyle::Hyphenated`],
/// 38 for [`UuidStyle::Braced`] and 45 for [`UuidStyle::Urn`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UuidFormat {
    style: UuidStyle,
    case: UuidCase,
    layout: UuidLayout,
}

impl UuidFormat {
    /// The default format: hyphenated, lowercase, in RFC byte order.
    pub const fn new() -> Self {
        Self {
            style: UuidStyle::Hyphenated,
            case: UuidCase::Lower,
            layout: UuidLayout::Rfc,
        }
    }

    /// This format, but with the punctuation `style`.
    pub const fn with_style(mut self, style: UuidStyle) -> Self {
        self.style = style;
        self
    }

    /// This format, but with digits in `case`.
    pub const fn with_case(mut self, case: UuidCase) -> Self {
        self.case = case;
        self
    }

    /// This format, but reading the bytes in `layout`.
    pub const fn with_layout(mut self, layout: UuidLayout) -> Self {
        self.layout = layout;
        self
    }

    /// The punctuation around the digits.
    pub const fn style(self) -> UuidStyle {
        self.style
    }

    /// The case of the digits.
    pub const fn case(self) -> UuidCase {
        self.case
    }

    /// The byte layout the UUID is read from.
    pub const fn layout(self) -> UuidLayout {
        self.layout
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `uuid` into `out` as `format` says. Clears `out` first.
///
/// The text is always the same length for one format, so the widest UUID is any UUID. See [`UuidFormat`].
pub(super) fn format_uuid_into(uuid: Uuid, format: UuidFormat, out: &mut String) {
    use std::fmt::Write as _;
    out.clear();

    let mut bytes = uuid.to_bytes();
    if format.layout == UuidLayout::Guid {
        // The first three fields are stored least significant byte first. Reversing each puts them in the order that
        // they are written.
        bytes[0..4].reverse();
        bytes[4..6].reverse();
        bytes[6..8].reverse();
    }

    let hyphens = format.style != UuidStyle::Simple;
    match format.style {
        UuidStyle::Braced => out.push('{'),
        UuidStyle::Urn => out.push_str("urn:uuid:"),
        UuidStyle::Hyphenated | UuidStyle::Simple => {},
    }
    for (i, b) in bytes.iter().enumerate() {
        if hyphens && matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        let _ = match format.case {
            UuidCase::Lower => write!(out, "{b:02x}"),
            UuidCase::Upper => write!(out, "{b:02X}"),
        };
    }
    if format.style == UuidStyle::Braced {
        out.push('}');
    }
}
