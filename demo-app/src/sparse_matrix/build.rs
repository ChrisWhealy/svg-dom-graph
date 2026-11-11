//! Building the scene for the controls as they are, and taking it down again.

use super::shell::{SceneSvg, add_scene_svg};
use super::{
    ERROR, LABELS_UP_TO, PANEL, PICKABLE, READABLE_UP_TO, VERIFY_UP_TO,
    controls::{body_elements, order_of, selected, set_class, set_list, spec_from_controls},
    frames::stop_run,
    interaction::{choose, scene_svg_click},
    live::Live,
    readout::{megabytes, ms, n, story_intro, task, text},
};
use demo_common::util::{clear_error, document};
use spmv_core::{Prepared, Run, budget::Budget, spec::Error};
use spmv_scene::{
    layout::Layout,
    probe::timer_step,
    scene::{BuildOptions, Built, Selection, heap_used},
    worked,
};
use svg_dom_graph::scene::InputMode;

/// Removes the scene and its `<svg>`, in that order, and its listener with it.
pub(super) fn teardown() -> Result<(), String> {
    stop_run();
    let live = PANEL.with(|p| p.borrow_mut().live.take());
    let Some(live) = live else { return Ok(()) };
    let Live { scene_svg, built, click, .. } = live;
    drop(built);
    scene_svg.remove();
    drop(click);
    text("spmv-selected", "Click a node, or choose one above, to see what it computes.");
    set_pick_options(0, 0);
    Ok(())
}

/// What the scene is built with, for a scenario of `nonzeros` nonzeros that is to begin at `origin`.
pub(super) fn options_for(prepared: &Prepared, origin: (f64, f64)) -> BuildOptions {
    let nonzeros = prepared.matrix().nnz();
    BuildOptions {
        readable: nonzeros <= READABLE_UP_TO,
        edge_labels: nonzeros <= LABELS_UP_TO,
        origin,
        worked: is_worked(prepared),
    }
}

/// Whether the multiplication is drawn as the operations that do it, which is for a matrix small enough to be read.
pub(super) fn is_worked(prepared: &Prepared) -> bool {
    let spec = prepared.spec();
    spec.rows <= worked::MAX_SIDE && spec.cols <= worked::MAX_SIDE
}

/// Whether only the selected node's edges are shown, from the list of edges and the size of the scenario.
pub(super) fn focus_wanted(nonzeros: usize) -> bool {
    match selected("spmv-edges").as_deref() {
        Ok("focus") => true,
        Ok("all") => false,
        _ => nonzeros > LABELS_UP_TO,
    }
}

/// Fills the list of nodes to highlight with the first of each side.
pub(super) fn set_pick_options(columns: usize, rows: usize) {
    let Some(list) = document().ok().and_then(|d| d.get_element_by_id("spmv-pick")) else {
        return;
    };
    let mut html = String::from("<option value=\"\" selected>None</option>");
    for j in 0..columns.min(PICKABLE) {
        html.push_str(&format!("<option value=\"x{j}\">x{j}, a column</option>"));
    }
    for i in 0..rows.min(PICKABLE) {
        html.push_str(&format!("<option value=\"y{i}\">y{i}, a row</option>"));
    }
    list.set_inner_html(&html);
}

pub(super) fn make_scene(
    document: &web_sys::Document,
    prepared: &Prepared,
    run: &Run,
) -> Result<(SceneSvg, Built), String> {
    let shell = PANEL
        .with(|p| p.borrow().shell.clone())
        .ok_or("the window has not been built")?;
    let layout = Layout::new(prepared.spec());
    let extent = if is_worked(prepared) {
        worked::extent(prepared.matrix())
    } else {
        (layout.width, layout.height)
    };
    let id = PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.svgs += 1;
        format!("spmv-svg-{}", p.svgs)
    });
    let svg = add_scene_svg(document, &shell, &id, extent, layout.single || is_worked(prepared))?;
    let made = Built::build_with(&id, prepared, run, options_for(prepared, svg.origin)).and_then(|built| {
        // The library's own toolbar is placed from the rendered size of a nested `<svg>`, which is the size of
        // everything it draws, and not of its room. So there is none, and the window's own buttons zoom. Dragging the
        // background pans, and a wheel turned with Ctrl or Cmd held zooms about the pointer.
        let scene = built.scene();
        scene
            .set_pan_mode(InputMode::On)
            .map_err(|e| format!("could not turn on panning: {e}"))?;
        scene
            .set_wheel_zoom_mode(InputMode::On)
            .map_err(|e| format!("could not turn on zooming by the wheel: {e}"))?;
        Ok(built)
    });
    match made {
        Ok(built) => Ok((svg, built)),
        Err(problem) => {
            svg.element.remove();
            Err(problem)
        },
    }
}

/// Builds the scenario the controls describe, or says why it was refused. What a request to build came to. A request
/// that is refused is not a failure, and it leaves the graph that was there, so a caller that goes on to tear that
/// graph down has to know that nothing new was built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BuildOutcome {
    Built,
    Refused,
}

pub(super) fn build() -> Result<BuildOutcome, String> {
    clear_error(ERROR);
    let spec = spec_from_controls()?;
    let budget = Budget::default();
    // The budget is checked, and the matrix generated, before the scene that is there is touched and before a single
    // scene call, so a scenario that is refused leaves the graph that a person has exactly as it was.
    let prepared = match Prepared::new(&spec, &budget) {
        Ok(p) => p,
        Err(Error::Refused(r)) => {
            let had_one = PANEL.with(|p| p.borrow().live.is_some());
            text(
                "spmv-verdict",
                &format!(
                    "Refused before anything was built: {}. {}",
                    r.message,
                    if had_one { "The graph you had is unchanged." } else { "Nothing was built." }
                ),
            );
            set_class("spmv-verdict", "spmv-verdict spmv-refused");
            if !had_one {
                text(
                    "spmv-n-plan",
                    &format!("Refused ({}). No node, edge or data node was made.", r.code),
                );
            }
            return Ok(BuildOutcome::Refused);
        },
        Err(e) => return Err(e.to_string()),
    };
    teardown()?;
    for id in [
        "spmv-n-plan", "spmv-n-page", "spmv-n-build", "spmv-n-step", "spmv-n-check", "spmv-n-drag", "spmv-n-rebuild",
    ] {
        text(id, "—");
    }
    let mut run = prepared.run();
    // A matrix that is worked is for watching, so its steps go in order, and a measured one keeps the contract's random
    // order.
    let order = if is_worked(&prepared) { "inorder" } else { "random" };
    set_list("spmv-order", order);
    set_list("spmv-speed", if is_worked(&prepared) { "1500" } else { "0" });
    run.set_order(order_of(order));
    let document = document()?;
    let before = body_elements()?;
    let heap_before = heap_used();
    let (svg, built) = make_scene(&document, &prepared, &run)?;
    let after = body_elements()?;
    let plan = *prepared.plan();
    let t = built.timings;
    let nonzeros = prepared.matrix().nnz();
    let verified = nonzeros <= VERIFY_UP_TO;
    let clock_step = timer_step();
    if built.is_readable() {
        built.set_focus(focus_wanted(nonzeros))?;
    }

    let click = scene_svg_click(&svg.element)?;
    set_pick_options(
        if is_worked(&prepared) { 0 } else { prepared.matrix().cols },
        prepared.matrix().rows,
    );
    text(
        "spmv-clock",
        &format!(
            "This page's clock steps in {}, so a time shorter than that is told as under one step, and not as zero.",
            ms(clock_step)
        ),
    );
    text(
        "spmv-verdict",
        &format!(
            "Accepted: {} nodes and {} edges, built in {}.",
            n(plan.nodes),
            n(plan.edges),
            ms(t.total_ms)
        ),
    );
    set_class("spmv-verdict", "spmv-verdict");
    text(
        "spmv-task",
        &task(
            prepared.spec().rows,
            prepared.spec().cols,
            prepared.matrix().nnz(),
            is_worked(&prepared),
        ),
    );
    text("spmv-story", story_intro(order_of(order)));
    text(
        "spmv-n-plan",
        &format!(
            "{} nodes, {} edges, {} data cells, the busiest node has {} edges. Estimated {} page elements, of a budget of {}.",
            n(plan.nodes),
            n(plan.edges),
            n(plan.cells),
            n(plan.max_degree),
            n(plan.est_dom),
            n(budget.max_dom_elements)
        ),
    );
    text(
        "spmv-n-page",
        &format!(
            "{} added to the page; the estimate was {}{}.",
            n(u64::from(after - before)),
            n(plan.est_dom),
            if nonzeros <= LABELS_UP_TO {
                ", which leaves out the labels on the edges"
            } else {
                ""
            }
        ),
    );
    text(
        "spmv-n-build",
        &format!(
            "{} in all: nodes {}, edges {}, data nodes {}, pattern square {}, readable styling {}.",
            ms(t.total_ms),
            ms(t.nodes_ms),
            ms(t.edges_ms),
            ms(t.data_ms),
            ms(t.spy_ms),
            ms(t.style_ms)
        ),
    );
    text(
        "spmv-n-heap",
        &format!("{} before, {} after.", megabytes(heap_before), megabytes(heap_used())),
    );
    text(
        "spmv-n-check",
        &if verified {
            "Every step is checked against a full recomputation of the product.".to_owned()
        } else {
            format!(
                "Not checked: over {} nonzeros the check is skipped, so that it does not time the step.",
                n(VERIFY_UP_TO as u64)
            )
        },
    );
    let readable = built.is_readable();
    let first_row = built.worked_row();
    PANEL.with(|p| {
        p.borrow_mut().live = Some(Live {
            scene_svg: svg.element,
            view_size: svg.view_size,
            built,
            prepared,
            run,
            verified,
            steps: Vec::new(),
            clock_step,
            selection: first_row.map(Selection::Row),
            click: Some(click),
        });
    });
    if let Some(row) = first_row {
        choose(Some(Selection::Row(row)))?;
    }
    if !readable {
        text(
            "spmv-selected",
            &format!(
                "Selecting a node is off above {} nonzeros, which the scene does not tag.",
                n(READABLE_UP_TO as u64)
            ),
        );
    }
    Ok(BuildOutcome::Built)
}
