//! The scene: one node for each column and each row, one edge for each nonzero, two data nodes that show windows of x
//! and y, and a square that shows where the nonzeros are.
//!
//! A scene is built only from a [`Prepared`] scenario, which exists only if the specification was valid and within the
//! budget, so a budget is enforced before any scene call by the types. Nothing here changes the shape of the scene
//! after it is built: a step writes the two data nodes and nothing else.

mod build_options;
mod build_timings;
mod built;
mod host;
mod marks;
mod selection;

use crate::layout::{Layout, MATRIX_CELL, MATRIX_LABELS, SPY_SIDE};
pub use build_options::BuildOptions;
pub use build_timings::BuildTimings;
pub use built::Built;
pub use host::Host;
use marks::Marks;
pub use selection::Selection;
use spmv_core::{Prepared, Run};
use wasm_bindgen::JsCast;

const SVG_NS: &str = "http://www.w3.org/2000/svg";

pub(crate) fn err<E: std::fmt::Display>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e}")
}

pub(crate) fn js_err<E: std::fmt::Debug>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e:?}")
}

/// The page's clock in milliseconds. Its step is 100 µs on a page that is not cross-origin isolated, so a single small
/// operation can read as zero, and the probes time a batch where that matters.
#[must_use]
pub fn now() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or(0.0, |p| p.now())
}

/// The bytes of JS heap in use, where the browser says (Chrome does, and Firefox does not).
#[must_use]
pub fn heap_used() -> Option<f64> {
    let performance = web_sys::window()?.performance()?;
    let memory = js_sys::Reflect::get(&performance, &"memory".into()).ok()?;
    js_sys::Reflect::get(&memory, &"usedJSHeapSize".into()).ok()?.as_f64()
}

/// How many elements `root` holds below it.
///
/// # Errors
///
/// Returns `Err` if the page could not be searched.
pub fn element_count(root: &web_sys::Element) -> Result<u32, String> {
    Ok(root.query_selector_all("*").map_err(js_err("count"))?.length())
}

fn has_class(e: &web_sys::Element, name: &str) -> bool {
    e.get_attribute("class")
        .is_some_and(|c| c.split_whitespace().any(|n| n == name))
}

pub(crate) fn add_class(e: &web_sys::Element, name: &str) -> Result<(), String> {
    if has_class(e, name) {
        return Ok(());
    }
    let old = e.get_attribute("class").unwrap_or_default();
    let new = if old.is_empty() { name.to_owned() } else { format!("{old} {name}") };
    e.set_attribute("class", &new).map_err(js_err("class"))
}

pub(crate) fn remove_class(e: &web_sys::Element, name: &str) -> Result<(), String> {
    let Some(old) = e.get_attribute("class") else { return Ok(()) };
    let kept: Vec<&str> = old.split_whitespace().filter(|n| *n != name).collect();
    e.set_attribute("class", &kept.join(" ")).map_err(js_err("class"))
}

/// The elements that match `selector` below `root`, in document order.
pub(crate) fn elements(root: &web_sys::Element, selector: &str) -> Result<Vec<web_sys::Element>, String> {
    let found = root.query_selector_all(selector).map_err(js_err("select"))?;
    Ok((0..found.length())
        .filter_map(|i| found.get(i))
        .filter_map(|n| n.dyn_into().ok())
        .collect())
}

/// The words under the three boxes that say what each is, in the scene, so that they pan and zoom with it. A caption is
/// a few short lines, each short enough to sit under the box it names.
fn add_captions(svg_id: &str, prepared: &Prepared, layout: &Layout) -> Result<(), String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
    let svg = document.get_element_by_id(svg_id).ok_or("no host")?;
    let content = svg
        .query_selector(":scope > g.svg-dom-graph-content")
        .map_err(js_err("content group"))?
        .ok_or("the scene has no content group")?;
    let (spec, cells) = (prepared.spec(), prepared.spec().cells);
    let group = document.create_element_ns(Some(SVG_NS), "g").map_err(js_err("group"))?;
    group.set_attribute("class", "spmv-captions").map_err(js_err("class"))?;
    let entries = |whole: u64| {
        if cells >= whole {
            format!("All {whole} entries.")
        } else {
            format!("Its first {cells} entries.")
        }
    };
    let pattern: Vec<String> = if layout.matrix.is_some() {
        [
            "A: the matrix, in full.",
            "A coloured cell is a number of A,",
            "and a blank cell is zero. The row",
            "under A is x, and the column is y.",
        ]
        .map(String::from)
        .to_vec()
    } else {
        [
            "Where A is nonzero.",
            "Each grey square is one",
            "nonzero: its column is",
            "across, its row down.",
        ]
        .map(String::from)
        .to_vec()
    };
    let captions: [((f64, f64), &str, Vec<String>); 3] = [
        (
            layout.x_caption,
            "x",
            vec![
                "x: the input vector.".into(),
                entries(spec.cols),
                "Entries in order, across.".into(),
            ],
        ),
        (
            layout.y_caption,
            "y",
            vec![
                "y = A·x: the output.".into(),
                entries(spec.rows),
                "Entries in order, across.".into(),
            ],
        ),
        (layout.spy_caption, "pattern", pattern),
    ];
    let mut captions = captions.to_vec();
    // The example is the first nonzero of the matrix, so that the caption says something that can be checked on the
    // picture.
    let (i, j, v) = (0..prepared.matrix().rows)
        .find_map(|i| prepared.matrix().row(i).next().map(|(j, v)| (i, j, v)))
        .unwrap_or((0, 0, 0));
    let example_lines: Vec<String> = vec![
        format!("Example: the line from x{j} to y{i}"),
        format!("carries {v}, so row {i}, column {j}"),
        format!("of the matrix A is {v}. These numbers"),
        "are fixed; only the values of x change.".to_owned(),
    ];
    if layout.single {
        captions.push((layout.channel_caption, "channel", example_lines));
    }
    for ((x, y), name, lines) in captions {
        for (n, line) in lines.iter().enumerate() {
            let text = document.create_element_ns(Some(SVG_NS), "text").map_err(js_err("text"))?;
            text.set_attribute("class", &format!("spmv-caption spmv-caption-{name}"))
                .map_err(js_err("class"))?;
            if name == "channel" {
                // The caption is centred on the middle of the channel, whatever style sheet the page has.
                text.set_attribute("text-anchor", "middle").map_err(js_err("anchor"))?;
            }
            text.set_attribute("x", &x.to_string()).map_err(js_err("x"))?;
            text.set_attribute("y", &(y + n as f64 * crate::layout::CAPTION_LINE).to_string())
                .map_err(js_err("y"))?;
            text.set_text_content(Some(line));
            group.append_child(&text).map_err(js_err("append"))?;
        }
    }
    content.append_child(&group).map_err(js_err("append"))?;
    Ok(())
}

/// Writes the value of each node beside it, the entry of x at the left of a column node and the entry of y at the right
/// of a row node, so that what a step changes is seen on the graph and not only in the boxes above it.
fn add_node_values(
    svg_id: &str,
    prepared: &Prepared,
    run: &Run,
    layout: &Layout,
) -> Result<(Vec<web_sys::Element>, Vec<web_sys::Element>), String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
    let svg = document.get_element_by_id(svg_id).ok_or("no host")?;
    let content = svg
        .query_selector(":scope > g.svg-dom-graph-content")
        .map_err(js_err("content group"))?
        .ok_or("the scene has no content group")?;
    let group = document.create_element_ns(Some(SVG_NS), "g").map_err(js_err("group"))?;
    group.set_attribute("class", "spmv-node-values").map_err(js_err("class"))?;
    let spec = prepared.spec();
    let make = |class: &str, x: f64, y: f64, value: i64| -> Result<web_sys::Element, String> {
        let e = document.create_element_ns(Some(SVG_NS), "text").map_err(js_err("text"))?;
        e.set_attribute("class", class).map_err(js_err("class"))?;
        e.set_attribute("x", &x.to_string()).map_err(js_err("x"))?;
        e.set_attribute("y", &y.to_string()).map_err(js_err("y"))?;
        e.set_text_content(Some(&value.to_string()));
        group.append_child(&e).map_err(js_err("append"))?;
        Ok(e)
    };
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for j in 0..spec.cols as usize {
        let r = layout.col_node(j);
        xs.push(make(
            "spmv-node-value spmv-node-value-x",
            r.x - 8.0,
            r.y + r.h / 2.0 + 4.0,
            run.x[j],
        )?);
    }
    for i in 0..spec.rows as usize {
        let r = layout.row_node(i);
        ys.push(make(
            "spmv-node-value spmv-node-value-y",
            r.x + r.w + 8.0,
            r.y + r.h / 2.0 + 4.0,
            run.y[i],
        )?);
    }
    content.append_child(&group).map_err(js_err("append"))?;
    Ok((xs, ys))
}

/// The matrix drawn in full: a grid with the index of each row and column, a cell with its value for each nonzero, and
/// nothing for a zero, so that the grid stays quiet. It also makes the two bands, hidden until a row or a column is
/// selected, that run across it.
///
/// It returns the cells of the nonzeros in the order of the nonzeros by row, and the row band and the column band.
fn add_matrix(svg_id: &str, prepared: &Prepared, run: &Run, layout: &Layout) -> Result<MatrixParts, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
    let svg = document.get_element_by_id(svg_id).ok_or("no host")?;
    let content = svg
        .query_selector(":scope > g.svg-dom-graph-content")
        .map_err(js_err("content group"))?
        .ok_or("the scene has no content group")?;
    let (spec, matrix) = (prepared.spec(), prepared.matrix());
    let geom = layout.matrix.ok_or("the matrix is not drawn in full")?;
    let make = |tag: &str, attributes: &[(&str, &str)]| -> Result<web_sys::Element, String> {
        let e = document.create_element_ns(Some(SVG_NS), tag).map_err(js_err("element"))?;
        for (name, value) in attributes {
            e.set_attribute(name, value).map_err(js_err(name))?;
        }
        Ok(e)
    };
    let group = make(
        "g",
        &[
            ("class", "spmv-aside spmv-matrix"),
            ("role", "img"),
            (
                "aria-label",
                &format!(
                    "The matrix A, of {} rows and {} columns, with {} nonzero values",
                    spec.rows,
                    spec.cols,
                    matrix.nnz()
                ),
            ),
        ],
    )?;
    let put = |parent: &web_sys::Element, child: web_sys::Element| -> Result<(), String> {
        parent.append_child(&child).map(|_| ()).map_err(js_err("append"))
    };
    let (width, height) = geom.size();
    let (ox, oy) = geom.origin;
    let (cell_w, cell_h) = MATRIX_CELL;
    put(
        &group,
        make(
            "rect",
            &[
                ("class", "spmv-matrix-frame"),
                ("x", &(ox + MATRIX_LABELS.0).to_string()),
                ("y", &(oy + MATRIX_LABELS.1).to_string()),
                ("width", &(width - MATRIX_LABELS.0).to_string()),
                ("height", &(height - MATRIX_LABELS.1).to_string()),
            ],
        )?,
    )?;
    // The bands go under the cells, so that a lit cell is seen through them.
    let band = || {
        make(
            "rect",
            &[
                ("class", "spmv-band"),
                ("visibility", "hidden"),
                ("x", "0"),
                ("y", "0"),
                ("width", "0"),
                ("height", "0"),
            ],
        )
    };
    let (row_band, column_band) = (band()?, band()?);
    group.append_child(&row_band).map_err(js_err("append"))?;
    group.append_child(&column_band).map_err(js_err("append"))?;
    let text = |class: &str, x: f64, y: f64, content: &str| -> Result<web_sys::Element, String> {
        let e = make("text", &[("class", class), ("x", &x.to_string()), ("y", &y.to_string())])?;
        e.set_text_content(Some(content));
        Ok(e)
    };
    for j in 0..geom.cols {
        let c = geom.cell(0, j);
        put(
            &group,
            text(
                "spmv-matrix-label",
                c.x + cell_w / 2.0,
                oy + MATRIX_LABELS.1 - 5.0,
                &format!("x{j}"),
            )?,
        )?;
    }
    for i in 0..geom.rows {
        let c = geom.cell(i, 0);
        put(
            &group,
            text(
                "spmv-matrix-label spmv-matrix-row-label",
                ox + MATRIX_LABELS.0 - 4.0,
                c.y + cell_h / 2.0 + 4.0,
                &format!("y{i}"),
            )?,
        )?;
    }
    let mut cells = Vec::with_capacity(matrix.nnz());
    for i in 0..matrix.rows {
        for (j, value) in matrix.row(i) {
            let c = geom.cell(i, j);
            let sign = if value < 0 { "spmv-neg" } else { "spmv-pos" };
            let cell = make("g", &[("class", &format!("spmv-matrix-cell {sign}"))])?;
            put(
                &cell,
                make(
                    "rect",
                    &[
                        ("x", &(c.x + 1.0).to_string()),
                        ("y", &(c.y + 1.0).to_string()),
                        ("width", &(c.w - 2.0).to_string()),
                        ("height", &(c.h - 2.0).to_string()),
                        ("rx", "3"),
                    ],
                )?,
            )?;
            put(
                &cell,
                text("spmv-matrix-value", c.x + c.w / 2.0, c.y + c.h / 2.0 + 4.0, &value.to_string())?,
            )?;
            group.append_child(&cell).map_err(js_err("append"))?;
            cells.push(cell);
        }
    }
    // The vectors, lined up with the matrix: x is a row under it, one value beneath each column, and y a column beside
    // it, one value level with each row, so that a row of A is multiplied by the row of x and its sum is the entry of y
    // at its end.
    let strip = |class: &str, r: crate::layout::Rect, value: i64| -> Result<web_sys::Element, String> {
        let cell = make("g", &[("class", &format!("spmv-vec-cell {class}"))])?;
        put(
            &cell,
            make(
                "rect",
                &[
                    ("x", &(r.x + 1.0).to_string()),
                    ("y", &(r.y + 1.0).to_string()),
                    ("width", &(r.w - 2.0).to_string()),
                    ("height", &(r.h - 2.0).to_string()),
                    ("rx", "3"),
                ],
            )?,
        )?;
        put(
            &cell,
            text("spmv-matrix-value", r.x + r.w / 2.0, r.y + r.h / 2.0 + 4.0, &value.to_string())?,
        )?;
        Ok(cell)
    };
    let (mut strip_x, mut strip_y) = (Vec::new(), Vec::new());
    for j in 0..geom.cols {
        let cell = strip("spmv-vec-x", geom.x_entry(j), run.x[j])?;
        put(&group, cell.clone())?;
        strip_x.push(cell);
    }
    for i in 0..geom.rows {
        let cell = strip("spmv-vec-y", geom.y_entry(i), run.y[i])?;
        put(&group, cell.clone())?;
        strip_y.push(cell);
    }
    let x_label = geom.x_entry(0);
    put(
        &group,
        text(
            "spmv-matrix-label spmv-matrix-row-label",
            ox + MATRIX_LABELS.0 - 4.0,
            x_label.y + x_label.h / 2.0 + 4.0,
            "x",
        )?,
    )?;
    let y_label = geom.y_entry(0);
    put(
        &group,
        text(
            "spmv-matrix-label",
            y_label.x + y_label.w / 2.0,
            oy + MATRIX_LABELS.1 - 5.0,
            "y",
        )?,
    )?;
    content.append_child(&group).map_err(js_err("append"))?;
    Ok((cells, (row_band, column_band), (strip_x, strip_y)))
}

type MatrixParts = (
    Vec<web_sys::Element>,
    (web_sys::Element, web_sys::Element),
    (Vec<web_sys::Element>, Vec<web_sys::Element>),
);

/// The square that shows where the nonzeros are: one `<path>` however many there are, in a group in the scene's content
/// group.
fn add_spy(svg_id: &str, prepared: &Prepared, layout: &Layout) -> Result<(), String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
    let svg = document.get_element_by_id(svg_id).ok_or("no host")?;
    let content = svg
        .query_selector(":scope > g.svg-dom-graph-content")
        .map_err(js_err("content group"))?
        .ok_or("the scene has no content group")?;
    let (spec, matrix) = (prepared.spec(), prepared.matrix());
    let side = spec.rows.max(spec.cols) as f64;
    let make = |tag: &str, attributes: &[(&str, &str)]| -> Result<web_sys::Element, String> {
        let e = document.create_element_ns(Some(SVG_NS), tag).map_err(js_err("element"))?;
        for (name, value) in attributes {
            e.set_attribute(name, value).map_err(js_err(name))?;
        }
        Ok(e)
    };
    let group = make(
        "g",
        &[
            ("class", "spmv-aside spmv-spy"),
            ("role", "img"),
            (
                "aria-label",
                &format!(
                    "Pattern of the {} nonzeros of a matrix of {} rows and {} columns",
                    matrix.nnz(),
                    spec.rows,
                    spec.cols
                ),
            ),
        ],
    )?;
    let frame = make(
        "rect",
        &[
            ("x", &layout.spy_origin.0.to_string()),
            ("y", &layout.spy_origin.1.to_string()),
            ("width", &SPY_SIDE.to_string()),
            ("height", &SPY_SIDE.to_string()),
            ("fill", "none"),
            ("stroke", "currentColor"),
        ],
    )?;
    let mut d = String::with_capacity(matrix.nnz() * 14);
    for i in 0..matrix.rows {
        for (j, _) in matrix.row(i) {
            d.push_str(&format!("M{j} {i}h1v1h-1z"));
        }
    }
    let k = SPY_SIDE / side;
    let path = make(
        "path",
        &[
            ("d", &d),
            (
                "transform",
                &format!("translate({} {}) scale({k})", layout.spy_origin.0, layout.spy_origin.1),
            ),
            ("fill", "currentColor"),
        ],
    )?;
    group.append_child(&frame).map_err(js_err("append"))?;
    group.append_child(&path).map_err(js_err("append"))?;
    content.append_child(&group).map_err(js_err("append"))?;
    let _ = content.dyn_ref::<web_sys::Element>();
    Ok(())
}
