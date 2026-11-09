//! The scene in a real browser: that it is the matrix it was built from, that its data nodes show the product after every step, that
//! a budget refuses before any scene call, that a teardown leaves the page as it was, and the costs that `docs/security.md` documents.
//! Run with `wasm-pack test --headless --chrome spmv-scene`.
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

/// The `n`th group of the scene's content group: the column nodes, then the row nodes, then the two data nodes, then the square.
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
/// The scene has one node for each column and row, one connector for each nonzero, the two data nodes and the square, every label is
/// the one the contract gives, and no id is used twice.
#[wasm_bindgen_test]
fn the_scene_is_the_matrix_it_was_built_from() -> Result<(), String> {
    for (n, s) in [
        spec(40, 30, 3, (4, 60), 6, 8, (3, 2)),
        spec(25, 25, 0, (0, 0), 3, 5, (1, 1)),
        spec(1, 1, 1, (0, 0), 1, 1, (1, 1)),
    ]
    .iter()
    .enumerate()
    {
        let p = prepared(s)?;
        let run = p.run();
        let (host, built, _) = probe::build(&format!("sm-shape-{n}"), &p, &run)?;
        let svg = host.svg()?;
        let (rows, cols, nnz) = (s.rows as usize, s.cols as usize, p.matrix().nnz());

        let groups = svg
            .query_selector_all(":scope > g.svg-dom-graph-content > g")
            .map_err(|e| format!("{e:?}"))?
            .length() as usize;
        check(
            groups == rows + cols + 3,
            &format!("case {n}: {groups} groups, and {} were meant", rows + cols + 3),
        )?;
        let edges = svg
            .query_selector_all(":scope > g.svg-dom-graph-content > path")
            .map_err(|e| format!("{e:?}"))?
            .length() as usize;
        check(edges == nnz, &format!("case {n}: {edges} connectors for {nnz} nonzeros"))?;

        for j in [0, cols / 2, cols - 1] {
            let first = texts_of(&group(&host, j as u32)?)?.into_iter().next().unwrap_or_default();
            check(
                first == label('x', j, s.label_len as usize),
                &format!("case {n}: column {j} says {first:?}"),
            )?;
        }
        for i in [0, rows / 2, rows - 1] {
            let first = texts_of(&group(&host, (cols + i) as u32)?)?
                .into_iter()
                .next()
                .unwrap_or_default();
            check(
                first == label('y', i, s.label_len as usize),
                &format!("case {n}: row {i} says {first:?}"),
            )?;
        }

        // The square of the pattern is one path with a cell for each nonzero, in the order of the rows.
        let spy = svg
            .query_selector(":scope > g.svg-dom-graph-content > g.spmv-spy path")
            .map_err(|e| format!("{e:?}"))?
            .ok_or("there is no square of the pattern")?;
        let expected: String = (0..rows)
            .flat_map(|i| p.matrix().row(i).map(move |(j, _)| format!("M{j} {i}h1v1h-1z")))
            .collect();
        check(
            spy.get_attribute("d").as_deref() == Some(expected.as_str()),
            &format!("case {n}: the square of the pattern is not one cell for each nonzero"),
        )?;

        let ids = svg.query_selector_all("[id]").map_err(|e| format!("{e:?}"))?;
        let mut all: Vec<String> = (0..ids.length())
            .filter_map(|i| ids.get(i))
            .filter_map(|e| e.dyn_into::<web_sys::Element>().ok())
            .map(|e| e.id())
            .collect();
        let count = all.len();
        all.sort();
        all.dedup();
        check(all.len() == count, &format!("case {n}: an id is used twice"))?;
        drop(built);
        host.remove();
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// After every step the two data nodes show the windows of x and y, exactly. A scene built afresh at that step shows the same cells,
/// so the cells the library rewrote are the cells a full render would have drawn.
#[wasm_bindgen_test]
fn the_data_nodes_show_the_product_after_every_step() -> Result<(), String> {
    for (n, s) in [
        spec(60, 50, 3, (0, 0), 4, 12, (4, 10)),
        spec(80, 40, 5, (3, 70), 4, 20, (8, 10)),
        spec(30, 30, 2, (0, 0), 4, 30, (30, 6)),
    ]
    .iter()
    .enumerate()
    {
        let p = prepared(s)?;
        let mut run = p.run();
        let (host, built, _) = probe::build(&format!("sm-values-{n}"), &p, &run)?;
        let (xg, yg) = ((s.rows + s.cols) as u32, (s.rows + s.cols + 1) as u32);
        let cells = s.cells as usize;
        check(
            texts_of(&group(&host, xg)?)? == want(&run.x[..cells]),
            &format!("case {n}: x at the start"),
        )?;
        check(
            texts_of(&group(&host, yg)?)? == want(&run.y[..cells]),
            &format!("case {n}: y at the start"),
        )?;

        for step in 1..=s.steps {
            run.advance(true).map_err(|e| e.to_string())?;
            built.apply(&run)?;
            let (xt, yt) = (texts_of(&group(&host, xg)?)?, texts_of(&group(&host, yg)?)?);
            check(xt == want(&run.x[..cells]), &format!("case {n}: x at step {step}"))?;
            check(yt == want(&run.y[..cells]), &format!("case {n}: y at step {step}"))?;

            // A scene built afresh from the state at this step.
            let mut fresh = Run::new(p.spec(), p.matrix().clone());
            for _ in 0..step {
                fresh.advance(false).map_err(|e| e.to_string())?;
            }
            let (host2, built2, _) = probe::build(&format!("sm-values-{n}-{step}"), &p, &fresh)?;
            check(
                texts_of(&group(&host2, xg)?)? == xt,
                &format!("case {n}: incremental and fresh x differ at step {step}"),
            )?;
            check(
                texts_of(&group(&host2, yg)?)? == yt,
                &format!("case {n}: incremental and fresh y differ at step {step}"),
            )?;
            drop(built2);
            host2.remove();
        }
        drop(built);
        host.remove();
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scenario over the budget is refused with a reason before anything is built: no scene exists to call, the empty host stays empty,
/// and a size that would take gigabytes is answered at once. A refusal is reported apart from a rendering.
#[wasm_bindgen_test]
fn a_scenario_over_the_budget_is_refused_before_any_scene_call() -> Result<(), String> {
    let host = Host::create("sm-refused", (100.0, 100.0))?;
    let before = element_count(&host.svg()?)?;
    let tight = Budget {
        max_nodes: 50,
        max_degree: 3,
        ..Budget::default()
    };

    let too_many = spec(40, 40, 2, (0, 0), 4, 5, (1, 1));
    let r = Prepared::new(&too_many, &tight);
    check(
        matches!(&r, Err(Error::Refused(x)) if x.code == "nodes"),
        &format!("nodes: {:?}", r.err()),
    )?;

    let hubby = spec(30, 20, 4, (1, 100), 4, 5, (1, 1));
    let r = Prepared::new(&hubby, &tight);
    check(
        matches!(&r, Err(Error::Refused(x)) if x.code == "degree"),
        &format!("degree: {:?}", r.err()),
    )?;

    let start = now();
    let hostile = spec(1_000_000, 1_000_000, 64, (0, 0), 4, 1000, (0, 0));
    let r = Prepared::new(&hostile, &Budget::default());
    check(
        matches!(&r, Err(Error::Refused(x)) if x.code == "nodes"),
        "a hostile size was not refused as nodes",
    )?;
    check(now() - start < 200.0, &format!("the refusal took {} ms", now() - start))?;

    check(element_count(&host.svg()?)? == before, "a refusal changed the page")?;
    check(host.svg()?.child_element_count() == 0, "the host is not empty")?;
    host.remove();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Building and tearing down again and again leaves the page with the elements it began with, every time.
#[wasm_bindgen_test]
fn teardown_returns_the_page_to_where_it_was_every_time() -> Result<(), String> {
    let p = prepared(&spec(50, 40, 3, (3, 50), 5, 10, (2, 2)))?;
    let report = probe::rebuild("sm-cycle", &p, 15)?;
    let before = report["body_elements_before"].as_u64().ok_or("no baseline")?;
    let after = report["body_elements_after"].as_array().ok_or("no list")?;
    check(after.len() == 15, "not all cycles ran")?;
    check(
        after.iter().all(|n| n.as_u64() == Some(before)),
        &format!("the page began with {before} elements and ended cycles with {after:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The documented risk of a node's degree: every edge appends a relationship clause to both ends, so the description of a node grows
/// with its degree. The hub's description is longer than any other node's, and is at least as long as its degree times a label.
#[wasm_bindgen_test]
fn the_description_of_a_node_grows_with_its_degree() -> Result<(), String> {
    let s = spec(120, 60, 4, (1, 90), 8, 6, (1, 1));
    let p = prepared(&s)?;
    let run = p.run();
    let (host, built, _) = probe::build("sm-degree", &p, &run)?;
    let degree = p.matrix().col(0).count();
    check(degree >= 50, &format!("the hub has degree {degree}, and the case needs a hub"))?;
    let label_of = |n: u32| -> Result<usize, String> {
        Ok(group(&host, n)?.get_attribute("aria-label").unwrap_or_default().len())
    };
    let hub = label_of(0)?;
    let plain = (1..s.cols as u32)
        .map(label_of)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    check(
        hub > plain,
        &format!("the hub's description is {hub} bytes, and the longest other is {plain}"),
    )?;
    check(
        hub >= degree * s.label_len as usize,
        &format!(
            "the hub's description is {hub} bytes for degree {degree} and labels of {}",
            s.label_len
        ),
    )?;
    drop(built);
    host.remove();
    Ok(())
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

/// The documented risk of a drag: a move redraws every connector on the node, and only those. Moving the hub changes exactly as many
/// connectors as its degree, and moving a node that has none changes none.
#[wasm_bindgen_test]
fn moving_a_node_redraws_exactly_its_connectors() -> Result<(), String> {
    let s = spec(80, 40, 3, (1, 80), 4, 5, (1, 1));
    let p = prepared(&s)?;
    let run = p.run();
    let (host, built, _) = probe::build("sm-drag", &p, &run)?;
    let degree = p.matrix().col(0).count();
    check(degree > 20, &format!("the hub has degree {degree}"))?;

    let before = connector_paths(&host)?;
    built.move_col(0, 30.0, 40.0)?;
    let after = connector_paths(&host)?;
    check(before.len() == after.len(), "a move changed the number of connectors")?;
    let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    check(
        changed == degree,
        &format!("{changed} connectors changed for a node of degree {degree}"),
    )?;

    if let Some(lonely) = (0..s.cols as usize).find(|j| p.matrix().col(*j).count() == 0) {
        let before = connector_paths(&host)?;
        built.move_col(lonely, 10.0, 10.0)?;
        check(
            before == connector_paths(&host)?,
            "moving a node with no edges redrew a connector",
        )?;
    }
    drop(built);
    host.remove();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The layout and the scene agree about where a node is, and the data nodes and the square do not overlap the graph or each other.
#[wasm_bindgen_test]
fn the_layout_is_where_the_scene_draws() -> Result<(), String> {
    let s = spec(70, 55, 3, (2, 40), 9, 20, (3, 1));
    let p = prepared(&s)?;
    let run = p.run();
    let (host, built, _) = probe::build("sm-layout", &p, &run)?;
    let l = Layout::new(&s);
    for (id, want) in [
        (built.col_node(0), l.col_node(0)),
        (built.col_node(54), l.col_node(54)),
        (built.row_node(0), l.row_node(0)),
        (built.row_node(69), l.row_node(69)),
    ] {
        let r = built.scene().node_rect(id).map_err(|e| e.to_string())?;
        check(
            (r.origin.x - want.x).abs() < 1e-6 && (r.origin.y - want.y).abs() < 1e-6,
            &format!(
                "a node is at ({}, {}), and the layout says ({}, {})",
                r.origin.x, r.origin.y, want.x, want.y
            ),
        )?;
    }
    let xr = built.scene().node_rect(built.x_node()).map_err(|e| e.to_string())?;
    let yr = built.scene().node_rect(built.y_node()).map_err(|e| e.to_string())?;
    check(
        xr.origin.y + xr.size.height <= l.header_height,
        &format!(
            "the x data node is {} by {} at ({}, {}), and the graph begins at {}",
            xr.size.width, xr.size.height, xr.origin.x, xr.origin.y, l.header_height
        ),
    )?;
    check(
        yr.origin.y + yr.size.height <= l.header_height,
        "the y data node reaches into the graph",
    )?;
    check(xr.origin.x + xr.size.width <= yr.origin.x, "the data nodes overlap")?;
    check(
        yr.origin.x + yr.size.width <= l.spy_origin.0,
        "the y data node reaches the square",
    )?;
    drop(built);
    host.remove();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The greatest figures the product can reach, written into every cell, stay inside the data node's frame: a cell keeps the width it
/// was drawn with, so a wider figure would show as text outside the frame.
#[wasm_bindgen_test]
fn the_greatest_figures_stay_inside_their_data_node() -> Result<(), String> {
    for (n, cells) in [(0, 20u64), (1, 100)] {
        let s = spec(150, 150, 2, (0, 0), 4, cells, (1, 1));
        let p = prepared(&s)?;
        let mut run = p.run();
        let (host, built, _) = probe::build(&format!("sm-extreme-{n}"), &p, &run)?;
        for v in run.y.iter_mut().take(cells as usize) {
            *v = -57_024;
        }
        for v in run.x.iter_mut().take(cells as usize) {
            *v = -99;
        }
        built.apply(&run)?;
        for g in [(s.rows + s.cols) as u32, (s.rows + s.cols + 1) as u32] {
            let node = group(&host, g)?;
            let frame = node
                .query_selector("rect")
                .map_err(|e| format!("{e:?}"))?
                .ok_or("a data node has no frame")?;
            let (outer, inner) = (node.get_bounding_client_rect(), frame.get_bounding_client_rect());
            check(
                inner.width() >= 0.9 * outer.width() && inner.height() >= 0.9 * outer.height(),
                &format!(
                    "the first rect of a data node is {} by {} in a node of {} by {}, so it is not the frame",
                    inner.width(),
                    inner.height(),
                    outer.width(),
                    outer.height()
                ),
            )?;
            check(
                outer.left() >= inner.left() - 0.5
                    && outer.right() <= inner.right() + 0.5
                    && outer.top() >= inner.top() - 0.5
                    && outer.bottom() <= inner.bottom() + 0.5,
                &format!("{cells} cells: the content of a data node reaches outside its frame"),
            )?;
        }
        drop(built);
        host.remove();
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The budget's model of the page never falls below the page: a budget that underestimated would let through a scene bigger than it
/// said. It also is not far above it, since an estimate that refused a third of what fits would be a poor budget. The model was fitted
/// to measured pages, and this keeps it honest for the shapes the tests build.
#[wasm_bindgen_test]
fn the_budgets_model_of_the_page_never_falls_below_the_page() -> Result<(), String> {
    let budget = Budget::default();
    for (n, s) in [
        spec(400, 400, 3, (0, 0), 6, 50, (5, 1)),
        spec(400, 400, 3, (10, 70), 6, 50, (5, 1)),
        spec(300, 200, 6, (2, 90), 40, 100, (5, 1)),
        spec(600, 600, 2, (0, 0), 4, 500, (5, 1)),
        spec(1200, 40, 1, (1, 100), 12, 10, (2, 1)),
    ]
    .iter()
    .enumerate()
    {
        let p = prepared(s)?;
        let run = p.run();
        let (host, built, report) = probe::build(&format!("sm-estimate-{n}"), &p, &run)?;
        let actual = report["dom_elements"].as_u64().ok_or("no count")?;
        let estimate = budget.estimate(p.plan().nodes, p.plan().edges, p.plan().cells);
        check(
            estimate >= actual,
            &format!("case {n}: the model says {estimate} elements and the page has {actual}"),
        )?;
        check(
            estimate <= actual + actual / 3,
            &format!(
                "case {n}: the model says {estimate} elements for a page of {actual}, which is over a third too many"
            ),
        )?;
        drop(built);
        host.remove();
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

/// A readable scene gives every connector the sign and the size of its value, in the order of the nonzeros by row, and every node the
/// tag that an event is traced back to a node by.
#[wasm_bindgen_test]
fn a_readable_scene_signs_every_edge_and_tags_every_node() -> Result<(), String> {
    let s = spec(9, 7, 3, (0, 0), 4, 5, (2, 2));
    let p = prepared(&s)?;
    let run = p.run();
    let (host, built, _) = probe::build_with("sm-readable", &p, &run, readable())?;
    check(built.is_readable(), "a scene built to be read says so")?;
    let paths = connector_elements(&host)?;
    let mut k = 0;
    for i in 0..p.matrix().rows {
        for (_, v) in p.matrix().row(i) {
            let (want_sign, other) = if v < 0 { ("spmv-neg", "spmv-pos") } else { ("spmv-pos", "spmv-neg") };
            check(
                has(&paths[k], "spmv-edge") && has(&paths[k], want_sign) && !has(&paths[k], other),
                &format!("edge {k} of value {v} has the classes {:?}", class_of(&paths[k])),
            )?;
            check(
                has(&paths[k], &format!("spmv-w{}", v.abs())),
                &format!("edge {k} has no class for the size {}", v.abs()),
            )?;
            check(
                has(&paths[k], "svg-dom-graph-connector") || !class_of(&paths[k]).is_empty(),
                "the library's own class was lost",
            )?;
            k += 1;
        }
    }
    check(k == paths.len(), "the edges and the nonzeros differ in number")?;
    for (n, tag) in (0..s.cols)
        .map(|j| format!("x{j}"))
        .chain((0..s.rows).map(|i| format!("y{i}")))
        .enumerate()
    {
        check(
            group(&host, n as u32)?.get_attribute("data-spmv-node").as_deref() == Some(&tag),
            &format!("node {n} is not tagged {tag}"),
        )?;
        check(
            Selection::parse(&tag).map(Selection::tag).as_deref() == Some(&tag),
            &format!("{tag} does not round-trip"),
        )?;
    }
    check(has(&host.svg()?, "spmv-scene"), "the scene is not marked")?;
    check(
        Selection::parse("z3").is_none() && Selection::parse("x").is_none() && Selection::parse("").is_none(),
        "a bad tag was accepted",
    )?;
    drop(built);
    host.remove();
    Ok(())
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

/// Selecting a node lights exactly its edges and the nodes at their other ends, and puts out what the last selection lit. Selecting
/// nothing puts everything out. A scene that was not built to be read refuses.
#[wasm_bindgen_test]
fn selecting_a_node_lights_exactly_its_edges_and_neighbours() -> Result<(), String> {
    let s = spec(30, 20, 3, (3, 60), 4, 6, (2, 2));
    let p = prepared(&s)?;
    let m = p.matrix();
    let run = p.run();
    let (host, mut built, _) = probe::build_with("sm-select", &p, &run, readable())?;
    let lit_paths = |h: &Host| -> Result<Vec<usize>, String> {
        Ok(connector_elements(h)?
            .iter()
            .enumerate()
            .filter(|(_, e)| has(e, "spmv-on"))
            .map(|(k, _)| k)
            .collect())
    };
    let lit_nodes = |h: &Host| -> Result<Vec<usize>, String> {
        let mut out = Vec::new();
        for n in 0..(s.cols + s.rows) as u32 {
            if has(&group(h, n)?, "spmv-on") {
                out.push(n as usize);
            }
        }
        Ok(out)
    };
    let picked = |h: &Host| -> Result<Vec<usize>, String> {
        let mut out = Vec::new();
        for n in 0..(s.cols + s.rows) as u32 {
            if has(&group(h, n)?, "spmv-picked") {
                out.push(n as usize);
            }
        }
        Ok(out)
    };

    let row = (0..m.rows).find(|i| m.row(*i).count() >= 2).ok_or("no row has two nonzeros")?;
    built.select(Some(Selection::Row(row)), m)?;
    let want: Vec<usize> = (m.row_ptr[row]..m.row_ptr[row + 1]).collect();
    check(
        lit_paths(&host)? == want,
        &format!("row {row}: the edges lit are {:?}, and {want:?} were meant", lit_paths(&host)?),
    )?;
    let mut nodes: Vec<usize> = m.row(row).map(|(j, _)| j).collect();
    nodes.sort_unstable();
    check(
        lit_nodes(&host)? == nodes,
        "row: the neighbours lit are not the columns of the row",
    )?;
    check(
        picked(&host)? == vec![s.cols as usize + row],
        "row: the picked node is not the row",
    )?;

    // A column, which replaces the row.
    let col = (0..m.cols).max_by_key(|j| m.col(*j).count()).ok_or("no column")?;
    built.select(Some(Selection::Column(col)), m)?;
    let mut want: Vec<usize> = m.col(col).map(|(i, _)| m.edge_index(i, col).expect("a nonzero")).collect();
    want.sort_unstable();
    check(
        lit_paths(&host)? == want,
        &format!("column {col}: the edges lit are {:?}", lit_paths(&host)?),
    )?;
    check(
        lit_paths(&host)?.len() == m.col(col).count(),
        "column: not one edge for each nonzero of the column",
    )?;
    let mut rows: Vec<usize> = m.col(col).map(|(i, _)| s.cols as usize + i).collect();
    rows.sort_unstable();
    check(lit_nodes(&host)? == rows, "column: the neighbours lit are not its rows")?;
    check(picked(&host)? == vec![col], "column: the picked node is not the column")?;

    check(has(&host.svg()?, "spmv-has-pick"), "a selection did not mark the scene")?;
    built.select(None, m)?;
    check(!has(&host.svg()?, "spmv-has-pick"), "selecting nothing left the scene marked")?;
    check(
        lit_paths(&host)?.is_empty() && lit_nodes(&host)?.is_empty() && picked(&host)?.is_empty(),
        "selecting nothing left something lit",
    )?;

    built.set_focus(true)?;
    check(has(&host.svg()?, "spmv-focus"), "focus mode was not turned on")?;
    built.set_focus(false)?;
    check(!has(&host.svg()?, "spmv-focus"), "focus mode was not turned off")?;
    drop(built);
    host.remove();

    let (host2, mut plain, _) = probe::build("sm-select-plain", &p, &run)?;
    check(
        plain.select(Some(Selection::Row(0)), m).is_err(),
        "a scene that was not built to be read accepted a selection",
    )?;
    drop(plain);
    host2.remove();
    Ok(())
}

/// A scene built with edge labels says each edge's value on it, and a scene without them does not.
#[wasm_bindgen_test]
fn edge_labels_say_the_value_of_each_edge() -> Result<(), String> {
    let s = spec(6, 6, 2, (0, 0), 4, 4, (1, 1));
    let p = prepared(&s)?;
    let run = p.run();
    let count_texts = |h: &Host| -> Result<usize, String> {
        Ok(h.svg()?
            .query_selector_all(
                "text:not(.spmv-caption):not(.spmv-matrix-label):not(.spmv-matrix-value):not(.spmv-node-value)",
            )
            .map_err(|e| format!("{e:?}"))?
            .length() as usize)
    };
    let (plain_host, plain, _) = probe::build_with("sm-nolabel", &p, &run, BuildOptions::default())?;
    let without = count_texts(&plain_host)?;
    drop(plain);
    plain_host.remove();
    let (host, built, _) = probe::build_with(
        "sm-label",
        &p,
        &run,
        BuildOptions {
            readable: true,
            edge_labels: true,
            ..BuildOptions::default()
        },
    )?;
    let with = count_texts(&host)?;
    check(
        with == without + p.matrix().nnz(),
        &format!("{with} texts with labels and {without} without, for {} edges", p.matrix().nnz()),
    )?;
    let texts: Vec<String> = {
        let found = host
            .svg()?
            .query_selector_all(
                "text:not(.spmv-caption):not(.spmv-matrix-label):not(.spmv-matrix-value):not(.spmv-node-value)",
            )
            .map_err(|e| format!("{e:?}"))?;
        (0..found.length())
            .filter_map(|i| found.get(i))
            .map(|t| t.text_content().unwrap_or_default().trim().to_owned())
            .collect()
    };
    for i in 0..p.matrix().rows {
        for (_, v) in p.matrix().row(i) {
            check(texts.contains(&v.to_string()), &format!("no label says {v}"))?;
        }
    }
    drop(built);
    host.remove();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn screen_rect(e: &web_sys::Element) -> (f64, f64, f64, f64) {
    let r = e.get_bounding_client_rect();
    (r.left(), r.top(), r.right(), r.bottom())
}

/// The three boxes of the window say what they are. Each caption says its words, is under its own box, and overlaps no node, no data
/// node, no square and no other caption, for a graph that is read and for one that is packed into blocks.
#[wasm_bindgen_test]
fn the_captions_explain_the_boxes_and_overlap_nothing() -> Result<(), String> {
    for (n, s) in [
        spec(8, 8, 3, (0, 0), 1, 8, (1, 1)),
        spec(120, 100, 3, (0, 0), 4, 10, (1, 1)),
        spec(40, 40, 3, (0, 0), 1, 20, (1, 1)),
    ]
    .iter()
    .enumerate()
    {
        let p = prepared(s)?;
        let run = p.run();
        let (host, built, _) = probe::build_with(&format!("sm-caption-{n}"), &p, &run, readable())?;
        let svg = host.svg()?;
        let texts = |class: &str| -> Result<Vec<web_sys::Element>, String> {
            let found = svg
                .query_selector_all(&format!(".spmv-caption-{class}"))
                .map_err(|e| format!("{e:?}"))?;
            Ok((0..found.length())
                .filter_map(|i| found.get(i))
                .filter_map(|e| e.dyn_into().ok())
                .collect())
        };
        let said = |class: &str| -> Result<String, String> {
            Ok(texts(class)?
                .iter()
                .map(|t| t.text_content().unwrap_or_default())
                .collect::<Vec<_>>()
                .join(" "))
        };
        let (x, y, pattern) = (said("x")?, said("y")?, said("pattern")?);
        check(
            x.contains("input vector") && x.contains("entries"),
            &format!("case {n}: the x caption says {x:?}"),
        )?;
        check(
            y.contains("output") && y.contains("entries"),
            &format!("case {n}: the y caption says {y:?}"),
        )?;
        check(
            (pattern.contains("nonzero") && pattern.contains("row")) || pattern.contains("A: the matrix"),
            &format!("case {n}: the square's caption says {pattern:?}"),
        )?;

        // Each caption's box: the union of its lines.
        let union = |class: &str| -> Result<(f64, f64, f64, f64), String> {
            let rects: Vec<_> = texts(class)?.iter().map(screen_rect).collect();
            Ok(rects.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |a, r| {
                (a.0.min(r.0), a.1.min(r.1), a.2.max(r.2), a.3.max(r.3))
            }))
        };
        let captions = [("x", union("x")?), ("y", union("y")?), ("pattern", union("pattern")?)];
        let spy = svg
            .query_selector("g.spmv-aside")
            .map_err(|e| format!("{e:?}"))?
            .ok_or("no square")?;
        let y_box = spy.previous_element_sibling().ok_or("no y data node")?;
        let x_box = y_box.previous_element_sibling().ok_or("no x data node")?;
        let own_box = [
            ("x", screen_rect(&x_box)),
            ("y", screen_rect(&y_box)),
            ("pattern", screen_rect(&spy)),
        ];
        let overlap = |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| {
            a.0 + 0.5 < b.2 && b.0 + 0.5 < a.2 && a.1 + 0.5 < b.3 && b.1 + 0.5 < a.3
        };
        for ((name, caption), (_, boxed)) in captions.iter().zip(&own_box) {
            check(
                caption.1 >= boxed.3 - 0.5,
                &format!("case {n}: the {name} caption is not under its box: {caption:?} and {boxed:?}"),
            )?;
            check(
                caption.0 >= boxed.0 - 30.0,
                &format!("case {n}: the {name} caption begins left of its box"),
            )?;
        }
        let mut others: Vec<(String, (f64, f64, f64, f64))> =
            own_box.iter().map(|(name, r)| (format!("the {name} box"), *r)).collect();
        let nodes = svg.query_selector_all("[data-spmv-node]").map_err(|e| format!("{e:?}"))?;
        for i in 0..nodes.length() {
            if let Some(e) = nodes.get(i).and_then(|e| e.dyn_into::<web_sys::Element>().ok()) {
                others.push((e.get_attribute("data-spmv-node").unwrap_or_default(), screen_rect(&e)));
            }
        }
        for (name, caption) in &captions {
            for (other, rect) in &others {
                // A caption is under its own box, which touches it, and that is not an overlap.
                check(
                    !overlap(*caption, *rect),
                    &format!("case {n}: the {name} caption overlaps {other}: {caption:?} and {rect:?}"),
                )?;
            }
        }
        // The caption in the channel, which says what the number on a line is, is above the first node and apart from everything.
        if !texts("channel")?.is_empty() {
            let channel = union("channel")?;
            check(
                said("channel")?.contains("fixed") && said("channel")?.contains("matrix A"),
                "the channel caption says that the number is a fixed value of the matrix",
            )?;
            for (other, rect) in others
                .iter()
                .chain(captions.iter().map(|(n, r)| (n.to_string(), *r)).collect::<Vec<_>>().iter())
            {
                check(
                    !overlap(channel, *rect),
                    &format!("case {n}: the channel caption overlaps {other}: {channel:?} and {rect:?}"),
                )?;
            }
        }
        for a in 0..captions.len() {
            for b in a + 1..captions.len() {
                check(
                    !overlap(captions[a].1, captions[b].1),
                    &format!("case {n}: the {} and {} captions overlap", captions[a].0, captions[b].0),
                )?;
            }
        }
        drop(built);
        host.remove();
    }
    Ok(())
}

/// A small matrix is drawn in full, with a cell for each nonzero and none for a zero, and a selection lights its cells, its band and the
/// entries of x and y that it uses; a step outlines the entries that it changed.
#[wasm_bindgen_test]
fn the_matrix_is_drawn_in_full_and_a_selection_lights_what_it_uses() -> Result<(), String> {
    let s = spec(8, 8, 3, (0, 0), 1, 8, (1, 1));
    let p = prepared(&s)?;
    let mut run = p.run();
    let (host, mut built, _) = probe::build_with("sm-matrix", &p, &run, readable())?;
    let svg = host.svg()?;
    let count = |selector: &str| -> Result<u32, String> {
        Ok(svg.query_selector_all(selector).map_err(|e| format!("{e:?}"))?.length())
    };
    check(
        count(".spmv-matrix-cell")? as usize == p.matrix().nnz(),
        "one cell for each nonzero, and none for a zero",
    )?;
    check(count(".spmv-spy")? == 0, "a matrix drawn in full has no square of its pattern")?;
    let values: Vec<String> = {
        let found = svg
            .query_selector_all(".spmv-matrix-cell .spmv-matrix-value")
            .map_err(|e| format!("{e:?}"))?;
        (0..found.length())
            .filter_map(|i| found.get(i))
            .map(|n| n.text_content().unwrap_or_default())
            .collect()
    };
    let expected: Vec<String> = (0..p.matrix().rows)
        .flat_map(|i| p.matrix().row(i).map(|(_, v)| v.to_string()).collect::<Vec<_>>())
        .collect();
    check(
        values == expected,
        &format!("the cells show the values: {values:?} against {expected:?}"),
    )?;

    built.select(Some(Selection::Row(2)), p.matrix())?;
    check(
        count(".spmv-matrix-cell.spmv-on")? as usize == p.matrix().row(2).count(),
        "a row lights the cells of its nonzeros",
    )?;
    let uses: std::collections::BTreeSet<usize> = p.matrix().row(2).map(|(j, _)| j).collect();
    check(
        count("rect.spmv-hot")? as usize == uses.len() + 1,
        "a row marks the entries of x that it uses, and its entry of y",
    )?;
    let band = svg
        .query_selector(".spmv-band")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no band")?;
    check(
        band.get_attribute("visibility").as_deref() == Some("visible"),
        "a selection shows a band",
    )?;

    check(
        count(".spmv-vec-x.spmv-hot")? as usize == uses.len() && count(".spmv-vec-y.spmv-hot")? == 1,
        "the vectors under and beside the matrix mark the entries that a row uses and makes",
    )?;
    built.select(Some(Selection::Column(3)), p.matrix())?;
    check(
        count(".spmv-matrix-cell.spmv-on")? as usize == p.matrix().col(3).count(),
        "a column lights the cells of its nonzeros",
    )?;
    check(
        count("rect.spmv-hot")? as usize == p.matrix().col(3).count() + 1,
        "a column marks its entry of x and the entries of y that it feeds",
    )?;
    built.select(None, p.matrix())?;
    check(
        count(".spmv-matrix-cell.spmv-on")? == 0 && count("rect.spmv-hot")? == 0,
        "clearing the selection puts everything out",
    )?;

    check(count("rect.spmv-changed")? == 0, "nothing is marked as changed before a step")?;
    let (before_x, before_y) = (run.x.clone(), run.y.clone());
    run.advance(false).map_err(|e| format!("{e:?}"))?;
    built.apply(&run)?;
    let changed =
        (0..8).filter(|n| before_x[*n] != run.x[*n]).count() + (0..8).filter(|n| before_y[*n] != run.y[*n]).count();
    check(
        changed > 0 && count("rect.spmv-changed")? as usize == changed,
        "a step outlines exactly the entries that it changed",
    )?;
    let beside = |selector: &str| -> Result<Vec<String>, String> {
        let found = svg.query_selector_all(selector).map_err(|e| format!("{e:?}"))?;
        Ok((0..found.length())
            .filter_map(|i| found.get(i))
            .map(|n| n.text_content().unwrap_or_default())
            .collect())
    };
    check(
        beside(".spmv-node-value-x")? == run.x.iter().map(i64::to_string).collect::<Vec<_>>(),
        "the value written beside each column node follows x",
    )?;
    check(
        beside(".spmv-node-value-y")? == run.y.iter().map(i64::to_string).collect::<Vec<_>>(),
        "the value written beside each row node follows y",
    )?;
    check(
        count(".spmv-node-value.spmv-changed")? as usize == changed,
        "the values beside the nodes that a step changed are marked",
    )?;
    check(
        count(".spmv-vec-cell.spmv-changed")? as usize == changed,
        "the vectors beside the matrix show the changed entries too",
    )?;
    let found = svg.query_selector_all(".spmv-vec-x text").map_err(|e| format!("{e:?}"))?;
    let shown_x: Vec<String> = (0..found.length())
        .filter_map(|i| found.get(i))
        .map(|n| n.text_content().unwrap_or_default())
        .collect();
    check(
        shown_x == run.x.iter().map(i64::to_string).collect::<Vec<_>>(),
        "the row of x under the matrix shows the values of x after the step",
    )?;
    drop(built);
    host.remove();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

/// The worked product draws the multiplication as A, x and y and then, for the selected row, a multiplication for each of its numbers
/// and the additions that make the entry of y, and every number in it is the number that the core holds.
#[wasm_bindgen_test]
fn the_worked_product_ends_at_the_entry_of_y_for_every_row() -> Result<(), String> {
    let s = spec(8, 8, 3, (0, 0), 1, 8, (1, 1));
    let p = prepared(&s)?;
    let mut run = p.run();
    let (host, mut built, _) = probe::build_with("sm-worked", &p, &run, worked_options())?;
    check(
        built.is_readable() && built.worked_row().is_some(),
        "the scene was built as a worked product",
    )?;
    for row in 0..8 {
        built.show_row(&p, &run, row)?;
        check(
            built.worked_row() == Some(row),
            "the row that is shown is the row that was chosen",
        )?;
        let svg = host.svg()?;
        let tagged = svg
            .query_selector_all(&format!("rect.spmv-a-cell[data-spmv-node=\"y{row}\"]"))
            .map_err(|e| format!("{e:?}"))?;
        check(
            tagged.length() == 8,
            &format!("row {row}: the eight cells of the row of A are tagged"),
        )?;
        let lit = svg
            .query_selector_all("rect.spmv-a-cell.spmv-on")
            .map_err(|e| format!("{e:?}"))?;
        check(
            lit.length() as usize == p.matrix().row(row).count(),
            &format!("row {row}: a cell is lit for each nonzero of the row"),
        )?;
        let hot_x = svg.query_selector_all("rect.spmv-hot").map_err(|e| format!("{e:?}"))?;
        check(
            hot_x.length() as usize == p.matrix().row(row).count() + 1,
            &format!("row {row}: the entries of x that it uses, and its entry of y, are marked"),
        )?;
        let result = numbers_in(&host, ".spmv-term-result text")?;
        let want = run.y[row].to_string();
        check(
            result.last() == Some(&want),
            &format!("row {row}: the node at the end of the sum holds {want}, and not {result:?}"),
        )?;
    }
    built.show_row(&p, &run, 2)?;
    for _ in 0..3 {
        run.advance(false).map_err(|e| format!("{e:?}"))?;
        built.apply(&run)?;
        let result = numbers_in(&host, ".spmv-term-result text")?;
        check(
            result.last() == Some(&run.y[2].to_string()),
            &format!("after a step the sum still ends at y2 = {}, and not {result:?}", run.y[2]),
        )?;
        let products: Vec<String> = numbers_in(&host, ".spmv-term-product text")?
            .into_iter()
            .filter(|t| t != "MUL")
            .collect();
        let wanted: Vec<String> = p.matrix().row(2).map(|(j, v)| (v * run.x[j]).to_string()).collect();
        check(
            products == wanted,
            &format!("the products follow the step: {products:?} against {wanted:?}"),
        )?;
        let all_x = numbers_in(&host, ".spmv-term-x text")?;
        let used: Vec<String> = p.matrix().row(2).map(|(j, _)| run.x[j].to_string()).collect();
        check(
            all_x.iter().filter(|t| !t.contains('x')).cloned().collect::<Vec<_>>() == used,
            &format!("the entries of x in the sum follow the step: {all_x:?} against {used:?}"),
        )?;
    }
    drop(built);
    host.remove();
    Ok(())
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

/// In the worked product the vector x is a single column, the matrix is a grid, and no node, however long the row, is drawn on another.
#[wasm_bindgen_test]
fn the_worked_product_is_laid_out_with_x_as_a_column_and_nothing_on_top_of_anything() -> Result<(), String> {
    for (n, s) in [
        spec(8, 8, 3, (0, 0), 1, 8, (1, 1)),
        spec(12, 12, 9, (0, 0), 1, 12, (1, 1)),
        spec(4, 6, 2, (0, 0), 1, 4, (1, 1)),
    ]
    .iter()
    .enumerate()
    {
        let p = prepared(s)?;
        let run = p.run();
        let (host, mut built, _) = probe::build_with(&format!("sm-layout-{n}"), &p, &run, worked_options())?;
        for row in 0..p.matrix().rows {
            built.show_row(&p, &run, row)?;
            let svg = host.svg()?;
            let groups = svg
                .query_selector_all(":scope > g.svg-dom-graph-content > g")
                .map_err(|e| format!("{e:?}"))?;
            let rects_of = |g: u32, count: usize| -> Result<Vec<(f64, f64, f64, f64)>, String> {
                let group = groups
                    .get(g)
                    .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
                    .ok_or("no group")?;
                let all = group.query_selector_all("rect").map_err(|e| format!("{e:?}"))?;
                let all: Vec<_> = (0..all.length())
                    .filter_map(|i| all.get(i))
                    .filter_map(|n| n.dyn_into::<web_sys::Element>().ok())
                    .collect();
                Ok(all[all.len() - count..].iter().map(screen_rect).collect())
            };
            for (name, g, count) in [("x", 1, p.matrix().cols), ("y", 2, p.matrix().rows)] {
                let cells = rects_of(g, count)?;
                check(
                    cells.iter().all(|c| (c.0 - cells[0].0).abs() < 1.0) && cells.windows(2).all(|w| w[1].1 > w[0].1),
                    &format!("case {n} row {row}: the vector {name} is a single column, one entry under another"),
                )?;
            }
            let a = rects_of(0, p.matrix().rows * p.matrix().cols)?;
            let columns = p.matrix().cols;
            check(
                a.iter()
                    .enumerate()
                    .all(|(k, c)| (c.0 - a[k % columns].0).abs() < 1.0 && (c.1 - a[k - k % columns].1).abs() < 1.0),
                &format!("case {n} row {row}: A is a grid, its cells in rows and in columns"),
            )?;
            // No node is on another.
            let rects = node_rects(&host)?;
            for i in 0..rects.len() {
                for j in i + 1..rects.len() {
                    let (a, b) = (rects[i].1, rects[j].1);
                    let overlap = a.0 + 0.5 < b.2 && b.0 + 0.5 < a.2 && a.1 + 0.5 < b.3 && b.1 + 0.5 < a.3;
                    check(
                        !overlap,
                        &format!(
                            "case {n} row {row}: node {i} ({}) overlaps node {j} ({}): {:?} and {:?}",
                            rects[i].0, rects[j].0, rects[i].1, rects[j].1
                        ),
                    )?;
                }
            }
            let terms = p.matrix().row(row).count();
            let multiplications = svg
                .query_selector_all(".spmv-term-product")
                .map_err(|e| format!("{e:?}"))?
                .length() as usize;
            check(
                multiplications == terms.min(spmv_scene::worked::MAX_TERMS),
                &format!("case {n} row {row}: {multiplications} multiplications for {terms} numbers"),
            )?;
            let folded = svg
                .query_selector_all(".spmv-term-rest")
                .map_err(|e| format!("{e:?}"))?
                .length() as usize;
            check(
                folded == usize::from(terms > spmv_scene::worked::MAX_TERMS),
                &format!(
                    "case {n} row {row}: a row of {terms} numbers folds its terms past the limit into one node, and a shorter one does not"
                ),
            )?;
            let result = numbers_in(&host, ".spmv-term-result text")?;
            check(
                result.last() == Some(&run.y[row].to_string()),
                &format!("case {n} row {row}: the sum ends at {}", run.y[row]),
            )?;
        }
        drop(built);
        host.remove();
    }
    Ok(())
}

/// The output is drawn as bars level with its entries, each as long as its number against the largest, to the right for a positive number
/// and to the left for a negative one, and a step marks the bars that changed and the numbers of A that the changed entry of x fed.
#[wasm_bindgen_test]
fn the_output_is_drawn_as_bars_and_a_step_marks_what_it_fed() -> Result<(), String> {
    let s = spec(8, 8, 3, (0, 0), 1, 8, (1, 1));
    let p = prepared(&s)?;
    let mut run = p.run();
    run.set_order(spmv_core::UpdateOrder::InOrder);
    let (host, built, _) = probe::build_with("sm-bars", &p, &run, worked_options())?;
    let svg = host.svg()?;
    let count = |selector: &str| -> Result<u32, String> {
        Ok(svg.query_selector_all(selector).map_err(|e| format!("{e:?}"))?.length())
    };
    let bars = |built_run: &spmv_core::Run| -> Result<(), String> {
        let found = svg.query_selector_all(".spmv-bar").map_err(|e| format!("{e:?}"))?;
        check(found.length() == 8, "a bar for each entry of y")?;
        let largest = built_run.y.iter().map(|v| v.abs()).max().unwrap_or(1).max(1) as f64;
        let widths: Vec<f64> = (0..found.length())
            .filter_map(|i| found.get(i))
            .filter_map(|n| n.dyn_into::<web_sys::Element>().ok())
            .map(|e| e.get_bounding_client_rect().width())
            .collect();
        let widest = widths.iter().copied().fold(0.0, f64::max);
        for (i, w) in widths.iter().enumerate() {
            let want = built_run.y[i].abs() as f64 / largest;
            check(
                (w / widest - want).abs() < 0.03,
                &format!(
                    "bar {i} is {:.2} of the longest and y{i} is {want:.2} of the largest",
                    w / widest
                ),
            )?;
        }
        let axis = svg
            .query_selector(".spmv-bar-axis")
            .map_err(|e| format!("{e:?}"))?
            .ok_or("no axis")?
            .get_bounding_client_rect()
            .left();
        for i in 0..found.length() {
            let r = found
                .get(i)
                .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
                .ok_or("no bar")?
                .get_bounding_client_rect();
            let edge = if built_run.y[i as usize] < 0 { r.right() } else { r.left() };
            check(
                (edge - axis).abs() < 1.5,
                &format!(
                    "bar {i} grows from the axis, to the left for a negative number and to the right for a positive one"
                ),
            )?;
        }
        let negative = count(".spmv-bar.spmv-neg")? as usize;
        check(
            negative == built_run.y.iter().filter(|v| **v < 0).count(),
            "a bar is red for a negative number and blue for a positive one",
        )?;
        let note = svg
            .query_selector(".spmv-bars-scale")
            .map_err(|e| format!("{e:?}"))?
            .ok_or("no scale")?
            .text_content()
            .unwrap_or_default();
        check(
            note == format!("a full bar is {}", largest as i64),
            &format!("the scale is written: {note}"),
        )
    };
    bars(&run)?;
    check(
        count(".spmv-bar-changed")? == 0 && count(".spmv-fed")? == 0,
        "nothing is marked before a step",
    )?;
    for step in 0..3 {
        let before_y = run.y.clone();
        run.advance(false).map_err(|e| format!("{e:?}"))?;
        built.apply(&run)?;
        bars(&run)?;
        let changed = (0..8).filter(|i| before_y[*i] != run.y[*i]).count();
        check(
            count(".spmv-bar-changed")? as usize == changed,
            &format!("step {step}: the bars of the {changed} entries that changed are marked"),
        )?;
        let j = run.last_updates[0].0;
        check(j == step, &format!("step {step} changes x{step}, in order, and not x{j}"))?;
        check(
            count(".spmv-fed")? as usize == p.matrix().col(j).count(),
            &format!("step {step}: the numbers of A in column {j} are marked as fed"),
        )?;
    }
    drop(built);
    host.remove();
    Ok(())
}
