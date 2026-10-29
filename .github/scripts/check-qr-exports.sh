#!/usr/bin/env bash
# Checks that the QR code demo's exported SVG files are scannable. Run from anywhere as `check-qr-exports.sh`.
#
# It does both halves in one command, so that a run is one result and not two steps that can be run apart:
#
# 1. The ignored test `write_exports_for_the_decoder_check` writes an export of each of a list of texts, and a manifest.
# 2. `decode_exports.py --strict` draws each file from its own rectangles at a whole number of pixels for each module,
#    decodes the drawing with ZXing-C++ (and with pyzbar, where it is installed), and compares the picture with the one
#    python-qrcode makes of the same text and mask.
#
# `--strict` makes a missing python-qrcode, or a manifest without the empty text, the longest text and one byte under it,
# a failure and not a skip. Pillow, zxing-cpp and python-qrcode must be installed for the Python in use, which is
# `python3`, or the one named by `PYTHON`. See the doc comment of decode_exports.py.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
python="${PYTHON:-python3}"

exports="$(mktemp -d)"
trap 'rm -rf "$exports"' EXIT

QR_EXPORT_DIR="$exports" cargo test -p svg-dom-graph-demo write_exports_for_the_decoder_check -- --ignored
"$python" demo-app/src/qr_code/export/decode_exports.py --strict "$exports"
