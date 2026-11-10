//! The surface: one `svg-dom-graph` `Scene` and the overlay on it, built once and written in place.
//!
//! The scene has the queue and the servers as nodes with connectors between them, and a data node of eight counters.
//! The overlay is a group of application-owned elements appended to the scene's content group, so that it pans and
//! zooms with the nodes. Nothing is added or removed after construction: a draw writes the text, class, accessible name
//! and hidden state of slots that exist.
//!
//! A draw that fails part way leaves the page partly written. The surface then forgets what it believes is drawn, so
//! the next draw is a full one and cannot be a delta from something untrue. Whether a draw may be committed at all is
//! the controller's decision (stage S3), and the surface does not touch the view or the focus.

use crate::layout::{
    CLOCK_AT, COUNTER_PITCH, COUNTERS_AT, DATA_AT, QUEUE_NODE, QUEUE_SLOT, SELECTED_AT, SERVER_SIZE, SERVER_X, VIEW,
    more_at, queue_slot_at, server_at,
};
use crate::projection::{COUNTER_NAMES, DATA_CELLS, Field, Projection, canonical_line, writes};
use svg_dom::SvgRoot;
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::NodeId;
use svg_dom_graph::scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene};
use wasm_bindgen::JsCast;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const QUEUE_ORIGIN_X: f64 = 24.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements of one slot: the group that carries the class and the accessible name, and the text inside it.
struct SlotEl {
    group: web_sys::Element,
    text: web_sys::Element,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a draw did not succeed. After any of them the surface no longer trusts what the page holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawError {
    /// The host `<svg>` is not in the document. A detached element still accepts writes, so this is checked and not
    /// discovered.
    Detached,
    /// The projection does not have the shape of the surface.
    Shape,
    /// A DOM write or the data setter failed. The page may be partly written.
    Write(String),
}

impl std::fmt::Display for DrawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Detached => f.write_str("the host is not in the document"),
            Self::Shape => f.write_str("the projection does not have the shape of the surface"),
            Self::Write(what) => f.write_str(what),
        }
    }
}

impl From<DrawError> for String {
    fn from(e: DrawError) -> Self {
        e.to_string()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A point of a draw at which a test can act, such as detaching the host. It exists only with the `fault-hook` feature,
/// which no clean build has.
#[cfg(feature = "fault-hook")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultPoint {
    /// After the nth field of the overlay was written.
    Overlay(usize),
    /// After the data node's setter returned.
    Setter,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What one draw wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drawn {
    /// The fields of slots that were written.
    pub writes: usize,
    /// Whether the data node's setter was called. It is called on every draw.
    pub data_set: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub struct Surface {
    scene: Scene,
    data_node: NodeId,
    host: web_sys::Element,
    overlay: web_sys::Element,
    data_group: web_sys::Element,
    keys: Vec<String>,
    slots: Vec<SlotEl>,
    /// What the page holds, if that is known. `None` after a failed draw and before the first.
    drawn: Option<Projection>,
    /// How many times the data node's setter returned without an error, over the life of the surface.
    setter_calls: u64,
    #[cfg(feature = "fault-hook")]
    hook: Option<Box<dyn FnMut(FaultPoint)>>,
}

fn fail<E: std::fmt::Debug>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e:?}")
}

fn write_error<E: std::fmt::Debug>(what: &str) -> impl FnOnce(E) -> DrawError + '_ {
    move |e| DrawError::Write(format!("{what}: {e:?}"))
}

fn make(document: &web_sys::Document, tag: &str, attributes: &[(&str, &str)]) -> Result<web_sys::Element, String> {
    let element = document
        .create_element_ns(Some(SVG_NS), tag)
        .map_err(fail("could not make an element"))?;
    for (name, value) in attributes {
        element.set_attribute(name, value).map_err(fail(name))?;
    }
    Ok(element)
}

fn add(parent: &web_sys::Element, child: &web_sys::Element) -> Result<(), String> {
    parent.append_child(child).map(|_| ()).map_err(fail("could not add an element"))
}

fn n(x: f64) -> String {
    format!("{x:.1}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Surface {
    /// The size of the view box that the host's `<svg>` needs.
    #[must_use]
    pub fn view_box() -> (f64, f64) {
        VIEW
    }

    /// Builds the scene and the overlay in the `<svg>` called `svg_id`, which has to exist, be empty and have a view
    /// box of [`view_box`](Self::view_box). Nothing is drawn: every slot is as empty as a projection leaves it, and the
    /// first [`draw`](Self::draw) fills them.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the `<svg>` is missing, an element could not be made, or the scene did not draw what was asked.
    ///
    /// A failure at any DOM call returns the error and makes no surface, so nothing is committed: no `Surface` exists
    /// to draw on. The caller removes the host `<svg>`, which is the only place that a half-built scene could be left,
    /// and builds again on a fresh one.
    pub fn build(svg_id: &str, servers: usize) -> Result<Self, String> {
        let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
        let host = document
            .get_element_by_id(svg_id)
            .ok_or_else(|| format!("no element #{svg_id}"))?;
        let scene = Scene::new(SvgRoot::attach(svg_id).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

        let queue = scene
            .add_node(
                Point::new(QUEUE_NODE.0.0, QUEUE_NODE.0.1),
                Size::new(QUEUE_NODE.1.0, QUEUE_NODE.1.1),
                "Waiting",
            )
            .map_err(|e| e.to_string())?;
        let mut server_nodes = Vec::with_capacity(servers);
        for i in 0..servers {
            let (x, y) = server_at(i);
            server_nodes.push(
                scene
                    .add_node(Point::new(x, y), Size::new(SERVER_SIZE.0, SERVER_SIZE.1), format!("S{i}"))
                    .map_err(|e| e.to_string())?,
            );
        }
        for node in &server_nodes {
            scene.add_edge(queue, *node).map_err(|e| e.to_string())?;
        }
        let values = NodeValues::U64(vec![0; DATA_CELLS]);
        let data_node = scene
            .add_data_node(
                Point::new(DATA_AT.0, DATA_AT.1),
                DataNodeContent::new(values, DataFormat::Decimal)
                    .with_layout(GridLayout::Columns(4))
                    .with_cell_width_hint("0000000"),
            )
            .map_err(|e| e.to_string())?;

        let content_selector = ":scope > g.svg-dom-graph-content";
        let found = host.query_selector_all(content_selector).map_err(fail("content group"))?;
        if found.length() != 1 {
            return Err(format!("the scene has {} content groups, and one was meant", found.length()));
        }
        let content: web_sys::Element = found
            .get(0)
            .and_then(|e| e.dyn_into().ok())
            .ok_or("the content group is not an element")?;
        let data_group = last_group_of(&content)?;

        // The overlay is added after the nodes, since the content group paints in append order.
        let overlay = make(
            &document,
            "g",
            &[("class", "ss-overlay"), ("role", "group"), ("aria-label", "Service simulator")],
        )?;
        add(&content, &overlay)?;
        // The words that never change. Assistive technology reads the slots, which carry the same words in their names.
        let mut captions = vec![
            ("Waiting queue".to_owned(), QUEUE_ORIGIN_X, 50.0),
            ("Servers".to_owned(), SERVER_X, 50.0),
            ("Counters".to_owned(), COUNTERS_AT.0, COUNTERS_AT.1 - 32.0),
        ];
        for (i, (_, name)) in COUNTER_NAMES.iter().enumerate() {
            captions.push(((*name).to_owned(), COUNTERS_AT.0, COUNTERS_AT.1 + i as f64 * COUNTER_PITCH));
        }
        for (word, x, y) in captions {
            let text = make(
                &document,
                "text",
                &[("class", "ss-caption"), ("aria-hidden", "true"), ("x", &n(x)), ("y", &n(y))],
            )?;
            text.set_text_content(Some(&word));
            add(&overlay, &text)?;
        }

        let keys = Projection::keys(servers);
        let queue_list = make(
            &document,
            "g",
            &[("class", "ss-queue-list"), ("role", "list"), ("aria-label", "Waiting queue")],
        )?;
        // The list is added where the first queue slot falls, so that the document order is the order of the keys.
        let mut slots = Vec::with_capacity(keys.len());
        for key in &keys {
            let at = slot_position(key);
            let in_list = key.starts_with('Q');
            if key == "Q0" {
                add(&overlay, &queue_list)?;
            }
            let role = if in_list { "listitem" } else { "group" };
            let group = make(
                &document,
                "g",
                &[
                    ("data-ss-key", key),
                    ("role", role),
                    ("tabindex", "-1"),
                    ("class", ""),
                    ("aria-hidden", "true"),
                ],
            )?;
            if in_list {
                let rect = make(
                    &document,
                    "rect",
                    &[
                        ("class", "ss-slot-frame"),
                        ("x", &n(at.0)),
                        ("y", &n(at.1 - 17.0)),
                        ("width", &n(QUEUE_SLOT.0)),
                        ("height", &n(QUEUE_SLOT.1)),
                    ],
                )?;
                add(&group, &rect)?;
            }
            let text = make(&document, "text", &[("x", &n(at.0 + 6.0)), ("y", &n(at.1))])?;
            add(&group, &text)?;
            add(if in_list { &queue_list } else { &overlay }, &group)?;
            slots.push(SlotEl { group, text });
        }
        Ok(Self {
            scene,
            data_node,
            host,
            overlay,
            data_group,
            keys,
            slots,
            drawn: None,
            setter_calls: 0,
            #[cfg(feature = "fault-hook")]
            hook: None,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The scene, for the view and for tests. The surface never changes the view.
    #[must_use]
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// The overlay group.
    #[must_use]
    pub fn overlay(&self) -> &web_sys::Element {
        &self.overlay
    }

    /// The group of one slot, which is what the keyboard focus goes to.
    #[must_use]
    pub fn slot_element(&self, key: &str) -> Option<web_sys::Element> {
        self.keys.iter().position(|k| k == key).map(|i| self.slots[i].group.clone())
    }

    /// How many elements the host holds. A draw does not change it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page could not be searched.
    pub fn element_count(&self) -> Result<u32, String> {
        Ok(self.host.query_selector_all("*").map_err(fail("count"))?.length())
    }

    /// Whether the surface believes it knows what the page holds.
    #[must_use]
    pub fn is_trusted(&self) -> bool {
        self.drawn.is_some()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes what differs from the last draw, or everything if that is not known, and then the data node's values.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the projection has another shape than the surface, or a write failed. The page may then be
    /// partly written, and the next draw is a full one.
    pub fn draw(&mut self, projection: &Projection) -> Result<Drawn, DrawError> {
        let old = self.drawn.take();
        self.write(projection, old.as_ref())
    }

    /// Writes every field of every slot and the data node's values, whatever was drawn before.
    ///
    /// # Errors
    ///
    /// As [`draw`](Self::draw).
    pub fn redraw(&mut self, projection: &Projection) -> Result<Drawn, DrawError> {
        self.drawn = None;
        self.write(projection, None)
    }

    /// Fails with [`DrawError::Detached`] if the host is not in the document. A detached element accepts writes without
    /// complaint, so the absence of an error from a write is no evidence that the page was written.
    fn require_connected(&self) -> Result<(), DrawError> {
        if self.host.is_connected() { Ok(()) } else { Err(DrawError::Detached) }
    }

    #[cfg(feature = "fault-hook")]
    fn fire(&mut self, point: FaultPoint) {
        if let Some(hook) = self.hook.as_mut() {
            hook(point);
        }
    }

    /// Sets the function that is called at each [`FaultPoint`] of a draw, for a test that detaches the host in the
    /// middle of one.
    #[cfg(feature = "fault-hook")]
    pub fn set_fault_hook(&mut self, hook: Option<Box<dyn FnMut(FaultPoint)>>) {
        self.hook = hook;
    }

    // The host is checked before every write, before the setter and after it (a draw with no writes is still checked
    // before the setter), so that no write or scene operation is begun on a detached root, and a host that leaves
    // during the draw is seen before the draw can count as done. `drawn` is set only at the end.
    fn write(&mut self, projection: &Projection, old: Option<&Projection>) -> Result<Drawn, DrawError> {
        if projection.slots.len() != self.keys.len()
            || projection.slots.iter().zip(&self.keys).any(|(s, k)| &s.key != k)
        {
            return Err(DrawError::Shape);
        }
        let list = writes(old, projection);
        for (index, w) in list.iter().enumerate() {
            #[cfg(not(feature = "fault-hook"))]
            let _ = index;
            self.require_connected()?;
            let (slot, el) = (&projection.slots[w.slot], &self.slots[w.slot]);
            match w.field {
                Field::Text => el.text.set_text_content(Some(&slot.text)),
                Field::Class => el.group.set_attribute("class", &slot.class).map_err(write_error("class"))?,
                Field::Label if slot.label.is_empty() => {
                    el.group.remove_attribute("aria-label").map_err(write_error("label"))?
                },
                Field::Label => el
                    .group
                    .set_attribute("aria-label", &slot.label)
                    .map_err(write_error("label"))?,
                Field::Hidden if slot.hidden => {
                    el.group.set_attribute("aria-hidden", "true").map_err(write_error("hidden"))?
                },
                Field::Hidden => el.group.remove_attribute("aria-hidden").map_err(write_error("hidden"))?,
            }
            #[cfg(feature = "fault-hook")]
            self.fire(FaultPoint::Overlay(index));
        }
        self.require_connected()?;
        self.scene
            .set_data_values(self.data_node, NodeValues::U64(projection.data.to_vec()))
            .map_err(|e| DrawError::Write(e.to_string()))?;
        self.setter_calls += 1;
        #[cfg(feature = "fault-hook")]
        self.fire(FaultPoint::Setter);
        self.require_connected()?;
        self.drawn = Some(projection.clone());
        Ok(Drawn {
            writes: list.len(),
            data_set: true,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many times the data node's setter has returned without an error, which is once for each draw that reached
    /// it.
    #[must_use]
    pub fn setter_calls(&self) -> u64 {
        self.setter_calls
    }

    /// What the page holds, read back from the elements in the form of [`Projection::canonical`]. No model reads this:
    /// it is what a projection is compared with.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an element could not be read.
    pub fn snapshot(&self) -> Result<String, String> {
        let mut text = String::new();
        for (key, el) in self.keys.iter().zip(&self.slots) {
            text.push_str(&canonical_line(
                key,
                &el.text.text_content().unwrap_or_default(),
                &el.group.get_attribute("class").unwrap_or_default(),
                &el.group.get_attribute("aria-label").unwrap_or_default(),
                el.group.get_attribute("aria-hidden").as_deref() == Some("true"),
            ));
        }
        text.push_str("data");
        for cell in self.data_texts()? {
            text.push('\t');
            text.push_str(&cell);
        }
        text.push('\n');
        Ok(text)
    }

    /// The text of each cell of the data node.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the cells could not be listed.
    pub fn data_texts(&self) -> Result<Vec<String>, String> {
        let found = self.data_group.query_selector_all("text").map_err(fail("cells"))?;
        Ok((0..found.length())
            .filter_map(|i| found.get(i))
            .map(|t| t.text_content().unwrap_or_default().trim().to_owned())
            .collect())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The last `<g>` child of `content`, which is the node that was added last.
fn last_group_of(content: &web_sys::Element) -> Result<web_sys::Element, String> {
    let groups = content.query_selector_all(":scope > g").map_err(fail("groups"))?;
    groups
        .get(groups.length().checked_sub(1).ok_or("the scene has no node")?)
        .and_then(|e| e.dyn_into().ok())
        .ok_or_else(|| "the last node is not an element".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the baseline of a slot's text starts.
fn slot_position(key: &str) -> (f64, f64) {
    let counter = COUNTER_NAMES.iter().position(|(k, _)| *k == key);
    match key {
        "clock" => CLOCK_AT,
        "queue" => (150.0, 50.0),
        "more" => {
            let (x, y) = more_at();
            (x, y + 17.0)
        },
        "selected" => SELECTED_AT,
        _ if key.starts_with('S') => {
            let i: usize = key[1..].parse().unwrap_or(0);
            let (x, y) = server_at(i);
            (x + 8.0, y + SERVER_SIZE.1 - 10.0)
        },
        _ if key.starts_with('Q') => {
            let i: usize = key[1..].parse().unwrap_or(0);
            let (x, y) = queue_slot_at(i);
            (x, y + 17.0)
        },
        _ => {
            let i = counter.unwrap_or(0);
            (COUNTERS_AT.0 + 120.0, COUNTERS_AT.1 + i as f64 * COUNTER_PITCH)
        },
    }
}
