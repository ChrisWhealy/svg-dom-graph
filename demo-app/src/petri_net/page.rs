//! The page: the scene that is drawn once, the presentation that was drawn last, and the controls. It is what the
//! controller draws on.

use super::{
    chrome::Chrome,
    controller::{Frame, Surface},
    controls::{ControlState, Controls, Journal},
    dom::focus,
    net::Net,
    projection::{Presentation, Source, Target, bounds, diff, full, present},
    scene::{Built, apply_writes, build_scene},
};
use std::cell::{Cell, RefCell};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(super) struct Page {
    /// The stage's `<svg>`, which the page puts the name of the scenario on.
    root: web_sys::Element,
    controls: Controls,
    chrome: Chrome,
    built: Built,
    /// The presentation that is on the page, or `None` when a write failed and what is there is not known.
    committed: RefCell<Option<Presentation>>,
    /// What the controls show, or `None` when a write to them failed.
    control_state: RefCell<Option<ControlState>>,
    playing: Cell<bool>,
    /// Whether every draw is a full one, which a test asks for, to compare a page that was diffed with one that was
    /// not.
    always_full: bool,
    error_shown: Cell<bool>,
    said: Cell<u32>,
}

impl Page {
    /// Draws the controls and the scene in `svg`, and finds the elements that the panel declares. The page is ready to
    /// be drawn on, and has drawn no snapshot yet.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the panel is missing an element, or an element could not be made.
    pub(super) fn new(
        document: &web_sys::Document,
        svg: &web_sys::Element,
        net: &Net,
        state: &ControlState,
        instance: u32,
        always_full: bool,
    ) -> Result<Self, String> {
        let controls = Controls::build(document, svg, state)?;
        let built = build_scene(document, svg, net, instance)?;
        // The toolbar is laid out against the size of the scene, so it is measured again now that the scene is in
        // place.
        built.scene().refresh_toolbar_layout().map_err(|e| e.to_string())?;
        Ok(Self {
            root: svg.clone(),
            controls,
            chrome: Chrome::find(document)?,
            built,
            committed: RefCell::new(None),
            control_state: RefCell::new(None),
            playing: Cell::new(false),
            always_full,
            error_shown: Cell::new(false),
            said: Cell::new(0),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(super) fn controls(&self) -> &Controls {
        &self.controls
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Gives the keyboard focus to the element of `target`.
    pub(super) fn focus_target(&self, target: Target) {
        if let Some(element) = self.built.element_of(target) {
            focus(&element);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn hide_error(&self) {
        if self.error_shown.replace(false) {
            let _ = self.chrome.error.set_attribute("hidden", "");
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn show_error(&self, message: &str) {
        self.chrome.error.set_text_content(Some(message));
        let _ = self.chrome.error.remove_attribute("hidden");
        self.error_shown.set(true);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the controls should show for `frame`.
    fn control_state(&self, frame: &Frame) -> ControlState {
        ControlState {
            bounds: bounds(frame.net, frame.trace, frame.index, frame.source, frame.view),
            wiring: frame.view.wiring,
            index: frame.index,
            last: frame.trace.last(),
            scenario: frame.source.scenario(),
            manual: frame.source.is_manual(),
            speed: frame.view.speed,
            playing: self.playing.get(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the name of the scenario and the kind of history on the stage, which is what a test reads them from.
    fn write_root(&self, source: Source) -> Result<(), String> {
        super::dom::set(&self.root, "data-pn-scenario", source.scenario().key())?;
        super::dom::set(
            &self.root,
            "data-pn-history",
            if source.is_manual() { "manual" } else { "script" },
        )
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the sentence that stays on the page until the next successful action, or hides the line if there is none.
    fn write_notice(&self, notice: &str) {
        self.chrome.notice.set_text_content(Some(notice));
        if notice.is_empty() {
            let _ = self.chrome.notice.set_attribute("hidden", "");
        } else {
            let _ = self.chrome.notice.remove_attribute("hidden");
        }
    }
}

impl Surface for Page {
    fn draw(&self, frame: &Frame, full_draw: bool) -> Result<(), String> {
        let next = present(frame.net, frame.trace, frame.index, frame.source, frame.view)
            .ok_or_else(|| format!("there is no snapshot {}", frame.index))?;
        let everything = full_draw || self.always_full;
        // Taken, so that a write that fails leaves nothing that claims to be on the page.
        let before = self.committed.borrow_mut().take().filter(|_| !everything);
        let previous_controls = self.control_state.borrow_mut().take().filter(|_| !everything);
        let writes = match &before {
            Some(before) => diff(before, &next),
            None => full(&next),
        };
        apply_writes(&self.built, &self.chrome, &writes)?;
        self.write_root(frame.source)?;
        let state = self.control_state(frame);
        // Kept, so that a control that was changed before a write failed is put back to what the page still commits.
        let mut journal = Journal::default();
        if let Err(problem) = self.controls.write_state(&mut journal, previous_controls.as_ref(), &state) {
            journal.restore();
            return Err(problem);
        }
        self.write_notice(frame.notice);
        *self.committed.borrow_mut() = Some(next);
        *self.control_state.borrow_mut() = Some(state);
        self.hide_error();
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn playing_changed(&self, playing: bool) -> Result<(), String> {
        self.playing.set(playing);
        let mut state = self.control_state.borrow_mut();
        // The label is written whether or not the rest of what the controls show is known.
        let mut journal = Journal::default();
        let written = self.controls.write_play(&mut journal, playing);
        match (&written, state.as_mut()) {
            (Ok(()), Some(shown)) => shown.playing = playing,
            // What the controls show is not known now, so the next draw writes it all.
            (Err(_), _) => {
                journal.restore();
                *state = None;
            },
            (Ok(()), None) => {},
        }
        written
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn say(&self, text: &str) {
        // A message that repeats the last one is still read out, since the text differs by an invisible space.
        let n = self.said.get();
        self.said.set(n.wrapping_add(1));
        let tail = if n % 2 == 0 { "" } else { "\u{a0}" };
        self.chrome.live.set_text_content(Some(&format!("{text}{tail}")));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn failed(&self, problem: &str) {
        let message = format!(
            "The net could not be drawn: {problem}. What is shown may be out of date. The next button, key or move \
             of the range draws it again."
        );
        self.show_error(&message);
        self.say(&message);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn playback_failed(&self, problem: &str) {
        let message = format!(
            "Play could not carry on: {problem}. The net is right, with the script paused at its current location. \
             Press Play to try again."
        );
        self.show_error(&message);
        self.say(&message);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn warn(&self, text: &str) {
        self.show_error(&format!("Warning: {text}"));
    }
}
