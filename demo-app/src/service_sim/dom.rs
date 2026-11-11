//! The page side of a session: it makes a scene in a host that nobody can see yet, reads it back, and shows it.
//!
//! None of the simulator's rules is here. The coordinator in `service-sim` decides when a candidate may replace the
//! session on the page, and this is the DOM that it asks about. A candidate is built into a host of its own that is
//! connected, laid out and measurable, but out of the flow of the panel and `inert`, so that it takes no room and
//! nobody can reach, click or hear it (design D3, shown by the P0 staging probe). Publishing removes that.
//!
//! A scene is handed off in three steps, each of which is undone if a later one fails: the new host is shown, the old
//! one is staged again, and the focus, if it was inside the old scene, is moved to a control that is outside every
//! scene. The pan and zoom of the old scene are carried to the new one.

use service_sim::controller::{Controller, DrawFailure, Factory, Host, Presence};
use service_sim::host::{BrowserScheduler, PageHost};
use service_sim::projection::{Projection, View};
use service_sim::session::{Candidate, SessionHost, Spec};
use service_sim::surface::Surface;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use svg_dom::SvgRoot;
use svg_dom::root::utils::Size;
use svg_dom_graph::scene::InputMode;
use wasm_bindgen::{JsCast, JsValue};

/// What a staged host carries. It is out of the flow, so that it moves nothing that follows it, and it is not
/// `display:none` or `visibility:hidden`, which would make the host unmeasurable.
const STAGED_STYLE: &str = "position:absolute;left:0;top:0;width:1000px;opacity:0;pointer-events:none";

/// The controller of the panel, on the page's own host and the page's own timers.
pub(super) type PanelController = Controller<PanelHost, BrowserScheduler>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the panel is told that something was drawn or that playback changed, so that what it shows can follow what the
/// controller committed.
///
/// It is empty until the panel exists. The first draw is made in the controller's own constructor, before that.
pub(super) type Notify = Rc<RefCell<Option<Weak<dyn Fn()>>>>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The page's host, and a word to the panel after each draw and each change of playback. It is not told what was drawn:
/// the panel reads the committed state from the controller once the call that drew has returned, so that it never shows
/// a slice that was not committed.
pub(super) struct PanelHost {
    inner: PageHost,
    notify: Notify,
}

impl PanelHost {
    fn told(&self) {
        let callback = self.notify.borrow().as_ref().and_then(Weak::upgrade);
        if let Some(callback) = callback {
            callback();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether a write of the Play control failed, so that it may not say what the controller says. The controller is
    /// not dirty then, since nothing that it drew was lost, and only a repair puts the control right.
    pub(super) fn controls_stale(&self) -> bool {
        self.inner.controls_stale()
    }

    /// The surface, which the readback of a candidate reads the page through.
    fn surface(&mut self) -> &mut Surface {
        self.inner.surface()
    }
}

impl Host for PanelHost {
    fn presence(&self) -> Presence {
        Host::presence(&self.inner)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn draw(&mut self, projection: &Projection, full: bool) -> Result<(), DrawFailure> {
        let result = self.inner.draw(projection, full);
        self.told();
        result
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn announce(&mut self, text: &str) {
        self.inner.announce(text);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn report(&mut self, text: &str) {
        self.inner.report(text);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn playing_changed(&mut self, playing: bool) {
        self.inner.playing_changed(playing);
        self.told();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const SVG_NS: &str = "http://www.w3.org/2000/svg";
/// The height of the band above the scene that the controls are in, and the width of the column beside it that the
/// inspector is in, in the scene's own units.
const TOOLBAR_HEIGHT: f64 = 150.0;
const SIDE_WIDTH: f64 = 340.0;

/// The view box of a scene on the page: the surface's own view box, which is at the origin, with a band above it for
/// the controls and a column beside it for the inspector. The surface's own coordinates do not move, so its layout is
/// the one that it was designed with.
fn page_view_box() -> (f64, f64, f64, f64) {
    let (width, height) = Surface::view_box();
    (0.0, -TOOLBAR_HEIGHT, width + SIDE_WIDTH, height + TOOLBAR_HEIGHT)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The controls and the inspector, which are written once in the panel's markup and live in whichever scene is shown. A
/// scene has a `foreignObject` for each, and they are moved from the one scene to the next, which keeps their listeners
/// and what they hold.
pub(super) struct Controls {
    /// Where they are written, and where they are kept when no scene is shown.
    pub(super) holder: web_sys::Element,
    pub(super) toolbar: web_sys::Element,
    pub(super) side: web_sys::Element,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements that say things to a person, which every host that is built is given.
pub(super) struct Voices {
    pub(super) status: web_sys::Element,
    pub(super) problems: web_sys::Element,
    pub(super) play: web_sys::Element,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The page's side of a session, for the coordinator.
pub(super) struct DomSessions {
    document: web_sys::Document,
    /// The element that every host is a child of.
    stage: web_sys::Element,
    voices: Voices,
    notify: Notify,
    built: u32,
    /// The control that takes the focus when it was inside a scene that is handed off, which is outside every scene.
    focus_target: web_sys::Element,
    controls: Controls,
    /// The scene that is shown, which is where the controls belong.
    shown: Option<u32>,
    /// The controllers that were built and have not been removed, so that the view of the old scene can be read when it
    /// is handed off.
    controllers: HashMap<u32, PanelController>,
}

impl DomSessions {
    pub(super) fn new(
        document: web_sys::Document,
        stage: web_sys::Element,
        voices: Voices,
        notify: Notify,
        focus_target: web_sys::Element,
        controls: Controls,
    ) -> Self {
        Self {
            document,
            stage,
            voices,
            notify,
            built: 0,
            focus_target,
            controls,
            shown: None,
            controllers: HashMap::new(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn host_element(&self, token: u32) -> Result<web_sys::Element, String> {
        self.document
            .get_element_by_id(&format!("dess-host-{token}"))
            .ok_or_else(|| format!("host {token} is not on the page"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `foreignObject` of a scene that the controls (`toolbar`) or the inspector (`side`) are put in.
    fn object(&self, name: &str, token: u32) -> Result<web_sys::Element, String> {
        self.document
            .get_element_by_id(&format!("dess-{name}-object-{token}"))
            .ok_or_else(|| format!("scene {token} has no place for the {name}"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves the controls and the inspector into the scene with this token, or back to where they are written. A
    /// control that is moved loses the focus, so the one that had it is returned, for the caller to give it back when
    /// the scene that it is in can take it.
    fn place_controls(&self, into: Option<u32>) -> Result<Option<web_sys::Element>, String> {
        let focused = self
            .document
            .active_element()
            .filter(|a| self.controls.toolbar.contains(Some(a)) || self.controls.side.contains(Some(a)));
        let (toolbar, side) = match into {
            Some(token) => (self.object("toolbar", token)?, self.object("side", token)?),
            None => (self.controls.holder.clone(), self.controls.holder.clone()),
        };
        // Both are tried, so that one that cannot be moved does not keep the other from being.
        let toolbar = toolbar
            .append_child(&self.controls.toolbar)
            .map_err(|e| format!("could not move the controls: {e:?}"));
        let side = side
            .append_child(&self.controls.side)
            .map_err(|e| format!("could not move the inspector: {e:?}"));
        toolbar.and(side).map(|_| focused)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Gives the focus back to a control that was moved.
    fn restore_focus(focused: Option<web_sys::Element>) {
        if let Some(html) = focused.as_ref().and_then(|e| e.dyn_ref::<web_sys::HtmlElement>()) {
            let _ = html.focus();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn remove(&self, token: u32) {
        // A scene that is removed takes with it whatever is inside it. The controls are only ever in a scene that is
        // shown, or that a handoff is in the middle of showing, and one that a failed handoff could not take them back
        // from would otherwise leave the page with none, so they are put back.
        if let Ok(host) = self.host_element(token) {
            if host.contains(Some(&self.controls.toolbar)) || host.contains(Some(&self.controls.side)) {
                let focused = self.place_controls(self.shown.filter(|shown| *shown != token)).ok().flatten();
                Self::restore_focus(focused);
            }
            host.remove();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Builds the scene into a staged host, which is removed again if anything fails.
    fn make(&mut self, token: u32, spec: &Spec) -> Result<PanelController, String> {
        let js = |what: &str| {
            let what = what.to_owned();
            move |e: JsValue| format!("{what}: {e:?}")
        };
        let host_id = format!("dess-host-{token}");
        let svg_id = format!("dess-svg-{token}");
        let container = self.document.create_element("div").map_err(js("host"))?;
        container.set_id(&host_id);
        container.set_attribute("style", STAGED_STYLE).map_err(js("host"))?;
        container.set_attribute("inert", "").map_err(js("host"))?;
        self.stage.append_child(&container).map_err(js("host"))?;
        let (x, y, width, height) = page_view_box();
        let svg = SvgRoot::create_in(&host_id, Size::new(width, height)).map_err(|e| e.to_string())?;
        svg.root.set_id(&svg_id);
        svg.set_view_box(x, y, width, height).map_err(|e| e.to_string())?;
        let element = self
            .document
            .get_element_by_id(&svg_id)
            .ok_or("the host element was not made")?;
        let surface = Surface::build(&svg_id, spec.servers)?;
        // The scene has no toolbar, so dragging the background pans it and a wheel turned with Ctrl or Cmd held zooms
        // it, and the controls do not zoom.
        surface
            .scene()
            .set_pan_mode(InputMode::On)
            .map_err(|e| format!("could not turn on panning: {e}"))?;
        surface
            .scene()
            .set_wheel_zoom_mode(InputMode::On)
            .map_err(|e| format!("could not turn on zooming by the wheel: {e}"))?;
        // The places for the controls, after the scene so that they are above it. They are empty until the scene is
        // shown.
        let scene_width = Surface::view_box().0;
        for (name, (fx, fw, fh)) in [
            ("toolbar", (0.0, scene_width, TOOLBAR_HEIGHT)),
            ("side", (scene_width, SIDE_WIDTH, height)),
        ] {
            let object = self
                .document
                .create_element_ns(Some(SVG_NS), "foreignObject")
                .map_err(js("controls"))?;
            object.set_id(&format!("dess-{name}-object-{token}"));
            for (attribute, value) in [("x", fx), ("y", -TOOLBAR_HEIGHT), ("width", fw), ("height", fh)] {
                object.set_attribute(attribute, &value.to_string()).map_err(js("controls"))?;
            }
            element.append_child(&object).map_err(js("controls"))?;
        }
        let page = PageHost::new(surface, element).with_voices(
            Some(self.voices.status.clone()),
            Some(self.voices.problems.clone()),
            Some(self.voices.play.clone()),
        );
        let host = PanelHost {
            inner: page,
            notify: Rc::clone(&self.notify),
        };
        let factory: Factory = Rc::clone(&spec.factory);
        Controller::new(host, BrowserScheduler::new(), factory, View::default()).map_err(|e| e.message())
    }
}

impl SessionHost for DomSessions {
    type H = PanelHost;
    type S = BrowserScheduler;
    type Token = u32;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn build(&mut self, spec: &Spec) -> Result<Candidate<PanelHost, BrowserScheduler, u32>, String> {
        let token = self.built;
        self.built += 1;
        match self.make(token, spec) {
            Ok(controller) => {
                self.controllers.insert(token, controller.clone());
                Ok(Candidate { controller, token })
            },
            Err(e) => {
                self.remove(token);
                Err(e)
            },
        }
    }

    // The page is read back from its own elements and compared with the projection that was drawn, so that a scene that
    // built and did not draw is found.
    fn verify(&mut self, candidate: &Candidate<PanelHost, BrowserScheduler, u32>) -> Result<(), String> {
        let expected = candidate.controller.committed().canonical();
        let found = candidate.controller.with_host(|host| host.surface().snapshot())?;
        if found == expected {
            Ok(())
        } else {
            Err("the page does not hold what was drawn".to_owned())
        }
    }

    fn publish(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        let new = self.host_element(*to)?;
        // The pan and zoom that a person gave the old scene are carried to the new one, which would otherwise start at
        // the identity.
        if let (Some(old), Some(next)) = (from.and_then(|f| self.controllers.get(f)), self.controllers.get(to)) {
            let view = old.with_host(|host| host.surface().scene().view());
            next.with_host(|host| host.surface().scene().set_view(view))
                .map_err(|e| format!("could not carry the view: {e}"))?;
        }
        let focused = self.place_controls(Some(*to))?;
        unstage(&new)?;
        if let Some(from) = from {
            stage(&self.host_element(*from)?)?;
            // A focus that was inside the old scene would be on something that nobody can reach now.
            let inside = self
                .document
                .active_element()
                .is_some_and(|active| self.host_element(*from).is_ok_and(|old| old.contains(Some(&active))));
            if inside {
                self.focus_target
                    .dyn_ref::<web_sys::HtmlElement>()
                    .ok_or("the focus target cannot take the focus")?
                    .focus()
                    .map_err(|e| format!("could not move the focus: {e:?}"))?;
            }
        }
        // The new scene is shown, so a control that had the focus can take it again.
        Self::restore_focus(focused);
        self.shown = Some(*to);
        Ok(())
    }

    fn verify_published(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        let host = self.host_element(*to)?;
        if is_staged(&host) || !host.is_connected() {
            return Err("the new scene is still staged".to_owned());
        }
        let in_new = |name: &str, element: &web_sys::Element| {
            self.object(name, *to)
                .is_ok_and(|object| element.parent_element().is_some_and(|parent| parent == object))
        };
        if !in_new("toolbar", &self.controls.toolbar) || !in_new("side", &self.controls.side) {
            return Err("the controls are not in the new scene".to_owned());
        }
        let Some(from) = from else { return Ok(()) };
        let old = self.host_element(*from)?;
        if !is_staged(&old) {
            return Err("the old scene is still shown beside the new one".to_owned());
        }
        // The focus is on nothing that is in a staged scene, so that it is somewhere that a person can use.
        let stranded = self.document.active_element().is_some_and(|active| old.contains(Some(&active)));
        if stranded {
            return Err("the focus is still in the old scene".to_owned());
        }
        Ok(())
    }

    fn undo_publish(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        self.shown = from.copied();
        // The controls come back first, since a page without them cannot be used, and every step is tried whatever the
        // one before it did.
        let moved = self.place_controls(from.copied());
        let staged = self.host_element(*to).and_then(|host| stage(&host));
        let shown = match from {
            Some(from) => self.host_element(*from).and_then(|host| unstage(&host)),
            None => Ok(()),
        };
        let focused = moved.as_ref().ok().cloned().flatten();
        Self::restore_focus(focused);
        moved.map(|_| ()).and(staged).and(shown)
    }

    fn discard(&mut self, token: u32) {
        self.controllers.remove(&token);
        self.remove(token);
    }

    fn finalize(&mut self, token: u32) {
        self.controllers.remove(&token);
        self.remove(token);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts a host out of the flow, invisible and inert, which is how a candidate is kept and how a scene that was replaced
/// is kept until it is removed.
fn stage(host: &web_sys::Element) -> Result<(), String> {
    host.set_attribute("style", STAGED_STYLE)
        .map_err(|e| format!("could not stage a scene: {e:?}"))?;
    host.set_attribute("inert", "")
        .map_err(|e| format!("could not stage a scene: {e:?}"))
}

/// Takes the staging off a host, which shows it.
fn unstage(host: &web_sys::Element) -> Result<(), String> {
    host.remove_attribute("style")
        .map_err(|e| format!("could not show a scene: {e:?}"))?;
    host.remove_attribute("inert")
        .map_err(|e| format!("could not show a scene: {e:?}"))
}

fn is_staged(host: &web_sys::Element) -> bool {
    host.has_attribute("inert") || host.has_attribute("style")
}
