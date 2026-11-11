//! What a person does to the scene and the controls: selecting, picking, stepping, dragging, zooming and rebuilding.

use super::{
    ERROR, PANEL, READABLE_UP_TO, SCALES, WINDOW, ZOOM_STEP,
    build::{BuildOutcome, build, focus_wanted, make_scene, teardown},
    controls::{body_elements, order_of, selected},
    frames::injected,
    readout::{describe, latency, megabytes, n, story, story_intro, text, timed},
};
use demo_common::util::{document, report_error};
use spmv_scene::{
    probe::timer_step,
    scene::{Selection, heap_used, now},
};
use svg_dom_graph::scene::ViewTransform;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// Turns a click on a node of the scene into a selection, and gives the listener, which the scene keeps.
pub(super) fn scene_svg_click(scene_svg: &web_sys::Element) -> Result<Closure<dyn FnMut(web_sys::Event)>, String> {
    let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        let tag = event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|e| e.closest("[data-spmv-node]").ok().flatten())
            .and_then(|e| e.get_attribute("data-spmv-node"));
        if let Some(selection) = tag.as_deref().and_then(Selection::parse) {
            if let Err(message) = choose(Some(selection)) {
                report_error(ERROR, &message);
            }
        }
    });
    scene_svg
        .add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
        .map_err(|e| format!("could not listen for a click on a node: {e:?}"))?;
    Ok(listener)
}

/// Selects a node, or none: its edges are lit, and the arithmetic it stands for is written out.
pub(super) fn choose(selection: Option<Selection>) -> Result<(), String> {
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        let live = p.live.as_mut().ok_or("Nothing is built yet.")?;
        if !live.built.is_readable() {
            return Ok(());
        }
        if live.built.worked_row().is_some() {
            // The multiplication shows one row, and a click on anything else leaves it as it is.
            if let Some(Selection::Row(i)) = selection {
                live.built.show_row(&live.prepared, &live.run, i)?;
                live.selection = selection;
            }
        } else {
            live.built.select(selection, live.prepared.matrix())?;
            live.selection = selection;
        }
        let selection = live.selection;
        text("spmv-selected", &describe(live));
        if let Some(list) = document()?.get_element_by_id("spmv-pick") {
            if let Some(list) = list.dyn_ref::<web_sys::HtmlSelectElement>() {
                let tag = selection.map(Selection::tag).unwrap_or_default();
                // A node beyond the first of each side is not in the list, and then the list says none.
                list.set_value(&tag);
                if list.value() != tag {
                    list.set_value("");
                }
            }
        }
        Ok(())
    })
}

/// Selects the node that a person typed, which can be any of them, and not only the first 100 that the list holds, so
/// that a node beyond those is reached from the keyboard too.
pub(super) fn goto_changed() -> Result<(), String> {
    let input = document()?
        .get_element_by_id("spmv-goto")
        .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
        .ok_or("the box for a node is missing")?;
    let tag = input.value().trim().to_lowercase();
    if tag.is_empty() {
        return Ok(());
    }
    let selection =
        Selection::parse(&tag).ok_or_else(|| "Type a node as x or y and its number, such as x250 or y7.".to_owned())?;
    let (cols, rows, worked, readable) = PANEL
        .with(|p| {
            let p = p.borrow();
            p.live.as_ref().map(|l| {
                let m = l.prepared.matrix();
                (m.cols, m.rows, l.built.worked_row().is_some(), l.built.is_readable())
            })
        })
        .ok_or("Nothing is built yet.")?;
    let last = |n: usize| n.saturating_sub(1);
    match selection {
        Selection::Column(j) if j >= cols => {
            return Err(format!("There is no {tag}: the columns are x0 to x{}.", last(cols)));
        },
        Selection::Row(i) if i >= rows => {
            return Err(format!("There is no {tag}: the rows are y0 to y{}.", last(rows)));
        },
        Selection::Column(_) if worked => {
            return Err(format!(
                "This view shows one row at a time, so choose a row: y0 to y{}.",
                last(rows)
            ));
        },
        _ if !readable => {
            return Err(format!(
                "Selecting a node is off above {} nonzeros, which the scene does not tag.",
                n(READABLE_UP_TO as u64)
            ));
        },
        _ => {},
    }
    choose(Some(selection))
}

pub(super) fn pick_changed() -> Result<(), String> {
    let tag = selected("spmv-pick")?;
    choose(Selection::parse(&tag))
}

pub(super) fn edges_changed() -> Result<(), String> {
    PANEL.with(|p| {
        let p = p.borrow();
        let live = p.live.as_ref().ok_or("Nothing is built yet.")?;
        if live.built.is_readable() {
            live.built.set_focus(focus_wanted(live.prepared.matrix().nnz()))?;
        }
        Ok(())
    })
}

/// One step: the updates, the product kept up to date, and the two data nodes written.
pub(super) fn step() -> Result<(), String> {
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        let live = p.live.as_mut().ok_or("Nothing is built yet.")?;
        if injected("__spmvFailNextStep") {
            return Err("A failure was injected into this step.".to_owned());
        }
        let start = now();
        let before = live.built.is_readable().then(|| (live.run.x.clone(), live.run.y.clone()));
        live.run.advance(live.verified).map_err(|e| e.to_string())?;
        live.built.apply(&live.run)?;
        if let Some((x, y)) = before {
            let mut said = story(live.run.step, &x, &y, &live.run.x, &live.run.y);
            if selected("spmv-order").as_deref() == Ok("inorder") {
                let window = (live.prepared.spec().cells as usize).min(live.run.x.len()).max(1);
                if let Some((last, _)) = live.run.last_updates.last() {
                    said.push_str(&format!(
                        " Inputs change in order, so the next step changes x{}.",
                        (last + 1) % window
                    ));
                }
            }
            text("spmv-story", &said);
        }
        let took = now() - start;
        if live.steps.len() == WINDOW {
            live.steps.remove(0);
        }
        live.steps.push(took);
        text(
            "spmv-n-step",
            &format!("Step {}: {}", live.run.step, latency(&live.steps, live.clock_step)),
        );
        // The vector has changed, so what a selected node computes has too.
        if live.selection.is_some() {
            text("spmv-selected", &describe(live));
        }
        Ok(())
    })
}

pub(super) fn order_changed() -> Result<(), String> {
    let order = order_of(&selected("spmv-order")?);
    PANEL.with(|p| {
        if let Some(live) = p.borrow_mut().live.as_mut() {
            live.run.set_order(order);
        }
    });
    text("spmv-story", story_intro(order));
    Ok(())
}

/// Moves the node with the most edges back and forth, which redraws every connector on it each time.
pub(super) fn drag() -> Result<(), String> {
    PANEL.with(|p| {
        let p = p.borrow();
        let live = p.live.as_ref().ok_or("Nothing is built yet.")?;
        if live.built.worked_row().is_some() {
            text(
                "spmv-n-drag",
                "Dragging a column is for the graph view of a larger matrix. Here you can drag any node yourself.",
            );
            return Ok(());
        }
        let matrix = live.prepared.matrix();
        let busiest = (0..matrix.cols).max_by_key(|j| matrix.col(*j).count()).unwrap_or(0);
        let degree = matrix.col(busiest).count();
        // The moves are timed together and divided, since one move can be shorter than a step of the page's clock.
        let moves = 30_u32;
        let start = now();
        for k in 0..moves {
            let d = if k % 2 == 0 { 7.0 } else { -7.0 };
            live.built.move_col(busiest, d, 3.0 * d)?;
        }
        let each = (now() - start) / f64::from(moves);
        text(
            "spmv-n-drag",
            &format!(
                "Column x{busiest} has {} edges: {moves} moves took {} each. A frame is 16.7 ms. You can also drag any node yourself.",
                n(degree as u64),
                timed(each, live.clock_step / f64::from(moves))
            ),
        );
        Ok(())
    })
}

/// Builds the scenario and tears it down ten times, and says whether the page ended where it began.
pub(super) fn rebuild() -> Result<(), String> {
    // A request that is refused builds nothing and leaves the graph that is there, and there is nothing here to tear
    // down and build again.
    if build()? == BuildOutcome::Refused {
        return Ok(());
    }
    let Some(prepared) = PANEL.with(|p| p.borrow().live.as_ref().map(|l| l.prepared.clone())) else {
        return Ok(());
    };
    teardown()?;
    let document = document()?;
    let before = body_elements()?;
    let heap_before = heap_used();
    let mut builds = Vec::new();
    for _ in 0..10 {
        let (svg, built) = make_scene(&document, &prepared, &prepared.run())?;
        builds.push(built.timings.total_ms);
        drop(built);
        svg.element.remove();
    }
    let after = body_elements()?;
    let message = format!(
        "10 builds and teardowns: the page had {} elements before and {} after{}; heap {} before, {} after (the collector decides when it falls). A build took {}.",
        n(u64::from(before)),
        n(u64::from(after)),
        if before == after {
            ", so nothing was left behind"
        } else {
            ", so something was left behind"
        },
        megabytes(heap_before),
        megabytes(heap_used()),
        latency(&builds, timer_step())
    );
    // The last build clears the readouts, so the result is written after it.
    build()?;
    text("spmv-n-rebuild", &message);
    Ok(())
}

/// Zooms by `factor` about the centre of the scene's room, so that what is in the middle stays there.
pub(super) fn zoom(factor: f64) -> Result<(), String> {
    PANEL.with(|p| {
        let p = p.borrow();
        let live = p.live.as_ref().ok_or("Nothing is built yet.")?;
        let view = live.built.scene().view();
        let scale = (view.scale * factor).clamp(SCALES.0, SCALES.1);
        let ratio = scale / view.scale;
        let (cx, cy) = (live.view_size.0 / 2.0, live.view_size.1 / 2.0);
        live.built
            .scene()
            .set_view(ViewTransform {
                scale,
                tx: cx - ratio * (cx - view.tx),
                ty: cy - ratio * (cy - view.ty),
            })
            .map_err(|e| format!("could not zoom: {e}"))
    })
}

pub(super) fn zoom_in() -> Result<(), String> {
    zoom(ZOOM_STEP)
}

pub(super) fn zoom_out() -> Result<(), String> {
    zoom(1.0 / ZOOM_STEP)
}

/// Puts the view back to the whole picture, as it was built.
pub(super) fn reset_view() -> Result<(), String> {
    PANEL.with(|p| {
        let p = p.borrow();
        let live = p.live.as_ref().ok_or("Nothing is built yet.")?;
        live.built
            .scene()
            .set_view(ViewTransform { scale: 1.0, tx: 0.0, ty: 0.0 })
            .map_err(|e| format!("could not reset the view: {e}"))
    })
}
