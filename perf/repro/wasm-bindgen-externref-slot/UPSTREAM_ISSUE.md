# Draft issue for wasm-bindgen/wasm-bindgen (not filed)

**Title:**<br>A `#[wasm_bindgen(catch)]` import that returns an object leaks one externref slot per exception

**Versions:**<br>`wasm-bindgen 0.2.129` and `0.2.130`, target `wasm32-unknown-unknown`, wasm-pack test --node.

**What happens.**<br>An import declared `#[wasm_bindgen(catch)] fn f() -> Result<JsValue, JsValue>` that throws leaves one entry live in the externref table for each throw, as reported by `wasm_bindgen::externref_heap_live_count()`.
The caller drops the returned `Err` immediately.
An import that returns nothing or a number leaves no slot when it throws, and an import that returns an object leaves none when it does not throw.

**Reproduction.**<br>Attached crate (`Cargo.toml`, `src/lib.rs`, `helpers.js`, `tests/externref_slot.rs`); `wasm-pack test --node`:

    1000 slots were left live by 1000 failures of an import that returns an object

with the three controls (returns nothing + throws, returns a number + throws, returns an object + does not throw) leaving 0.

**Expected.**<br>No slot left live after the `Err` is dropped.

**Where it matters.**<br>The same happens with web-sys: `Document::create_element_ns`, `Node::append_child` and `SvgGraphicsElement::get_b_box`, when the browser throws, each leave one slot.
`setAttribute` and `addEventListener`, which return `()`, leave none.
In a long-running application that sees DOM errors the externref table only grows.
Rust-side allocation accounting stays flat, so the growth is invisible to it.

**Related.**<br>#5404 / #5406 (fixed in 0.2.130) cover an exported function whose later argument is rejected.
This is the import-return side and was still present in 0.2.130.

**Not yet established.**<br>Whether the slot is allocated for the return value before the exception is observed.
I have not read the generated code far enough to say, so I have not proposed a fix.
