//! The captions and the matrix drawn beside the graph.

use super::*;

/// The three boxes of the window say what they are. Each caption says its words, is under its own box, and overlaps no
/// node, no data node, no square and no other caption, for a graph that is read and for one that is packed into blocks.
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
        // The caption in the channel, which says what the number on a line is, is above the first node and apart from
        // everything.
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

/// A small matrix is drawn in full, with a cell for each nonzero and none for a zero, and a selection lights its cells,
/// its band and the entries of x and y that it uses; a step outlines the entries that it changed.
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
