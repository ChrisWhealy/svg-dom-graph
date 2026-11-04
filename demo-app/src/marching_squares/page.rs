//! The page: the scene that is drawn, the inspector, the controls, and the focus. It is what the controller draws on.
//!
//! The scene is a real `Scene` with one plot node, drawn at the grid's own coordinates, and it is the part that zooms
//! and pans. It is built in a hidden `<svg>` that is in the document, so that it can be measured, and it is swapped in
//! with one call only when all of it has been built. The controls are in the same stage but outside it, so they stay
//! one size at any zoom, and a swap never touches them.

use super::{
    chrome::{Chrome, MAX_COLUMNS, MAX_ROWS},
    controller::{CommitError, DrawKind, Meta, Surface},
    dom::{Journal, add, make, set},
    field::{FieldId, LEVELS, SLOT_COLOURS, SLOT_DASHES, SLOT_NAMES, SLOT_PATTERNS},
    layout::{BUTTONS, PADDING, SCENE_HEIGHT, STAGE_WIDTH, cell_at_point, is_drag, panned},
    projection::{CellView, Inspected, Projection, State, level_text, project},
    solver::{Decision, Trace},
};
use demo_common::util::stringify;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        LegendEntry, PlotAxis, PlotConfig, PlotItem, PlotLine, PlotPoints, Scene, Side, ToolbarOptions, ViewTransform,
    },
};
use wasm_bindgen::{JsCast, JsValue};

/// Makes a line of text at a place, with an anchor, a size and the words.
type TextMaker<'a> = dyn Fn(f64, f64, &str, f64, &str) -> Result<web_sys::Element, String> + 'a;

/// What a click on the plot asks for: the point, in the grid's coordinates.
type ClickHandler = Rc<dyn Fn(f64, f64)>;

/// The width of a contour, and so the width of the dot at the seam of a loop.
const LINE_WIDTH: f64 = 2.5;
const CELL_OUTLINE: &str = "#be123c";
const AMBIGUOUS_OUTLINE: &str = "#7c3aed";
const SAMPLE_COLOURS: [&str; 3] = ["#94a3b8", "#cbd5e1", "#e2e8f0"];

/// One scene: the group that is swapped in, the scene, and its plot.
struct Live {
    root: web_sys::Element,
    scene: Scene,
    plot: NodeId,
    /// The width and the height of the grid that the scene's axes are for.
    size: (usize, usize),
}

/// Why the one call that swaps the scene in failed.
struct ReplaceError {
    problem: String,
    /// Whether the old scene is back.
    restored: bool,
}

pub(super) struct Page {
    document: web_sys::Document,
    chrome: Chrome,
    live: RefCell<Option<Live>>,
    instance: u32,
    /// Counted up for each scene that is built, so that no ID is ever used twice.
    built: Cell<u64>,
    said: Cell<u32>,
    error_shown: Cell<bool>,
    click: RefCell<Option<ClickHandler>>,
    /// The view of a scene that was swapped out and could not be put back, which belongs to the committed state, so the
    /// scene that is built again from it takes it.
    lost_view: Cell<Option<ViewTransform>>,
    /// A press on the plot that may become a pan.
    pan: Cell<Option<Pan>>,
    /// Whether the click that follows a pan is to be swallowed, so that panning does not choose a cell.
    swallow_click: Cell<bool>,
}

/// A press on the plot: where it went down, the view then, and how big a unit of the scene is on the screen.
#[derive(Debug, Clone, Copy)]
struct Pan {
    from: (f64, f64),
    view: ViewTransform,
    per_unit: f64,
    dragged: bool,
}

impl Page {
    pub(super) fn new(document: &web_sys::Document, instance: u32, last: usize) -> Result<Self, String> {
        let fields: Vec<(String, String)> = FieldId::all()
            .into_iter()
            .map(|f| (f.key().to_owned(), f.label().to_owned()))
            .collect();
        Ok(Self {
            document: document.clone(),
            chrome: Chrome::build(document, &fields, last)?,
            live: RefCell::new(None),
            instance,
            built: Cell::new(0),
            said: Cell::new(0),
            error_shown: Cell::new(false),
            click: RefCell::new(None),
            lost_view: Cell::new(None),
            pan: Cell::new(None),
            swallow_click: Cell::new(false),
        })
    }

    pub(super) fn chrome(&self) -> &Chrome {
        &self.chrome
    }

    /// Sets what a click on the plot does. It is held by the scene's listener, so it must hold the controller weakly.
    pub(super) fn set_click_handler(&self, handler: ClickHandler) {
        *self.click.borrow_mut() = Some(handler);
    }

    /// A press went down at `x`, `y` on `target`. If that is on the plot, and not on the toolbar or the control that
    /// the keyboard pans with, it may become a pan.
    ///
    /// The plot fills the scene, so the surface behind it that the library pans with is never reached. The pan is done
    /// here, by setting the view, which is the same view that the toolbar and the keyboard change.
    pub(super) fn pan_start(&self, target: &web_sys::Element, x: f64, y: f64) {
        self.swallow_click.set(false);
        let on_control = target
            .closest("[aria-label=\"Scene controls\"], [aria-label^=\"Graph view\"]")
            .ok()
            .flatten()
            .is_some();
        if on_control || !self.chrome.host.contains(Some(target)) {
            return;
        }
        let Some(view) = self.live.borrow().as_ref().map(|l| l.scene.view()) else {
            return;
        };
        let per_unit = self.chrome.stage_svg.get_bounding_client_rect().width() / STAGE_WIDTH;
        self.pan.set(Some(Pan {
            from: (x, y),
            view,
            per_unit,
            dragged: false,
        }));
    }

    /// The pointer moved to `x`, `y`. If a press is down and has moved far enough, the view follows it.
    pub(super) fn pan_move(&self, x: f64, y: f64) {
        let Some(mut pan) = self.pan.get() else { return };
        let (dx, dy) = (x - pan.from.0, y - pan.from.1);
        if !pan.dragged && !is_drag(dx, dy) {
            return;
        }
        pan.dragged = true;
        self.pan.set(Some(pan));
        let (tx, ty) = panned(pan.view.tx, pan.view.ty, dx, dy, pan.per_unit);
        let moved = self
            .live
            .borrow()
            .as_ref()
            .map(|l| l.scene.set_view(ViewTransform { tx, ty, ..pan.view }));
        if let Some(Err(problem)) = moved {
            // The view that was set stays, and the press is let go, so that one failure does not repeat on every move.
            self.pan.set(None);
            self.warn_text(&format!("The view could not be moved: {problem}"));
        }
    }

    /// The press was let go. If it was a pan, the click that follows is swallowed.
    pub(super) fn pan_end(&self) {
        if let Some(pan) = self.pan.take() {
            self.swallow_click.set(pan.dragged);
        }
    }

    /// Whether this click is the end of a pan, which must not choose a cell. Asking clears it.
    pub(super) fn take_swallowed_click(&self) -> bool {
        self.swallow_click.replace(false)
    }

    fn warn_text(&self, text: &str) {
        self.show_error(&format!("Warning: {text}"));
    }

    /// The zoom of the scene on the page, as a percentage.
    pub(super) fn zoom_percent(&self) -> Option<f64> {
        self.live.borrow().as_ref().map(|l| (l.scene.view().scale * 100.0).round())
    }

    /// Writes the zoom into the footer, which a person changes with the toolbar and the wheel and not with a control.
    pub(super) fn refresh_zoom(&self) {
        if let Some(percent) = self.zoom_percent() {
            self.chrome.fields[5].set_text_content(Some(&format!("{percent}%")));
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The items of the plot for `projection`, drawn in order so that a later one is over an earlier one.
    fn items(trace: &Trace, state: &State, projection: &Projection) -> Vec<PlotItem> {
        let (w, h) = (trace.grid.width, trace.grid.height);
        let mut items: Vec<PlotItem> = Vec::new();
        // The samples, in three bands by value against the lowest and the highest level.
        let mut bands: [Vec<(f64, f64)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for r in 0..h {
            for c in 0..w {
                let v = trace.grid.at(r, c);
                let band = if v < state.levels[0] {
                    0
                } else if v > state.levels[LEVELS - 1] {
                    2
                } else {
                    1
                };
                bands[band].push((c as f64, (h - 1 - r) as f64));
            }
        }
        for (band, points) in bands.into_iter().enumerate() {
            if !points.is_empty() {
                items.push(PlotPoints::new(points, SAMPLE_COLOURS[band]).with_radius(2.2).into());
            }
        }
        let outline = |(r, c): (usize, usize), colour: &str, width: f64| -> PlotItem {
            let (x, top) = (c as f64, (h - 1 - r) as f64);
            let corners = [(x, top), (x + 1.0, top), (x + 1.0, top - 1.0), (x, top - 1.0), (x, top)];
            PlotLine::new(corners, colour).with_width(width).into()
        };
        for &cell in &projection.ambiguous {
            items.push(outline(cell, AMBIGUOUS_OUTLINE, 2.0));
        }
        for line in &projection.lines {
            let mut item = PlotLine::new(line.points.iter().copied(), SLOT_COLOURS[line.slot]).with_width(LINE_WIDTH);
            if let Some(dash) = SLOT_DASHES[line.slot] {
                item = item.with_dash(dash);
            }
            items.push(item.into());
            // A loop repeats its first point, which leaves two butt ends meeting at the seam and a notch on the outside
            // of the bend. A dot as wide as the line, in its colour, fills it, as a round join would.
            if let (true, Some(&seam)) = (line.closed, line.points.first()) {
                let colour = SLOT_COLOURS[line.slot];
                items.push(
                    PlotPoints::new([seam], colour)
                        .with_radius(LINE_WIDTH / 2.0)
                        .with_outline(colour)
                        .into(),
                );
            }
        }
        if let Some(cell) = projection.cell {
            items.push(outline(cell, CELL_OUTLINE, 3.0));
        }
        items
    }

    fn config(trace: &Trace, projection: &Projection) -> PlotConfig {
        let (w, h) = (trace.grid.width as f64, trace.grid.height as f64);
        let interval = if w <= 9.0 { 1.0 } else { 4.0 };
        let axis = |max: f64, title: &str| {
            PlotAxis::linear(-PADDING, max - 1.0 + PADDING)
                .with_title(title)
                .with_major_interval(interval)
        };
        let legend = (0..LEVELS)
            .map(|s| {
                let mut entry = LegendEntry::line(format!("{}, {}", SLOT_NAMES[s], SLOT_PATTERNS[s]), SLOT_COLOURS[s]);
                if let Some(dash) = SLOT_DASHES[s] {
                    entry = entry.with_dash(dash);
                }
                entry
            })
            .collect();
        PlotConfig::new(
            Size::new(STAGE_WIDTH, SCENE_HEIGHT),
            axis(w, "column"),
            axis(h, "height (the top row is highest)"),
        )
        .with_legend(legend)
        .with_description(projection.description.clone())
    }

    /// Builds the scene of `state` in the hidden `<svg>`, and checks it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if anything could not be made. What was made is taken out again.
    fn build_scene(&self, trace: &Trace, state: &State, projection: &Projection) -> Result<Live, String> {
        let n = self.built.get() + 1;
        self.built.set(n);
        let scene_id = format!("ms-{}-g{n}-scene", self.instance);
        let root = make(&self.document, "g", &[("class", "ms-scene-root")])?;
        add(&self.chrome.staging, &root)?;
        match self.fill_scene(&root, &scene_id, trace, state, projection) {
            Ok((scene, plot)) => Ok(Live {
                root,
                scene,
                plot,
                size: (trace.grid.width, trace.grid.height),
            }),
            Err(problem) => {
                root.remove();
                Err(problem)
            },
        }
    }

    fn fill_scene(
        &self,
        root: &web_sys::Element,
        scene_id: &str,
        trace: &Trace,
        state: &State,
        projection: &Projection,
    ) -> Result<(Scene, NodeId), String> {
        let (width, height) = (STAGE_WIDTH.to_string(), SCENE_HEIGHT.to_string());
        let view_box = format!("0 0 {width} {height}");
        let svg = make(
            &self.document,
            "svg",
            &[
                ("id", scene_id),
                ("class", "ms-scene"),
                ("x", "0"),
                ("y", "0"),
                ("width", &width),
                ("height", &height),
                ("viewBox", &view_box),
            ],
        )?;
        add(root, &svg)?;
        let scene = Scene::new(SvgRoot::attach(scene_id).map_err(stringify)?).map_err(stringify)?;
        let plot = scene
            .add_plot_node(Point::new(0.0, 0.0), Self::config(trace, projection))
            .map_err(stringify)?;
        scene
            .set_plot_items(plot, &Self::items(trace, state, projection))
            .map_err(stringify)?;
        scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
        if let Some(handler) = self.click.borrow().clone() {
            // The scene holds the closure, and the closure holds the controller weakly.
            scene
                .on_plot_click(plot, move |_, _, click| handler(click.x, click.y))
                .map_err(stringify)?;
        }
        Ok((scene, plot))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces what is in the host with `root`, in the one call the browser has for it. If the call throws, the host
    /// may be untouched or already changed, so what was there is taken out again and put back, as far as that works.
    /// The call is made by name, so that a test can make the real one throw.
    fn replace_scene(&self, root: &web_sys::Element) -> Result<(), ReplaceError> {
        let host = &self.chrome.host;
        let mut old = Vec::new();
        let mut child = host.first_child();
        while let Some(node) = child {
            child = node.next_sibling();
            old.push(node);
        }
        let called = js_sys::Reflect::get(host, &JsValue::from_str("replaceChildren"))
            .and_then(|function| function.dyn_into::<js_sys::Function>())
            .and_then(|function| function.call1(host, root));
        let Err(problem) = called else {
            return Ok(());
        };
        let mut restored = true;
        while let Some(node) = host.first_child() {
            if host.remove_child(&node).is_err() {
                restored = false;
                break;
            }
        }
        for node in &old {
            restored &= host.append_child(node).is_ok();
        }
        Err(ReplaceError {
            problem: format!("the scene could not be swapped in: {problem:?}"),
            restored,
        })
    }

    /// The accessible name of the control inside the scene that has the focus, if one has.
    fn focus_label(&self) -> Option<String> {
        let active = self.document.active_element()?;
        if !self.chrome.host.contains(Some(&active)) {
            return None;
        }
        active
            .closest("[aria-label]")
            .ok()
            .flatten()
            .and_then(|e| e.get_attribute("aria-label"))
    }

    /// Puts the focus on the control of the new scene with the name `label`, and checks that it took.
    fn restore_focus(&self, label: &str) -> Result<(), String> {
        let found = self.chrome.host.query_selector_all("[aria-label]").ok().and_then(|list| {
            (0..list.length()).filter_map(|i| list.item(i)).find_map(|n| {
                n.dyn_into::<web_sys::Element>()
                    .ok()
                    .filter(|e| e.get_attribute("aria-label").as_deref() == Some(label))
            })
        });
        let target = found.ok_or_else(|| format!("the control {label:?} is not in the new scene"))?;
        if let Some(element) = target.dyn_ref::<web_sys::SvgElement>() {
            let _ = element.focus();
        }
        match self.document.active_element() {
            Some(active) if self.chrome.host.contains(Some(&active)) => Ok(()),
            _ => Err("the focus could not be put back".to_owned()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes every control for `state`, through `journal`.
    fn write_controls(
        &self,
        journal: &mut Journal,
        trace: &Trace,
        state: &State,
        projection: &Projection,
        meta: Meta,
    ) -> Result<(), String> {
        let c = &self.chrome;
        let (last, step) = (trace.grid.cell_count(), state.step);
        let dimmed = |action: &str| match action {
            "restart" | "previous" => step == 0,
            "next" => step >= last,
            _ => false,
        };
        // A dimmed button is `aria-disabled` and keeps its place in the tab order, and a press of it is still heard.
        for (&(action, ..), button) in BUTTONS.iter().zip(&c.buttons) {
            let off = dimmed(action);
            journal.attr(button, "aria-disabled", if off { "true" } else { "false" })?;
            journal.attr(
                button,
                "style",
                if off { "cursor: default; opacity: 0.45;" } else { "cursor: pointer;" },
            )?;
        }
        journal.select(&c.field, state.field.key());
        journal.select(&c.show, state.show.key());
        journal.select(&c.inspect, &state.inspected.to_string());
        journal.input(&c.threshold, &format!("{}", state.levels[state.inspected]));
        let cell = |v: Option<usize>| v.map_or_else(String::new, |v| v.to_string());
        journal.select(&c.row, &cell(state.selected.map(|s| s.0)));
        journal.select(&c.column, &cell(state.selected.map(|s| s.1)));
        journal.attr(&c.seek, "max", &last.to_string())?;
        journal.input(&c.seek, &step.to_string());
        let text = format!("{step} of {last} cells");
        journal.attr(&c.seek, "aria-valuetext", &text)?;
        journal.text(&c.seek_text, &text);
        let totals: String = (0..LEVELS)
            .map(|s| {
                let t = projection.totals[s];
                format!(
                    "{} {}: {} open, {} closed, {} ties",
                    SLOT_NAMES[s],
                    level_text(state.levels[s]),
                    t.open,
                    t.closed,
                    t.ties
                )
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let legend: String = (0..LEVELS)
            .map(|s| format!("{} {} ({})", SLOT_NAMES[s], level_text(state.levels[s]), SLOT_PATTERNS[s]))
            .collect::<Vec<_>>()
            .join(", ");
        let completeness = if projection.complete {
            "The contour set is complete."
        } else {
            "The contour set is partial: an open end may be provisional, on the edge of the cells visited."
        };
        let selected = state.selected.map_or_else(|| "none".to_owned(), |(r, c)| format!("({r}, {c})"));
        let footer = format!(
            "{} {}×{} · visited {step}/{last} · selected {selected} · segments {} · generation {} · token {} · digest {:08x}",
            state.field.label(),
            trace.grid.width,
            trace.grid.height,
            projection.segments,
            meta.generation,
            meta.token,
            projection.digest >> 32,
        );
        let fields = [
            format!("Step {step} of {last}"),
            projection.status.clone(),
            totals,
            format!("{legend}. {completeness}"),
            footer,
        ];
        for (field, text) in c.fields.iter().zip(&fields) {
            journal.text(field, text);
        }
        Ok(())
    }

    /// Enables the options of the row and column selectors that the field has, and no others. These are written afresh
    /// on every draw, so a write that fails is put right by the next.
    fn write_options(&self, trace: &Trace) -> Result<(), String> {
        let (rows, columns) = (trace.grid.height - 1, trace.grid.cells_across());
        for (select, count, max) in [(&self.chrome.row, rows, MAX_ROWS), (&self.chrome.column, columns, MAX_COLUMNS)] {
            let options = select.query_selector_all("option").map_err(|e| format!("{e:?}"))?;
            for i in 0..max {
                if let Some(option) = options.item(i as u32 + 1).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                    if i < count {
                        option.remove_attribute("disabled").map_err(|e| format!("{e:?}"))?;
                    } else {
                        set(&option, "disabled", "")?;
                    }
                }
            }
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The picture of the inspected cell, enlarged, in a group that is not yet in the page.
    fn inspector_picture(&self, trace: &Trace, projection: &Projection) -> Result<web_sys::Element, String> {
        let d = &self.document;
        let group = make(d, "g", &[])?;
        let text = |x: f64, y: f64, anchor: &str, size: f64, body: &str| -> Result<web_sys::Element, String> {
            let t = make(
                d,
                "text",
                &[
                    ("x", &x.to_string()),
                    ("y", &y.to_string()),
                    ("text-anchor", anchor),
                    ("font-size", &size.to_string()),
                    ("fill", "currentColor"),
                ],
            )?;
            t.set_text_content(Some(body));
            Ok(t)
        };
        match &projection.inspected {
            Inspected::NoCell => add(&group, &text(110.0, 120.0, "middle", 14.0, "No cell selected")?)?,
            Inspected::NotYet { row, col } => {
                add(&group, &text(110.0, 112.0, "middle", 14.0, &format!("Cell ({row}, {col})"))?)?;
                add(&group, &text(110.0, 134.0, "middle", 14.0, "Not yet evaluated")?)?;
            },
            Inspected::Cell(view) => self.draw_cell(&group, trace, view, &text)?,
        }
        Ok(group)
    }

    fn draw_cell(
        &self,
        group: &web_sys::Element,
        trace: &Trace,
        view: &CellView,
        text: &TextMaker,
    ) -> Result<(), String> {
        let d = &self.document;
        let (left, top, size) = (40.0, 44.0, 140.0);
        let (row, col) = (view.row, view.col);
        let local = |x: f64, y: f64| {
            (
                left + (x - col as f64) * size,
                top + ((trace.grid.height - 1 - row) as f64 - y) * size,
            )
        };
        add(
            group,
            &make(
                d,
                "rect",
                &[
                    ("x", &left.to_string()),
                    ("y", &top.to_string()),
                    ("width", &size.to_string()),
                    ("height", &size.to_string()),
                    ("fill", "none"),
                    ("stroke", "currentColor"),
                    ("stroke-width", "1"),
                ],
            )?,
        )?;
        let colour = SLOT_COLOURS[view.slot];
        for s in &view.result.segments {
            let ((x1, y1), (x2, y2)) = (local(s.a.point.x, s.a.point.y), local(s.b.point.x, s.b.point.y));
            let mut attributes = vec![
                ("x1", x1.to_string()),
                ("y1", y1.to_string()),
                ("x2", x2.to_string()),
                ("y2", y2.to_string()),
                ("stroke", colour.to_owned()),
                ("stroke-width", "3".to_owned()),
            ];
            if let Some(dash) = SLOT_DASHES[view.slot] {
                attributes.push(("stroke-dasharray", dash.to_owned()));
            }
            let attrs: Vec<(&str, &str)> = attributes.iter().map(|(a, b)| (*a, b.as_str())).collect();
            add(group, &make(d, "line", &attrs)?)?;
        }
        for (_, (x, y)) in &view.crossings {
            let (cx, cy) = local(*x, *y);
            add(
                group,
                &make(
                    d,
                    "circle",
                    &[("cx", &cx.to_string()), ("cy", &cy.to_string()), ("r", "4"), ("fill", colour)],
                )?,
            )?;
        }
        if view.result.decision == Some(Decision::Tie) {
            let s = view.result.segments[0].b.point;
            let (cx, cy) = local(s.x, s.y);
            add(
                group,
                &make(
                    d,
                    "circle",
                    &[
                        ("cx", &cx.to_string()),
                        ("cy", &cy.to_string()),
                        ("r", "6"),
                        ("fill", "none"),
                        ("stroke", AMBIGUOUS_OUTLINE),
                        ("stroke-width", "2"),
                    ],
                )?,
            )?;
        }
        let [tl, tr, br, bl] = view.samples;
        let level = view.level;
        for (value, x, y, anchor, below) in [
            (tl, left, top, "end", false),
            (tr, left + size, top, "start", false),
            (br, left + size, top + size, "start", true),
            (bl, left, top + size, "end", true),
        ] {
            let above = value > level;
            add(
                group,
                &make(
                    d,
                    "circle",
                    &[
                        ("cx", &x.to_string()),
                        ("cy", &y.to_string()),
                        ("r", "5"),
                        ("fill", if above { "currentColor" } else { "none" }),
                        ("stroke", "currentColor"),
                        ("stroke-width", "1.5"),
                    ],
                )?,
            )?;
            let dx = if anchor == "end" { -8.0 } else { 8.0 };
            let dy = if below { 14.0 } else { -6.0 };
            add(group, &text(x + dx, y + dy, anchor, 11.0, &format!("{value:.3}"))?)?;
        }
        add(
            group,
            &text(
                110.0,
                20.0,
                "middle",
                13.0,
                &format!("Cell ({row}, {col}) · mask {}", view.result.mask),
            )?,
        )?;
        let caption = match (view.result.q, view.result.rho) {
            (Some(q), Some(rho)) => format!("Q {q:.3e} · rho {rho:.3}"),
            _ => "no decider".to_owned(),
        };
        add(group, &text(110.0, 214.0, "middle", 12.0, &caption)?)?;
        let level_line = format!("{} {}", SLOT_NAMES[view.slot], level_text(view.level));
        add(group, &text(110.0, 230.0, "middle", 12.0, &level_line)?)?;
        Ok(())
    }

    /// Puts `picture` in the inspector in place of what it held, in the one call the browser has for it.
    fn show_picture(&self, picture: &web_sys::Element) -> Result<(), String> {
        let target = &self.chrome.inspector;
        js_sys::Reflect::get(target, &JsValue::from_str("replaceChildren"))
            .and_then(|function| function.dyn_into::<js_sys::Function>())
            .and_then(|function| function.call1(target, picture))
            .map(|_| ())
            .map_err(|e| format!("the inspector could not be drawn: {e:?}"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn hide_error(&self) -> Result<(), String> {
        if self.error_shown.get() {
            set(&self.chrome.error, "hidden", "")?;
            self.chrome.error.set_text_content(None);
            self.error_shown.set(false);
        }
        Ok(())
    }

    fn show_error(&self, message: &str) -> bool {
        self.chrome.error.set_text_content(Some(message));
        let shown = self.chrome.error.remove_attribute("hidden").is_ok();
        self.error_shown.set(true);
        shown
    }

    fn report(&self, message: &str) {
        if self.show_error(message) {
            self.say(message);
        } else {
            self.say(&format!("{message} The error line could not be shown."));
        }
    }

    fn draw_step(&self, trace: &Trace, state: &State, meta: Meta) -> Result<(), CommitError> {
        // A step only replaces the items. If the scene on the page is for another grid, as it is after a swap that
        // could not be put back, its axes are wrong for these items, so the whole scene is built again from the state.
        let size = self.live.borrow().as_ref().map(|l| l.size);
        if size != Some((trace.grid.width, trace.grid.height)) {
            return self.draw_scene(trace, state, true, meta);
        }
        let projection = project(trace, state);
        let picture = self.inspector_picture(trace, &projection).map_err(CommitError::Detached)?;
        let (plot, scene) = {
            let live = self.live.borrow();
            let live = live
                .as_ref()
                .ok_or_else(|| CommitError::Detached("there is no scene".to_owned()))?;
            (live.plot, live.scene.clone())
        };
        let mut journal = Journal::default();
        if let Err(problem) = self.write_controls(&mut journal, trace, state, &projection, meta) {
            let whole = journal.restore();
            return Err(commit_error(whole, put_back(problem, whole)));
        }
        if let Err(problem) = scene.set_plot_items(plot, &Self::items(trace, state, &projection)) {
            let whole = journal.restore();
            return Err(commit_error(whole, put_back(stringify(problem), whole)));
        }
        self.finish(&scene, plot, trace, &projection, &picture)
    }

    /// The writes that follow the plot and cannot be put back: the description, the options, the inspector and the
    /// error line. A failure of any is reported, and the page is dirty.
    fn finish(
        &self,
        scene: &Scene,
        plot: NodeId,
        trace: &Trace,
        projection: &Projection,
        picture: &web_sys::Element,
    ) -> Result<(), CommitError> {
        scene
            .set_plot_description(plot, &projection.description)
            .map_err(|e| CommitError::Live(stringify(e)))?;
        self.write_options(trace).map_err(CommitError::Live)?;
        self.show_picture(picture).map_err(CommitError::Live)?;
        self.refresh_zoom();
        self.hide_error().map_err(CommitError::Live)
    }

    fn draw_scene(&self, trace: &Trace, state: &State, keep_view: bool, meta: Meta) -> Result<(), CommitError> {
        let projection = project(trace, state);
        let picture = self.inspector_picture(trace, &projection).map_err(CommitError::Detached)?;
        let candidate = self.build_scene(trace, state, &projection).map_err(CommitError::Detached)?;
        let discard = |candidate: &Live| candidate.root.remove();
        // The view is carried over before the swap, so that a view that cannot be set never replaces a working one. It
        // is the view of the scene on the page, if that is for the same axes, or else the one that was lost with a swap
        // that could not be put back.
        let lost = self.lost_view.get();
        let live_view: Option<(ViewTransform, (usize, usize))> =
            self.live.borrow().as_ref().map(|l| (l.scene.view(), l.size));
        let same_axes = live_view.is_some_and(|(_, size)| size == candidate.size);
        let carried = match (lost, live_view) {
            (Some(view), _) => Some(view),
            (None, Some((view, _))) if keep_view && same_axes => Some(view),
            _ => None,
        };
        if let Some(view) = carried {
            if let Err(problem) = candidate.scene.set_view(view) {
                discard(&candidate);
                return Err(CommitError::Detached(stringify(problem)));
            }
        }
        let label = self.focus_label();
        let mut journal = Journal::default();
        if let Err(problem) = self.write_controls(&mut journal, trace, state, &projection, meta) {
            let whole = journal.restore();
            discard(&candidate);
            return Err(commit_error(whole, put_back(problem, whole)));
        }
        if let Err(failure) = self.replace_scene(&candidate.root) {
            let whole = journal.restore();
            discard(&candidate);
            return Err(commit_error(failure.restored && whole, put_back(failure.problem, whole)));
        }
        let (scene, plot) = (candidate.scene.clone(), candidate.plot);
        // The scene that was swapped out is kept until everything after the swap has worked, so that it can be put
        // back.
        let previous = self.live.replace(Some(candidate));
        let after = match label {
            Some(label) => self.restore_focus(&label),
            None => Ok(()),
        }
        .map_err(CommitError::Live)
        .and_then(|()| self.finish(&scene, plot, trace, &projection, &picture));
        let Err(CommitError::Live(problem)) = after else {
            self.lost_view.set(None);
            drop(previous);
            return after;
        };
        Err(CommitError::Live(self.roll_back(previous, &mut journal, lost, problem)))
    }

    /// Puts the scene that was swapped out back, and the controls with it, after a write that followed the swap failed.
    ///
    /// The page is left dirty either way, so that the next action draws it again from the committed state. If the old
    /// scene is back, that is a step on it. If it cannot be put back, the scene on the page is for the wrong grid, and
    /// the next draw builds the committed one again, with the view that the old scene had.
    fn roll_back(
        &self,
        previous: Option<Live>,
        journal: &mut Journal,
        lost: Option<ViewTransform>,
        problem: String,
    ) -> String {
        let controls = journal.restore();
        let Some(previous) = previous else {
            return format!("{problem}, and there was no earlier scene to put back");
        };
        let view = previous.scene.view();
        match self.replace_scene(&previous.root) {
            Ok(()) => {
                let removed = self.live.replace(Some(previous));
                drop(removed);
                self.lost_view.set(lost);
                format!(
                    "{problem}, and the earlier scene was put back{}",
                    if controls { "" } else { ", but not all of its controls" }
                )
            },
            Err(failure) => {
                self.lost_view.set(Some(lost.unwrap_or(view)));
                format!("{problem}, and the earlier scene could not be put back: {}", failure.problem)
            },
        }
    }

    /// Draws the first scene, which has no earlier one to carry anything over from.
    pub(super) fn first(&self, trace: &Trace, state: &State, meta: Meta) -> Result<(), CommitError> {
        self.draw_scene(trace, state, false, meta)
    }
}

/// The cell that a click at `x`, `y` is in.
pub(super) fn cell_of_click(trace: &Trace, x: f64, y: f64) -> Option<(usize, usize)> {
    cell_at_point(x, y, trace.grid.width, trace.grid.height)
}

/// Adds to `problem` that the controls could not all be put back, if that is so.
fn put_back(problem: String, whole: bool) -> String {
    if whole {
        problem
    } else {
        format!("{problem}, and the controls could not all be put back")
    }
}

fn commit_error(whole: bool, problem: String) -> CommitError {
    if whole { CommitError::Detached(problem) } else { CommitError::Live(problem) }
}

impl Surface for Page {
    fn draw(&self, trace: &Trace, state: &State, kind: DrawKind, meta: Meta) -> Result<(), CommitError> {
        match kind {
            DrawKind::Step => self.draw_step(trace, state, meta),
            DrawKind::Scene { keep_view } => self.draw_scene(trace, state, keep_view, meta),
        }
    }

    fn sync_controls(&self, state: &State) -> Result<(), String> {
        let c = &self.chrome;
        c.field.set_value(state.field.key());
        c.show.set_value(state.show.key());
        c.inspect.set_value(&state.inspected.to_string());
        c.threshold.set_value(&format!("{}", state.levels[state.inspected]));
        let cell = |v: Option<usize>| v.map_or_else(String::new, |v| v.to_string());
        c.row.set_value(&cell(state.selected.map(|s| s.0)));
        c.column.set_value(&cell(state.selected.map(|s| s.1)));
        Ok(())
    }

    fn say(&self, text: &str) {
        // A message that repeats the last one is still read out, since the text differs by an invisible space.
        let n = self.said.get();
        self.said.set(n.wrapping_add(1));
        let tail = if n % 2 == 0 { "" } else { "\u{a0}" };
        self.chrome.live.set_text_content(Some(&format!("{text}{tail}")));
    }

    fn failed(&self, problem: &str) {
        self.report(&format!(
            "That could not be done: {problem}. What is shown may be out of date. The next button, selector or move of the range draws it again."
        ));
    }

    fn warn(&self, text: &str) {
        self.show_error(&format!("Warning: {text}"));
    }
}
