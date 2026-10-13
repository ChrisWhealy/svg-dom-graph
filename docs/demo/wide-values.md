# 128-bit Values and UUIDs

Demonstrates `NodeValues::U128`, `I128` and `Uuid`, and every way a `UuidFormat` can write a UUID.

* **Lines 1 and 2: integers.**<br>
  A `u128` from zero to the largest, which has 39 digits, then an `i128` from the most negative to the largest, each in decimal and in hexadecimal.
  A negative `i128` has a sign in decimal, and in hexadecimal it is two's complement, so `-1` is sixteen bytes of `FF`.
  The most negative value is the widest in its node, because its sign makes it a character longer than the largest.
* **Line 3: a UUID of every version.**<br>
  There is one of each version that RFC 9562 defines, and the nil and max UUIDs, which have no version.
  The first digit of the third group is the version.
  The second node is the same UUIDs under `DataFormat::Decimal`, as the numbers they are.
* **Lines 4 and 5: every way to write one UUID.**<br>
  The same UUID is hyphenated, simple, braced, a URN and uppercase.
  Every format writes a fixed number of characters, so a node is as wide for one UUID as for any other.
* **Lines 6 and 7: byte layouts.**<br>
  The same UUID is sixteen different bytes in memory as a Microsoft GUID than it is as an RFC 9562 UUID, since the GUID stores its first three fields the other way round.
  The last line reads the GUID's bytes in each layout: read as a GUID they are the UUID, and read as RFC 9562 they are a different one.

See [128-bit Values and UUIDs](../library/uuids-and-128-bit-values.md) for the types and formats themselves.
