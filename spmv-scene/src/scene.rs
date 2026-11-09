//! The scene: one node for each column and each row, one edge for each nonzero, two data nodes that show windows of x and y, and a
//! square that shows where the nonzeros are.
//!
//! A scene is built only from a [`Prepared`] scenario, which exists only if the specification was valid and within the budget, so a
//! budget is enforced before any scene call by the types. Nothing here changes the shape of the scene after it is built: a step
//! writes the two data nodes and nothing else.

use crate::layout::{Layout, MATRIX_CELL, MATRIX_LABELS, SPY_SIDE};
use crate::worked::Worked;
use spmv_core::matrix::{Matrix, label};
use spmv_core::{Prepared, Run};
use std::cell::RefCell;
use svg_dom::SvgRoot;
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::NodeId;
use svg_dom_graph::scene::{
    ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, InputMode, NodeValues, Scene,
};
use wasm_bindgen::JsCast;

const SVG_NS: &str = "http://www.w3.org/2000/svg";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) fn err<E: std::fmt::Display>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) fn js_err<E: std::fmt::Debug>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e:?}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The page's clock in milliseconds. Its step is 100 µs on a page that is not cross-origin isolated, so a single small operation
/// can read as zero, and the probes time a batch where that matters.
#[must_use]
pub fn now() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or(0.0, |p| p.now())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The bytes of JS heap in use, where the browser says (Chrome does, and Firefox does not).
#[must_use]
pub fn heap_used() -> Option<f64> {
    let performance = web_sys::window()?.performance()?;
    let memory = js_sys::Reflect::get(&performance, &"memory".into()).ok()?;
    js_sys::Reflect::get(&memory, &"usedJSHeapSize".into()).ok()?.as_f64()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many elements `root` holds below it.
///
/// # Errors
///
/// Returns `Err` if the page could not be searched.
pub fn element_count(root: &web_sys::Element) -> Result<u32, String> {
    Ok(root.query_selector_all("*").map_err(js_err("count"))?.length())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An `<svg>` in a `<div>` of its own, which is what a scene is built on and what a teardown removes.
pub struct Host {
    pub container_id: String,
    pub svg_id: String,
    /// Whether the host made its container, and so removes it. A host made in a container that was already in the page removes
    /// only its own `<svg>`.
    owns_container: bool,
}

impl Host {
    /// Makes the host in a `<div>` of its own in the document body, with a view box of `extent`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the elements could not be made.
    pub fn create(id: &str, extent: (f64, f64)) -> Result<Self, String> {
        let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
        let container_id = format!("{id}-container");
        let div = document.create_element("div").map_err(js_err("div"))?;
        div.set_id(&container_id);
        document.body().ok_or("no body")?.append_child(&div).map_err(js_err("append"))?;
        let mut host = Self::create_in(&container_id, id, extent)?;
        host.owns_container = true;
        Ok(host)
    }

    /// Makes the host's `<svg>` in the element called `container_id`, which is already in the page. Removing the host removes only
    /// the `<svg>`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the container is missing or the `<svg>` could not be made.
    pub fn create_in(container_id: &str, id: &str, extent: (f64, f64)) -> Result<Self, String> {
        // The viewport has the aspect of the layout, so that a wide, short graph is not drawn small in a tall box.
        let height = (900.0 * extent.1 / extent.0.max(1.0)).clamp(240.0, 1400.0);
        let svg = SvgRoot::create_in(container_id, Size::new(900.0, height)).map_err(err("svg"))?;
        svg.root.set_id(id);
        svg.set_view_box(0.0, 0.0, extent.0.max(1.0), extent.1.max(1.0))
            .map_err(err("view box"))?;
        Ok(Self {
            container_id: container_id.to_owned(),
            svg_id: id.to_owned(),
            owns_container: false,
        })
    }

    /// The `<svg>`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if it is no longer in the document.
    pub fn svg(&self) -> Result<web_sys::Element, String> {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(&self.svg_id))
            .ok_or_else(|| format!("no element #{}", self.svg_id))
    }

    /// Removes what the host made: the container and everything in it, or, for a host made in a container of the page's, only its
    /// `<svg>`.
    pub fn remove(&self) {
        let id = if self.owns_container { &self.container_id } else { &self.svg_id };
        if let Some(element) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(id))
        {
            element.remove();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How long each part of a build took, in milliseconds.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildTimings {
    pub nodes_ms: f64,
    pub edges_ms: f64,
    pub data_ms: f64,
    pub spy_ms: f64,
    /// The time to give the edges and nodes the classes that a person reads them by, which is nothing if that is not asked for.
    pub style_ms: f64,
    pub total_ms: f64,
}

/// What a build adds to the scene so that a person can read it. The probes use neither, so that the figures they record are those of the
/// scene alone.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildOptions {
    /// Gives every edge a class for its sign and its size and every node a tag, so that a node can be selected and its edges
    /// highlighted.
    pub readable: bool,
    /// Writes the value of each edge on it. Every label is more elements, so this is for a small graph.
    pub edge_labels: bool,
    /// Where the scene's own space begins, which moves the layout and not the view box (see [`Layout::shifted`]).
    pub origin: (f64, f64),
    /// Draws the multiplication as the operations that do it, for the row that is selected, and not as a graph (see [`crate::worked`]).
    pub worked: bool,
}

/// A node that is selected: a column, which feeds rows, or a row, which is fed by columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Column(usize),
    Row(usize),
}

impl Selection {
    /// The tag a node carries, which is what an event is traced back to a node by: `x7` for column 7 and `y3` for row 3.
    #[must_use]
    pub fn tag(self) -> String {
        match self {
            Self::Column(j) => format!("x{j}"),
            Self::Row(i) => format!("y{i}"),
        }
    }

    /// The selection that a tag names, if it names one.
    #[must_use]
    pub fn parse(tag: &str) -> Option<Self> {
        let (kind, number) = tag.split_at_checked(1)?;
        let n: usize = number.parse().ok()?;
        match kind {
            "x" => Some(Self::Column(n)),
            "y" => Some(Self::Row(n)),
            _ => None,
        }
    }
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
    /// The connectors, in the order of the nonzeros by row, and the groups of the column nodes and then the row nodes. Both are
    /// empty unless the build was asked to be readable.
    edge_paths: Vec<web_sys::Element>,
    node_groups: Vec<web_sys::Element>,
    picked: Vec<web_sys::Element>,
    lit_paths: Vec<usize>,
    lit_nodes: Vec<usize>,
    /// The cells of the matrix that were drawn in full, in the order of the nonzeros by row, and the band across a row and the band down
    /// a column. They are empty unless the matrix was small enough to be drawn in full.
    matrix_cells: Vec<web_sys::Element>,
    bands: Option<(web_sys::Element, web_sys::Element)>,
    lit_cells: Vec<usize>,
    /// The values of x, in a row under the matrix, and of y, in a column beside it, as groups of a rectangle and a text.
    strip_x: Vec<web_sys::Element>,
    strip_y: Vec<web_sys::Element>,
    /// The value of each node, written beside it: the entries of x at the left of the column nodes and of y at the right of the rows.
    node_x: Vec<web_sys::Element>,
    node_y: Vec<web_sys::Element>,
    changed_values: RefCell<Vec<web_sys::Element>>,
    hot_strips: Vec<web_sys::Element>,
    changed_strips: RefCell<Vec<web_sys::Element>>,
    /// The rectangles of the cells of the two data nodes, which the library draws, and the ones that a selection has made hot.
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

/// What a step marks: the windows as they were, so that the cells that differ can be told, and the cells marked.
#[derive(Default)]
struct Marks {
    prev_x: Vec<i64>,
    prev_y: Vec<i64>,
    changed: Vec<web_sys::Element>,
}

impl Built {
    /// Builds the scene of a prepared scenario on the `<svg>` called `svg_id`, which has to exist and be empty. The two data nodes
    /// start with the values of `run`.
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
        // The box at the right is the matrix itself, with its values, when it is small enough and the graph is made to be read, and
        // the square of its pattern otherwise.
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

    /// Highlights the edges of the selected node and the nodes at their other ends, and puts out what the last selection lit. Only
    /// the edges of the node are written, however many the scene has.
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

        // The matrix, drawn in full: the nonzeros of the node are lit, and a band runs across its row or down its column.
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

        // The entries of x and y that the node uses or makes: a row uses the entries of x at its nonzeros and makes its own entry of y, and
        // a column is one entry of x that goes into the entries of y at its nonzeros. Only the entries that a data node shows can be marked.
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

    /// Writes the windows of x and y into the two data nodes, and gives the time each took in milliseconds. The library rewrites
    /// the cells that changed.
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

    /// Draws the worked product again for another row, keeping the view that a person has panned and zoomed to. The library cannot
    /// take a node away, so the scene is made again, which for a graph small enough to be worked is a few tens of nodes.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements that match `selector` below `root`, in document order.
pub(crate) fn elements(root: &web_sys::Element, selector: &str) -> Result<Vec<web_sys::Element>, String> {
    let found = root.query_selector_all(selector).map_err(js_err("select"))?;
    Ok((0..found.length())
        .filter_map(|i| found.get(i))
        .filter_map(|n| n.dyn_into().ok())
        .collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The words under the three boxes that say what each is, in the scene, so that they pan and zoom with it. A caption is a few short lines,
/// each short enough to sit under the box it names.
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
    // The example is the first nonzero of the matrix, so that the caption says something that can be checked on the picture.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the value of each node beside it, the entry of x at the left of a column node and the entry of y at the right of a row node,
/// so that what a step changes is seen on the graph and not only in the boxes above it.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The matrix drawn in full: a grid with the index of each row and column, a cell with its value for each nonzero, and nothing for a
/// zero, so that the grid stays quiet. It also makes the two bands, hidden until a row or a column is selected, that run across it.
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
    // The vectors, lined up with the matrix: x is a row under it, one value beneath each column, and y a column beside it, one value
    // level with each row, so that a row of A is multiplied by the row of x and its sum is the entry of y at its end.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The square that shows where the nonzeros are: one `<path>` however many there are, in a group in the scene's content group.
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
