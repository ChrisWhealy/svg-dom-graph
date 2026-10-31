# Demo App Tests

```sh
cargo test -p svg-dom-graph-demo
cargo test -p demo-server
```

Neither of the above crates are default workspace members (see `Cargo.toml`'s own comment), so these must be tested explicitly with `-p <package-name>`.

The first runs `demo-app`'s native unit tests.
These are the syntax highlighter in `highlight/unit_tests.rs`, and `unit_tests.rs`, whose `every_registered_demo_has_extractable_source` guards against a `demo_gallery!` entry whose source text `demo_fn_source` can no longer locate.
Each hash demo's own panel also has a test that it shows the calculation and the walk behind its one function.

`complex_numbers/argand/unit_tests.rs` checks the arithmetic of the Argand diagram, which needs no browser:

* how the plot node is configured: a complex plane, a square plotted area, arrowheads, and the tick step
* what is drawn for a number: the legs, the modulus, the arc, the dot and each label, in order
* the arc, which is a line through points that stays on its circle and runs the right way
* how a number is written, without trailing zeros or a negative zero
* where a label goes beside a point or along a line, and that it is never upside down

`complex_numbers/sliders/unit_tests.rs` checks the arithmetic of a slider:

* a slider is one thumb longer than the axis it lies along, so that its thumb's centre covers the axis
* the vertical slider is turned so that its minimum is at the bottom
* where a thumb sits for a value
* the rounding of a value to a step, never to a negative zero

The explorer and roots tests check these properties:

* a slider sets only its own part
* what the sliders reach
* there are `n` roots of unity for every `n` the slider gives
* the stage is the same size for all of them

The hash demos are the bulk of these tests, because each keeps its calculation and its walk in plain Rust that needs no browser:

* `sha1/algorithm/` and `sha2_256/algorithm/` check the standard's constants.
  They also check the digests of several messages against published values.
  The messages include the empty message, `"abc"`, the demo's own message, and the longest message that fits one block.
  SHA-2's constants are also recomputed independently from the primes, with exact integer arithmetic.
* `sha1/walk/` and `sha2_256/walk/` check that each position of the walk maps onto the right step.
  They also check what every node shows at that position.
  Values appear at their own step and stay.
  Cells that are not reached yet stay hidden, and the last two steps show the digest and hash.
  SHA-1's walk also checks that the diagram changes stage only at the boundary of a section of 20 rounds.
* `sha3_sponge/` checks each step of `Keccak-f` against a reference written from the specification, all 24 rounds, and the published SHA3-256 digests.

The QR code demo keeps its encoder, its symbol, its walk and its export in plain Rust, so these need no browser either:

* `qr_code/qr_core/` checks the encoder against golden vectors from python-qrcode, and against independent implementations written in the tests.
  The finite field's tables are checked for every pair of values against a loop that shares no code with them.
  The Reed-Solomon generator is checked against the standard's own table, and every encoding must have a zero syndrome at all seven roots.
  The eight candidate matrices of eleven inputs, their format words and their scores are compared with python-qrcode, and the scores with segno's too.
  The N3 conventions of the three differ, and the golden file records all three.
* `qr_code/snapshots.rs` is checked by `snapshot_tests.rs`: 62 snapshots for every text, what each shows, which cells are not yet reached, and what is marked.
* `qr_code/export/` checks the file: 29 by 29 modules in whole pixels, a quiet zone with no dark module, pure black on white, and the symbol read back from the path.
  Text that looks like markup stays character data, and a control character is written as a visible escape.
  An ignored test writes exports for `decode_exports.py`, which decodes them with ZXing-C++ and compares them with python-qrcode.
  `.github/scripts/check-qr-exports.sh` runs both together with `--strict`, and the `qr-export-decoders` job of the CI workflow runs that on every push.
  A plain `cargo test` does not establish that the exports scan, since that needs Python and the decoders.
* `qr_code/unit_tests.rs` checks that every text fits the room for it, for the longest inputs too, and the grid and the layout.

The second runs `demo-server`'s own `panels`/`validate`/`build` unit tests.
They include end-to-end checks that the `assemble` and `validate` steps both succeed against this project's real `demo/` directory and `demo-app/src/lib.rs`, not just synthetic fixtures.
