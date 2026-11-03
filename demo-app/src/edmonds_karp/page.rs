//! The page: the scene that is drawn, the presentation that was drawn last, and the controls. It is what the controller
//! draws on.

use super::{
    chrome::Chrome,
    controller::{Surface, SwitchError},
    controls::{ControlState, Controls, Journal},
    dom::{add, focus, make},
    fixtures::Fixture,
    geometry::geometry,
    layout::TOP,
    projection::{Presentation, Target, View, bounds, diff, full, present_at},
    scene::{Scene, apply_scene_writes, apply_writes, build_scene},
    solver::Trace,
};
use std::cell::{Cell, RefCell};
use wasm_bindgen::{JsCast, JsValue};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(super) struct Page {
    document: web_sys::Document,
    /// The group that the scene is put in, and replaced in. The controls are beside it in the svg and stay.
    host: web_sys::Element,
    controls: Controls,
    chrome: Chrome,
    scene: RefCell<Scene>,
    /// The presentation that is on the page, or `None` when a write failed and what is there is not known.
    committed: RefCell<Option<Presentation>>,
    /// What the controls show, or `None` when a write to them failed.
    control_state: RefCell<Option<ControlState>>,
    key: Cell<&'static str>,
    instance: u32,
    playing: Cell<bool>,
    /// Whether every draw is a full one, which a test asks for, to compare a page that was diffed with one that was
    /// not.
    always_full: bool,
    error_shown: Cell<bool>,
    said: Cell<u32>,
}

impl Page {
    /// Draws the controls and the scene of `fixture` in `svg`, and finds the elements that the panel declares. The page
    /// is ready to be drawn on, and has drawn no snapshot yet.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the panel is missing an element, or an element could not be made.
    pub(super) fn new(
        document: &web_sys::Document,
        svg: &web_sys::Element,
        fixture: &'static Fixture,
        trace: &Trace,
        view: &View,
        instance: u32,
        always_full: bool,
    ) -> Result<Self, String> {
        let state = ControlState {
            bounds: bounds(trace, 0),
            prefs: view.prefs,
            index: 0,
            last: trace.last(),
            fixture_key: fixture.key,
            speed: view.speed,
            playing: false,
        };
        let controls = Controls::build(document, svg, &state)?;
        let host = make(
            document,
            "g",
            &[("class", "ek-host"), ("transform", &format!("translate(0 {TOP})"))],
        )?;
        add(svg, &host)?;
        let scene = build_scene(document, fixture, trace, &geometry(fixture, trace), instance, 0)?;
        add(&host, &scene.root)?;
        Ok(Self {
            document: document.clone(),
            host,
            controls,
            chrome: Chrome::find(document)?,
            scene: RefCell::new(scene),
            committed: RefCell::new(None),
            control_state: RefCell::new(None),
            key: Cell::new(fixture.key),
            instance,
            playing: Cell::new(false),
            always_full,
            error_shown: Cell::new(false),
            said: Cell::new(0),
        })
    }

    pub(super) fn controls(&self) -> &Controls {
        &self.controls
    }

    /// Gives the keyboard focus to the element of `target`.
    pub(super) fn focus_target(&self, target: Target) {
        if let Some(element) = self.scene.borrow().element_of(target) {
            focus(&element);
        }
    }

    /// Whether the keyboard focus is somewhere in the scene.
    fn graph_has_focus(&self) -> bool {
        self.document
            .active_element()
            .is_some_and(|active| self.scene.borrow().contains(&active))
    }

    fn hide_error(&self) {
        if self.error_shown.replace(false) {
            let _ = self.chrome.error.set_attribute("hidden", "");
        }
    }

    fn show_error(&self, message: &str) {
        self.chrome.error.set_text_content(Some(message));
        let _ = self.chrome.error.remove_attribute("hidden");
        self.error_shown.set(true);
    }

    /// What the controls should show for the snapshot at `index`.
    fn control_state(&self, trace: &Trace, index: usize, view: &View) -> ControlState {
        ControlState {
            bounds: bounds(trace, index),
            prefs: view.prefs,
            index,
            last: trace.last(),
            fixture_key: self.key.get(),
            speed: view.speed,
            playing: self.playing.get(),
        }
    }

    /// Replaces what is in the host with `root`, in the one call the browser has for it.
    ///
    /// If the call throws, the host may be untouched or already changed, so what was there is taken out of it again and
    /// put back, as far as that works. The call is made by name, so that a test can make the real one throw.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the call threw.
    fn replace_scene(&self, root: &web_sys::Element) -> Result<(), String> {
        let mut old = Vec::new();
        let mut child = self.host.first_child();
        while let Some(node) = child {
            child = node.next_sibling();
            old.push(node);
        }
        let called = js_sys::Reflect::get(&self.host, &JsValue::from_str("replaceChildren"))
            .and_then(|function| function.dyn_into::<js_sys::Function>())
            .and_then(|function| function.call1(&self.host, root));
        let Err(problem) = called else {
            return Ok(());
        };
        while let Some(node) = self.host.first_child() {
            if self.host.remove_child(&node).is_err() {
                break;
            }
        }
        for node in &old {
            let _ = self.host.append_child(node);
        }
        Err(format!("the scene could not be replaced: {problem:?}"))
    }
}

fn seek_text(index: usize, last: usize) -> String {
    format!("Step {index} of {last}")
}

impl Surface for Page {
    fn draw(&self, trace: &Trace, index: usize, view: &View, full_draw: bool) -> Result<(), String> {
        let next = present_at(trace, index, view).ok_or_else(|| format!("there is no snapshot {index}"))?;
        let everything = full_draw || self.always_full;
        // Taken, so that a write that fails leaves nothing that claims to be on the page.
        let before = self.committed.borrow_mut().take().filter(|_| !everything);
        let previous_controls = self.control_state.borrow_mut().take().filter(|_| !everything);
        let writes = match &before {
            Some(before) => diff(before, &next),
            None => full(&next),
        };
        apply_writes(&self.scene.borrow(), &self.chrome, &writes)?;
        let state = self.control_state(trace, index, view);
        // Kept, so that a control that was changed before a write failed is put back to what the page still commits.
        let mut journal = Journal::default();
        if let Err(problem) = self.controls.write_state(
            &mut journal,
            previous_controls.as_ref(),
            &state,
            &seek_text(index, trace.last()),
        ) {
            journal.restore();
            return Err(problem);
        }
        *self.committed.borrow_mut() = Some(next);
        *self.control_state.borrow_mut() = Some(state);
        self.hide_error();
        Ok(())
    }

    fn switch(
        &self,
        fixture: &'static Fixture,
        trace: &Trace,
        view: &View,
        generation: u64,
    ) -> Result<(), SwitchError> {
        // Everything that can fail without touching the live page is done first, on a scene that is not in it.
        let candidate = build_scene(
            &self.document,
            fixture,
            trace,
            &geometry(fixture, trace),
            self.instance,
            generation,
        )
        .map_err(SwitchError::Detached)?;
        let presentation = present_at(trace, 0, view)
            .ok_or_else(|| SwitchError::Detached("the new graph has no first snapshot".to_owned()))?;
        apply_scene_writes(&candidate, &full(&presentation)).map_err(SwitchError::Detached)?;
        let had_focus = self.graph_has_focus();
        // Then the live controls and the text panel, through a journal, so that a failure can put them back.
        let mut journal = Journal::default();
        let state = ControlState {
            bounds: bounds(trace, 0),
            prefs: view.prefs,
            index: 0,
            last: trace.last(),
            fixture_key: fixture.key,
            speed: view.speed,
            playing: false,
        };
        let written = self
            .controls
            .write_state(&mut journal, None, &state, &seek_text(0, trace.last()))
            .map(|()| {
                for (field, text) in self.chrome.panel.iter().zip(&presentation.panel) {
                    journal.text(field, text);
                }
            });
        if let Err(problem) = written {
            journal.restore();
            self.forget();
            return Err(SwitchError::Live(problem));
        }
        // Last, the one replacement of the scene.
        if let Err(problem) = self.replace_scene(&candidate.root) {
            journal.restore();
            self.forget();
            return Err(SwitchError::Live(problem));
        }
        *self.scene.borrow_mut() = candidate;
        *self.committed.borrow_mut() = Some(presentation);
        *self.control_state.borrow_mut() = Some(state);
        self.key.set(fixture.key);
        self.playing.set(false);
        if had_focus {
            self.focus_target(Target::Vertex(0));
        }
        self.hide_error();
        Ok(())
    }

    fn playing_changed(&self, playing: bool) -> Result<(), String> {
        self.playing.set(playing);
        let mut state = self.control_state.borrow_mut();
        // The label is written whether or not the rest of what the controls show is known, since a switch that failed
        // puts the old label back, and the person must see what Play will do now.
        let mut journal = Journal::default();
        let written = self.controls.write_play(&mut journal, playing);
        match (&written, state.as_mut()) {
            (Ok(()), Some(shown)) => shown.playing = playing,
            // The label could not be written, so what the controls show is not known, and the next draw writes it all.
            // What was written before the failure is put back, so that the button does not show half a change.
            (Err(_), _) => {
                journal.restore();
                *state = None;
            },
            (Ok(()), None) => {},
        }
        written
    }

    fn say(&self, text: &str) {
        // A message that repeats the last one is still read out, since the text differs by an invisible space.
        let n = self.said.get();
        self.said.set(n.wrapping_add(1));
        let tail = if n % 2 == 0 { "" } else { "\u{a0}" };
        self.chrome.live.set_text_content(Some(&format!("{text}{tail}")));
    }

    fn failed(&self, problem: &str) {
        let message = format!(
            "The graph could not be drawn: {problem}. What is shown may be out of date. The next button, key or move of the \
             range draws it again."
        );
        self.show_error(&message);
        self.say(&message);
    }

    fn playback_failed(&self, problem: &str) {
        let message = format!(
            "Play could not carry on: {problem}. The graph is right, and the run is paused where it is. Press Play to try \
             again."
        );
        self.show_error(&message);
        self.say(&message);
    }

    fn warn(&self, text: &str) {
        self.show_error(&format!("Warning: {text}"));
    }
}

impl Page {
    /// Says that what is on the page is not known, so that the next draw writes all of it.
    fn forget(&self) {
        *self.committed.borrow_mut() = None;
        *self.control_state.borrow_mut() = None;
    }
}
