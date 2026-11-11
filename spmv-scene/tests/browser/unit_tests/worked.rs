//! The worked product, laid out with x as a column, and the output drawn as bars.

use super::*;

/// The worked product draws the multiplication as A, x and y and then, for the selected row, a multiplication for each
/// of its numbers and the additions that make the entry of y, and every number in it is the number that the core holds.
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

/// In the worked product the vector x is a single column, the matrix is a grid, and no node, however long the row, is
/// drawn on another.
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

/// The output is drawn as bars level with its entries, each as long as its number against the largest, to the right for
/// a positive number and to the left for a negative one, and a step marks the bars that changed and the numbers of A
/// that the changed entry of x fed.
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
