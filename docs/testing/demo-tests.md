# Demo App Tests

```sh
cargo test -p svg-dom-graph-demo
cargo test -p demo-server
```

Neither of the above crates are default workspace members (see `Cargo.toml`'s own comment), so these must be tested explicitly with `-p <package-name>`.

The first runs `demo-app`'s native unit tests.
These are the syntax highlighter in `highlight/unit_tests.rs`, and `unit_tests.rs`, whose `every_registered_demo_has_extractable_source` guards against a `demo_gallery!` entry whose source text `demo_fn_source` can no longer locate.
Each hash demo's own panel also has a test that it shows the calculation and the walk behind its one function.

`complex_numbers/argand/unit_tests.rs` checks the arithmetic of the Argand diagram, which needs no browser: where a number lands on the page and that the imaginary axis points up it, which way an arc sweeps and when it is the large one, the tick step, how a number is written without trailing zeros or a negative zero, and where a label goes beside a point or along a line, and that it is never upside down.

The hash demos are the bulk of these tests, because each keeps its calculation and its walk in plain Rust that needs no browser:

* `sha1/algorithm/` and `sha2_256/algorithm/` check the standard's constants, and check the digests of several messages against published values, including the empty message, `"abc"`, the demo's own message, and the longest message that fits one block.
  SHA-2's constants are also recomputed independently from the primes, with exact integer arithmetic.
* `sha1/walk/` and `sha2_256/walk/` check that each position of the walk maps onto the right step, and that what every node shows at that position is right: values that appear at their own step and stay, cells that are not reached yet, and the digest and hash at the last two steps.
  SHA-1's walk also checks that the diagram changes stage only at the boundary of a section of 20 rounds.
* `sha3_sponge/` checks each step of `Keccak-f` against a reference written from the specification, all 24 rounds, and the published SHA3-256 digests.

The second runs `demo-server`'s own `panels`/`validate`/`build` unit tests, including end-to-end checks that the `assemble` and `validate` steps both succeed against this project's real `demo/` directory and `demo-app/src/lib.rs`, not just synthetic fixtures.
