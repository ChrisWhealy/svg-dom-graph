//! The scene, built and in the page.

use super::{
    BuildOptions, BuildTimings, Marks, Selection, add_captions, add_class, add_matrix, add_node_values, add_spy,
    elements, err, js_err, now, remove_class,
};
use crate::layout::Layout;
use crate::worked::Worked;
use spmv_core::{
    Prepared, Run,
    matrix::{Matrix, label},
};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, InputMode, NodeValues, Scene},
};

/// The scene, built and in the page. Dropping it releases the listeners that the scene put on the page.
pub struct Built {
    scene: Scene,
    col_nodes: Vec<NodeId>,
    row_nodes: Vec<NodeId>,
    x_node: NodeId,
    y_node: NodeId,
    layout: Layout,
    cells: usize,
    svg: web_sys::Element,
    /// The connectors, in the order of the nonzeros by row, and the groups of the column nodes and then the row nodes.
    /// Both are empty unless the build was asked to be readable.
    edge_paths: Vec<web_sys::Element>,
    node_groups: Vec<web_sys::Element>,
    picked: Vec<web_sys::Element>,
    lit_paths: Vec<usize>,
    lit_nodes: Vec<usize>,
    /// The cells of the matrix that were drawn in full, in the order of the nonzeros by row, and the band across a row
    /// and the band down a column. They are empty unless the matrix was small enough to be drawn in full.
    matrix_cells: Vec<web_sys::Element>,
    bands: Option<(web_sys::Element, web_sys::Element)>,
    lit_cells: Vec<usize>,
    /// The values of x, in a row under the matrix, and of y, in a column beside it, as groups of a rectangle and a
    /// text.
    strip_x: Vec<web_sys::Element>,
    strip_y: Vec<web_sys::Element>,
    /// The value of each node, written beside it: the entries of x at the left of the column nodes and of y at the
    /// right of the rows.
    node_x: Vec<web_sys::Element>,
    node_y: Vec<web_sys::Element>,
    changed_values: RefCell<Vec<web_sys::Element>>,
    hot_strips: Vec<web_sys::Element>,
    changed_strips: RefCell<Vec<web_sys::Element>>,
    /// The rectangles of the cells of the two data nodes, which the library draws, and the ones that a selection has
    /// made hot.
    x_cells: Vec<web_sys::Element>,
    y_cells: Vec<web_sys::Element>,
    hot: Vec<web_sys::Element>,
    /// What the data nodes held before the last step, and the cells that were marked as changed by it.
    marks: RefCell<Marks>,
    /// The multiplication drawn as its operations, when the scene was built that way.
    worked: Option<Worked>,
    origin: (f64, f64),
    pub timings: BuildTimings,
}

impl Built {
    /// Builds the scene of a prepared scenario on the `<svg>` called `svg_id`, which has to exist and be empty. The two
    /// data nodes start with the values of `run`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the `<svg>` is missing or a scene call failed.
    pub fn build(svg_id: &str, prepared: &Prepared, run: &Run) -> Result<Self, String> {
        Self::build_with(svg_id, prepared, run, BuildOptions::default())
    }

    /// As [`build`](Self::build), with the options that make the scene readable.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the `<svg>` is missing or a scene call failed.
    pub fn build_with(svg_id: &str, prepared: &Prepared, run: &Run, options: BuildOptions) -> Result<Self, String> {
        if options.worked {
            return Self::build_worked(svg_id, prepared, run, options);
        }
        let start = now();
        let (spec, matrix) = (prepared.spec(), prepared.matrix());
        let layout = Layout::new(spec).shifted(options.origin.0, options.origin.1);
        let scene = Scene::new(SvgRoot::attach(svg_id).map_err(err("attach"))?).map_err(err("scene"))?;
        let len = spec.label_len as usize;

        let t0 = now();
        let node = |kind: char, index: usize, rect: crate::layout::Rect| {
            scene
                .add_node(Point::new(rect.x, rect.y), Size::new(rect.w, rect.h), label(kind, index, len))
                .map_err(err("node"))
        };
        let col_nodes = (0..matrix.cols)
            .map(|j| node('x', j, layout.col_node(j)))
            .collect::<Result<Vec<_>, _>>()?;
        let row_nodes = (0..matrix.rows)
            .map(|i| node('y', i, layout.row_node(i)))
            .collect::<Result<Vec<_>, _>>()?;
        let nodes_ms = now() - t0;

        let t1 = now();
        for (i, row_node) in row_nodes.iter().enumerate() {
            for (j, value) in matrix.row(i) {
                let edge = scene
                    .add_edge_with(
                        col_nodes[j],
                        *row_node,
                        ConnectorOptions::default().with_connector_type(ConnectorType::Straight),
                    )
                    .map_err(err("edge"))?;
                if options.edge_labels {
                    scene
                        .set_edge_label(edge, Some(&value.to_string()))
                        .map_err(err("edge label"))?;
                }
            }
        }
        let edges_ms = now() - t1;

        let t2 = now();
        let cells = spec.cells as usize;
        let data = |at: (f64, f64), values: &[i64]| {
            scene
                .add_data_node(
                    Point::new(at.0, at.1),
                    DataNodeContent::new(NodeValues::I64(values.to_vec()), DataFormat::Decimal)
                        .with_layout(GridLayout::Columns(layout.data_columns)),
                )
                .map_err(err("data node"))
        };
        let x_node = data(layout.x_node, &run.x[..cells])?;
        let y_node = data(layout.y_node, &run.y[..cells])?;
        let data_ms = now() - t2;

        let t3 = now();
        // The box at the right is the matrix itself, with its values, when it is small enough and the graph is made to
        // be read, and the square of its pattern otherwise.
        let (mut strip_x, mut strip_y) = (Vec::new(), Vec::new());
        let (matrix_cells, bands) = if options.readable && layout.matrix.is_some() {
            let (cells, bands, strips) = add_matrix(svg_id, prepared, run, &layout)?;
            strip_x = strips.0;
            strip_y = strips.1;
            (cells, Some(bands))
        } else {
            add_spy(svg_id, prepared, &layout)?;
            (Vec::new(), None)
        };
        let (node_x, node_y) = if options.readable && layout.single {
            add_node_values(svg_id, prepared, run, &layout)?
        } else {
            (Vec::new(), Vec::new())
        };
        let spy_ms = now() - t3;

        if options.readable {
            add_captions(svg_id, prepared, &layout)?;
        }

        let t4 = now();
        let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
        let svg = document.get_element_by_id(svg_id).ok_or("no host")?;
        let (mut edge_paths, mut node_groups) = (Vec::new(), Vec::new());
        let (mut x_cells, mut y_cells) = (Vec::new(), Vec::new());
        if options.readable {
            let content = svg
                .query_selector(":scope > g.svg-dom-graph-content")
                .map_err(js_err("content group"))?
                .ok_or("the scene has no content group")?;
            edge_paths = elements(&content, ":scope > path")?;
            if edge_paths.len() != matrix.nnz() {
                return Err(format!(
                    "the scene drew {} connectors for {} nonzeros",
                    edge_paths.len(),
                    matrix.nnz()
                ));
            }
            node_groups = elements(&content, ":scope > g")?;
            node_groups.truncate(matrix.cols + matrix.rows);
            let mut k = 0;
            for i in 0..matrix.rows {
                for (_, value) in matrix.row(i) {
                    let sign = if value < 0 { "spmv-neg" } else { "spmv-pos" };
                    edge_paths[k]
                        .set_attribute(
                            "class",
                            &format!(
                                "{} spmv-edge {sign} spmv-w{}",
                                edge_paths[k].get_attribute("class").unwrap_or_default(),
                                value.abs()
                            ),
                        )
                        .map_err(js_err("edge class"))?;
                    k += 1;
                }
            }
            for (n, group) in node_groups.iter().enumerate() {
                let tag = if n < matrix.cols {
                    Selection::Column(n)
                } else {
                    Selection::Row(n - matrix.cols)
                };
                group.set_attribute("data-spmv-node", &tag.tag()).map_err(js_err("node tag"))?;
            }
            let aside = content
                .query_selector(":scope > g.spmv-aside")
                .map_err(js_err("aside"))?
                .ok_or("the box at the right was not drawn")?;
            let y_group = aside.previous_element_sibling().ok_or("there is no y data node")?;
            let x_group = y_group.previous_element_sibling().ok_or("there is no x data node")?;
            // A data node is a title, its frame, and then a rectangle and a text for each cell.
            let cell_rects = |group: &web_sys::Element| -> Result<Vec<web_sys::Element>, String> {
                let mut rects = elements(group, "rect")?;
                if rects.len() != cells + 1 {
                    return Err(format!("a data node of {cells} cells drew {} rectangles", rects.len()));
                }
                rects.remove(0);
                Ok(rects)
            };
            x_cells = cell_rects(&x_group)?;
            y_cells = cell_rects(&y_group)?;
            svg.set_attribute(
                "class",
                &format!("{} spmv-scene", svg.get_attribute("class").unwrap_or_default()),
            )
            .map_err(js_err("scene class"))?;
        }
        let style_ms = if options.readable { now() - t4 } else { 0.0 };

        let timings = BuildTimings {
            nodes_ms,
            edges_ms,
            data_ms,
            spy_ms,
            style_ms,
            total_ms: now() - start,
        };
        Ok(Self {
            scene,
            col_nodes,
            row_nodes,
            x_node,
            y_node,
            layout,
            cells,
            svg,
            edge_paths,
            node_groups,
            picked: Vec::new(),
            lit_paths: Vec::new(),
            lit_nodes: Vec::new(),
            matrix_cells,
            bands,
            lit_cells: Vec::new(),
            strip_x,
            strip_y,
            node_x,
            node_y,
            changed_values: RefCell::new(Vec::new()),
            hot_strips: Vec::new(),
            changed_strips: RefCell::new(Vec::new()),
            x_cells,
            y_cells,
            hot: Vec::new(),
            marks: RefCell::new(Marks {
                prev_x: run.x[..cells].to_vec(),
                prev_y: run.y[..cells].to_vec(),
                changed: Vec::new(),
            }),
            worked: None,
            origin: options.origin,
            timings,
        })
    }

    /// Whether the scene was built to be read, so that a node can be selected.
    #[must_use]
    pub fn is_readable(&self) -> bool {
        self.worked.is_some() || !self.edge_paths.is_empty() || !self.node_groups.is_empty()
    }

    /// Dims every edge but the ones of the selected node, or shows them all alike.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a class could not be written.
    pub fn set_focus(&self, on: bool) -> Result<(), String> {
        if on {
            add_class(&self.svg, "spmv-focus")
        } else {
            remove_class(&self.svg, "spmv-focus")
        }
    }

    /// Highlights the edges of the selected node and the nodes at their other ends, and puts out what the last
    /// selection lit. Only the edges of the node are written, however many the scene has.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the scene was not built to be read, or a class could not be written.
    pub fn select(&mut self, selection: Option<Selection>, matrix: &Matrix) -> Result<(), String> {
        if !self.is_readable() {
            return Err("the scene was not built to be read".to_owned());
        }
        if self.worked.is_some() {
            // The worked product shows one row, which is chosen with `show_row`.
            return Ok(());
        }
        for k in std::mem::take(&mut self.lit_paths) {
            remove_class(&self.edge_paths[k], "spmv-on")?;
        }
        for n in std::mem::take(&mut self.lit_nodes) {
            remove_class(&self.node_groups[n], "spmv-on")?;
        }
        for group in std::mem::take(&mut self.picked) {
            remove_class(&group, "spmv-picked")?;
        }
        for k in std::mem::take(&mut self.lit_cells) {
            remove_class(&self.matrix_cells[k], "spmv-on")?;
        }
        for cell in std::mem::take(&mut self.hot)
            .into_iter()
            .chain(std::mem::take(&mut self.hot_strips))
        {
            remove_class(&cell, "spmv-hot")?;
        }
        if let Some((row, column)) = &self.bands {
            row.set_attribute("visibility", "hidden").map_err(js_err("band"))?;
            column.set_attribute("visibility", "hidden").map_err(js_err("band"))?;
        }
        let Some(selection) = selection else {
            return remove_class(&self.svg, "spmv-has-pick");
        };
        add_class(&self.svg, "spmv-has-pick")?;
        let (paths, nodes, own): (Vec<usize>, Vec<usize>, usize) = match selection {
            Selection::Row(i) => {
                let paths = (matrix.row_ptr[i]..matrix.row_ptr[i + 1]).collect();
                (paths, matrix.row(i).map(|(j, _)| j).collect(), matrix.cols + i)
            },
            Selection::Column(j) => {
                let paths = matrix.col(j).filter_map(|(i, _)| matrix.edge_index(i, j)).collect();
                (paths, matrix.col(j).map(|(i, _)| matrix.cols + i).collect(), j)
            },
        };
        for k in &paths {
            add_class(&self.edge_paths[*k], "spmv-on")?;
        }
        for n in &nodes {
            add_class(&self.node_groups[*n], "spmv-on")?;
        }
        add_class(&self.node_groups[own], "spmv-picked")?;
        self.picked.push(self.node_groups[own].clone());

        // The matrix, drawn in full: the nonzeros of the node are lit, and a band runs across its row or down its
        // column.
        if !self.matrix_cells.is_empty() {
            for k in &paths {
                add_class(&self.matrix_cells[*k], "spmv-on")?;
            }
            if let (Some((row_band, column_band)), Some(geom)) = (&self.bands, self.layout.matrix) {
                let (cell_w, cell_h) = (crate::layout::MATRIX_CELL.0, crate::layout::MATRIX_CELL.1);
                match selection {
                    Selection::Row(i) => {
                        let at = geom.cell(i, 0);
                        for (name, value) in [
                            ("x", at.x),
                            ("y", at.y),
                            ("width", cell_w * geom.cols as f64),
                            ("height", cell_h),
                        ] {
                            row_band.set_attribute(name, &value.to_string()).map_err(js_err("band"))?;
                        }
                        row_band.set_attribute("visibility", "visible").map_err(js_err("band"))?;
                    },
                    Selection::Column(j) => {
                        let at = geom.cell(0, j);
                        for (name, value) in [
                            ("x", at.x),
                            ("y", at.y),
                            ("width", cell_w),
                            ("height", cell_h * geom.rows as f64),
                        ] {
                            column_band.set_attribute(name, &value.to_string()).map_err(js_err("band"))?;
                        }
                        column_band.set_attribute("visibility", "visible").map_err(js_err("band"))?;
                    },
                }
            }
        }

        // The entries of x and y that the node uses or makes: a row uses the entries of x at its nonzeros and makes its
        // own entry of y, and a column is one entry of x that goes into the entries of y at its nonzeros. Only the
        // entries that a data node shows can be marked.
        let (x_hot, y_hot): (Vec<usize>, Vec<usize>) = match selection {
            Selection::Row(i) => (matrix.row(i).map(|(j, _)| j).collect(), vec![i]),
            Selection::Column(j) => (vec![j], matrix.col(j).map(|(i, _)| i).collect()),
        };
        for (strip, entries) in [(&self.node_x, &x_hot), (&self.node_y, &y_hot)] {
            for n in entries.iter().filter(|n| **n < strip.len()) {
                add_class(&strip[*n], "spmv-hot")?;
                self.hot_strips.push(strip[*n].clone());
            }
        }
        for (strip, entries) in [(&self.strip_x, &x_hot), (&self.strip_y, &y_hot)] {
            for n in entries.iter().filter(|n| **n < strip.len()) {
                add_class(&strip[*n], "spmv-hot")?;
                self.hot_strips.push(strip[*n].clone());
            }
        }
        for j in x_hot.into_iter().filter(|j| *j < self.x_cells.len()) {
            add_class(&self.x_cells[j], "spmv-hot")?;
            self.hot.push(self.x_cells[j].clone());
        }
        for i in y_hot.into_iter().filter(|i| *i < self.y_cells.len()) {
            add_class(&self.y_cells[i], "spmv-hot")?;
            self.hot.push(self.y_cells[i].clone());
        }
        self.lit_cells = if self.matrix_cells.is_empty() { Vec::new() } else { paths.clone() };
        self.lit_paths = paths;
        self.lit_nodes = nodes;
        Ok(())
    }

    #[must_use]
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    #[must_use]
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    #[must_use]
    pub fn x_node(&self) -> NodeId {
        self.x_node
    }

    #[must_use]
    pub fn y_node(&self) -> NodeId {
        self.y_node
    }

    #[must_use]
    pub fn col_node(&self, j: usize) -> NodeId {
        self.col_nodes[j]
    }

    #[must_use]
    pub fn row_node(&self, i: usize) -> NodeId {
        self.row_nodes[i]
    }

    /// Writes the windows of x and y into the two data nodes, and gives the time each took in milliseconds. The library
    /// rewrites the cells that changed.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a write failed.
    pub fn apply(&self, run: &Run) -> Result<(f64, f64), String> {
        let t0 = now();
        if let Some(worked) = &self.worked {
            worked.apply(&self.scene, run)?;
            self.mark_changes(run)?;
            return Ok((now() - t0, 0.0));
        }
        self.scene
            .set_data_values(self.x_node, NodeValues::I64(run.x[..self.cells].to_vec()))
            .map_err(err("x"))?;
        let t1 = now();
        self.scene
            .set_data_values(self.y_node, NodeValues::I64(run.y[..self.cells].to_vec()))
            .map_err(err("y"))?;
        let timed = (t1 - t0, now() - t1);
        self.mark_changes(run)?;
        Ok(timed)
    }

    /// Outlines the cells of the two data nodes that this step changed, and puts out the outlines of the last step.
    fn mark_changes(&self, run: &Run) -> Result<(), String> {
        if self.x_cells.is_empty() {
            return Ok(());
        }
        for value in self.changed_values.borrow_mut().drain(..) {
            remove_class(&value, "spmv-changed")?;
        }
        for (values, now) in [(&self.node_x, &run.x), (&self.node_y, &run.y)] {
            for (n, value) in values.iter().enumerate() {
                let new = now[n].to_string();
                if value.text_content().as_deref() != Some(new.as_str()) {
                    value.set_text_content(Some(&new));
                    add_class(value, "spmv-changed")?;
                    self.changed_values.borrow_mut().push(value.clone());
                }
            }
        }
        for cell in self.changed_strips.borrow_mut().drain(..) {
            remove_class(&cell, "spmv-changed")?;
        }
        for (strip, now) in [(&self.strip_x, &run.x), (&self.strip_y, &run.y)] {
            for (n, cell) in strip.iter().enumerate() {
                let text = cell.last_element_child().ok_or("a vector cell has no text")?;
                let new = now[n].to_string();
                if text.text_content().as_deref() != Some(new.as_str()) {
                    text.set_text_content(Some(&new));
                    add_class(cell, "spmv-changed")?;
                    self.changed_strips.borrow_mut().push(cell.clone());
                }
            }
        }
        let mut marks = self.marks.borrow_mut();
        for cell in std::mem::take(&mut marks.changed) {
            remove_class(&cell, "spmv-changed")?;
        }
        let (x, y) = (&run.x[..self.x_cells.len()], &run.y[..self.y_cells.len()]);
        for (cells, now, before) in [
            (&self.x_cells, x, marks.prev_x.clone()),
            (&self.y_cells, y, marks.prev_y.clone()),
        ] {
            for (n, cell) in cells.iter().enumerate() {
                if now[n] != before[n] {
                    add_class(cell, "spmv-changed")?;
                    marks.changed.push(cell.clone());
                }
            }
        }
        marks.prev_x = x.to_vec();
        marks.prev_y = y.to_vec();
        Ok(())
    }

    /// Builds the worked product, which draws the multiplication as the operations that do it for one row.
    fn build_worked(svg_id: &str, prepared: &Prepared, run: &Run, options: BuildOptions) -> Result<Self, String> {
        let start = now();
        let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
        let svg = document.get_element_by_id(svg_id).ok_or("no host")?;
        let matrix = prepared.matrix();
        // The first row that has something to show, since a row of zeros has nothing to add up.
        let row = (0..matrix.rows).find(|i| matrix.row(*i).next().is_some()).unwrap_or(0);
        let scene = Scene::new(SvgRoot::attach(svg_id).map_err(err("attach"))?).map_err(err("scene"))?;
        scene.set_pan_mode(InputMode::On).map_err(err("pan"))?;
        scene.set_wheel_zoom_mode(InputMode::On).map_err(err("zoom"))?;
        let worked = Worked::build(&scene, svg_id, prepared, run, row, options.origin)?;
        let total = now() - start;
        let (x_cells, y_cells) = (worked.x_cells.clone(), worked.y_cells.clone());
        add_class(&svg, "spmv-scene")?;
        Ok(Self {
            scene,
            col_nodes: Vec::new(),
            row_nodes: Vec::new(),
            x_node: worked.x_node,
            y_node: worked.y_node,
            layout: Layout::new(prepared.spec()).shifted(options.origin.0, options.origin.1),
            cells: matrix.cols.min(matrix.rows),
            svg,
            edge_paths: Vec::new(),
            node_groups: Vec::new(),
            picked: Vec::new(),
            lit_paths: Vec::new(),
            lit_nodes: Vec::new(),
            matrix_cells: Vec::new(),
            bands: None,
            lit_cells: Vec::new(),
            strip_x: Vec::new(),
            strip_y: Vec::new(),
            hot_strips: Vec::new(),
            changed_strips: RefCell::new(Vec::new()),
            node_x: Vec::new(),
            node_y: Vec::new(),
            changed_values: RefCell::new(Vec::new()),
            x_cells,
            y_cells,
            hot: Vec::new(),
            marks: RefCell::new(Marks {
                prev_x: run.x[..matrix.cols].to_vec(),
                prev_y: run.y[..matrix.rows].to_vec(),
                changed: Vec::new(),
            }),
            worked: Some(worked),
            origin: options.origin,
            timings: BuildTimings {
                nodes_ms: total,
                total_ms: total,
                ..BuildTimings::default()
            },
        })
    }

    /// The row that the worked product shows, if the scene was built that way.
    #[must_use]
    pub fn worked_row(&self) -> Option<usize> {
        self.worked.as_ref().map(|w| w.row)
    }

    /// Draws the worked product again for another row, keeping the view that a person has panned and zoomed to. The
    /// library cannot take a node away, so the scene is made again, which for a graph small enough to be worked is a
    /// few tens of nodes.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the scene was not built as a worked product or a scene call failed.
    pub fn show_row(&mut self, prepared: &Prepared, run: &Run, row: usize) -> Result<(), String> {
        if self.worked.is_none() {
            return Err("the scene was not built as a worked product".to_owned());
        }
        let view = self.scene.view();
        let id = self.svg.id();
        // What the library made goes, and the backdrop that the window drew under it stays.
        let children = self.svg.children();
        for n in (0..children.length()).rev() {
            if let Some(child) = children.item(n) {
                if !child.class_list().contains("spmv-backdrop") {
                    child.remove();
                }
            }
        }
        let scene = Scene::new(SvgRoot::attach(&id).map_err(err("attach"))?).map_err(err("scene"))?;
        scene.set_pan_mode(InputMode::On).map_err(err("pan"))?;
        scene.set_wheel_zoom_mode(InputMode::On).map_err(err("zoom"))?;
        let worked = Worked::build(&scene, &id, prepared, run, row, self.origin)?;
        scene.set_view(view).map_err(err("view"))?;
        self.x_cells = worked.x_cells.clone();
        self.y_cells = worked.y_cells.clone();
        *self.marks.borrow_mut() = Marks {
            prev_x: run.x[..self.x_cells.len()].to_vec(),
            prev_y: run.y[..self.y_cells.len()].to_vec(),
            changed: Vec::new(),
        };
        self.scene = scene;
        self.worked = Some(worked);
        Ok(())
    }

    /// Moves column node `j` by `(dx, dy)`, which redraws every connector on it, and gives the time in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the move failed.
    pub fn move_col(&self, j: usize, dx: f64, dy: f64) -> Result<f64, String> {
        let at = self.scene.node_rect(self.col_nodes[j]).map_err(err("rect"))?.origin;
        let t0 = now();
        self.scene
            .move_node(self.col_nodes[j], Point::new(at.x + dx, at.y + dy))
            .map_err(err("move"))?;
        Ok(now() - t0)
    }
}
