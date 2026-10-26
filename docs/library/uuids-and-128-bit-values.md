# 128-bit Values and UUIDs

`NodeValues::U128` and `NodeValues::I128` are the 128-bit integers.
`NodeValues::Uuid` holds `Uuid` values, which are also 128 bits wide but are not numbers.
Like every other type, all the values of a node share one.
Each has its own colour and its own name for assistive technology: `u128`, `i128` and `uuid`.

## `u128` and `i128`

These work as every other integer width does.
A decimal `u128` runs to 39 digits, and an `i128` shows a sign only if it is negative.
In hexadecimal and binary an `i128` is shown as its two's complement, so `-1` is sixteen bytes of `FF`.
The widest value of a decimal node is found without formatting every value, as it is for the narrower widths.
For an `i128`, the most negative value is the widest, since it has one more digit than the largest and a sign.

## UUIDs

A `Uuid` is a plain 128-bit value.
It is not the `uuid` crate's type, and it generates, parses and validates nothing, so any 128 bits can be shown as one.
`Uuid::from_u128` and `Uuid::from_bytes` make one, and `as_u128` and `to_bytes` read it back.
The sixteen bytes are the big-endian bytes of the `u128`, so `0x0123...` has `01` as its first byte on every host.

Content that holds UUIDs is written by `DataFormat::Uuid(UuidFormat)`.
A `UuidFormat` has three parts, each with a builder method (`with_style`, `with_case` and `with_layout`):

| Part | Choices |
|---|---|
| `UuidStyle` | `Hyphenated` (the default, `123e4567-e89b-12d3-a456-426614174000`), `Simple` (no hyphens), `Braced` (`{...}`) and `Urn` (`urn:uuid:...`) |
| `UuidCase` | `Lower` (the default, and what RFC 9562 says) and `Upper` |
| `UuidLayout` | `Rfc` (the default) and `Guid` |

A `Urn` always has a lowercase `urn:uuid:` prefix, whatever the case of its digits.
Every format writes the same number of characters whatever the UUID: 32 for `Simple`, 36 for `Hyphenated`, 38 for `Braced` and 45 for `Urn`.
So a node is as wide for one UUID as for any other.

### Byte layouts

A UUID has five fields: a 32-bit one, two 16-bit ones, and then eight bytes.
RFC 9562 stores every field most significant byte first, so the sixteen bytes are the digits of the text in order.
A Microsoft GUID in memory stores the first three fields least significant byte first.
So the bytes `33 22 11 00 55 44 77 66 88 99 AA BB CC DD EE FF` are the UUID `00112233-4455-6677-8899-aabbccddeeff` under `UuidLayout::Guid`, and a different UUID, `33221100-5544-7766-8899-aabbccddeeff`, under `UuidLayout::Rfc`.
The text of a UUID is the same in both, since it writes the fields themselves.
The layout changes the meaning of the bytes held in the `Uuid`.

### Other formats

`DataFormat::Hexadecimal`, `Binary` and `Ascii` show a UUID's sixteen bytes as they are held, in the node's `ByteOrder`.
`DataFormat::Decimal` shows the 128 bits as one number.
`ByteOrder` has no effect on `DataFormat::Uuid`, since its format names the layout itself.

### Restrictions

`DataFormat::Uuid` is only for `Uuid` values, and any other content is rejected with `Error::InvalidUuidFormat` before anything is drawn.

No operator node accepts a UUID, of any kind, since an identifier has no sum, no remainder and no meaningful rotation.
`Error::OperatorOnUuid` names the operator.
A 128-bit number is an ordinary operand, so use `U128` or `I128` for one.
