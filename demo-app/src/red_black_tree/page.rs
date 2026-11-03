//! The page: the tree that is drawn, the controls, and the focus. It is what the controller draws on.
//!
//! Each snapshot is built whole, as a board of two layers: the real [`Scene`], whose circles and connectors are the
//! picture, and above it a semantic overlay, a `role="tree"` of transparent hit circles, which is the only thing a
//! screen reader or the keyboard sees. A board is built in a hidden place that is in the document, so that the label of
//! every node can be measured, and it is swapped in with one call only when all of it has been built.

use super::{
    a11y::{Item, items, resolve, resolve_roving, selection_note, tree_name},
    chrome::Chrome,
    controller::{CommitError, Surface, View},
    dom::{Journal, add, focus, make, set},
    layout::{BUTTONS, RADIUS, VIEW_HEIGHT, VIEW_WIDTH, centres},
    solver::{Colour, Key, Snapshot, Trace},
};
use demo_common::util::stringify;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};
use svg_dom::{SvgRoot, root::utils::Point};
use svg_dom_graph::scene::{ConnectorOptions, ConnectorType, EllipseOptions, InputMode, Scene};
use wasm_bindgen::{JsCast, JsValue};

/// The fills of the two colours of node. The letter in each label says the colour as well.
const BLACK_FILL: &str = "#94a3b8";
const RED_FILL: &str = "#fecaca";

/// One board: the scene, its overlay, and the item of every key.
struct Board {
    root: web_sys::Element,
    /// Kept so that the scene, and the listeners it put on the page, live as long as the board is shown.
    _scene: Scene,
    overlay: web_sys::Element,
    items: BTreeMap<Key, web_sys::Element>,
}

/// Where the keyboard focus was before a swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    /// On the item of this key.
    Item(Key),
    /// On the tree itself, which is where it goes when there is no key.
    Tree,
}

/// Why the one call that swaps the board in failed.
struct ReplaceError {
    problem: String,
    /// Whether the old board is back.
    restored: bool,
}

pub(super) struct Page {
    document: web_sys::Document,
    chrome: Chrome,
    board: RefCell<Option<Board>>,
    instance: u32,
    generation: Cell<u64>,
    said: Cell<u32>,
    /// Set while the page itself moves the focus, so that the focus event it causes is not taken for a person's.
    restoring: Cell<bool>,
    error_shown: Cell<bool>,
}

impl Page {
    pub(super) fn new(document: &web_sys::Document, instance: u32, last: usize) -> Result<Self, String> {
        Ok(Self {
            document: document.clone(),
            chrome: Chrome::build(document, last)?,
            board: RefCell::new(None),
            instance,
            generation: Cell::new(0),
            said: Cell::new(0),
            restoring: Cell::new(false),
            error_shown: Cell::new(false),
        })
    }

    pub(super) fn chrome(&self) -> &Chrome {
        &self.chrome
    }

    /// Whether the page is moving the focus itself.
    pub(super) fn is_restoring(&self) -> bool {
        self.restoring.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Builds the board of `snapshot` in the hidden place, and checks it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if anything could not be made. What was made is taken out again.
    fn build_board(&self, snapshot: &Snapshot, view: &View) -> Result<Board, String> {
        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        let scene_id = format!("rbt-{}-g{generation}-scene", self.instance);
        let root = make(&self.document, "g", &[("class", "rbt-board")])?;
        add(&self.chrome.staging, &root)?;
        match self.fill_board(&root, &scene_id, snapshot, view) {
            Ok(board) => Ok(board),
            Err(problem) => {
                root.remove();
                Err(problem)
            },
        }
    }

    fn fill_board(
        &self,
        root: &web_sys::Element,
        scene_id: &str,
        snapshot: &Snapshot,
        view: &View,
    ) -> Result<Board, String> {
        let (width, height) = (VIEW_WIDTH.to_string(), VIEW_HEIGHT.to_string());
        let view_box = format!("0 0 {width} {height}");
        let svg = make(
            &self.document,
            "svg",
            &[
                ("id", scene_id),
                ("class", "rbt-scene"),
                ("x", "0"),
                ("y", "0"),
                ("width", &width),
                ("height", &height),
                ("viewBox", &view_box),
                // The scene is the picture. The overlay is what is read, so the picture is left out of the tree.
                ("aria-hidden", "true"),
            ],
        )?;
        add(root, &svg)?;
        let scene = Scene::new(SvgRoot::attach(scene_id).map_err(stringify)?).map_err(stringify)?;
        // Panning or zooming would move the circles away from the overlay, and would add a tab stop of its own.
        scene.set_pan_mode(InputMode::Off).map_err(stringify)?;
        scene.set_wheel_zoom_mode(InputMode::Off).map_err(stringify)?;

        let mut ids = BTreeMap::new();
        for (key, (x, y)) in centres(snapshot) {
            let colour = snapshot.node(key).map_or(Colour::Black, |n| n.colour);
            let options = EllipseOptions::default().with_fill(Some(match colour {
                Colour::Black => BLACK_FILL,
                Colour::Red => RED_FILL,
            }));
            let label = format!("{key} {}", colour.letter());
            let id = scene
                .add_circle_node_with(Point::new(x, y), RADIUS, label, options)
                .map_err(stringify)?;
            ids.insert(key, id);
        }
        let straight = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
        for &(parent, child) in &snapshot.edges {
            let (from, to) = (ids.get(&parent), ids.get(&child));
            let (Some(&from), Some(&to)) = (from, to) else {
                return Err(format!("the edge {parent} to {child} has no node"));
            };
            scene.add_edge_with(from, to, straight).map_err(stringify)?;
        }

        // The overlay is measured from the nodes as they were drawn, since a label can make a circle larger.
        let mut circles = BTreeMap::new();
        for (&key, &id) in &ids {
            let rect = scene.node_rect(id).map_err(stringify)?;
            let (cx, cy) = (rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0);
            circles.insert(key, (cx, cy, rect.size.width / 2.0));
        }
        let overlay = make(
            &self.document,
            "svg",
            &[
                ("class", "rbt-overlay"),
                ("x", "0"),
                ("y", "0"),
                ("width", &width),
                ("height", &height),
                ("viewBox", &view_box),
                ("role", "tree"),
                ("aria-label", &tree_name(snapshot)),
                ("tabindex", "-1"),
            ],
        )?;
        add(root, &overlay)?;
        let all = items(snapshot, resolve(view.chosen, snapshot));
        let mut elements = BTreeMap::new();
        if let Some(top) = snapshot.root {
            let tree = self.item_element(snapshot, &all, &circles, top, &mut elements)?;
            add(&overlay, &tree)?;
        }
        if elements.len() != snapshot.nodes.len() || ids.len() != snapshot.nodes.len() {
            return Err("the board has a different number of keys than the snapshot".to_owned());
        }
        apply_view(&elements, snapshot, view)?;
        Ok(Board {
            root: root.clone(),
            _scene: scene,
            overlay,
            items: elements,
        })
    }

    /// The treeitem of `key` and, inside it, a group of the items of its children. The hit circle is placed by its own
    /// centre, so that nesting the groups does not move anything.
    fn item_element(
        &self,
        snapshot: &Snapshot,
        all: &BTreeMap<Key, Item>,
        circles: &BTreeMap<Key, (f64, f64, f64)>,
        key: Key,
        elements: &mut BTreeMap<Key, web_sys::Element>,
    ) -> Result<web_sys::Element, String> {
        let item = all.get(&key).ok_or_else(|| format!("{key} has no item"))?;
        let &(cx, cy, r) = circles.get(&key).ok_or_else(|| format!("{key} has no circle"))?;
        let key_text = key.to_string();
        let g = make(
            &self.document,
            "g",
            &[
                ("role", "treeitem"),
                ("data-rbt-key", &key_text),
                ("data-rbt-colour", item.colour.word()),
                ("data-rbt-active", if snapshot.active.contains(&key) { "true" } else { "false" }),
                ("tabindex", "-1"),
                ("aria-level", &item.level.to_string()),
                ("aria-posinset", &item.pos_in_set.to_string()),
                ("aria-setsize", &item.set_size.to_string()),
                ("aria-selected", "false"),
                ("aria-label", &item.name),
            ],
        )?;
        if item.expanded.is_some() {
            set(&g, "aria-expanded", "true")?;
        }
        let (cx_text, cy_text) = (cx.to_string(), cy.to_string());
        add(
            &g,
            &make(
                &self.document,
                "circle",
                &[
                    ("class", "rbt-hit"),
                    ("cx", &cx_text),
                    ("cy", &cy_text),
                    ("r", &(r + 3.0).to_string()),
                ],
            )?,
        )?;
        add(
            &g,
            &make(
                &self.document,
                "circle",
                &[
                    ("class", "rbt-focus-ring"),
                    ("cx", &cx_text),
                    ("cy", &cy_text),
                    ("r", &(r + 9.0).to_string()),
                ],
            )?,
        )?;
        if !item.children.is_empty() {
            let group = make(&self.document, "g", &[("role", "group")])?;
            for &child in &item.children {
                add(&group, &self.item_element(snapshot, all, circles, child, elements)?)?;
            }
            add(&g, &group)?;
        }
        elements.insert(key, g.clone());
        Ok(g)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces what is in the stage with `root`, in the one call the browser has for it.
    ///
    /// If the call throws, the stage may be untouched or already changed, so what was there is taken out again and put
    /// back, as far as that works. The call is made by name, so that a test can make the real one throw.
    fn replace_board(&self, root: &web_sys::Element) -> Result<(), ReplaceError> {
        let stage = &self.chrome.host;
        let mut old = Vec::new();
        let mut child = stage.first_child();
        while let Some(node) = child {
            child = node.next_sibling();
            old.push(node);
        }
        let called = js_sys::Reflect::get(stage, &JsValue::from_str("replaceChildren"))
            .and_then(|function| function.dyn_into::<js_sys::Function>())
            .and_then(|function| function.call1(stage, root));
        let Err(problem) = called else {
            return Ok(());
        };
        let mut restored = true;
        while let Some(node) = stage.first_child() {
            if stage.remove_child(&node).is_err() {
                restored = false;
                break;
            }
        }
        for node in &old {
            restored &= stage.append_child(node).is_ok();
        }
        Err(ReplaceError {
            problem: format!("the tree could not be swapped in: {problem:?}"),
            restored,
        })
    }

    /// Where the focus is, if it is in the tree.
    fn focus_now(&self) -> Option<Focus> {
        let active = self.document.active_element()?;
        if !self.chrome.host.contains(Some(&active)) {
            return None;
        }
        match active.closest("[data-rbt-key]").ok().flatten() {
            Some(item) => item.get_attribute("data-rbt-key")?.parse().ok().map(Focus::Item),
            None => Some(Focus::Tree),
        }
    }

    /// Puts the focus on the item that the tab stop is on, or on the tree if there is none, and checks that it took.
    fn restore_focus(&self, snapshot: &Snapshot, view: &View) -> Result<(), String> {
        let board = self.board.borrow();
        let board = board.as_ref().ok_or("there is no tree to focus")?;
        let target = resolve_roving(view.roving, snapshot)
            .and_then(|k| board.items.get(&k))
            .unwrap_or(&board.overlay);
        self.restoring.set(true);
        focus(target);
        self.restoring.set(false);
        match self.document.active_element() {
            Some(active) if &active == target => Ok(()),
            _ => Err("the focus could not be put back".to_owned()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the buttons, the range and the status fields for the snapshot at `index`, through `journal`.
    fn write_controls(&self, journal: &mut Journal, trace: &Trace, index: usize, view: &View) -> Result<(), String> {
        let snapshot = trace.snapshot(index).ok_or_else(|| format!("there is no snapshot {index}"))?;
        let last = trace.last();
        let dimmed = |action: &str| match action {
            "restart" | "previous" => index == 0,
            "next" | "play" => index >= last,
            _ => false,
        };
        // A dimmed button is `aria-disabled` and keeps its place in the tab order, so one that has the focus when the
        // run reaches an end keeps it, and a press of it is still heard.
        for (&(action, ..), button) in BUTTONS.iter().zip(&self.chrome.buttons) {
            let off = dimmed(action);
            journal.attr(button, "aria-disabled", if off { "true" } else { "false" })?;
            journal.attr(
                button,
                "style",
                if off { "cursor: default; opacity: 0.45;" } else { "cursor: pointer;" },
            )?;
        }
        let seek = &self.chrome.seek;
        journal.attr(seek, "max", &last.to_string())?;
        journal.input(seek, &index.to_string());
        let step = format!("Step {index} of {last}");
        journal.attr(seek, "aria-valuetext", &step)?;
        journal.text(&self.chrome.seek_text, &step);
        let state = if snapshot.settled {
            "Settled: a valid red-black tree"
        } else {
            "Repair in progress: not yet a valid red-black tree"
        };
        let size = snapshot.nodes.len();
        let fields = [
            step,
            state.to_owned(),
            snapshot.description.clone(),
            format!("{size} {}", if size == 1 { "key" } else { "keys" }),
            snapshot.root.map_or_else(|| "none".to_owned(), |k| k.to_string()),
        ];
        for (field, text) in self.chrome.fields.iter().zip(&fields) {
            journal.text(field, text);
        }
        let (selected, note) = selection_texts(snapshot, view);
        journal.text(&self.chrome.fields[5], &selected);
        journal.text(&self.chrome.fields[6], &note);
        Ok(())
    }

    fn hide_error(&self) {
        if self.error_shown.replace(false) {
            let _ = set(&self.chrome.error, "hidden", "");
            self.chrome.error.set_text_content(None);
        }
    }

    fn show_error(&self, message: &str) {
        self.chrome.error.set_text_content(Some(message));
        let _ = self.chrome.error.remove_attribute("hidden");
        self.error_shown.set(true);
    }
}

/// What the status field of the selection says, and its note about a replacement.
fn selection_texts(snapshot: &Snapshot, view: &View) -> (String, String) {
    let shown = resolve(view.chosen, snapshot);
    let selected = match shown.and_then(|k| snapshot.node(k)) {
        Some(n) if view.chosen.is_some() => format!("{}, {}", n.key, n.colour.word()),
        _ => "none".to_owned(),
    };
    (
        selected,
        selection_note(view.chosen, snapshot).unwrap_or_else(|| "\u{2014}".to_owned()),
    )
}

/// Writes who is selected and which item holds the tab stop on every item. The same words are written on every call, so
/// a call that failed part way is put right by the next.
fn apply_view(items: &BTreeMap<Key, web_sys::Element>, snapshot: &Snapshot, view: &View) -> Result<(), String> {
    let (selected, roving) = (resolve(view.chosen, snapshot), resolve_roving(view.roving, snapshot));
    for (&key, element) in items {
        let is_selected = view.chosen.is_some() && selected == Some(key);
        set(element, "aria-selected", if is_selected { "true" } else { "false" })?;
        set(element, "data-rbt-selected", if is_selected { "true" } else { "false" })?;
        set(element, "tabindex", if roving == Some(key) { "0" } else { "-1" })?;
    }
    Ok(())
}

fn commit_error(restored: bool, problem: String) -> CommitError {
    if restored {
        CommitError::Detached(problem)
    } else {
        CommitError::Live(problem)
    }
}

impl Surface for Page {
    fn commit(&self, trace: &Trace, index: usize, view: &View) -> Result<(), CommitError> {
        let snapshot = trace
            .snapshot(index)
            .ok_or_else(|| CommitError::Detached(format!("there is no snapshot {index}")))?;
        // Taken by key before anything is removed, and never as an element, which the swap makes stale.
        let focus_before = self.focus_now();
        let board = self.build_board(snapshot, view).map_err(CommitError::Detached)?;
        let mut journal = Journal::default();
        if let Err(problem) = self.write_controls(&mut journal, trace, index, view) {
            journal.restore();
            board.root.remove();
            return Err(CommitError::Detached(problem));
        }
        if let Err(failure) = self.replace_board(&board.root) {
            journal.restore();
            board.root.remove();
            return Err(commit_error(failure.restored, failure.problem));
        }
        let old = self.board.replace(Some(board));
        drop(old);
        if focus_before.is_some() {
            self.restore_focus(snapshot, view).map_err(CommitError::Live)?;
        }
        self.hide_error();
        Ok(())
    }

    fn project(&self, trace: &Trace, index: usize, view: &View) -> Result<(), String> {
        let snapshot = trace.snapshot(index).ok_or_else(|| format!("there is no snapshot {index}"))?;
        if let Some(board) = self.board.borrow().as_ref() {
            apply_view(&board.items, snapshot, view)?;
        }
        let (selected, note) = selection_texts(snapshot, view);
        self.chrome.fields[5].set_text_content(Some(&selected));
        self.chrome.fields[6].set_text_content(Some(&note));
        self.hide_error();
        Ok(())
    }

    fn focus_key(&self, key: Key) {
        if let Some(element) = self.board.borrow().as_ref().and_then(|b| b.items.get(&key)) {
            self.restoring.set(true);
            focus(element);
            self.restoring.set(false);
        }
    }

    fn playing_changed(&self, playing: bool) -> Result<(), String> {
        let label = if playing { "Pause" } else { "Play" };
        let at = BUTTONS.iter().position(|&(action, ..)| action == "play").unwrap_or(3);
        let Some(button) = self.chrome.buttons.get(at) else {
            return Err("there is no Play button".to_owned());
        };
        let mut journal = Journal::default();
        if let Err(problem) = journal.attr(button, "aria-label", label) {
            journal.restore();
            return Err(problem);
        }
        if let Some(text) = button.query_selector("text").ok().flatten() {
            journal.text(&text, label);
        }
        Ok(())
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
            "The tree could not be drawn: {problem}. What is shown may be out of date. The next button, key or move of the \
             range draws it again."
        );
        self.show_error(&message);
        self.say(&message);
    }

    fn playback_failed(&self, problem: &str) {
        let message = format!(
            "Play could not carry on: {problem}. The tree is right, and the run is paused where it is. Press Play to try \
             again."
        );
        self.show_error(&message);
        self.say(&message);
    }
}
