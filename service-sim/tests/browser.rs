//! The surface in a real browser: E0 on the page, deltas against full redraws, the data node on every slice, a view and a focus that
//! a draw leaves alone, and the content group that the overlay lives in. Run with `wasm-pack test --headless --chrome service-sim`.
#![cfg(target_arch = "wasm32")]

use service_sim::projection::{DATA_CELLS, Projection, View, project, writes};
use service_sim::surface::Surface;
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Config, Engine, Limits, Model, ScriptEvent};
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
fn models(name: &str) -> (usize, Vec<Model>) {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("fixtures");
    let s = Scenario::fixture(name, &fixtures[name]).expect("a fixture");
    let mut engine = Engine::new(&s.config_id, s.seed, s.config, &s.script, s.params.as_ref(), Limits::default())
        .expect("an engine");
    let mut all = vec![engine.model()];
    while engine.step().expect("a slice").is_some() {
        all.push(engine.model());
    }
    (s.config.servers, all)
}

fn e0_projections(view: &View) -> (usize, Vec<Projection>) {
    let (servers, all) = models("E0");
    (servers, all.iter().map(|m| project(m, view)).collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// After each of E0's 12 slices, and before the first, the page holds exactly what the projection says, and the data node holds the
/// slice ordinal. The comparison is of what the elements say, read back, with the model nowhere in it.
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
/// A page reached by deltas is the page that a fresh surface draws in full, at every slice. A seek is a delta across any distance.
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
/// A delta writes exactly the fields that differ, so a stale-only slice writes four, and the first draw writes everything.
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
/// 1,000 consecutive displayed slices go through the one data node and the one overlay, and each changes its content, leaves the
/// page equal to its projection, and adds no element.
#[wasm_bindgen_test]
fn a_thousand_displayed_slices_add_no_element_and_each_calls_the_setter() -> Result<(), String> {
    let (servers, projections) = e0_projections(&View::default());
    let mut s = surface("ss-thousand", servers)?;
    s.draw(&projections[0])?;
    let elements = s.element_count()?;
    let mut previous_ordinal = None;
    for commit in 0..1000_usize {
        // Walk forwards through the run, and give each commit its own ordinal so that every write changes the node's content.
        let mut p = projections[commit % projections.len()].clone();
        p.data[0] = u32::try_from(commit + 1).map_err(|e| e.to_string())?;
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
/// A draw leaves the view and the focus where they were, including a draw that changes the selection.
#[wasm_bindgen_test]
fn drawing_leaves_the_view_and_the_focus_alone() -> Result<(), String> {
    let (servers, all) = models("E0");
    let mut s = surface("ss-focus", servers)?;
    let view = ViewTransform {
        scale: 1.5,
        tx: 20.0,
        ty: -10.0,
    };
    s.scene().set_view(view).map_err(|e| e.to_string())?;
    s.draw(&project(&all[0], &View::default()))?;
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
    for (k, model) in all.iter().enumerate() {
        // Every other slice also changes which job is selected.
        let selected = if k % 2 == 0 { Some(2) } else { None };
        s.draw(&project(model, &View { selected }))?;
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
    let after_full = s.redraw(&project(&all[12], &View::default()))?;
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
/// The content group is the one the overlay relies on: there is exactly one, the overlay is inside it and painted last, and the view
/// scales the overlay as it scales a node.
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
    let width = |e: &web_sys::Element| e.get_bounding_client_rect().width();
    let (overlay_before, node_before) = (width(s.overlay()), width(&node));
    s.scene()
        .set_view(ViewTransform { scale: 2.0, tx: 0.0, ty: 0.0 })
        .map_err(|e| e.to_string())?;
    let (overlay_ratio, node_ratio) = (width(s.overlay()) / overlay_before, width(&node) / node_before);
    check((node_ratio - 2.0).abs() < 1e-6, &format!("the node scaled by {node_ratio}"))?;
    check(
        (overlay_ratio - 2.0).abs() < 1e-6,
        &format!("the overlay scaled by {overlay_ratio}, and a node by {node_ratio}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A projection of another shape is refused, and the surface then no longer trusts what it holds, so the next draw is a full one.
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

/// Everything that is drawn is inside the view box, and no two things that are not meant to share ground do. A server's status text
/// is meant to sit inside its node, and nothing else is. This is the only check of the layout, since no test reads a screenshot.
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
            // The node's own label is on the node, and an edge is not a group, so only groups are compared here.
            check(
                !overlap(named[i].1, named[j].1),
                &format!("{} and {} overlap: {:?} {:?}", named[i].0, named[j].0, named[i].1, named[j].1),
            )?;
        }
    }
    Ok(())
}

/// The widest cases of the layout: E0 with a job selected, and four servers with a full window of twelve and "+N more" under it.
#[wasm_bindgen_test]
fn nothing_is_drawn_outside_the_view_or_over_anything_else() -> Result<(), String> {
    let (servers, all) = models("E0");
    layout_is_sound("ss-layout-e0", servers, &project(&all[4], &View { selected: Some(2) }))?;

    let config = Config {
        servers: 4,
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
    let wide = project(&engine.model(), &View { selected: Some(5) });
    check(
        wide.slots.iter().any(|s| s.key == "more" && !s.hidden),
        "the wide case has no \"more\" line",
    )?;
    layout_is_sound("ss-layout-wide", 4, &wide)
}
