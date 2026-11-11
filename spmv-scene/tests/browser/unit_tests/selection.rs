//! A readable scene: signed edges, tagged nodes, labels, and what selecting a node lights.

use super::*;

/// A readable scene gives every connector the sign and the size of its value, in the order of the nonzeros by row, and
/// every node the tag that an event is traced back to a node by.
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

/// Selecting a node lights exactly its edges and the nodes at their other ends, and puts out what the last selection
/// lit. Selecting nothing puts everything out. A scene that was not built to be read refuses.
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
