//! The surface in a real browser: E0 on the page, deltas against full redraws, the data node on every slice, a view and
//! a focus that a draw leaves alone, and the content group that the overlay lives in. Run with `wasm-pack test
//! --headless --chrome service-sim`.
#![cfg(target_arch = "wasm32")]

use service_sim::projection::{DATA_CELLS, Projection, View, extract, project, writes};
use service_sim::surface::{DrawError, FaultPoint, Surface};
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Config, Engine, Limits, ScriptEvent};
use svg_dom::SvgRoot;
use svg_dom::root::utils::Size;
use svg_dom_graph::scene::ViewTransform;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

const FIXTURES: &str = include_str!("../../dess/s1/fixtures.json");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check(ok: bool, what: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(what.to_owned()) }
}

fn document() -> web_sys::Document {
    web_sys::window().and_then(|w| w.document()).expect("a document")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An empty `<svg>` of the size the surface wants, in a `<div>` of its own.
fn host(id: &str) -> Result<(), String> {
    let (w, h) = Surface::view_box();
    let container = format!("{id}-container");
    let div = document().create_element("div").map_err(|e| format!("{e:?}"))?;
    div.set_id(&container);
    document()
        .body()
        .ok_or("no body")?
        .append_child(&div)
        .map_err(|e| format!("{e:?}"))?;
    let svg = SvgRoot::create_in(&container, Size::new(w, h)).map_err(|e| e.to_string())?;
    svg.root.set_id(id);
    svg.set_view_box(0.0, 0.0, w, h).map_err(|e| e.to_string())
}

fn surface(id: &str, servers: usize) -> Result<Surface, String> {
    host(id)?;
    Surface::build(id, servers)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The projection after the snapshots of a fixture, each extracted for the view that `view_at` gives for that
/// snapshot's index.
fn projections_with(name: &str, view_at: impl Fn(usize) -> View) -> (usize, Vec<Projection>) {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("fixtures");
    let s = Scenario::fixture(name, &fixtures[name]).expect("a fixture");
    let mut engine = Engine::new(&s.config_id, s.seed, s.config, &s.script, s.params.as_ref(), Limits::default())
        .expect("an engine");
    let one = |engine: &Engine, k: usize| {
        let view = view_at(k);
        project(&extract(engine, &view), &view)
    };
    let mut all = vec![one(&engine, 0)];
    while engine.step().expect("a slice").is_some() {
        all.push(one(&engine, all.len()));
    }
    (s.config.servers, all)
}

fn e0_projections(view: &View) -> (usize, Vec<Projection>) {
    projections_with("E0", |_| *view)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// After each of E0's 12 slices, and before the first, the page holds exactly what the projection says, and the data
/// node holds the slice ordinal. The comparison is of what the elements say, read back, with the model nowhere in it.
#[wasm_bindgen_test]
fn e0_is_on_the_page_exactly_after_every_slice() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut s = surface("ss-e0", servers)?;
    for (k, p) in projections.iter().enumerate() {
        let drawn = s.draw(p)?;
        check(drawn.data_set, &format!("the data setter was not called on slice {k}"))?;
        check(
            s.snapshot()? == p.canonical(),
            &format!("the page differs from the projection at slice {k}"),
        )?;
        let cells = s.data_texts()?;
        check(cells.len() == DATA_CELLS, &format!("{} data cells at slice {k}", cells.len()))?;
        let want: Vec<String> = p.data.iter().map(ToString::to_string).collect();
        check(cells == want, &format!("data cells {cells:?}, wanted {want:?}, at slice {k}"))?;
        check(
            cells[0] == k.to_string(),
            &format!("the ordinal cell says {} at slice {k}", cells[0]),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A page reached by deltas is the page that a fresh surface draws in full, at every slice. A seek is a delta across
/// any distance.
#[wasm_bindgen_test]
fn a_page_reached_by_deltas_equals_a_page_drawn_in_full() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut walker = surface("ss-delta", servers)?;
    let order: Vec<usize> = (0..13).chain((0..12).rev()).chain([5, 12, 0, 7, 3, 3]).collect();
    for (step, k) in order.into_iter().enumerate() {
        walker.draw(&projections[k])?;
        let mut fresh = surface(&format!("ss-full-{step}"), servers)?;
        fresh.draw(&projections[k])?;
        check(
            walker.snapshot()? == fresh.snapshot()?,
            &format!("step {step}, slice {k}: delta and full differ"),
        )?;
        let again = walker.redraw(&projections[k])?;
        check(
            again.writes == projections[k].slots.len() * 4,
            "a full redraw writes every field of every slot",
        )?;
        check(
            walker.snapshot()? == fresh.snapshot()?,
            &format!("step {step}: a redraw changed the page"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A delta writes exactly the fields that differ, so a stale-only slice writes four, and the first draw writes
/// everything.
#[wasm_bindgen_test]
fn a_delta_writes_only_what_differs() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut s = surface("ss-writes", servers)?;
    check(
        s.draw(&projections[9])?.writes == projections[9].slots.len() * 4,
        "the first draw is a full one",
    )?;
    let stale_only = s.draw(&projections[10])?;
    check(
        stale_only.writes == 4,
        &format!("a stale-only slice wrote {} fields, and 4 were expected", stale_only.writes),
    )?;
    check(stale_only.data_set, "the setter is called on a stale-only slice")?;
    let counted = writes(Some(&projections[10]), &projections[11]).len();
    check(
        s.draw(&projections[11])?.writes == counted,
        "the surface wrote a different number than the projection's delta",
    )?;
    check(
        s.draw(&projections[11])?.writes == 0,
        "drawing the same slice again wrote fields",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An endurance test of the setter and the overlay: 1,000 commits go through the one data node and the one overlay, and
/// each changes the node's content, leaves the page equal to what was drawn, and adds no element.
///
/// It is not a sequence of simulation states. It cycles through E0's 13 projections and overwrites the ordinal cell
/// with the commit number, so the cell no longer matches the clock slot. The test that follows it draws 1,200 genuine
/// slices.
#[wasm_bindgen_test]
fn the_setter_endures_a_thousand_commits_with_no_new_element() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut s = surface("ss-thousand", servers)?;
    s.draw(&projections[0])?;
    let elements = s.element_count()?;
    let mut previous_ordinal = None;
    for commit in 0..1000_usize {
        // Walk forwards through the run, and give each commit its own ordinal so that every write changes the node's
        // content.
        let mut p = projections[commit % projections.len()].clone();
        p.data[0] = (commit + 1) as u64;
        let drawn = s.draw(&p)?;
        check(drawn.data_set, &format!("the setter was not called on commit {commit}"))?;
        let ordinal = s.data_texts()?[0].clone();
        check(
            previous_ordinal.as_ref() != Some(&ordinal),
            &format!("commit {commit} did not change the ordinal cell"),
        )?;
        check(
            ordinal == (commit + 1).to_string(),
            &format!("the ordinal cell says {ordinal} at commit {commit}"),
        )?;
        check(
            s.snapshot()? == p.canonical(),
            &format!("the page differs from its projection at commit {commit}"),
        )?;
        check(
            s.element_count()? == elements,
            &format!("the element count changed at commit {commit}"),
        )?;
        previous_ordinal = Some(ordinal);
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// 1,200 consecutive slices of a generated run, each projected from the engine and drawn as it comes, with nothing in a
/// projection changed by hand. After every slice the page is what the projection says, and the data node and the clock
/// are what the engine's own slice record says, which the projection did not produce. The run is the frozen
/// `mixed_outage/7` at the manifest seed.
#[wasm_bindgen_test]
fn twelve_hundred_genuine_slices_of_a_generated_run_are_drawn_exactly() -> Result<(), String> {
    const SLICES: u64 = 1_200;
    let scenario = Scenario::mixed_outage_7(20_261_106).map_err(|e| e.to_string())?;
    let mut engine = Engine::new(
        &scenario.config_id,
        scenario.seed,
        scenario.config,
        &scenario.script,
        scenario.params.as_ref(),
        Limits::default(),
    )
    .map_err(|e| e.to_string())?;
    let servers = scenario.config.servers;
    // A job is selected at every slice, and it changes, so that the highlight moves through real jobs as they come and
    // go.
    let view_at = |slice: u64| View { selected: Some(slice % 40) };
    let mut s = surface("ss-genuine", servers)?;
    let first = view_at(0);
    s.draw(&project(&extract(&engine, &first), &first))?;
    let elements = s.element_count()?;

    let (mut saw_down, mut saw_busy, mut saw_queue, mut saw_retry, mut saw_rejection) =
        (false, false, false, false, false);
    let mut previous = 0;
    while previous < SLICES {
        let slice = engine
            .step()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("the run ended after {previous} slices"))?;
        check(
            slice.index == previous + 1,
            &format!("slice {} followed slice {previous}", slice.index),
        )?;
        previous = slice.index;

        let view = view_at(slice.index);
        let p = project(&extract(&engine, &view), &view);
        let drawn = s.draw(&p)?;
        check(drawn.data_set, &format!("the setter was not called on slice {}", slice.index))?;
        check(
            s.snapshot()? == p.canonical(),
            &format!("the page differs from its projection at slice {}", slice.index),
        )?;

        // What the engine's own record says, which the projection did not produce.
        let cells = s.data_texts()?;
        let wanted = [slice.index, slice.tick, slice.waiting as u64, slice.busy as u64];
        check(
            cells.iter().take(4).map(String::as_str).eq(wanted
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .iter()
                .map(String::as_str)),
            &format!(
                "data cells {:?}, and the slice record says {wanted:?}, at slice {}",
                &cells[..4],
                slice.index
            ),
        )?;
        let clock = p.slots.iter().find(|x| x.key == "clock").ok_or("no clock slot")?;
        check(
            clock.text == format!("tick {} · slice {}", slice.tick, slice.index),
            &format!("the clock says {:?} at slice {}", clock.text, slice.index),
        )?;
        check(
            s.element_count()? == elements,
            &format!("the element count changed at slice {}", slice.index),
        )?;

        for slot in &p.slots {
            saw_down |= slot.key.starts_with('S') && slot.text == "down";
            saw_busy |= slot.key.starts_with('S') && slot.class.contains("ss-busy");
            saw_queue |= slot.key == "Q0" && !slot.hidden;
            saw_retry |= slot.key == "retrying" && slot.text != "0";
            saw_rejection |= slot.key == "rejected" && slot.text != "0";
        }
        // Now and then the page reached by deltas is compared with a fresh surface drawn in full.
        if slice.index % 300 == 0 {
            let mut fresh = surface(&format!("ss-genuine-fresh-{}", slice.index), servers)?;
            fresh.draw(&p)?;
            check(
                s.snapshot()? == fresh.snapshot()?,
                &format!("delta and full differ at slice {}", slice.index),
            )?;
        }
    }
    check(
        saw_down && saw_busy && saw_queue && saw_retry && saw_rejection,
        &format!(
            "the run was too tame to prove much: down {saw_down}, busy {saw_busy}, queue {saw_queue}, retry {saw_retry}, rejection {saw_rejection}"
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A draw leaves the view and the focus where they were, including a draw that changes the selection.
#[wasm_bindgen_test]
fn drawing_leaves_the_view_and_the_focus_alone() -> Result<(), String> {
    // Every other slice also changes which job is selected.
    let (servers, all) = projections_with("E0", |k| View {
        selected: if k % 2 == 0 { Some(2) } else { None },
    });
    let plain = e0_projections(&View::default()).1;
    let mut s = surface("ss-focus", servers)?;
    let view = ViewTransform {
        scale: 1.5,
        tx: 20.0,
        ty: -10.0,
    };
    s.scene().set_view(view).map_err(|e| e.to_string())?;
    s.draw(&plain[0])?;
    let target = s.slot_element("S1").ok_or("no slot S1")?;
    target.dyn_ref::<web_sys::HtmlElement>().map_or_else(
        || {
            target
                .dyn_ref::<web_sys::SvgElement>()
                .ok_or_else(|| "slot is neither html nor svg".to_owned())
                .and_then(|e| e.focus().map_err(|x| format!("{x:?}")))
        },
        |e| e.focus().map_err(|x| format!("{x:?}")),
    )?;
    check(
        document().active_element().as_ref() == Some(&target),
        "the focus did not reach the slot",
    )?;
    for (k, projection) in all.iter().enumerate() {
        s.draw(projection)?;
        check(
            document().active_element().as_ref() == Some(&target),
            &format!("the focus moved at slice {k}"),
        )?;
        let now = s.scene().view();
        check(
            (now.scale, now.tx, now.ty) == (view.scale, view.tx, view.ty),
            &format!("the view changed at slice {k}: {now:?}"),
        )?;
    }
    let after_full = s.redraw(&plain[12])?;
    check(after_full.writes > 0, "a full redraw wrote nothing")?;
    check(
        document().active_element().as_ref() == Some(&target),
        "a full redraw moved the focus",
    )?;
    let now = s.scene().view();
    check(
        (now.scale, now.tx, now.ty) == (view.scale, view.tx, view.ty),
        "a full redraw changed the view",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The content group is the one the overlay relies on: there is exactly one, the overlay is inside it and painted last,
/// and the view scales the overlay as it scales a node.
#[wasm_bindgen_test]
fn the_overlay_is_in_the_one_content_group_and_follows_the_view() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut s = surface("ss-hook", servers)?;
    s.draw(&projections[4])?;
    let host = document().get_element_by_id("ss-hook").ok_or("no host")?;
    let groups = host
        .query_selector_all(":scope > g.svg-dom-graph-content")
        .map_err(|e| format!("{e:?}"))?;
    check(
        groups.length() == 1,
        &format!("{} content groups, and one was meant", groups.length()),
    )?;
    let content: web_sys::Element = groups.get(0).and_then(|g| g.dyn_into().ok()).ok_or("not an element")?;
    check(
        content.contains(Some(s.overlay())),
        "the overlay is not inside the content group",
    )?;
    check(
        content.last_element_child().as_ref() == Some(s.overlay()),
        "the overlay is not the last child, so a node would be painted over it",
    )?;
    let node = content
        .query_selector(":scope > g")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no node")?;
    // The scale that the browser applies to an element, which is a property of the transforms above it and does not depend on what is drawn
    // in it. A bounding box would measure the glyphs of the overlay's text, and a browser need not lay text out exactly twice as wide at
    // twice the size: Firefox gives 1.993, from the advance widths of the scaled font. That is the browser's text and not the view.
    let scale = |e: &web_sys::Element| -> Result<f64, String> {
        let matrix = js_sys::Reflect::get(e, &"getCTM".into())
            .ok()
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
            .ok_or("the element has no getCTM")?
            .call0(e)
            .map_err(|e| format!("{e:?}"))?;
        js_sys::Reflect::get(&matrix, &"a".into())
            .map_err(|e| format!("{e:?}"))?
            .as_f64()
            .ok_or_else(|| "the matrix has no scale".to_owned())
    };
    let (overlay_before, node_before) = (scale(s.overlay())?, scale(&node)?);
    s.scene()
        .set_view(ViewTransform { scale: 2.0, tx: 0.0, ty: 0.0 })
        .map_err(|e| e.to_string())?;
    let (overlay_ratio, node_ratio) = (scale(s.overlay())? / overlay_before, scale(&node)? / node_before);
    check((node_ratio - 2.0).abs() < 1e-6, &format!("the node scaled by {node_ratio}"))?;
    check(
        (overlay_ratio - 2.0).abs() < 1e-6,
        &format!("the overlay scaled by {overlay_ratio}, and a node by {node_ratio}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A projection of another shape is refused, and the surface then no longer trusts what it holds, so the next draw is a
/// full one.
#[wasm_bindgen_test]
fn a_refused_draw_makes_the_next_one_a_full_redraw() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut s = surface("ss-refuse", servers)?;
    s.draw(&projections[3])?;
    check(s.is_trusted(), "a draw should leave the surface trusting its page")?;
    let mut wrong = projections[4].clone();
    wrong.slots.pop();
    check(s.draw(&wrong).is_err(), "a projection of another shape was drawn")?;
    check(!s.is_trusted(), "a refused draw left the surface trusting its page")?;
    let next = s.draw(&projections[4])?;
    check(
        next.writes == projections[4].slots.len() * 4,
        "the draw after a refusal was not a full one",
    )?;
    check(
        s.snapshot()? == projections[4].canonical(),
        "the page is not the projection after the full redraw",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every id on the page is unique, and the slots have the stable keys, whatever is drawn.
#[wasm_bindgen_test]
fn ids_and_slot_keys_are_unique_and_do_not_change() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut s = surface("ss-ids", servers)?;
    let host = document().get_element_by_id("ss-ids").ok_or("no host")?;
    let keys_now = || -> Result<Vec<String>, String> {
        let found = host.query_selector_all("[data-ss-key]").map_err(|e| format!("{e:?}"))?;
        Ok((0..found.length())
            .filter_map(|i| found.get(i))
            .filter_map(|e| e.dyn_into::<web_sys::Element>().ok())
            .filter_map(|e| e.get_attribute("data-ss-key"))
            .collect())
    };
    let before = keys_now()?;
    check(
        before == Projection::keys(servers),
        "the slots are not the keys of the projection",
    )?;
    for p in &projections {
        s.draw(p)?;
    }
    check(keys_now()? == before, "drawing changed the slot keys")?;
    let all = host.query_selector_all("[id]").map_err(|e| format!("{e:?}"))?;
    let mut ids: Vec<String> = (0..all.length())
        .filter_map(|i| all.get(i))
        .filter_map(|e| e.dyn_into::<web_sys::Element>().ok())
        .map(|e| e.id())
        .collect();
    let count = ids.len();
    ids.sort();
    ids.dedup();
    check(ids.len() == count, "an id is used twice on the page")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn rect_of(e: &web_sys::Element) -> (f64, f64, f64, f64) {
    let r = e.get_bounding_client_rect();
    (r.left(), r.top(), r.right(), r.bottom())
}

fn overlap((a, b, c, d): (f64, f64, f64, f64), (e, f, g, h): (f64, f64, f64, f64)) -> bool {
    a.max(e) + 0.5 < c.min(g) && b.max(f) + 0.5 < d.min(h)
}

/// Everything that is drawn is inside the view box, and no two things that are not meant to share ground do. A server's
/// status text is meant to sit inside its node, and nothing else is. This is the only check of the layout, since no
/// test reads a screenshot.
fn layout_is_sound(id: &str, servers: usize, projection: &Projection) -> Result<(), String> {
    let mut s = surface(id, servers)?;
    s.draw(projection)?;
    let host = document().get_element_by_id(id).ok_or("no host")?;
    let view = rect_of(&host);

    let mut named: Vec<(String, (f64, f64, f64, f64))> = Vec::new();
    let nodes = host
        .query_selector_all(":scope > g.svg-dom-graph-content > g")
        .map_err(|e| format!("{e:?}"))?;
    // The queue node, then the servers, then the data node; the overlay is the last group and is not a node.
    for i in 0..nodes.length() - 1 {
        let node: web_sys::Element = nodes.get(i).and_then(|n| n.dyn_into().ok()).ok_or("not an element")?;
        named.push((format!("node {i}"), rect_of(&node)));
    }
    let node_count = named.len();
    for key in Projection::keys(servers) {
        let el = s.slot_element(&key).ok_or("no slot")?;
        if el.get_attribute("aria-hidden").as_deref() == Some("true") {
            continue;
        }
        named.push((key, rect_of(&el)));
    }
    let captions = s.overlay().query_selector_all(".ss-caption").map_err(|e| format!("{e:?}"))?;
    for i in 0..captions.length() {
        let c: web_sys::Element = captions.get(i).and_then(|n| n.dyn_into().ok()).ok_or("not an element")?;
        named.push((format!("caption {}", c.text_content().unwrap_or_default()), rect_of(&c)));
    }
    for (name, r) in &named {
        check(
            r.0 >= view.0 - 0.5 && r.1 >= view.1 - 0.5 && r.2 <= view.2 + 0.5 && r.3 <= view.3 + 0.5,
            &format!("{name} is outside the view: {r:?} in {view:?}"),
        )?;
    }
    for i in 0..named.len() {
        for j in i + 1..named.len() {
            // A server's text is meant to be inside its own node: node 1 + k is server Sk.
            let inside_own_node = |a: usize, b: usize| {
                a < node_count && (1..=servers).contains(&a) && named[b].0 == format!("S{}", a - 1)
            };
            if inside_own_node(i, j) || inside_own_node(j, i) {
                continue;
            }
            // The rows of the data node are named beside it, at the place that figures of six digits leave. A node that
            // is given figures of twenty digits is wider, as it is meant to be, and then covers those names, which is
            // not what this test is for.
            let data_node = servers + 1;
            let is_row_name = |n: usize| named[n].0.starts_with("caption ") && named[n].0.contains(" · ");
            if (i == data_node && is_row_name(j)) || (j == data_node && is_row_name(i)) {
                continue;
            }
            // The node's own label is on the node, and an edge is not a group, so only groups are compared here.
            check(
                !overlap(named[i].1, named[j].1),
                &format!("{} and {} overlap: {:?} {:?}", named[i].0, named[j].0, named[i].1, named[j].1),
            )?;
        }
    }
    Ok(())
}

/// The widest cases of the layout: E0 with a job selected, and four and five servers with a full window of twelve and
/// "+N more"
/// under it.
#[wasm_bindgen_test]
fn nothing_is_drawn_outside_the_view_or_over_anything_else() -> Result<(), String> {
    let (servers, selected) = projections_with("E0", |_| View { selected: Some(2) });
    layout_is_sound("ss-layout-e0", servers, &selected[4])?;

    for servers in [4, 5] {
        let config = Config {
            servers,
            queue_capacity: 40,
            max_attempts: 1,
            deferral_cap: 4,
        };
        let script: Vec<ScriptEvent> = (0..30)
            .map(|i| ScriptEvent::Arrive {
                at: 0,
                key: format!("job-{i}"),
                priority: u8::from(i % 3 == 0),
                durations: vec![100],
                deadline: 500,
            })
            .collect();
        let mut engine = Engine::new("wide", 0, config, &script, None, Limits::default()).map_err(|e| e.to_string())?;
        engine.step().map_err(|e| e.to_string())?;
        let view = View { selected: Some(5) };
        let wide = project(&extract(&engine, &view), &view);
        check(
            wide.slots.iter().any(|s| s.key == "more" && !s.hidden),
            "the wide case has no \"more\" line",
        )?;
        layout_is_sound(&format!("ss-layout-wide-{servers}"), servers, &wide)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Takes the host out of the document, and gives the means of putting it back.
fn detach(id: &str) -> Result<impl Fn() -> Result<(), String> + Clone, String> {
    let host = document().get_element_by_id(id).ok_or("no host")?;
    let parent = host.parent_node().ok_or("the host has no parent")?;
    let take_out = {
        let (host, parent) = (host.clone(), parent.clone());
        move || parent.remove_child(&host).map(|_| ()).map_err(|e| format!("{e:?}"))
    };
    take_out()?;
    Ok(move || parent.append_child(&host).map(|_| ()).map_err(|e| format!("{e:?}")))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A host that is not in the document is refused before anything is written, a draw or a full redraw, and the surface
/// then draws in full once the host is back. A detached `<svg>` accepts writes without complaint, so the surface has to
/// ask.
#[wasm_bindgen_test]
fn a_detached_host_is_refused_before_anything_is_written() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut s = surface("ss-detached", servers)?;
    s.draw(&projections[3])?;
    let reattach = detach("ss-detached")?;
    check(
        s.draw(&projections[4]) == Err(DrawError::Detached),
        "a draw on a detached host was not refused",
    )?;
    check(
        s.redraw(&projections[4]) == Err(DrawError::Detached),
        "a full redraw on a detached host was not refused",
    )?;
    check(!s.is_trusted(), "a refused draw left the surface trusting its page")?;
    check(
        s.snapshot()? == projections[3].canonical(),
        "a refused draw wrote to the detached host",
    )?;
    reattach()?;
    let back = s.draw(&projections[4])?;
    check(
        back.writes == projections[4].slots.len() * 4,
        "the draw after reattaching was not a full one",
    )?;
    check(
        s.snapshot()? == projections[4].canonical(),
        "the page is not the projection after reattaching",
    )?;
    check(s.is_trusted(), "the surface does not trust a page it has just drawn in full")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The host leaves the document after each possible write of a draw in turn, and after the data node's setter.
/// Whichever it is, the draw fails with `Detached`, the surface does not trust the page, no write follows the
/// detachment, the setter is not begun on a detached root, and a full redraw after reattaching restores the projection.
#[wasm_bindgen_test]
fn a_host_that_leaves_in_the_middle_of_a_draw_never_leaves_the_surface_trusted() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let (from, to) = (&projections[0], &projections[4]);
    let n = writes(Some(from), to).len();
    check(n > 10, "the sweep needs a draw with many writes")?;
    // What the page holds if every overlay write is made and the setter is not: the new slots with the old data.
    let overlay_only = Projection {
        slots: to.slots.clone(),
        data: from.data,
    }
    .canonical();

    // The overlay's writes are numbered from 0, and the setter's point comes after them.
    for point in 0..=n {
        let id = format!("ss-sweep-{point}");
        let mut s = surface(&id, servers)?;
        s.draw(from)?;
        let host = document().get_element_by_id(&id).ok_or("no host")?;
        let parent = host.parent_node().ok_or("no parent")?;
        let (leaving_host, leaving_parent) = (host.clone(), parent.clone());
        s.set_fault_hook(Some(Box::new(move |at| {
            let here = match at {
                FaultPoint::Overlay(i) => i,
                FaultPoint::Setter => n,
            };
            if here == point {
                let _ = leaving_parent.remove_child(&leaving_host);
            }
        })));
        let result = s.draw(to);
        check(
            result == Err(DrawError::Detached),
            &format!("point {point} of {n}: the draw gave {result:?}"),
        )?;
        check(
            !s.is_trusted(),
            &format!("point {point} of {n}: the surface trusts a page it lost the host of"),
        )?;
        let held = s.snapshot()?;
        if point + 1 < n {
            check(
                held != overlay_only,
                &format!("point {point} of {n}: writes went on after the host left"),
            )?;
        } else if point + 1 == n {
            check(
                held == overlay_only,
                "the last overlay write was the one that the host left after",
            )?;
        }
        if point < n {
            check(
                s.data_texts()? == from.data.iter().map(ToString::to_string).collect::<Vec<_>>(),
                &format!("point {point} of {n}: the setter was begun on a detached root"),
            )?;
        } else {
            check(
                held == to.canonical(),
                "a host that left after the setter should have had every write",
            )?;
        }
        s.set_fault_hook(None);
        parent.append_child(&host).map_err(|e| format!("{e:?}"))?;
        let repaired = s.draw(to)?;
        check(
            repaired.writes == to.slots.len() * 4,
            &format!("point {point} of {n}: the draw after reattaching was not a full one"),
        )?;
        check(
            s.snapshot()? == to.canonical(),
            &format!("point {point} of {n}: the page was not restored"),
        )?;
        check(
            s.is_trusted(),
            &format!("point {point} of {n}: the surface does not trust a restored page"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A tick at the edge of what a `u32`, an `f64` and a `u64` can hold is on the page exactly, in the clock and in the
/// data node, and the layout still holds, since a data cell keeps the width it was drawn with and has to fit the widest
/// figure it will be given.
#[wasm_bindgen_test]
fn ticks_at_the_edges_of_the_integers_are_on_the_page_exactly() -> Result<(), String> {
    let ticks: [u64; 6] = [
        4_294_967_295,
        4_294_967_296,
        1 << 53,
        (1 << 53) + 1,
        u64::MAX / 2,
        u64::MAX - 100,
    ];
    for (n, tick) in ticks.into_iter().enumerate() {
        let config = Config {
            servers: 1,
            queue_capacity: 2,
            max_attempts: 1,
            deferral_cap: 4,
        };
        let script = [ScriptEvent::Arrive {
            at: tick,
            key: "J".into(),
            priority: 1,
            durations: vec![5],
            deadline: tick + 50,
        }];
        let mut engine = Engine::new("edge", 0, config, &script, None, Limits::default()).map_err(|e| e.to_string())?;
        let mut s = surface(&format!("ss-edge-{n}"), 1)?;
        let mut last = None;
        while let Some(slice) = engine.step().map_err(|e| e.to_string())? {
            let p = project(&extract(&engine, &View::default()), &View::default());
            s.draw(&p)?;
            let cells = s.data_texts()?;
            check(
                cells[1] == slice.tick.to_string(),
                &format!("the data node says {} for tick {}", cells[1], slice.tick),
            )?;
            check(cells[0] == slice.index.to_string(), "the ordinal is not exact")?;
            check(
                s.snapshot()? == p.canonical(),
                &format!("the page differs from its projection at tick {}", slice.tick),
            )?;
            last = Some(p);
        }
        // The widest figures the node can be given: every cell at the edge, in the layout that the node was built for.
        let mut widest = last.ok_or("the run had no slice")?;
        widest.data = [u64::MAX; DATA_CELLS];
        s.draw(&widest)?;
        check(
            s.data_texts()?.iter().all(|c| *c == u64::MAX.to_string()),
            "a cell does not show the largest figure in full",
        )?;
        layout_is_sound(&format!("ss-edge-layout-{n}"), 1, &widest)?;
    }
    Ok(())
}
