//! The scene in a real browser: that it is the matrix it was built from, that its data nodes show the product after
//! every step, that a budget refuses before any scene call, that a teardown leaves the page as it was, and the costs
//! that `docs/security.md` documents. Run with `wasm-pack test --headless --chrome spmv-scene`.
#![cfg(target_arch = "wasm32")]

use spmv_core::budget::Budget;
use spmv_core::matrix::label;
use spmv_core::spec::{Error, Spec};
use spmv_core::{Prepared, Run};
use spmv_scene::layout::Layout;
use spmv_scene::probe;
use spmv_scene::scene::{BuildOptions, Host, Selection, element_count, now};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check(ok: bool, what: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(what.to_owned()) }
}

fn spec(rows: u64, cols: u64, per_row: u64, hubs: (u64, u64), label_len: u64, cells: u64, motion: (u64, u64)) -> Spec {
    Spec {
        seed: 17,
        rows,
        cols,
        per_row,
        hub_cols: hubs.0,
        hub_percent: hubs.1,
        label_len,
        cells,
        updates: motion.0,
        steps: motion.1,
    }
}

fn prepared(s: &Spec) -> Result<Prepared, String> {
    Prepared::new(s, &Budget::default()).map_err(|e| e.to_string())
}

/// The `n`th group of the scene's content group: the column nodes, then the row nodes, then the two data nodes, then
/// the square.
fn group(host: &Host, n: u32) -> Result<web_sys::Element, String> {
    let found = host
        .svg()?
        .query_selector_all(":scope > g.svg-dom-graph-content > g")
        .map_err(|e| format!("{e:?}"))?;
    found
        .get(n)
        .and_then(|g| g.dyn_into().ok())
        .ok_or_else(|| format!("no group {n}"))
}

fn texts_of(e: &web_sys::Element) -> Result<Vec<String>, String> {
    let found = e.query_selector_all("text").map_err(|x| format!("{x:?}"))?;
    Ok((0..found.length())
        .filter_map(|i| found.get(i))
        .map(|t| t.text_content().unwrap_or_default().trim().to_owned())
        .collect())
}

fn want(values: &[i64]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn connector_paths(host: &Host) -> Result<Vec<String>, String> {
    let found = host
        .svg()?
        .query_selector_all(":scope > g.svg-dom-graph-content > path")
        .map_err(|e| format!("{e:?}"))?;
    Ok((0..found.length())
        .filter_map(|i| found.get(i))
        .filter_map(|e| e.dyn_into::<web_sys::Element>().ok())
        .map(|e| e.get_attribute("d").unwrap_or_default())
        .collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn readable() -> BuildOptions {
    BuildOptions {
        readable: true,
        ..BuildOptions::default()
    }
}

fn class_of(e: &web_sys::Element) -> String {
    e.get_attribute("class").unwrap_or_default()
}

fn has(e: &web_sys::Element, name: &str) -> bool {
    class_of(e).split_whitespace().any(|c| c == name)
}

fn connector_elements(host: &Host) -> Result<Vec<web_sys::Element>, String> {
    let found = host
        .svg()?
        .query_selector_all(":scope > g.svg-dom-graph-content > path")
        .map_err(|e| format!("{e:?}"))?;
    Ok((0..found.length())
        .filter_map(|i| found.get(i))
        .filter_map(|e| e.dyn_into().ok())
        .collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn screen_rect(e: &web_sys::Element) -> (f64, f64, f64, f64) {
    let r = e.get_bounding_client_rect();
    (r.left(), r.top(), r.right(), r.bottom())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn worked_options() -> BuildOptions {
    BuildOptions {
        readable: true,
        worked: true,
        ..BuildOptions::default()
    }
}

/// The text of every cell of the nodes of the scene, which is what a person reads: the numbers drawn in it.
fn numbers_in(host: &Host, selector: &str) -> Result<Vec<String>, String> {
    let found = host.svg()?.query_selector_all(selector).map_err(|e| format!("{e:?}"))?;
    Ok((0..found.length())
        .filter_map(|i| found.get(i))
        .map(|n| n.text_content().unwrap_or_default())
        .collect())
}

/// The rectangles of the nodes that the library drew, in the page.
type Rectangle = (f64, f64, f64, f64);

fn node_rects(host: &Host) -> Result<Vec<(String, Rectangle)>, String> {
    let groups = host
        .svg()?
        .query_selector_all(":scope > g.svg-dom-graph-content > g:not(.spmv-worked-notes)")
        .map_err(|e| format!("{e:?}"))?;
    Ok((0..groups.length())
        .filter_map(|i| groups.get(i))
        .filter_map(|n| n.dyn_into::<web_sys::Element>().ok())
        .map(|e| (e.get_attribute("class").unwrap_or_default(), screen_rect(&e)))
        .collect())
}

mod unit_tests;
