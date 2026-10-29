#!/usr/bin/env python3
"""Generates symbol_golden.rs: for each input, the eight candidate matrices of a Version 1-L symbol, their format
information and their penalty scores, from independent encoders.

Run it from the repository root, in a virtualenv that has `qrcode` and `segno` installed:

    python3 demo-app/src/qr_code/qr_core/generate_symbol_golden.py | rustfmt --edition 2024 \\
        --config-path rustfmt.toml > demo-app/src/qr_code/qr_core/symbol_golden.rs

The candidate matrices come from python-qrcode, which takes the mask as an argument. Each is the whole symbol with its
mask applied and its own format information written, which is what the encoder under test scores. segno is not used for
matrices: it makes other codewords (see `generate_golden.py`), so its symbols differ in every data module. It is used
only to score python-qrcode's matrices, which is a fair comparison, since both then score the same modules.

The three differ in how they count the third penalty, N3, and the script records all three on every matrix:

- The design for this demo (the normative one): every row and column is padded with four light modules at each end,
  and every start position of either 11 module pattern, 00001011101 or 10111010000, scores 40. A pattern with light on
  both sides is found by both patterns, so it scores 80.
- python-qrcode: the same two patterns in the same 11 module windows, but with no padding, so nothing counts at the edge
  of the symbol unless its four light modules are inside it.
- segno: the 1011101 pattern scores 40 once, if four light modules or the symbol's edge lie on either side. A pattern
  with light on both sides therefore scores 40, not 80.

N1, N2 and N4 are the same in all three, and the script asserts that. It also asserts that its own plain implementations
of every score give what each library gives, so each reading of each convention is checked against the library that it
describes. The mask each convention would choose is recorded too, so that a difference in choice is on record.

python-qrcode differs from the design in a second way, which is not about N3. It chooses its mask by scoring candidates
that have no format information and a light dark module, because it builds them in a "test" mode before writing either.
The design scores the finished candidate, format information included. The script asserts that this is what python-qrcode
does, by scoring the unfinished candidates itself and checking that the mask it chooses is the one the library chose. So
there are four masks on record for each input: the design's, the one python-qrcode's scores choose on finished
candidates, the one python-qrcode actually chooses, and the one segno's scores choose on finished candidates.
"""
import importlib.metadata
import random

import qrcode
import qrcode.constants
import qrcode.util
import segno
import segno.encoder

SIZE = 21

# Each input is a byte string. These are the ones in golden.rs that matter to the symbol, and a few more, with some chosen
# at random below to find cases where the conventions choose another mask.
INPUTS = [
    b"",
    b"A",
    b"Hello, World!",
    b"HELLO WORLD",
    b"https://a.co/x",
    b"abcdefghijklmnopq",
    b"!\"#$%&'()*+,-./:;",
    b"\xff\xfe\xec\x11",
]

# How many random inputs to try for a difference between the conventions, and how many differing ones to keep.
SEARCH = 400
KEEP = 3


def format_bits(mask):
    """The 15 format bits for level L (01) and `mask`: BCH remainder with 0x537, then XOR with 0x5412."""
    data = (0b01 << 3) | mask
    rem = data << 10
    for shift in range(14, 9, -1):
        if rem >> shift & 1:
            rem ^= 0x537 << (shift - 10)
    return ((data << 10) | rem) ^ 0x5412


def format_places():
    """The two copies of the format bits as the design lists them: bit i of the word is at (row, column)."""
    a = [(i, 8) for i in range(6)] + [(7, 8), (8, 8), (8, 7)] + [(8, c) for c in range(5, -1, -1)]
    b = [(8, SIZE - 1 - i) for i in range(8)] + [(SIZE - 15 + i, 8) for i in range(8, 15)]
    return a, b


def candidate_matrix(data, mask):
    """The symbol python-qrcode makes with mask `mask`, as rows of 0 and 1."""
    symbol = qrcode.QRCode(version=1, error_correction=qrcode.constants.ERROR_CORRECT_L, box_size=1, border=0,
                           mask_pattern=mask)
    symbol.add_data(qrcode.util.QRData(data, mode=qrcode.util.MODE_8BIT_BYTE))
    symbol.make(fit=False)
    assert symbol.version == 1 and symbol.mask_pattern == mask
    matrix = [[int(bool(cell)) for cell in row] for row in symbol.modules]
    assert len(matrix) == SIZE and all(len(row) == SIZE for row in matrix)
    return matrix


def mask_of(matrix):
    """The mask whose format information `matrix` holds."""
    places, _ = format_places()
    word = sum(matrix[r][c] << i for i, (r, c) in enumerate(places))
    masks = [mask for mask in range(8) if format_bits(mask) == word]
    assert len(masks) == 1, "the format information is not that of any mask"
    return masks[0]


def unformatted(matrix):
    """`matrix` without its format information, and with a light dark module, as python-qrcode scores it to choose."""
    plain = [row[:] for row in matrix]
    for places in format_places():
        for r, c in places:
            plain[r][c] = 0
    plain[13][8] = 0
    return plain


def check_format(matrix, mask):
    """Both copies of the format bits in `matrix` must be the word worked out here, and the dark module must be dark."""
    word = format_bits(mask)
    for places in format_places():
        assert sum(matrix[r][c] << i for i, (r, c) in enumerate(places)) == word, "format information"
    assert matrix[13][8] == 1, "dark module"
    return word


# Plain implementations of the four scores, written from the design and nothing else.
def n1(matrix):
    score = 0
    for lines in (matrix, [list(col) for col in zip(*matrix)]):
        for line in lines:
            run = 1
            for before, now in zip(line, line[1:]):
                if now == before:
                    run += 1
                else:
                    score += run - 2 if run >= 5 else 0
                    run = 1
            score += run - 2 if run >= 5 else 0
    return score


def n2(matrix):
    return 3 * sum(
        matrix[r][c] == matrix[r][c + 1] == matrix[r + 1][c] == matrix[r + 1][c + 1]
        for r in range(SIZE - 1) for c in range(SIZE - 1)
    )


def n3(matrix, border):
    """40 for every start position of either 11 module pattern, with four light modules of padding if `border`."""
    patterns = ("00001011101", "10111010000")
    pad = "0000" if border else ""
    score = 0
    for lines in (matrix, [list(col) for col in zip(*matrix)]):
        for line in lines:
            text = pad + "".join(map(str, line)) + pad
            for start in range(len(text) - 10):
                score += 40 * (text[start:start + 11] in patterns)
    return score


def n4(matrix):
    dark = sum(map(sum, matrix))
    return 10 * (abs(2 * dark - SIZE * SIZE) * 10 // (SIZE * SIZE))


def scores(matrix):
    """Every score of `matrix`, from the libraries and from the plain implementations above, checked against each other."""
    q1 = qrcode.util._lost_point_level1(matrix, SIZE)
    q2 = qrcode.util._lost_point_level2(matrix, SIZE)
    q3 = qrcode.util._lost_point_level3(matrix, SIZE)
    q4 = qrcode.util._lost_point_level4(matrix, SIZE)
    s1, s2, s3, s4 = segno.encoder.mask_scores([bytearray(row) for row in matrix], SIZE, SIZE)
    assert (n1(matrix), n2(matrix), n4(matrix)) == (q1, q2, q4) == (s1, s2, s4), "N1, N2 and N4 differ between the three"
    assert n3(matrix, border=False) == q3, "the plain reading of python-qrcode's N3 is not what it gives"
    return {
        "n1": n1(matrix),
        "n2": n2(matrix),
        "n3": n3(matrix, border=True),
        "n3_qrcode": q3,
        "n3_segno": s3,
        "n4": n4(matrix),
    }


def choose(totals):
    """The lowest mask with the least total."""
    return min(range(8), key=lambda mask: (totals[mask], mask))


def symbol(data):
    candidates = []
    for mask in range(8):
        matrix = candidate_matrix(data, mask)
        word = check_format(matrix, mask)
        candidates.append({"mask": mask, "word": word, "matrix": matrix, **scores(matrix)})
    spec = choose([c["n1"] + c["n2"] + c["n3"] + c["n4"] for c in candidates])
    by_qrcode = choose([c["n1"] + c["n2"] + c["n3_qrcode"] + c["n4"] for c in candidates])
    by_segno = choose([c["n1"] + c["n2"] + c["n3_segno"] + c["n4"] for c in candidates])
    # What python-qrcode really chooses. It scores each candidate before its format information and dark module are
    # written, and the finished ones above are the same modules but for those. Scoring them unfinished must give its choice.
    own = qrcode.QRCode(version=1, error_correction=qrcode.constants.ERROR_CORRECT_L, box_size=1, border=0)
    own.add_data(qrcode.util.QRData(data, mode=qrcode.util.MODE_8BIT_BYTE))
    own.make(fit=False)
    # The library does not say which mask it used, so it is read from the format information of the symbol it made.
    chosen = mask_of([[int(bool(cell)) for cell in row] for row in own.modules])
    unfinished = [qrcode.util.lost_point(unformatted(c["matrix"])) for c in candidates]
    assert chosen == choose(unfinished), "python-qrcode does not choose by scoring the unfinished candidates"
    return {"input": data, "candidates": candidates, "spec": spec, "qrcode": by_qrcode, "chosen": chosen, "segno": by_segno}


def rust_bytes(values):
    return ", ".join(f"0x{v:02X}" for v in values)


def emit(item):
    print("    SymbolGolden {")
    print(f"        input: &[{rust_bytes(item['input'])}],")
    print("        candidates: [")
    for c in item["candidates"]:
        print("            CandidateGolden {")
        print(f"                mask: {c['mask']},")
        print(f"                format_bits: 0b{c['word']:015b},")
        print("                rows: [")
        for row in c["matrix"]:
            print(f"                    \"{''.join(map(str, row))}\",")
        print("                ],")
        for name in ("n1", "n2", "n3", "n3_qrcode", "n3_segno", "n4"):
            print(f"                {name}: {c[name]},")
        print("            },")
    print("        ],")
    print(f"        mask: {item['spec']},")
    print(f"        mask_qrcode: {item['qrcode']},")
    print(f"        mask_qrcode_chosen: {item['chosen']},")
    print(f"        mask_segno: {item['segno']},")
    print("    },")


def differing(count):
    """Random inputs of every length where the three conventions do not all choose the same mask."""
    rng = random.Random(20261029)
    found = []
    for _ in range(SEARCH):
        data = bytes(rng.randrange(32, 127) for _ in range(rng.randrange(0, 18)))
        if data in INPUTS or any(data == f["input"] for f in found):
            continue
        item = symbol(data)
        if len({item["spec"], item["qrcode"], item["chosen"], item["segno"]}) > 1:
            found.append(item)
            if len(found) == count:
                break
    return found


items = [symbol(data) for data in INPUTS]
extra = differing(KEEP)
items += extra

print("//! Golden vectors for the symbol: the eight candidate matrices python-qrcode makes for each input, with the format")
print("//! information and the penalty scores of each. Generated by `generate_symbol_golden.py`; do not edit by hand.")
print("//!")
print(f"//! Matrices: qrcode {importlib.metadata.version('qrcode')}, `version=1`, `ERROR_CORRECT_L`, byte mode, `mask_pattern` 0 to 7.")
print(f"//! Scores: python-qrcode's and segno {segno.__version__}'s own, on the same matrices. N1, N2 and N4 agree in all three.")
print("//! N3 differs between the conventions, so `n3` is the design's, and `n3_qrcode` and `n3_segno` are the libraries'.")
print("//! The doc comment of the script says how each counts. `mask` is the one the design's scores choose, `mask_qrcode`")
print("//! the one python-qrcode's scores choose on finished candidates, `mask_qrcode_chosen` the one python-qrcode really")
print("//! chooses, which scores unfinished ones, and `mask_segno` the one segno's scores choose.")
print(f"//! The last {len(extra)} inputs were found among {SEARCH} random ones because the libraries and the design do not all choose one mask.")
print()
print("/// One candidate: the matrix with this mask and its own format information, and its scores.")
print("pub(super) struct CandidateGolden {")
print("    pub(super) mask: u8,")
print("    pub(super) format_bits: u16,")
print(f"    pub(super) rows: [&'static str; {SIZE}],")
for name in ("n1", "n2", "n3", "n3_qrcode", "n3_segno", "n4"):
    print(f"    pub(super) {name}: u32,")
print("}")
print()
print("/// One input, with its eight candidates and the mask each convention chooses.")
print("pub(super) struct SymbolGolden {")
print("    pub(super) input: &'static [u8],")
print("    pub(super) candidates: [CandidateGolden; 8],")
print("    pub(super) mask: u8,")
print("    pub(super) mask_qrcode: u8,")
print("    pub(super) mask_qrcode_chosen: u8,")
print("    pub(super) mask_segno: u8,")
print("}")
print()
print("pub(super) const SYMBOL_GOLDEN: &[SymbolGolden] = &[")
for item in items:
    emit(item)
print("];")
