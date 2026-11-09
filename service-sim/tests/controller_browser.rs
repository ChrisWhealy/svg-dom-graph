//! The controller on a real page: the real surface, the page's own timers and frames, a real mid-draw detach, a real
//! hidden panel, and the closures that the scheduler holds. Run with `wasm-pack test --headless --chrome service-sim`.
#![cfg(target_arch = "wasm32")]

use service_sim::controller::{Controller, Outcome, Presence};
use service_sim::host::{BrowserScheduler, GalleryHint, PageHost};
use service_sim::projection::View;
use service_sim::surface::{FaultPoint, Surface};
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Engine, Limits};
use std::cell::RefCell;
use std::rc::Rc;
use svg_dom::SvgRoot;
use svg_dom::root::utils::Size;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

const FIXTURES: &str = include_str!("../../dess/s1/fixtures.json");

type Page = Controller<PageHost, BrowserScheduler>;

fn check(ok: bool, what: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(what.to_owned()) }
}

fn document() -> web_sys::Document {
    web_sys::window().and_then(|w| w.document()).expect("a document")
}

/// Waits `ms` milliseconds on the page's own timer.
async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .expect("a window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("a timer");
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.expect("a timer that ran");
}

async fn until(what: &str, mut done: impl FnMut() -> bool) -> Result<(), String> {
    for _ in 0..400 {
        if done() {
            return Ok(());
        }
        sleep(10).await;
    }
    Err(format!("timed out waiting for {what}"))
}

fn e0_scenario() -> Scenario {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("fixtures");
    Scenario::fixture("E0", &fixtures["E0"]).expect("a fixture")
}

fn e0() -> Rc<dyn Fn() -> Result<Engine, service_sim_engine::Error>> {
    let s = e0_scenario();
    Rc::new(move || Engine::new(&s.config_id, s.seed, s.config, &s.script, s.params.as_ref(), Limits::default()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An `<svg>` for the surface, in `parent` (the body if none), and the element and its container.
fn host_in(parent: Option<&web_sys::Element>, id: &str) -> Result<(web_sys::Element, web_sys::Element), String> {
    let (w, h) = Surface::view_box();
    let container = format!("{id}-container");
    let div = document().create_element("div").map_err(|e| format!("{e:?}"))?;
    div.set_id(&container);
    match parent {
        Some(p) => p.append_child(&div),
        None => document().body().ok_or("no body")?.append_child(&div),
    }
    .map_err(|e| format!("{e:?}"))?;
    let svg = SvgRoot::create_in(&container, Size::new(w, h)).map_err(|e| e.to_string())?;
    svg.root.set_id(id);
    svg.set_view_box(0.0, 0.0, w, h).map_err(|e| e.to_string())?;
    let element = document().get_element_by_id(id).ok_or("no host")?;
    Ok((div, element))
}

fn page_in(parent: Option<&web_sys::Element>, id: &str) -> Result<(Page, web_sys::Element, web_sys::Element), String> {
    let (container, svg) = host_in(parent, id)?;
    let surface = Surface::build(id, e0_scenario().config.servers)?;
    let status = document().create_element("p").map_err(|e| format!("{e:?}"))?;
    container.append_child(&status).map_err(|e| format!("{e:?}"))?;
    let host = PageHost::new(surface, svg.clone()).with_voices(Some(status), None, None);
    let c = Controller::new(host, BrowserScheduler::new(), e0(), View::default()).map_err(|e| e.to_string())?;
    Ok((c, container, svg))
}

fn snapshot(c: &Page) -> Result<String, String> {
    c.with_host(|h| h.surface().snapshot())
}

fn status_text(container: &web_sys::Element) -> String {
    container
        .query_selector("p")
        .ok()
        .flatten()
        .and_then(|p| p.text_content())
        .unwrap_or_default()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// By Next and by a seek on real frames, the page is what the committed projection says, and equals a fresh surface
/// drawn in full.
#[wasm_bindgen_test]
async fn the_page_is_the_committed_slice_by_next_and_by_seek() -> Result<(), String> {
    let (c, _, _) = page_in(None, "ct-walk")?;
    for k in 1..=12 {
        check(c.next() == Outcome::Done, &format!("next to {k}"))?;
        check(snapshot(&c)? == c.committed().canonical(), &format!("the page after slice {k}"))?;
    }
    check(c.next() == Outcome::AtBoundary, "the last slice has nowhere to go")?;
    check(c.seek(5) == Outcome::Pending, "a seek is asked for")?;
    until("the seek", || c.committed_slice() == 5).await?;
    check(snapshot(&c)? == c.committed().canonical(), "the page after the seek")?;
    // A fresh surface drawn once, in full, says the same.
    let (_, _, _) = host_in(None, "ct-walk-fresh").map(|x| (x.0, x.1, ()))?;
    let mut fresh = Surface::build("ct-walk-fresh", e0_scenario().config.servers)?;
    fresh.redraw(&c.committed()).map_err(|e| e.to_string())?;
    check(fresh.snapshot()? == snapshot(&c)?, "a delta page equals a full one")?;
    check(
        c.scheduler().live_closures() == 0 || {
            sleep(0).await;
            c.scheduler().live_closures() == 0
        },
        "no closure is left",
    )?;
    Ok(())
}

#[wasm_bindgen_test]
async fn playback_on_the_pages_timers_runs_to_the_end_and_leaves_no_closure() -> Result<(), String> {
    let (c, _, _) = page_in(None, "ct-play")?;
    c.set_speed(15);
    check(c.play() == Outcome::Done, "play")?;
    check(c.scheduler().waiting() == 1, "one timer is waiting")?;
    until("the last slice", || c.committed_slice() == 12).await?;
    // The end is known only when a step finds nothing after it, so playback stops one delay after the last slice.
    until("playback to stop", || !c.is_playing()).await?;
    sleep(0).await;
    check(
        !c.is_playing() && c.scheduler().waiting() == 0,
        "it stopped at the end with nothing waiting",
    )?;
    check(c.scheduler().live_closures() == 0, "no closure is held once it has run")?;
    check(snapshot(&c)? == c.committed().canonical(), "the page")?;
    Ok(())
}

#[wasm_bindgen_test]
async fn cancelling_a_task_drops_its_closure_over_many_cycles() -> Result<(), String> {
    let (c, _, _) = page_in(None, "ct-cycles")?;
    c.set_speed(5_000);
    for cycle in 0..60 {
        c.play();
        check(
            c.scheduler().waiting() <= 2 && c.scheduler().live_closures() <= 4,
            &format!("a timer and a frame at most at cycle {cycle}, and no pile of closures"),
        )?;
        if cycle % 3 == 0 {
            c.seek(3);
            check(c.scheduler().waiting() <= 2, "a timer and a frame at most")?;
        }
        c.pause();
    }
    sleep(50).await;
    check(
        c.scheduler().live_closures() == 0 && c.scheduler().waiting() == 0,
        "nothing is held after 60 cycles",
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The host leaves the document after each field of the overlay is written, and after the setter. Every time the slice
/// is not committed, nothing writes after the departure, and a return repairs the page.
#[wasm_bindgen_test]
fn a_host_that_leaves_in_the_middle_of_a_draw_is_never_committed_and_is_repaired_on_return() -> Result<(), String> {
    // How many times the hook fires in the draw of slice 1, which is a delta.
    let probe_points = {
        let (c, _, _) = page_in(None, "ct-probe")?;
        let count = Rc::new(RefCell::new(0_usize));
        let seen = Rc::clone(&count);
        c.with_host(|h| h.surface().set_fault_hook(Some(Box::new(move |_| *seen.borrow_mut() += 1))));
        c.next();
        *count.borrow()
    };
    check(probe_points >= 5, "a draw has points at which a host can leave")?;
    for point in 0..probe_points {
        let id = format!("ct-detach-{point}");
        let (c, container, svg) = page_in(None, &id)?;
        let fired = Rc::new(RefCell::new(0_usize));
        let (n, svg2) = (Rc::clone(&fired), svg.clone());
        c.with_host(|h| {
            h.surface().set_fault_hook(Some(Box::new(move |_p: FaultPoint| {
                let mut k = n.borrow_mut();
                if *k == point {
                    svg2.remove();
                }
                *k += 1;
            })))
        });
        let outcome = c.next();
        check(
            outcome == Outcome::Unavailable,
            &format!("point {point}: the draw was refused ({outcome:?})"),
        )?;
        check(
            c.committed_slice() == 0,
            &format!("point {point}: the slice that was drawn as the host left was not committed"),
        )?;
        check(
            c.is_dirty() && !c.is_playing(),
            &format!("point {point}: the page is dirty and stopped"),
        )?;
        check(
            !c.with_host(|h| h.surface().is_trusted()),
            &format!("point {point}: the surface does not trust the page"),
        )?;
        check(
            status_text(&container).contains("not available"),
            &format!("point {point}: one announcement"),
        )?;
        // It comes back.
        c.with_host(|h| h.surface().set_fault_hook(None));
        container.append_child(&svg).map_err(|e| format!("{e:?}"))?;
        c.presence_changed();
        check(!c.is_dirty(), &format!("point {point}: repaired on return"))?;
        check(
            snapshot(&c)? == c.committed().canonical(),
            &format!("point {point}: the page is the committed slice"),
        )?;
        check(
            c.next() == Outcome::Done && c.committed_slice() == 1,
            &format!("point {point}: the next slice then commits"),
        )?;
        check(
            snapshot(&c)? == c.committed().canonical(),
            &format!("point {point}: and the page is it"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A page with the gallery's markup and rules, with the surface in the first panel.
fn gallery(id: &str) -> Result<(web_sys::Element, web_sys::Element, web_sys::Element), String> {
    let style = document().create_element("style").map_err(|e| format!("{e:?}"))?;
    style.set_text_content(Some(&format!(
        "#{id} .section {{ display: none }} #{id} .section.active {{ display: block }}"
    )));
    document()
        .body()
        .ok_or("no body")?
        .append_child(&style)
        .map_err(|e| format!("{e:?}"))?;
    let stage = document().create_element("div").map_err(|e| format!("{e:?}"))?;
    stage.set_id(id);
    stage.set_inner_html("<div class=\"section active\" id=\"one\"></div><div class=\"section\" id=\"two\"></div>");
    document()
        .body()
        .ok_or("no body")?
        .append_child(&stage)
        .map_err(|e| format!("{e:?}"))?;
    let one = stage.query_selector("#one").map_err(|e| format!("{e:?}"))?.ok_or("no panel")?;
    let two = stage.query_selector("#two").map_err(|e| format!("{e:?}"))?.ok_or("no panel")?;
    Ok((stage, one, two))
}

fn toggle(one: &web_sys::Element, two: &web_sys::Element, first_shown: bool) {
    let _ = if first_shown {
        one.class_list().add_1("active")
    } else {
        one.class_list().remove_1("active")
    };
    let _ = if first_shown {
        two.class_list().remove_1("active")
    } else {
        two.class_list().add_1("active")
    };
}

/// A panel that is hidden is not live though its host is in the document, and a callback that was already queued is
/// inert, whether or not the hint arrived. The hint, when there is one, comes before a timer that was queued before the
/// hide.
#[wasm_bindgen_test]
async fn a_hidden_panel_is_not_live_and_a_callback_queued_before_the_hide_is_inert() -> Result<(), String> {
    for hinted in [true, false] {
        let id = format!("ct-gallery-{hinted}");
        let (stage, one, two) = gallery(&id)?;
        let (c, container, svg) = page_in(Some(&one), &format!("ct-hidden-{hinted}"))?;
        let _ = (&container, &svg);
        let hint = if hinted {
            let again = c.clone();
            Some(GalleryHint::install(&stage, move || again.presence_changed())?)
        } else {
            None
        };
        c.set_speed(40);
        c.play();
        until("a first slice", || c.committed_slice() >= 1).await?;
        let before = snapshot(&c)?;
        let slice = c.committed_slice();
        check(c.with_host(|h| h.presence_for_test()) == Presence::Live, "live while shown")?;

        // One task: a timer is queued, then the panel is hidden. The hint, if there is one, is a microtask of this
        // task.
        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let o = Rc::clone(&order);
        let timer = wasm_bindgen::closure::Closure::once_into_js(move || o.borrow_mut().push("timer"));
        web_sys::window()
            .ok_or("no window")?
            .set_timeout_with_callback_and_timeout_and_arguments_0(timer.unchecked_ref(), 0)
            .map_err(|e| format!("{e:?}"))?;
        let epoch = c.epoch();
        toggle(&one, &two, false);
        check(
            c.with_host(|h| h.presence_for_test()) == Presence::Hidden,
            "a hidden panel is hidden though the host is in the document",
        )?;
        check(svg.is_connected(), "and the host is still connected")?;
        sleep(150).await;
        check(
            c.committed_slice() == slice,
            &format!("hinted {hinted}: nothing committed while hidden"),
        )?;
        check(
            snapshot(&c)? == before,
            &format!("hinted {hinted}: nothing written while hidden"),
        )?;
        check(!c.is_playing(), &format!("hinted {hinted}: playback stopped"))?;
        check(c.epoch() > epoch, &format!("hinted {hinted}: the epoch moved"))?;
        check(c.scheduler().waiting() == 0, &format!("hinted {hinted}: nothing is waiting"))?;
        check(
            status_text(&container).contains("not available"),
            &format!("hinted {hinted}: one announcement"),
        )?;
        check(order.borrow().as_slice() == ["timer"], "the timer queued before the hide ran")?;

        // Showing again never resumes playback.
        toggle(&one, &two, true);
        sleep(100).await;
        check(
            !c.is_playing() && c.committed_slice() == slice,
            &format!("hinted {hinted}: shown, and still paused"),
        )?;
        if hinted {
            check(
                !c.is_dirty() && snapshot(&c)? == c.committed().canonical(),
                "the hint repaired the page on show",
            )?;
        } else {
            check(c.repair() == Outcome::Done, "without a hint a repair is asked for")?;
        }
        check(c.play() == Outcome::Done, "play is asked for again")?;
        until("a slice after the show", || c.committed_slice() > slice).await?;
        c.pause();
        drop(hint);
    }
    Ok(())
}

#[wasm_bindgen_test]
async fn a_disposed_controller_holds_nothing_and_late_callbacks_do_nothing() -> Result<(), String> {
    let (c, _, _) = page_in(None, "ct-dispose")?;
    c.set_speed(30);
    c.play();
    c.seek(4);
    c.dispose();
    sleep(150).await;
    check(
        c.committed_slice() == 0 && !c.is_playing(),
        "nothing arrived after the disposal",
    )?;
    check(
        c.scheduler().waiting() == 0 && c.scheduler().live_closures() == 0,
        "nothing is held",
    )?;
    check(c.next() == Outcome::Unavailable, "a disposed controller does nothing")?;
    Ok(())
}
