#!/usr/bin/env python3
"""Decodes the exported SVG files of the QR code demo with independent readers, and checks what they say.

Run it in a virtualenv that has Pillow and zxing-cpp installed, and pyzbar too if the system has libzbar:

    QR_EXPORT_DIR=/some/dir cargo test -p svg-dom-graph-demo write_exports -- --ignored
    python3 demo-app/src/qr_code/export/decode_exports.py /some/dir

The test writes one export for each of a list of texts, and a manifest.json that says which text each holds. This script
then does, for each file:

1. It reads the file as XML, so a file that is not well formed fails here.
2. It checks the picture: the quiet zone of four modules is light on every side, the picture is a whole number of pixels
   for each module, and the only colours are black and white.
3. It draws the picture, from the rectangles in the file and nothing else, at a whole number of pixels for each module.
4. It decodes the drawing with ZXing-C++, and with pyzbar if libzbar is installed, and checks that the decoded bytes are
   the text, which is the only thing a reader of the symbol ever sees.
5. It reads the mask from the format information in the picture, makes the symbol of the same text and mask with
   python-qrcode, and checks that the two pictures are the same modules. This needs python-qrcode.

ZXing-C++ reports no symbol at all for a QR code whose content is empty, even python-qrcode's own at every mask. So for
an empty text the decoders are expected to find nothing, which is checked on python-qrcode's symbol too, and it is the
comparison in step 5 that shows that the symbol is right.

The file is drawn here, not rendered by a browser or a graphics library, so that what is checked is the file's own
geometry. The reader of it shares nothing with the encoder that made the symbol.

pyzbar needs the libzbar system library, and is skipped, with a message, where that is missing. The exit status is 1 if
any check fails, and 0 if all that ran passed.

With `--strict`, which the CI job passes, a check that cannot run is a failure instead of a skip, so that the result is
the same wherever it is run: python-qrcode must be installed, so that every picture is compared, and the manifest must
cover the cases the acceptance gate names, which are the empty text, one byte under the longest, and the longest. pyzbar
stays optional as a second decoder, whether or not the option is given. `.github/scripts/check-qr-exports.sh` makes the
exports and runs this with `--strict` in one command.
"""
import importlib.metadata
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

from PIL import Image, ImageDraw

try:
    import zxingcpp
except ImportError:
    sys.exit("zxing-cpp is not installed, so there is nothing to decode with")

try:
    import qrcode
    import qrcode.constants
    import qrcode.util
except ImportError:
    qrcode = None

try:
    from pyzbar import pyzbar
except (ImportError, OSError) as problem:
    pyzbar = None
    PYZBAR_SKIPPED = f"{type(problem).__name__}: {str(problem)[:80]}"

SVG = "{http://www.w3.org/2000/svg}"
QUIET_ZONE = 4
PIXELS = 8  # the scale each module is drawn at, whatever the file's own size


def read_modules(path):
    """The picture of an export as a square of booleans, true for dark, from the file's own rectangles."""
    root = ET.parse(path).getroot()  # raises if the file is not well formed
    assert root.tag == SVG + "svg", f"the root is {root.tag}"
    left, top, width, height = (int(v) for v in root.get("viewBox").split())
    assert (left, top) == (0, 0) and width == height, f"the viewBox is {root.get('viewBox')}"
    size_px = int(root.get("width"))
    assert size_px == int(root.get("height")) and size_px % width == 0, "a module is not a whole number of pixels"
    assert root.get("shape-rendering") == "crispEdges", "the edges are antialiased"

    fills = {element.get("fill") for element in root if element.tag in (SVG + "rect", SVG + "path")}
    assert fills == {"#ffffff", "#000000"}, f"the colours are {fills}"
    background = root.find(SVG + "rect")
    assert background.get("fill") == "#ffffff" and int(background.get("width")) == width, "the background is not white"

    modules = [[False] * width for _ in range(height)]
    for part in root.find(SVG + "path").get("d").split("z"):
        part = part.strip()
        if not part:
            continue
        corner, rest = part.split("h", 1)
        x, y = (int(v) for v in corner.removeprefix("M").split())
        run, rest = rest.split("v", 1)
        assert rest == f"1h-{run}", f"unexpected rectangle {part!r}"
        for col in range(x, x + int(run)):
            assert not modules[y][col], "two rectangles cover the same module"
            modules[y][col] = True

    for index in range(width):
        for edge in range(QUIET_ZONE):
            assert not (modules[edge][index] or modules[height - 1 - edge][index]), "the quiet zone is dark"
            assert not (modules[index][edge] or modules[index][width - 1 - edge]), "the quiet zone is dark"
    return modules


def draw(modules):
    """The picture drawn in black on white, with a whole number of pixels for each module."""
    size = len(modules)
    image = Image.new("L", (size * PIXELS, size * PIXELS), 255)
    pen = ImageDraw.Draw(image)
    for row, line in enumerate(modules):
        for col, dark in enumerate(line):
            if dark:
                pen.rectangle([col * PIXELS, row * PIXELS, (col + 1) * PIXELS - 1, (row + 1) * PIXELS - 1], fill=0)
    return image


def decode_with_zxing(image):
    found = zxingcpp.read_barcodes(image)
    qr = [result for result in found if result.format == zxingcpp.BarcodeFormat.QRCode]
    assert len(qr) <= 1, f"ZXing found {len(qr)} QR codes"
    return qr[0].bytes if qr else None


def decode_with_pyzbar(image):
    found = [symbol for symbol in pyzbar.decode(image) if symbol.type == "QRCODE"]
    assert len(found) <= 1, f"pyzbar found {len(found)} QR codes"
    return found[0].data if found else None


def format_word(mask):
    """The 15 format bits for level L and `mask`: BCH remainder with 0x537, then XOR with 0x5412."""
    data = (0b01 << 3) | mask
    remainder = data << 10
    for shift in range(14, 9, -1):
        if remainder >> shift & 1:
            remainder ^= 0x537 << (shift - 10)
    return ((data << 10) | remainder) ^ 0x5412


def mask_of(modules):
    """The mask that the picture says it uses, read from the first copy of the format information."""
    places = [(i, 8) for i in range(6)] + [(7, 8), (8, 8), (8, 7)] + [(8, c) for c in range(5, -1, -1)]
    word = sum(modules[QUIET_ZONE + r][QUIET_ZONE + c] << i for i, (r, c) in enumerate(places))
    masks = [mask for mask in range(8) if format_word(mask) == word]
    assert len(masks) == 1, "the format information is not that of any mask"
    return masks[0]


def oracle_modules(text, mask):
    """The picture python-qrcode makes of `text` at `mask`, with the quiet zone, in the same form as read_modules."""
    symbol = qrcode.QRCode(version=1, error_correction=qrcode.constants.ERROR_CORRECT_L, box_size=1, border=0, mask_pattern=mask)
    symbol.add_data(qrcode.util.QRData(text, mode=qrcode.util.MODE_8BIT_BYTE))
    symbol.make(fit=False)
    size = len(symbol.modules) + 2 * QUIET_ZONE
    modules = [[False] * size for _ in range(size)]
    for row, line in enumerate(symbol.modules):
        for col, dark in enumerate(line):
            modules[row + QUIET_ZONE][col + QUIET_ZONE] = bool(dark)
    return modules


def check_coverage(manifest):
    """Why the manifest does not cover what the acceptance gate needs, or an empty list if it does."""
    lengths = {len(entry["text"].encode("ascii")) for entry in manifest}
    needed = {0: "the empty text", 16: "one byte under the longest text", 17: "the longest text"}
    missing = [what for length, what in needed.items() if length not in lengths]
    problems = [f"the manifest has no case for {what}" for what in missing]
    if len(manifest) < 10:
        problems.append(f"the manifest has {len(manifest)} cases, and at least 10 are expected")
    return problems


def main(directory, strict=False):
    directory = Path(directory)
    manifest = json.loads((directory / "manifest.json").read_text())
    if strict:
        problems = check_coverage(manifest)
        if qrcode is None:
            problems.append("python-qrcode is not installed, so the pictures cannot be compared")
        if problems:
            print("\n".join(f"STRICT: {problem}" for problem in problems))
            return 1
    print(f"zxing-cpp {importlib.metadata.version('zxing-cpp')}", end="")
    print(f", pyzbar {importlib.metadata.version('pyzbar')}" if pyzbar else f", pyzbar skipped ({PYZBAR_SKIPPED})", end="")
    print(", python-qrcode compared" if qrcode else ", python-qrcode missing, so the pictures are not compared")
    failures = 0
    for entry in manifest:
        expected = entry["text"].encode("ascii")
        outcome = []
        try:
            modules = read_modules(directory / entry["file"])
            image = draw(modules)
            readers = {"ZXing": decode_with_zxing}
            if pyzbar:
                readers["pyzbar"] = decode_with_pyzbar
            for name, reader in readers.items():
                got = reader(image)
                if expected:
                    ok = got == expected
                    outcome.append(f"{name} {'ok' if ok else 'WRONG ' + repr(got)}")
                else:
                    # No reader reports an empty QR code, so none is expected to, and none may report something else.
                    ok = got in (None, b"")
                    outcome.append(f"{name} {'reports no symbol, as for every empty QR code' if ok else 'WRONG ' + repr(got)}")
                failures += not ok
            if qrcode:
                mask = mask_of(modules)
                same = modules == oracle_modules(expected, mask)
                outcome.append(f"mask {mask} {'same as python-qrcode' if same else 'DIFFERENT from python-qrcode'}")
                failures += not same
        except (AssertionError, ET.ParseError, ValueError, KeyError) as problem:
            outcome.append(f"FAILED: {problem}")
            failures += 1
        print(f"{entry['file']}  {len(expected):2} bytes  {expected[:24]!r:30}  {'; '.join(outcome)}")
    print(f"{len(manifest)} files, {failures} failures")
    return 1 if failures else 0


if __name__ == "__main__":
    arguments = [argument for argument in sys.argv[1:] if argument != "--strict"]
    if len(arguments) != 1 or len(sys.argv) - 1 - len(arguments) > 1:
        sys.exit(__doc__)
    sys.exit(main(arguments[0], strict="--strict" in sys.argv[1:]))
