//! `panel-game-of-life`: Conway's Game of Life on a board that a person can draw on. See [`build_game_of_life_demo`]
//! for what the page does, [`board`] for the rules and the patterns, and [`view`] for where everything is in the scene.

mod board;
mod view;

use crate::plot_scene::listen;
use board::{Board, PATTERNS, SPACESHIPS, pattern, period_ms, spaceship, steps_due};
use demo_common::{
    force::animation::AnimationLoop,
    util::{document, ensure_svg_in, required_element, stringify},
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::{Rc, Weak},
};
use svg_dom::SvgRoot;
use svg_dom_graph::{
    controls::{
        sliders::{self, Slider},
        svg_buttons::{self, Button},
    },
    scene::Scene,
};
use view::{
    BOARD, CELL, COLS, CONTROLS, FIRST_SPEED, ROWS, SCENE, SLIDER_LEFT, SLIDER_LENGTH, SLIDER_MIDDLE, SPEED_STEP,
    SPEEDS, TOP, board_label, cell_at, cell_origin, cells_between, control_area, cursor_text, grid_path, live_path,
    move_cursor, pattern_area, readout, scene_label, spaceship_area, speed_text,
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the rules and the patterns, and where everything is in the scene. The
/// buttons and the slider are in `src/controls`, which the other demos share.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("game_of_life/board/mod.rs", "board/mod.rs"),
    crate::source_file!("game_of_life/view/mod.rs", "view/mod.rs"),
];

/// The ids of the elements that the page declares. The stage is empty, and gets an `<svg>` of the same name.
const STAGE: &str = "game-of-life-stage";
const SVG: &str = "game-of-life-stage-svg";
const READOUT: &str = "game-of-life-readout";
const CURSOR_READOUT: &str = "game-of-life-cursor";
const ERROR: &str = "game-of-life-error";

/// The id of the slider, which is also what [`sliders::watch`] says moved.
const SPEED: &str = "speed";

/// The actions of the buttons, and what a button for a pattern starts with, followed by the id of the pattern.
const PLAY: &str = "play";
const STEP: &str = "step";
const RANDOM: &str = "random";
const CLEAR: &str = "clear";
const WRAP: &str = "wrap";
const PATTERN: &str = "pattern-";
const SPACESHIP: &str = "spaceship-";

/// The fill and the outline of a button, and of one that is pressed.
const BUTTON: (&str, &str) = ("#ffffff", "#64748b");
const PRESSED: (&str, &str) = ("#ccfbf1", "#0f766e");

/// The colours of the board, the grid, the live cells and the cursor.
const BACKGROUND: &str = "#eef4ff";
const GRID: &str = "#c7d2e0";
const LIVE: &str = "#0f766e";
const CURSOR: &str = "#d63c32";

/// How likely a cell is to be alive on a random board.
const DENSITY: f64 = 0.3;

/// The longest gap between two frames that is counted as time passing, in milliseconds, and the most generations that
/// are worked out in one frame. A page that was in the background for a while resumes from where it was and does not
/// race to catch up.
const LONGEST_FRAME: f64 = 250.0;
const MOST_STEPS: u32 = 4;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
thread_local! {
    static DEMO: RefCell<Option<Rc<Demo>>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A drag across the board that is going on: the pointer that is making it, and what it paints. Only that pointer can
/// move it on or end it, so another finger or a mouse cannot join it or cut it short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Drag {
    pointer: i32,
    /// Whether it brings cells to life, or kills them.
    alive: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the page needs after it is built: the board, what is drawn of it, and whether it is going.
struct Demo {
    board: RefCell<Board>,
    generation: Cell<u64>,
    /// The cell that the keys act on, and that is outlined while the board has the focus.
    cursor: Cell<(usize, usize)>,
    /// The drag across the board that is going on, if there is one.
    drag: Cell<Option<Drag>>,
    /// Whether what is drawn is out of date, since a write to the page failed after the board had changed. The board is
    /// the truth, and every draw is of the whole of it, so the next draw makes the page right.
    stale: Cell<bool>,
    /// A change to the error message that is waiting for a drag to end: `Some(Some(text))` is to show `text`, and
    /// `Some(None)` is to hide it. The message is above the board, so showing or hiding it moves the board on the page.
    /// A pointer that is held down would then be over other cells, and the browser reports that as the pointer moving.
    pending_error: RefCell<Option<Option<String>>>,
    /// The cell that a drag last painted, which the next cell is joined to by a line, or `None` if the pointer is not
    /// on the board, and the next cell it comes to starts a new line.
    last_cell: Cell<Option<(usize, usize)>>,
    playing: Cell<bool>,
    /// Generations in each second.
    speed: Cell<f64>,
    /// The time that has passed since the last generation, in milliseconds, which was not enough for another.
    carried: Cell<f64>,
    /// When the last frame came, or `None` when the next frame has no earlier one to measure from.
    last_frame: Cell<Option<f64>>,
    /// Set once, just after the demo is made, since the loop's callback needs the demo.
    animation: OnceCell<Rc<AnimationLoop>>,
    svg: web_sys::Element,
    layer: web_sys::Element,
    board_group: web_sys::Element,
    live: web_sys::Element,
    cursor_box: web_sys::Element,
    readout: web_sys::Element,
    cursor_readout: web_sys::Element,
    error: web_sys::Element,
    /// Kept so that the scene, which the buttons and the slider are drawn in, lasts as long as the page.
    _scene: Scene,
}

impl Demo {
    /// Writes the live cells, the cursor and the readout for the board as it is now, all from the board.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an attribute could not be written.
    fn render(&self) -> Result<(), String> {
        let board = self.board.borrow();
        set(&self.live, "d", &live_path(&board))?;
        let at = cell_origin(self.cursor.get().0, self.cursor.get().1);
        set(&self.cursor_box, "x", &at.x.to_string())?;
        set(&self.cursor_box, "y", &at.y.to_string())?;
        self.readout
            .set_text_content(Some(&readout(self.generation.get(), board.population())));
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the board as it is now. If a write fails, what is on the page is out of date, which is remembered, and
    /// [`Demo::repair`] draws it again at the next chance. A draw that works after a failure hides what the failure
    /// said.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an attribute could not be written.
    fn draw(&self) -> Result<(), String> {
        match self.render() {
            Ok(()) => {
                if self.stale.replace(false) {
                    self.show_error(None);
                }
                Ok(())
            },
            Err(problem) => {
                self.stale.set(true);
                Err(problem)
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the board again if a write failed and left the page out of date. It is the first thing that every button,
    /// key, drag and frame does, so a failure that was only of the page is put right by the next thing a person does.
    fn repair(&self) {
        if self.stale.get() {
            let _ = self.draw();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows `message` in the error line above the board, or hides it if it is `None`. While a drag goes on, that waits
    /// until it ends, since the board would move under the pointer and the cells it was then over would be painted.
    fn show_error(&self, message: Option<String>) {
        if self.drag.get().is_some() {
            *self.pending_error.borrow_mut() = Some(message);
        } else {
            self.put_error(message);
        }
    }

    /// Writes the error line, and shows or hides it.
    fn put_error(&self, message: Option<String>) {
        match message {
            Some(text) => {
                self.error.set_text_content(Some(&text));
                let _ = self.error.remove_attribute("hidden");
            },
            None => {
                let _ = self.error.set_attribute("hidden", "");
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Says what is under the cursor, which assistive technology reads out, since the cursor is only seen otherwise.
    fn announce_cursor(&self) {
        let (col, row) = self.cursor.get();
        let alive = self.board.borrow().get(col, row);
        self.cursor_readout.set_text_content(Some(&cursor_text((col, row), alive)));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Works out one generation more, and draws it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the board could not be drawn.
    fn step(&self) -> Result<(), String> {
        self.board.borrow_mut().step();
        self.generation.set(self.generation.get() + 1);
        self.draw()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Works out the generations that are due since the last frame, and asks for the next frame. A failure stops it.
    fn frame(&self, now: f64) {
        if !self.playing.get() {
            return;
        }
        self.repair();
        let elapsed = self
            .last_frame
            .get()
            .map_or(0.0, |before| (now - before).clamp(0.0, LONGEST_FRAME));
        self.last_frame.set(Some(now));
        let (due, carried) = steps_due(self.carried.get(), elapsed, period_ms(self.speed.get()), MOST_STEPS);
        self.carried.set(carried);
        let mut result = Ok(());
        for _ in 0..due {
            result = self.step();
            if result.is_err() {
                break;
            }
        }
        match result {
            Ok(()) => self.request_frame(),
            Err(problem) => self.fail(&problem),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn request_frame(&self) {
        if let Some(animation) = self.animation.get() {
            animation.request();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Starts or stops the generations, and writes on the button what it will do next. While they go, the readout is
    /// not announced, since it would be read out every time it changes.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button could not be relabelled.
    fn set_playing(&self, playing: bool) -> Result<(), String> {
        self.playing.set(playing);
        self.last_frame.set(None);
        self.carried.set(0.0);
        match (playing, self.animation.get()) {
            (true, _) => self.request_frame(),
            (false, Some(animation)) => animation.cancel(),
            (false, None) => {},
        }
        set(&self.readout, "aria-live", if playing { "off" } else { "polite" })?;
        svg_buttons::set_label(&self.layer, PLAY, if playing { "Pause" } else { "Play" })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Does what the button with `action` is for, and says on the page if that went wrong.
    fn pressed(&self, action: &str) {
        self.repair();
        let result = match action {
            PLAY => self.set_playing(!self.playing.get()),
            STEP => self.set_playing(false).and_then(|()| self.step()),
            RANDOM => self.random(),
            CLEAR => self.restart(|board| board.clear()),
            WRAP => self.toggle_wrap(),
            other => match (
                other.strip_prefix(PATTERN).and_then(pattern),
                other.strip_prefix(SPACESHIP).and_then(spaceship),
            ) {
                (Some(pattern), _) => self.restart(|board| {
                    board.clear();
                    board.place_centred(pattern);
                }),
                (None, Some(ship)) => self.add(ship),
                (None, None) => Err(format!("there is no button {other:?}")),
            },
        };
        if let Err(problem) = result {
            self.fail(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Changes the board by `change`, which starts a new count of the generations, and draws it.
    fn restart(&self, change: impl FnOnce(&mut Board)) -> Result<(), String> {
        change(&mut self.board.borrow_mut());
        self.generation.set(0);
        self.draw()?;
        self.announce_cursor();
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds `ship` at the left edge of the board, level with the cursor, so that it flies into what is there. The board
    /// is not cleared, and the count of the generations goes on.
    fn add(&self, ship: &board::Pattern) -> Result<(), String> {
        let (_, row) = self.cursor.get();
        self.board.borrow_mut().place_level_with(ship, row);
        self.draw()?;
        self.announce_cursor();
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Fills the board at random.
    fn random(&self) -> Result<(), String> {
        let seed = (js_sys::Math::random() * f64::from(u32::MAX)) as u32;
        self.restart(|board| board.randomise(seed, DENSITY))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the edges wrap round or stop, and shows which on the button and for assistive technology.
    fn toggle_wrap(&self) -> Result<(), String> {
        let wrap = !self.board.borrow().wraps();
        self.board.borrow_mut().set_wrap(wrap);
        set(&self.svg, "aria-label", &scene_label(wrap))?;
        svg_buttons::set_pressed(&self.layer, WRAP, wrap, if wrap { PRESSED } else { BUTTON })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Sets the speed for a person who has moved the slider.
    fn set_speed(&self, speed: f64) {
        self.repair();
        self.speed.set(speed);
        if let Err(problem) = sliders::set_value(&self.svg, SPEED, speed, &speed_text(speed)) {
            self.fail(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Brings every cell of `cells` to life or kills it, for a person who has drawn on them, and draws that once. The
    /// cursor goes to the last of them.
    fn paint(&self, cells: &[(usize, usize)], alive: bool) {
        let mut changed = false;
        for &(col, row) in cells {
            if self.board.borrow().get(col, row) != alive {
                self.board.borrow_mut().set(col, row, alive);
                changed = true;
            }
        }
        if let Some(&last) = cells.last() {
            self.cursor.set(last);
        }
        if changed {
            if let Err(problem) = self.draw() {
                self.fail(&problem);
            }
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves the cursor, or changes the cell that is under it, for a person who has pressed a key.
    fn pressed_key(&self, key: &str) -> bool {
        self.repair();
        if let Some(to) = move_cursor(self.cursor.get(), key) {
            self.cursor.set(to);
            self.announce_cursor();
            if let Err(problem) = self.draw() {
                self.fail(&problem);
            }
            return true;
        }
        if key == " " || key == "Enter" {
            let (col, row) = self.cursor.get();
            self.board.borrow_mut().toggle(col, row);
            self.redraw_after_key();
            return true;
        }
        false
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws a cell that a key has changed, and says what it is now.
    fn redraw_after_key(&self) {
        self.announce_cursor();
        if let Err(problem) = self.draw() {
            self.fail(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the pointer of `event` is in the scene, in the scene's own units. The page can shrink the scene, so that
    /// is the position in the `<svg>` scaled back up.
    fn scene_point(&self, event: &web_sys::PointerEvent) -> (f64, f64) {
        let rect = self.svg.get_bounding_client_rect();
        let scale = if rect.width() > 0.0 { SCENE.width / rect.width() } else { 1.0 };
        (
            (f64::from(event.client_x()) - rect.left()) * scale,
            (f64::from(event.client_y()) - rect.top()) * scale,
        )
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Starts a drag if the pointer is on a cell. The first cell is changed, and the cells that the drag then goes over
    /// are given the same state. If another pointer is already dragging, this one is left alone. If one is recorded
    /// that no longer holds the pointer capture, its end was never reported, so it is ended first.
    fn pointer_down(&self, event: &web_sys::PointerEvent) {
        if event.button() != 0 {
            return;
        }
        self.repair();
        if let Some(active) = self.drag.get() {
            if self.svg.has_pointer_capture(active.pointer) {
                return;
            }
            self.end_drag();
        }
        let (x, y) = self.scene_point(event);
        let Some((col, row)) = cell_at(x, y) else {
            return;
        };
        event.prevent_default();
        let _ = self.svg.set_pointer_capture(event.pointer_id());
        focus(&self.board_group);
        let alive = !self.board.borrow().get(col, row);
        self.drag.set(Some(Drag {
            pointer: event.pointer_id(),
            alive,
        }));
        self.last_cell.set(Some((col, row)));
        self.paint(&[(col, row)], alive);
        self.announce_cursor();
    }

    /// Gives the cells that the pointer has crossed the state that is being drawn. The browser can report the pointer
    /// only now and then, so it can have moved over several cells since the last report, and a line of cells joins the
    /// cell that was painted last to the one that the pointer is over now. A pointer that is off the board paints
    /// nothing, and the cell that it comes back to starts a new line, so that it does not draw across the board. A
    /// pointer that is not the one that is dragging does nothing.
    fn pointer_move(&self, event: &web_sys::PointerEvent) {
        let Some(drag) = self.drag.get() else {
            return;
        };
        if event.pointer_id() != drag.pointer {
            return;
        }
        let (x, y) = self.scene_point(event);
        let Some(at) = cell_at(x, y) else {
            self.last_cell.set(None);
            return;
        };
        let cells = match self.last_cell.replace(Some(at)) {
            Some(before) => cells_between(before, at),
            None => vec![at],
        };
        self.paint(&cells, drag.alive);
    }

    /// Ends the drag if the pointer of `event` is the one that is making it. This is what the pointer being let go, the
    /// browser cancelling it, and the pointer capture being lost all come to, since any of them can be the only one
    /// that is reported.
    fn pointer_end(&self, event: &web_sys::PointerEvent) {
        if self.drag.get().is_some_and(|drag| drag.pointer == event.pointer_id()) {
            self.end_drag();
        }
    }

    /// Ends the drag that is going on, if there is one, and lets go of the pointer. It can be called again, and does
    /// nothing then, since the pointer being let go is followed by the capture being lost.
    fn end_drag(&self) {
        let Some(drag) = self.drag.replace(None) else {
            return;
        };
        self.last_cell.set(None);
        if let Some(message) = self.pending_error.take() {
            self.put_error(message);
        }
        let _ = self.svg.release_pointer_capture(drag.pointer);
        self.announce_cursor();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops the generations and says why on the page. The board is right, and what is drawn may not be, so the message
    /// says that, and that the next button, key or drawing on the board draws it again.
    fn fail(&self, problem: &str) {
        // The button is put back to Play and the readout to being announced, which can fail as the draw did.
        let _ = self.set_playing(false);
        self.show_error(Some(format!(
            "The board could not be drawn: {problem}. What is shown may be out of date. The next button, key or \
             drawing on the board draws it again."
        )));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the person has asked their system for less motion.
fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|window| window.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .is_some_and(|query| query.matches())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives the keyboard focus to `element`, if it is one that can have it.
fn focus(element: &web_sys::Element) {
    use wasm_bindgen::JsCast;
    if let Some(element) = element.dyn_ref::<web_sys::SvgElement>() {
        let _ = element.focus();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sets the attribute `name` of `element` to `value`.
fn set(element: &web_sys::Element, name: &str, value: &str) -> Result<(), String> {
    element
        .set_attribute(name, value)
        .map_err(|e| format!("could not set {name}: {e:?}"))
}

/// Makes an SVG element called `tag` with `attributes`.
fn make(document: &web_sys::Document, tag: &str, attributes: &[(&str, &str)]) -> Result<web_sys::Element, String> {
    let element = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), tag)
        .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
    for (name, value) in attributes {
        set(&element, name, value)?;
    }
    Ok(element)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is drawn in the scene once: the group for the board, which can take the keyboard focus, and what is in it.
struct Parts {
    scene: Scene,
    svg: web_sys::Element,
    layer: web_sys::Element,
    board_group: web_sys::Element,
    live: web_sys::Element,
    cursor_box: web_sys::Element,
}

/// Draws the scene in the empty `#game-of-life-stage`: a library scene, since the buttons are drawn in its layer, and
/// in the layer the board with its grid, its live cells and its cursor.
///
/// # Errors
///
/// Returns `Err` if the page has no stage, or if an element could not be made or added.
fn draw_scene(document: &web_sys::Document, wrap: bool) -> Result<Parts, String> {
    ensure_svg_in(document, STAGE, SVG, SCENE)?;
    let scene = Scene::new(SvgRoot::attach(SVG).map_err(stringify)?).map_err(stringify)?;
    let svg = required_element(document, SVG)?;
    set(&svg, "role", "group")?;
    set(&svg, "aria-label", &scene_label(wrap))?;
    // A finger that draws on the board must not scroll the page.
    set(&svg, "style", "touch-action: none;")?;
    let layer = svg_buttons::layer(document, SVG)?;

    let board_group = make(
        document,
        "g",
        &[
            ("class", "life-board"),
            ("role", "application"),
            ("tabindex", "0"),
            ("aria-label", &board_label()),
        ],
    )?;
    let background = make(
        document,
        "rect",
        &[
            ("x", "0"),
            ("y", &TOP.to_string()),
            ("width", &BOARD.width.to_string()),
            ("height", &BOARD.height.to_string()),
            ("fill", BACKGROUND),
        ],
    )?;
    let grid = make(
        document,
        "path",
        &[("d", &grid_path()), ("fill", "none"), ("stroke", GRID), ("stroke-width", "1")],
    )?;
    let live = make(document, "path", &[("fill", LIVE)])?;
    let cursor_box = make(
        document,
        "rect",
        &[
            ("class", "life-cursor"),
            ("width", &CELL.to_string()),
            ("height", &CELL.to_string()),
            ("stroke", CURSOR),
        ],
    )?;
    for child in [&background, &grid, &live, &cursor_box] {
        board_group
            .append_child(child)
            .map_err(|e| format!("could not add to the board: {e:?}"))?;
    }
    layer
        .append_child(&board_group)
        .map_err(|e| format!("could not add the board to the scene: {e:?}"))?;
    Ok(Parts {
        scene,
        svg,
        layer,
        board_group,
        live,
        cursor_box,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Game of Life demo: a board of 40 by 30 cells that a person draws on, and that lives by Conway's rules.
///
/// The page shows one scene, 560 pixels wide, with:
///
/// 1. At the top, the buttons: Play or Pause, Step, Random, Clear, and Wrap edges, which says whether a cell on one
///    edge is next to the cells on the opposite edge. Under them is a slider for the speed, from 1 to 30 generations in
///    each second.
/// 2. The board. A click or a touch brings a cell to life or kills it, and a drag gives every cell that it goes over
///    the same state as the first. With the focus on the board, the arrow keys move a cursor and the Space or Enter key
///    changes the cell under it.
/// 3. At the bottom, two rows of buttons. The first has buttons for patterns that replace the board: a glider, a
///    blinker, the R-pentomino, the acorn and the Gosper glider gun. The second has buttons that add a light, medium or
///    heavy weight spaceship to what is on the board, at the left edge and level with the cursor, flying to the right.
///
/// The generation and the number of live cells are written above the scene. The page starts with a glider, going,
/// unless the person has asked their system for less motion.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if the scene or a listener could not be made.
pub(crate) fn build_game_of_life_demo() -> Result<(), String> {
    let document = document()?;
    let mut first = Board::new(COLS, ROWS, true);
    first.place_centred(&PATTERNS[0]);
    let parts = draw_scene(&document, first.wraps())?;

    for (index, &(action, label, _)) in CONTROLS.iter().enumerate() {
        let wrap = action == WRAP;
        let button = Button {
            label,
            action,
            colours: if wrap { PRESSED } else { BUTTON },
            pressed: wrap.then_some(true),
        };
        svg_buttons::add_button(&document, &parts.layer, control_area(index), &button)?;
    }
    for (index, pattern) in PATTERNS.iter().enumerate() {
        let action = format!("{PATTERN}{}", pattern.id);
        let button = Button {
            label: pattern.name,
            action: &action,
            colours: BUTTON,
            pressed: None,
        };
        svg_buttons::add_button(&document, &parts.layer, pattern_area(index, PATTERNS.len()), &button)?;
    }
    for (index, ship) in SPACESHIPS.iter().enumerate() {
        let action = format!("{SPACESHIP}{}", ship.id);
        let button = Button {
            label: ship.name,
            action: &action,
            colours: BUTTON,
            pressed: None,
        };
        svg_buttons::add_button(&document, &parts.layer, spaceship_area(index, SPACESHIPS.len()), &button)?;
    }
    let slider = Slider {
        label: "Speed",
        id: SPEED,
        left: SLIDER_LEFT,
        middle: SLIDER_MIDDLE,
        width: SLIDER_LENGTH,
        range: SPEEDS,
        step: SPEED_STEP,
    };
    sliders::add_slider(&document, &parts.svg, &slider, FIRST_SPEED, &speed_text(FIRST_SPEED))?;

    let demo = Rc::new(Demo {
        board: RefCell::new(first),
        generation: Cell::new(0),
        cursor: Cell::new((COLS / 2, ROWS / 2)),
        drag: Cell::new(None),
        stale: Cell::new(false),
        pending_error: RefCell::new(None),
        last_cell: Cell::new(None),
        playing: Cell::new(false),
        speed: Cell::new(FIRST_SPEED),
        carried: Cell::new(0.0),
        last_frame: Cell::new(None),
        animation: OnceCell::new(),
        svg: parts.svg,
        layer: parts.layer,
        board_group: parts.board_group,
        live: parts.live,
        cursor_box: parts.cursor_box,
        readout: required_element(&document, READOUT)?,
        cursor_readout: required_element(&document, CURSOR_READOUT)?,
        error: required_element(&document, ERROR)?,
        _scene: parts.scene,
    });
    // The loop holds the demo weakly, since the demo holds the loop.
    let weak: Weak<Demo> = Rc::downgrade(&demo);
    let animation = AnimationLoop::new(move |now| {
        if let Some(demo) = weak.upgrade() {
            demo.frame(now);
        }
    })?;
    let _ = demo.animation.set(animation);

    demo.draw()?;
    demo.announce_cursor();

    let pressed = Rc::clone(&demo);
    svg_buttons::watch(&document, STAGE, move |action| pressed.pressed(action))?;
    let moved = Rc::clone(&demo);
    sliders::watch(&document, STAGE, move |_, speed| moved.set_speed(speed))?;
    for event in ["pointerdown", "pointermove", "pointerup", "pointercancel", "lostpointercapture"] {
        let demo = Rc::clone(&demo);
        listen(&demo.svg.clone(), event, move |e: web_sys::PointerEvent| {
            match e.type_().as_str() {
                "pointerdown" => demo.pointer_down(&e),
                "pointermove" => demo.pointer_move(&e),
                _ => demo.pointer_end(&e),
            }
        })?;
    }
    let typed = Rc::clone(&demo);
    listen(&demo.board_group.clone(), "keydown", move |e: web_sys::KeyboardEvent| {
        if typed.pressed_key(&e.key()) {
            e.prevent_default();
        }
    })?;

    demo.set_playing(!prefers_reduced_motion())?;
    DEMO.with_borrow_mut(|slot| *slot = Some(demo));
    Ok(())
}
