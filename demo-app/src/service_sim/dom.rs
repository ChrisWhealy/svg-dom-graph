//! The page side of a session: it makes a scene in a host that nobody can see yet, reads it back, and shows it.
//!
//! None of the simulator's rules is here. The coordinator in `service-sim` decides when a candidate may replace the
//! session on the page, and this is the DOM that it asks about. A candidate is built into a host of its own that is
//! connected, laid out and measurable, but out of the flow of the panel and `inert`, so that it takes no room and
//! nobody can reach, click or hear it (design D3, shown by the P0 staging probe). Publishing removes that.
//!
//! Only the first scene is published at present. A scenario with another number of servers needs the old scene to be
//! handed off, which is the lifecycle stage, and a request for one is refused here, so that the coordinator rolls it
//! back and leaves the page as it was.

use service_sim::controller::{Controller, DrawFailure, Factory, Host, Presence};
use service_sim::host::{BrowserScheduler, PageHost};
use service_sim::projection::{Projection, View};
use service_sim::session::{Candidate, SessionHost, Spec};
use service_sim::surface::Surface;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use svg_dom::SvgRoot;
use svg_dom::root::utils::Size;
use wasm_bindgen::JsValue;

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
    /// Whether a write of the Play control failed, so that it may not say what the controller says. The controller is not dirty then, since nothing that
    /// it drew was lost, and only a repair puts the control right.
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
}

impl DomSessions {
    pub(super) fn new(document: web_sys::Document, stage: web_sys::Element, voices: Voices, notify: Notify) -> Self {
        Self {
            document,
            stage,
            voices,
            notify,
            built: 0,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn host_element(&self, token: u32) -> Result<web_sys::Element, String> {
        self.document
            .get_element_by_id(&format!("dess-host-{token}"))
            .ok_or_else(|| format!("host {token} is not on the page"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn remove(&self, token: u32) {
        if let Ok(host) = self.host_element(token) {
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
        let (width, height) = Surface::view_box();
        let svg = SvgRoot::create_in(&host_id, Size::new(width, height)).map_err(|e| e.to_string())?;
        svg.root.set_id(&svg_id);
        svg.set_view_box(0.0, 0.0, width, height).map_err(|e| e.to_string())?;
        let element = self
            .document
            .get_element_by_id(&svg_id)
            .ok_or("the host element was not made")?;
        let surface = Surface::build(&svg_id, spec.servers)?;
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
            Ok(controller) => Ok(Candidate { controller, token }),
            Err(e) => {
                self.remove(token);
                Err(e)
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn publish(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        if from.is_some() {
            return Err("a scenario with another number of servers cannot be shown yet".to_owned());
        }
        let host = self.host_element(*to)?;
        host.remove_attribute("style")
            .map_err(|e| format!("could not show the scene: {e:?}"))?;
        host.remove_attribute("inert")
            .map_err(|e| format!("could not show the scene: {e:?}"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn verify_published(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        let host = self.host_element(*to)?;
        if host.has_attribute("inert") || host.has_attribute("style") || !host.is_connected() {
            return Err("the new scene is still staged".to_owned());
        }
        match from {
            Some(from) => {
                let old = self.host_element(*from)?;
                if old.has_attribute("inert") && old.has_attribute("style") {
                    Ok(())
                } else {
                    Err("the old scene is still shown beside the new one".to_owned())
                }
            },
            None => Ok(()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn undo_publish(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        let host = self.host_element(*to)?;
        host.set_attribute("style", STAGED_STYLE).map_err(|e| format!("{e:?}"))?;
        host.set_attribute("inert", "").map_err(|e| format!("{e:?}"))?;
        match from {
            Some(from) => {
                let old = self.host_element(*from)?;
                old.remove_attribute("style").map_err(|e| format!("{e:?}"))?;
                old.remove_attribute("inert").map_err(|e| format!("{e:?}"))
            },
            None => Ok(()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn discard(&mut self, token: u32) {
        self.remove(token);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn finalize(&mut self, token: u32) {
        self.remove(token);
    }
}
