//! The worked product: the multiplication `y = A·x` drawn as the operations that do it, for the matrix that a person can read.
//!
//! The matrix A is a grid at the left, with the vector x beside it as a single column, and the vector y that they make. One row of A is
//! selected, and the operations that make its entry of y are drawn out as nodes: each number of the row is multiplied by the entry of x
//! under its column, and the products are added, one after another, into the entry of y. Every number in the operations is the number
//! that the row, the vector and the product hold, and nothing in this module works out a number that `spmv-core` did not.

use crate::scene::{add_class, elements, err, js_err, remove_class};
use spmv_core::{Prepared, Run, matrix::Matrix};
use std::collections::HashMap;
use svg_dom::root::utils::Point;
use svg_dom_graph::NodeId;
use svg_dom_graph::scene::{
    ArithmeticOperator, DataFormat, DataNodeContent, GridLayout, LabellingStyle, NodeValues, Scene,
};

/// The most rows or columns of a matrix that are worked as operations. A larger one is drawn as a graph, since the grid of A would be
/// wider than a window.
pub const MAX_SIDE: u64 = 12;

/// The most terms of a row that are drawn as operations. A longer row folds the rest into one node, so that the picture stays the size
/// of a page and the sum is still the whole row.
pub const MAX_TERMS: usize = 6;

/// The room above the nodes for the heading of the operations.
const HEADING_ROOM: f64 = 40.0;
const GAP: f64 = 30.0;
/// The width of the bars that draw y, and the room left of an operand for the name of what it is.
const BAR_WIDTH: f64 = 120.0;
const LABEL_ROOM: f64 = 58.0;
const SIGN_ROOM: f64 = 60.0;
const OPERAND_GAP: f64 = 4.0;
const ROW_GAP: f64 = 12.0;
const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// The labels of a grid, which the library holds as a `'static` list. A list is made once for each kind and length, and kept.
fn labels(kind: char, n: usize) -> &'static [&'static str] {
    thread_local! {
        static LISTS: std::cell::RefCell<HashMap<(char, usize), &'static [&'static str]>> = std::cell::RefCell::default();
    }
    LISTS.with(|lists| {
        *lists.borrow_mut().entry((kind, n)).or_insert_with(|| {
            let names: Vec<&'static str> = (0..n).map(|i| &*Box::leak(format!("{kind}{i}").into_boxed_str())).collect();
            Box::leak(names.into_boxed_slice())
        })
    })
}

/// The numbers of the sum for one row: what each shown term makes, what the terms past the last shown one make together, and the sums
/// as they are added up one after another.
struct Sum {
    /// The nonzeros of the row that are drawn, as the column, the number of A, and the product.
    shown: Vec<(usize, i64, i64)>,
    /// How many nonzeros there are past those, and what they make together.
    rest: Option<(usize, i64)>,
    /// The running total after each item is added, an item being a shown term or the rest.
    running: Vec<i64>,
}

fn sum_of(terms: &[(usize, i64)], x: &[i64]) -> Sum {
    let shown: Vec<(usize, i64, i64)> = terms.iter().take(MAX_TERMS).map(|(c, v)| (*c, *v, v * x[*c])).collect();
    let rest = (terms.len() > MAX_TERMS).then(|| {
        (
            terms.len() - MAX_TERMS,
            terms[MAX_TERMS..].iter().map(|(c, v)| v * x[*c]).sum::<i64>(),
        )
    });
    let mut running = Vec::new();
    let mut total = 0;
    for item in shown.iter().map(|t| t.2).chain(rest.map(|r| r.1)) {
        total += item;
        running.push(total);
    }
    Sum { shown, rest, running }
}

/// The texts that follow a cell, up to the next cell: the label of its row, if it begins one, and its number, which is the last.
fn texts_of(cell: &web_sys::Element) -> Vec<web_sys::Element> {
    let mut found = Vec::new();
    let mut next = cell.next_element_sibling();
    while let Some(e) = next {
        if e.tag_name().eq_ignore_ascii_case("rect") {
            break;
        }
        if e.tag_name().eq_ignore_ascii_case("text") {
            found.push(e.clone());
        }
        next = e.next_element_sibling();
    }
    found
}

/// The nodes of one term.
struct Term {
    column: usize,
    x: NodeId,
    product: NodeId,
}

/// A worked product that is drawn.
pub struct Worked {
    pub row: usize,
    /// The nonzeros of the row, as the column and the number of A.
    nonzeros: Vec<(usize, i64)>,
    pub(crate) x_node: NodeId,
    pub(crate) y_node: NodeId,
    terms: Vec<Term>,
    rest: Option<NodeId>,
    adds: Vec<NodeId>,
    result: NodeId,
    /// The rectangle of each cell of A, x and y, which the highlight and the marks are written on.
    cols: usize,
    /// The bars that draw y, one for each entry, the line they grow from, the note of the scale, and what they showed last.
    bars: Vec<web_sys::Element>,
    scale_note: web_sys::Element,
    bar_zero: f64,
    shown_y: std::cell::RefCell<Vec<i64>>,
    /// The cells of A that the entries changed by the last step feed, which are marked.
    fed: std::cell::RefCell<Vec<web_sys::Element>>,
    pub a_cells: Vec<web_sys::Element>,
    pub x_cells: Vec<web_sys::Element>,
    pub y_cells: Vec<web_sys::Element>,
}

/// What the room that the picture takes is expected to be, before it is drawn, for the window to give it a view box: about, and not
/// exactly, since the browser decides the size of text.
#[must_use]
pub fn extent(matrix: &Matrix) -> (f64, f64) {
    let degree = (0..matrix.rows).map(|i| matrix.row(i).count()).max().unwrap_or(0);
    let longest = degree.clamp(1, MAX_TERMS + 1) as f64;
    let (a_w, a_h) = (matrix.cols as f64 * 44.0 + 40.0, matrix.rows as f64 * 36.0 + 110.0);
    let chain_w = BAR_WIDTH + LABEL_ROOM + 300.0 + (longest - 1.0) * 84.0;
    let chain_h = longest * 92.0 + 40.0;
    (
        a_w + 2.0 * 110.0 + SIGN_ROOM * 2.0 + chain_w,
        a_h.max(chain_h) + HEADING_ROOM + 40.0,
    )
}

impl Worked {
    /// Draws the worked product of `row` on `scene`, which has to be empty.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a scene call failed or the nodes were not found in the page.
    #[allow(clippy::too_many_lines)]
    pub fn build(
        scene: &Scene,
        svg_id: &str,
        prepared: &Prepared,
        run: &Run,
        row: usize,
        origin: (f64, f64),
    ) -> Result<Self, String> {
        let matrix = prepared.matrix();
        let (rows, cols) = (matrix.rows, matrix.cols);
        // The heading is drawn above the nodes, and the origin is where the nodes begin.
        let (ox, oy) = (origin.0, origin.1 + HEADING_ROOM);
        let content_of = |v: Vec<i64>| DataNodeContent::new(NodeValues::I64(v), DataFormat::Decimal);
        let named = |at: (f64, f64), name: &str, content: DataNodeContent| {
            scene
                .add_named_data_node(Point::new(at.0, at.1), name, content)
                .map_err(err("data node"))
        };
        let rect = |id: NodeId| scene.node_rect(id).map_err(err("node rect"));

        // The three things that are multiplied and made: A, x, and y.
        let mut dense = vec![0_i64; rows * cols];
        for i in 0..rows {
            for (j, v) in matrix.row(i) {
                dense[i * cols + j] = v;
            }
        }
        let a_node = named(
            (ox, oy),
            "A",
            content_of(dense.clone())
                .with_layout(GridLayout::Columns(cols))
                .with_labelling_style(LabellingStyle::Text(labels('y', rows))),
        )?;
        let a_rect = rect(a_node)?;
        let x_at = a_rect.origin.x + a_rect.size.width + SIGN_ROOM;
        let x_node = named(
            (x_at, oy),
            "x",
            content_of(run.x[..cols].to_vec())
                .with_layout(GridLayout::Columns(1))
                .with_labelling_style(LabellingStyle::Text(labels('x', cols))),
        )?;
        let x_rect = rect(x_node)?;
        let y_at = x_rect.origin.x + x_rect.size.width + SIGN_ROOM;
        let y_node = named(
            (y_at, oy),
            "y",
            content_of(run.y[..rows].to_vec())
                .with_layout(GridLayout::Columns(1))
                .with_labelling_style(LabellingStyle::Text(labels('y', rows))),
        )?;
        let y_rect = rect(y_node)?;

        // The sum for the selected row, from the right of y, after the bars that draw y.
        let nonzeros: Vec<(usize, i64)> = matrix.row(row).collect();
        let sum = sum_of(&nonzeros, &run.x);
        let bar_zero = y_rect.origin.x + y_rect.size.width + 14.0 + BAR_WIDTH / 2.0;
        let chain_x = bar_zero + BAR_WIDTH / 2.0 + 24.0 + LABEL_ROOM;
        let item_count = sum.shown.len() + usize::from(sum.rest.is_some());
        let plain = |at: (f64, f64), value: i64| {
            scene
                .add_data_node(Point::new(at.0, at.1), content_of(vec![value]))
                .map_err(err("data node"))
        };
        let (mut terms, mut adds): (Vec<Term>, Vec<NodeId>) = (Vec::new(), Vec::new());
        let mut item_nodes: Vec<NodeId> = Vec::new();
        let mut labelled: Vec<(String, f64, f64)> = Vec::new();
        let mut rest = None;
        let mut row_pitch = 0.0;
        let mut mid_of_row = Vec::new();
        let mut multiply_right = chain_x;
        for (k, (column, a_value, product)) in sum.shown.iter().enumerate() {
            let top = oy + k as f64 * row_pitch;
            let a_term = plain((chain_x, top), *a_value)?;
            let a_term_rect = rect(a_term)?;
            let x_term = plain((chain_x, top + a_term_rect.size.height + OPERAND_GAP), run.x[*column])?;
            let x_term_rect = rect(x_term)?;
            labelled.push((
                format!("A[{row}][{column}]"),
                a_term_rect.origin.x,
                a_term_rect.origin.y + a_term_rect.size.height / 2.0,
            ));
            labelled.push((
                format!("x{column}"),
                x_term_rect.origin.x,
                x_term_rect.origin.y + x_term_rect.size.height / 2.0,
            ));
            if k == 0 {
                row_pitch = a_term_rect.size.height + OPERAND_GAP + x_term_rect.size.height + ROW_GAP;
            }
            let operands_w = a_term_rect.size.width.max(x_term_rect.size.width);
            let mid = top + (a_term_rect.size.height + OPERAND_GAP + x_term_rect.size.height) / 2.0;
            let product_node = scene
                .add_arithmetic_operator_node(
                    Point::new(chain_x + operands_w + GAP, mid),
                    ArithmeticOperator::Multiply,
                    (a_term, x_term),
                    content_of(vec![*product]),
                )
                .map_err(err("multiply"))?;
            let product_rect = rect(product_node)?;
            if k == 0 {
                // A row of the sum is as tall as an operator, so that the sums in two rows are never on each other.
                row_pitch = row_pitch.max(product_rect.size.height + OPERAND_GAP + ROW_GAP / 2.0);
            }
            scene
                .move_node(
                    product_node,
                    Point::new(product_rect.origin.x, mid - product_rect.size.height / 2.0),
                )
                .map_err(err("centre"))?;
            multiply_right = multiply_right.max(product_rect.origin.x + product_rect.size.width);
            mid_of_row.push(mid);
            item_nodes.push(product_node);
            terms.push(Term {
                column: *column,
                x: x_term,
                product: product_node,
            });
        }
        if let Some((count, together)) = sum.rest {
            let top = oy + sum.shown.len() as f64 * row_pitch;
            let node = plain((chain_x, top), together)?;
            let r = rect(node)?;
            labelled.push((format!("+{count} more"), r.origin.x, r.origin.y + r.size.height / 2.0));
            mid_of_row.push(top + r.size.height / 2.0);
            item_nodes.push(node);
            rest = Some(node);
        }

        // The terms are added one after another, each sum a step further right and a row lower than the one before.
        let mut previous = item_nodes.first().copied();
        let mut last_right = multiply_right;
        let mut last_mid = mid_of_row.first().copied().unwrap_or(oy);
        let mut next_x = multiply_right + GAP;
        for k in 1..item_count {
            let at_x = next_x;
            let node = scene
                .add_arithmetic_operator_node(
                    Point::new(at_x, mid_of_row[k]),
                    ArithmeticOperator::Add,
                    (previous.ok_or("no first term")?, item_nodes[k]),
                    content_of(vec![sum.running[k]]),
                )
                .map_err(err("add"))?;
            let r = rect(node)?;
            scene
                .move_node(node, Point::new(r.origin.x, mid_of_row[k] - r.size.height / 2.0))
                .map_err(err("centre"))?;
            last_right = r.origin.x + r.size.width;
            next_x = last_right + 14.0;
            last_mid = mid_of_row[k];
            adds.push(node);
            previous = Some(node);
        }
        let total = sum.running.last().copied().unwrap_or(0);
        let result = plain((last_right + GAP, last_mid - 17.0), total)?;
        let result_rect = rect(result)?;
        if let Some(last) = previous {
            scene.add_edge(last, result).map_err(err("result edge"))?;
        }

        // The cells of the three, which the page marks.
        let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
        let svg = document.get_element_by_id(svg_id).ok_or("no host")?;
        let content = svg
            .query_selector(":scope > g.svg-dom-graph-content")
            .map_err(js_err("content group"))?
            .ok_or("the scene has no content group")?;
        let groups = elements(&content, ":scope > g")?;
        let cells_of = |group: &web_sys::Element, n: usize| -> Result<Vec<web_sys::Element>, String> {
            let rects = elements(group, "rect")?;
            if rects.len() < n {
                return Err(format!("a node of {n} cells drew {} rectangles", rects.len()));
            }
            Ok(rects[rects.len() - n..].to_vec())
        };
        let a_cells = cells_of(groups.first().ok_or("no A")?, rows * cols)?;
        let x_cells = cells_of(groups.get(1).ok_or("no x")?, cols)?;
        let y_cells = cells_of(groups.get(2).ok_or("no y")?, rows)?;

        // What is drawn around them: the signs between, the numbers of the columns under A, and what the operations are.
        let notes = document.create_element_ns(Some(SVG_NS), "g").map_err(js_err("group"))?;
        notes.set_attribute("class", "spmv-worked-notes").map_err(js_err("class"))?;
        let note = |class: &str, x: f64, y: f64, text: &str| -> Result<web_sys::Element, String> {
            let e = document.create_element_ns(Some(SVG_NS), "text").map_err(js_err("text"))?;
            e.set_attribute("class", class).map_err(js_err("class"))?;
            e.set_attribute("x", &x.to_string()).map_err(js_err("x"))?;
            e.set_attribute("y", &y.to_string()).map_err(js_err("y"))?;
            e.set_text_content(Some(text));
            notes.append_child(&e).map_err(js_err("append"))?;
            Ok(e)
        };
        let sign_y = oy + a_rect.size.height / 2.0;
        note("spmv-sign", a_rect.origin.x + a_rect.size.width + SIGN_ROOM / 2.0, sign_y, "×")?;
        note("spmv-sign", x_rect.origin.x + x_rect.size.width + SIGN_ROOM / 2.0, sign_y, "=")?;
        let below = a_rect.origin.y + a_rect.size.height + 16.0;
        for j in 0..cols {
            let c = scene.cell_rect(a_node, j).map_err(err("cell"))?;
            note("spmv-col-label", c.origin.x + c.size.width / 2.0, below, &format!("x{j}"))?;
        }
        note(
            "spmv-heading",
            chain_x,
            oy - 30.0,
            &format!("Row {row} of A times x, added up, makes y{row}:"),
        )?;
        for (label, x, y) in &labelled {
            note("spmv-term-label", x - 6.0, y + 4.0, label)?;
        }
        note(
            "spmv-term-label spmv-result-label",
            result_rect.origin.x + result_rect.size.width + 6.0,
            result_rect.origin.y + result_rect.size.height / 2.0 + 4.0,
            &format!("y{row}"),
        )?;
        // The bars that draw y, level with its entries, growing right for a positive number and left for a negative one.
        let shape = |tag: &str, attributes: &[(&str, String)]| -> Result<web_sys::Element, String> {
            let e = document.create_element_ns(Some(SVG_NS), tag).map_err(js_err("shape"))?;
            for (name, value) in attributes {
                e.set_attribute(name, value).map_err(js_err(name))?;
            }
            notes.append_child(&e).map_err(js_err("append"))?;
            Ok(e)
        };
        let first = scene.cell_rect(y_node, 0).map_err(err("cell"))?;
        let last = scene.cell_rect(y_node, rows - 1).map_err(err("cell"))?;
        shape(
            "line",
            &[
                ("class", "spmv-bar-axis".to_owned()),
                ("x1", bar_zero.to_string()),
                ("x2", bar_zero.to_string()),
                ("y1", (first.origin.y - 2.0).to_string()),
                ("y2", (last.origin.y + last.size.height + 2.0).to_string()),
            ],
        )?;
        let mut bars = Vec::with_capacity(rows);
        for i in 0..rows {
            let c = scene.cell_rect(y_node, i).map_err(err("cell"))?;
            bars.push(shape(
                "rect",
                &[
                    ("class", "spmv-bar".to_owned()),
                    ("x", bar_zero.to_string()),
                    ("y", (c.origin.y + c.size.height * 0.2).to_string()),
                    ("width", "0".to_owned()),
                    ("height", (c.size.height * 0.6).to_string()),
                    ("rx", "2".to_owned()),
                ],
            )?);
        }
        note("spmv-bars-title", bar_zero, oy - 8.0, "y drawn as bars")?;
        let scale_note = note("spmv-bars-scale", bar_zero, last.origin.y + last.size.height + 18.0, "")?;
        content.append_child(&notes).map_err(js_err("append"))?;

        let worked = Self {
            cols,
            bars,
            scale_note,
            bar_zero,
            shown_y: std::cell::RefCell::new(run.y[..rows].to_vec()),
            fed: std::cell::RefCell::new(Vec::new()),
            row,
            nonzeros,
            x_node,
            y_node,
            terms,
            rest,
            adds,
            result,
            a_cells,
            x_cells,
            y_cells,
        };
        worked.mark(prepared, &groups, cols)?;
        worked.draw_bars(&run.y[..rows])?;
        if run.step > 0 {
            worked.mark_fed(&run.last_updates)?;
        }
        Ok(worked)
    }

    /// Writes the classes that colour the row, the entries that it uses and the entry that it makes, and the tags that a click is
    /// traced back to a row by.
    fn mark(&self, prepared: &Prepared, groups: &[web_sys::Element], cols: usize) -> Result<(), String> {
        let matrix = prepared.matrix();
        let used: Vec<usize> = matrix.row(self.row).map(|(j, _)| j).collect();
        let tag = |e: &web_sys::Element, row: usize| {
            e.set_attribute("data-spmv-node", &format!("y{row}")).map_err(js_err("tag"))
        };
        for (n, cell) in self.a_cells.iter().enumerate() {
            let (i, j) = (n / cols, n % cols);
            tag(cell, i)?;
            add_class(cell, "spmv-a-cell")?;
            let value = matrix.row(i).find(|(c, _)| *c == j).map(|(_, v)| v);
            let texts = texts_of(cell);
            for t in &texts {
                tag(t, i)?;
            }
            let text = texts.last();
            match value {
                None => {
                    add_class(cell, "spmv-zero")?;
                    if let Some(t) = text {
                        add_class(t, "spmv-zero")?;
                    }
                },
                Some(v) => add_class(cell, if v < 0 { "spmv-neg" } else { "spmv-pos" })?,
            }
            if i == self.row {
                add_class(cell, "spmv-row")?;
                if value.is_some() {
                    add_class(cell, "spmv-on")?;
                }
            }
        }
        for (i, cell) in self.y_cells.iter().enumerate() {
            tag(cell, i)?;
            for t in texts_of(cell) {
                tag(&t, i)?;
            }
            if i == self.row {
                add_class(cell, "spmv-hot")?;
            }
        }
        for j in used {
            add_class(&self.x_cells[j], "spmv-hot")?;
        }
        // Each node of the operations carries the class of what it is, in the order that they were drawn: A, x and y, and then for each
        // term the number of A, the entry of x and the product.
        let mut next = 3;
        let mut classify = |class: &str, how_many: usize| -> Result<(), String> {
            for _ in 0..how_many {
                if let Some(group) = groups.get(next) {
                    add_class(group, class)?;
                }
                next += 1;
            }
            Ok(())
        };
        for _ in &self.terms {
            classify("spmv-term-a", 1)?;
            classify("spmv-term-x", 1)?;
            classify("spmv-term-product", 1)?;
        }
        if self.rest.is_some() {
            classify("spmv-term-rest", 1)?;
        }
        classify("spmv-term-sum", self.adds.len())?;
        classify("spmv-term-result", 1)?;
        Ok(())
    }

    /// Writes the numbers of a step in place: the vectors, the entries of x that the operations use, and every product and sum.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a node could not be written.
    pub fn apply(&self, scene: &Scene, run: &Run) -> Result<(), String> {
        let set = |id: NodeId, v: Vec<i64>| scene.set_data_values(id, NodeValues::I64(v)).map_err(err("write"));
        set(self.x_node, run.x[..self.x_cells.len()].to_vec())?;
        set(self.y_node, run.y[..self.y_cells.len()].to_vec())?;
        let sum = sum_of(&self.nonzeros, &run.x);
        for (term, (_, _, product)) in self.terms.iter().zip(&sum.shown) {
            set(term.x, vec![run.x[term.column]])?;
            set(term.product, vec![*product])?;
        }
        if let (Some(node), Some((_, together))) = (self.rest, sum.rest) {
            set(node, vec![together])?;
        }
        for (node, total) in self.adds.iter().zip(sum.running.iter().skip(1)) {
            set(*node, vec![*total])?;
        }
        set(self.result, vec![sum.running.last().copied().unwrap_or(0)])?;
        self.draw_bars(&run.y[..self.y_cells.len()])?;
        self.mark_fed(&run.last_updates)
    }

    /// Draws the bars for `y`, the longest one filling its half of the width, and marks the ones that changed since they were last drawn.
    fn draw_bars(&self, y: &[i64]) -> Result<(), String> {
        let largest = y.iter().map(|v| v.abs()).max().unwrap_or(0).max(1);
        let half = BAR_WIDTH / 2.0 - 4.0;
        let before = self.shown_y.replace(y.to_vec());
        for (i, (bar, v)) in self.bars.iter().zip(y).enumerate() {
            let width = v.abs() as f64 / largest as f64 * half;
            let x = if *v < 0 { self.bar_zero - width } else { self.bar_zero };
            let sign = if *v < 0 { "spmv-neg" } else { "spmv-pos" };
            let hot = if i == self.row { " spmv-hot-bar" } else { "" };
            let changed = if before.get(i).is_some_and(|old| old != v) { " spmv-bar-changed" } else { "" };
            bar.set_attribute("class", &format!("spmv-bar {sign}{hot}{changed}"))
                .map_err(js_err("bar"))?;
            bar.set_attribute("x", &x.to_string()).map_err(js_err("bar"))?;
            bar.set_attribute("width", &width.to_string()).map_err(js_err("bar"))?;
        }
        self.scale_note.set_text_content(Some(&format!("a full bar is {largest}")));
        Ok(())
    }

    /// Marks the nonzeros of A in the columns whose entry of x the last step changed, which are the numbers that it was multiplied by,
    /// and puts out the marks of the step before.
    fn mark_fed(&self, updates: &[(usize, i64)]) -> Result<(), String> {
        let mut fed = self.fed.borrow_mut();
        for cell in fed.drain(..) {
            remove_class(&cell, "spmv-fed")?;
        }
        let rows = self.a_cells.len() / self.cols.max(1);
        for (j, _) in updates {
            for i in 0..rows {
                let cell = &self.a_cells[i * self.cols + j];
                if !cell.class_list().contains("spmv-zero") {
                    add_class(cell, "spmv-fed")?;
                    fed.push(cell.clone());
                }
            }
        }
        Ok(())
    }

    /// Takes the row marks off, so that the page is as it was when the nodes were made.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a class could not be written.
    pub fn unmark(&self) -> Result<(), String> {
        for cell in self.a_cells.iter().chain(&self.x_cells).chain(&self.y_cells) {
            for class in ["spmv-on", "spmv-row", "spmv-hot"] {
                remove_class(cell, class)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod unit_tests;
