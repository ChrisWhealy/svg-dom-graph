//! The scene as the matrix it was built from, its layout, its budget and its teardown.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene has one node for each column and row, one connector for each nonzero, the two data nodes and the square,
/// every label is the one the contract gives, and no id is used twice.
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
/// After every step the two data nodes show the windows of x and y, exactly. A scene built afresh at that step shows
/// the same cells, so the cells the library rewrote are the cells a full render would have drawn.
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
/// A scenario over the budget is refused with a reason before anything is built: no scene exists to call, the empty
/// host stays empty, and a size that would take gigabytes is answered at once. A refusal is reported apart from a
/// rendering.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The documented risk of a node's degree: every edge appends a relationship clause to both ends, so the description of
/// a node grows with its degree. The hub's description is longer than any other node's, and is at least as long as its
/// degree times a label.
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

/// The documented risk of a drag: a move redraws every connector on the node, and only those. Moving the hub changes
/// exactly as many connectors as its degree, and moving a node that has none changes none.
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
/// The layout and the scene agree about where a node is, and the data nodes and the square do not overlap the graph or
/// each other.
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
/// The greatest figures the product can reach, written into every cell, stay inside the data node's frame: a cell keeps
/// the width it was drawn with, so a wider figure would show as text outside the frame.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The budget's model of the page never falls below the page: a budget that underestimated would let through a scene
/// bigger than it said. It also is not far above it, since an estimate that refused a third of what fits would be a
/// poor budget. The model was fitted to measured pages, and this keeps it honest for the shapes the tests build.
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
