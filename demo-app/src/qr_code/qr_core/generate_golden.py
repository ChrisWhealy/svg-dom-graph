#!/usr/bin/env python3
"""Generates golden.rs: the data and error correction codewords that two independent encoders produce for a list of
inputs.

Run it from the repository root, in a virtualenv that has `qrcode` and `segno` installed:

    python3 demo-app/src/qr_code/qr_core/generate_golden.py | rustfmt --edition 2024 --config-path rustfmt.toml \\
        > demo-app/src/qr_code/qr_core/golden.rs

It prints the versions it used, which golden.rs records.

The vectors come from python-qrcode, which keeps its codewords in `data_cache`, so they are read directly.

segno is a cross-check, and it disagrees with the standard in one respect that this script asserts, so that it is on
record and not hidden. segno pads the data to a codeword boundary with `8 - length % 8` zero bits. A stream that is
already on a boundary then gets a whole extra zero codeword. In byte mode at Version 1-L, the 12 bits of the header, the
bytes and a 4 bit terminator are always on a boundary, so every segno symbol has that extra zero codeword before its
pad codewords, and its error correction bytes follow from it. Such a symbol still scans, since a reader stops at the
terminator. The standard says to pad only a stream that is not on a boundary, which is what python-qrcode and the
encoder under test do.

For whole-matrix fixtures, this means the two encoders do not start from the same codewords. Their symbols differ in
every module that carries data or error correction, whichever mask each chooses. So a matrix, a candidate for a mask, a
penalty score or a chosen mask from segno cannot be compared with the Rust encoder's. Use python-qrcode for those, which
takes the mask as an argument. Keep segno only as a cross-check, with this difference stated and asserted, as it is here.

segno does not hand out its codewords, so each is decoded from its finished matrix by code that shares nothing with the
Rust encoder. Two checks guard that decoding: the format information read from the matrix must be the one worked out
here for level L and the mask segno says it chose, and the error correction bytes must be what the plain Reed-Solomon
implementation in this file makes of the data bytes. python-qrcode's own error correction bytes get the second check
too.
"""
import sys

import importlib.metadata

import qrcode
import qrcode.constants
import qrcode.util
import segno

# Each input is a byte string, because the encoder under test takes bytes. They cover the empty text, one byte, ordinary
# text, punctuation, a control character, bytes above 127, and the capacity limit and one byte under it.
INPUTS = [
    b"",
    b"A",
    b"Hello",
    b"Hello, World!",
    b"HELLO WORLD",
    b"https://a.co/x",
    b"0123456789012345",
    b"abcdefghijklmnop",
    b"abcdefghijklmnopq",
    b"!\"#$%&'()*+,-./:;",
    b"  tabs\tand\nlines ",
    b"\x00\x01\x7f",
    b"\xff\xfe\xec\x11",
]

SIZE = 21


def function_modules():
    """The 233 function modules, from the layout in the standard, as a set of (row, column)."""
    cells = set()
    for r0, c0 in [(0, 0), (0, SIZE - 7), (SIZE - 7, 0)]:
        for r in range(r0 - 1, r0 + 8):
            for c in range(c0 - 1, c0 + 8):
                if 0 <= r < SIZE and 0 <= c < SIZE:
                    cells.add((r, c))
    cells |= {(6, c) for c in range(8, SIZE - 8)} | {(r, 6) for r in range(8, SIZE - 8)}
    cells.add((13, 8))
    cells |= {(r, 8) for r in range(0, 9)} | {(8, c) for c in range(0, 9)}
    cells |= {(8, c) for c in range(SIZE - 8, SIZE)} | {(r, 8) for r in range(SIZE - 7, SIZE)}
    return cells


def mask_condition(mask, r, c):
    return [
        (r + c) % 2 == 0,
        r % 2 == 0,
        c % 3 == 0,
        (r + c) % 3 == 0,
        (r // 2 + c // 3) % 2 == 0,
        (r * c) % 2 + (r * c) % 3 == 0,
        ((r * c) % 2 + (r * c) % 3) % 2 == 0,
        ((r + c) % 2 + (r * c) % 3) % 2 == 0,
    ][mask]


def format_bits(mask):
    """The 15 format bits for level L (01) and `mask`, with the BCH code and the XOR mask."""
    data = (0b01 << 3) | mask
    rem = data << 10
    for shift in range(14, 9, -1):
        if rem >> shift & 1:
            rem ^= 0x537 << (shift - 10)
    return ((data << 10) | rem) ^ 0x5412


def read_format(matrix):
    """Copy A of the format bits, as the design puts them: bit i at the listed (row, column)."""
    places = [(i, 8) for i in range(6)] + [(7, 8), (8, 8), (8, 7)] + [(8, c) for c in range(5, -1, -1)]
    return sum(matrix[r][c] << i for i, (r, c) in enumerate(places))


def codewords_of(matrix, mask):
    reserved = function_modules()
    assert len(reserved) == 233, len(reserved)
    bits = []
    right = SIZE - 1
    upward = True
    while right >= 1:
        if right == 6:
            right = 5
        for step in range(SIZE):
            r = SIZE - 1 - step if upward else step
            for c in (right, right - 1):
                if (r, c) not in reserved:
                    bits.append(matrix[r][c] ^ mask_condition(mask, r, c))
        upward = not upward
        right -= 2
    assert len(bits) == 208, len(bits)
    return [sum(bit << (7 - i) for i, bit in enumerate(bits[n:n + 8])) for n in range(0, 208, 8)]


# A second Reed-Solomon implementation, written as plainly as possible and unlike the encoder under test: multiplication
# is the shift-and-reduce loop, not a table, and the division is done on whole polynomials.
def gf_mul(a, b):
    product = 0
    while b:
        if b & 1:
            product ^= a
        a <<= 1
        if a & 0x100:
            a ^= 0x11D
        b >>= 1
    return product


def poly_mul(p, q):
    out = [0] * (len(p) + len(q) - 1)
    for i, a in enumerate(p):
        for j, b in enumerate(q):
            out[i + j] ^= gf_mul(a, b)
    return out


def rs_remainder(data, degree):
    generator = [1]
    alpha_power = 1
    for _ in range(degree):
        generator = poly_mul(generator, [1, alpha_power])
        alpha_power = gf_mul(alpha_power, 2)
    work = list(data) + [0] * degree
    for i in range(len(data)):
        factor = work[i]
        if factor:
            for j, g in enumerate(generator):
                work[i + j] ^= gf_mul(g, factor)
    return work[len(data):]


def segno_codewords(data):
    """The data and error correction codewords segno makes, decoded from its matrix."""
    symbol = segno.make(data, version=1, error="L", mode="byte", micro=False, boost_error=False)
    assert symbol.version == 1 and symbol.error == "L" and symbol.mode == "byte", (symbol.version, symbol.error, symbol.mode)
    matrix = [list(row) for row in symbol.matrix]
    assert len(matrix) == SIZE
    assert read_format(matrix) == format_bits(symbol.mask), "the format information is not the one for the mask segno chose"
    words = codewords_of(matrix, symbol.mask)
    codewords, parity = words[:19], words[19:]
    assert parity == rs_remainder(codewords, 7), "segno's error correction bytes are not what plain Reed-Solomon gives"
    return codewords, parity


def qrcode_codewords(data):
    """The data and error correction codewords python-qrcode makes, in byte mode at Version 1-L."""
    symbol = qrcode.QRCode(version=1, error_correction=qrcode.constants.ERROR_CORRECT_L, box_size=1, border=0)
    symbol.add_data(qrcode.util.QRData(data, mode=qrcode.util.MODE_8BIT_BYTE))
    symbol.make(fit=False)
    assert symbol.version == 1
    words = list(symbol.data_cache)
    assert len(words) == 26, len(words)
    codewords, parity = words[:19], words[19:]
    assert parity == rs_remainder(codewords, 7), "qrcode's error correction bytes are not what plain Reed-Solomon gives"
    # The data codewords must start with the byte mode indicator and the count, and hold the bytes after them.
    bitstream = "".join(f"{w:08b}" for w in codewords)
    assert bitstream[:4] == "0100" and int(bitstream[4:12], 2) == len(data), "not a byte mode symbol of this length"
    assert bytes(int(bitstream[12 + 8 * i:20 + 8 * i], 2) for i in range(len(data))) == data
    return codewords, parity


def golden(data):
    codewords, parity = qrcode_codewords(data)
    other_codewords, _ = segno_codewords(data)
    # Both agree on the header, the bytes and the terminator, which are whole codewords: the header is 12 bits, the
    # bytes are 8 each and the terminator 4. Where segno differs is the extra zero codeword it then adds.
    terminated = 2 + len(data)
    assert other_codewords[:terminated] == codewords[:terminated], "the two disagree before the padding"
    expected = (codewords[:terminated] + [0x00] + codewords[terminated:])[:19]
    assert other_codewords == expected, "segno's padding is not the extra zero codeword that was found"
    return codewords, parity


def rust_bytes(values):
    return ", ".join(f"0x{v:02X}" for v in values)


def rust_literal(data):
    return "&[" + rust_bytes(data) + "]"


print("//! Golden vectors for the encoder: the codewords python-qrcode makes for each input. Generated by")
print("//! `generate_golden.py`; do not edit by hand.")
print("//!")
print(f"//! Oracle: qrcode {importlib.metadata.version('qrcode')} (python-qrcode), `version=1`, `ERROR_CORRECT_L`, byte mode, `fit=False`.")
print(f"//! Cross-check: segno {segno.__version__}, `version=1, error=\"L\", mode=\"byte\", micro=False, boost_error=False`.")
print("//! segno pads a stream already on a codeword boundary with a whole extra zero codeword, which the standard does")
print("//! not. The script asserts that, and so these vectors are not segno's. Its doc comment gives the reason.")
print("//! A matrix, mask candidate or penalty score from segno therefore cannot be compared with the encoder's.")
print()
print("/// One input, and the codewords python-qrcode made of it.")
print("pub(super) struct Golden {")
print("    pub(super) input: &'static [u8],")
print("    pub(super) data: [u8; 19],")
print("    pub(super) error_correction: [u8; 7],")
print("}")
print()
print("pub(super) const GOLDEN: &[Golden] = &[")
for data in INPUTS:
    words, parity = golden(data)
    print("    Golden {")
    print(f"        input: {rust_literal(data)},")
    print(f"        data: [{rust_bytes(words)}],")
    print(f"        error_correction: [{rust_bytes(parity)}],")
    print("    },")
print("];")
