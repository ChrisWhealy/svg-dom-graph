//! The scene: the real nodes and connectors of the net, the overlay on top of them, and the writes that make the
//! overlay match a presentation. It is built once for a page and never changed in shape.
//!
//! The places are circle nodes, the gates are narrow box nodes and every arc is a directed straight connector. Above
//! them, in the same transformed group, is an overlay of the things that change or can be selected: token counts,
//! status words, rings, and the targets that the keyboard and the pointer use. The library's own nodes and connectors
//! are hidden from assistive technology, since the overlay says all that they do.

use super::{
    chrome::Chrome,
    dom::{add, make, remove, set},
    layout::{
        CAPTION_AT, CELL_GAP, CELL_ORIGIN, CELL_SIZE, End, FREE_CENTRE, GATE, HUB_LABEL_LIFT, HUB_RADIUS, RADIUS, TOP,
        USED_CENTRE, VIEW_HEIGHT, VIEW_WIDTH, arc_ends, gate_centre, gate_label, gate_rect, place_centre,
        ring_title_at,
    },
    net::{ARCS, CAPACITY, CONSUMERS, Net, PRODUCERS, PlaceId, PlaceKind, TransitionId},
    projection::{SLOTS, Target, Write, arc_slot, cell_slot, gate_slot, place_slot},
};
use demo_common::util::stringify;
use svg_dom::SvgRoot;
use svg_dom_graph::scene::{
    ClockPosition, ConnectorOptions, ConnectorType, EllipseOptions, NodeOptions, Scene, Side, ToolbarOptions,
};

/// The fills of the places and the gates. The words in the nodes and the overlay say the same things as the colours.
const ACTOR_FILL: &str = "#ffffff";
const HUB_FILL: &str = "#dbeafe";
const GATE_FILL: &str = "#334155";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One element of the list of slots: the element that the attributes are written to, and the one that the text is
/// written to, if the slot has text.
pub(super) struct SlotEl {
    target: web_sys::Element,
    text: Option<web_sys::Element>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene, built and in the page.
pub(super) struct Built {
    /// Kept so that the scene, and the listeners it put on the page, live as long as the page does.
    scene: Scene,
    slots: Vec<SlotEl>,
}

impl Built {
    pub(super) fn scene(&self) -> &Scene {
        &self.scene
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The element of a place or a gate, which is what the keyboard focus goes to.
    pub(super) fn element_of(&self, target: Target) -> Option<web_sys::Element> {
        let at = match target {
            Target::Place(p) => place_slot(p),
            Target::Transition(t) => gate_slot(t),
        };
        self.slots.get(at).map(|slot| slot.target.clone())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the writes that concern the scene, and not the text panel.
///
/// # Errors
///
/// Returns `Err` if an attribute could not be written or removed, part way through, which leaves the scene partly
/// written.
pub(super) fn apply_scene_writes(built: &Built, writes: &[Write]) -> Result<(), String> {
    for write in writes {
        match write {
            Write::Attr(at, name, value) => set(&built.slots[*at].target, name, value)?,
            Write::RemoveAttr(at, name) => remove(&built.slots[*at].target, name)?,
            Write::Text(at, text) => {
                if let Some(element) = &built.slots[*at].text {
                    element.set_text_content(Some(text));
                }
            },
            Write::Panel(..) => {},
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes all of `writes`, the text panel's too.
///
/// # Errors
///
/// Returns `Err` if an attribute could not be written, part way through.
pub(super) fn apply_writes(built: &Built, chrome: &Chrome, writes: &[Write]) -> Result<(), String> {
    apply_scene_writes(built, writes)?;
    for write in writes {
        if let Write::Panel(at, text) = write {
            if let Some(field) = chrome.panel.get(*at) {
                field.set_text_content(Some(text));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn n(x: f64) -> String {
    format!("{x:.1}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a place says inside its circle.
fn place_word(p: PlaceId) -> &'static str {
    match p.kind() {
        PlaceKind::ProducerIdle | PlaceKind::ConsumerIdle => "Idle",
        PlaceKind::ProducerBusy => "Busy",
        PlaceKind::ProducerReady => "Ready",
        PlaceKind::ConsumerProcessing => "Work",
        PlaceKind::Free => "Free",
        PlaceKind::Used => "Used",
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The options of a straight connector whose ends are fixed as `from` and `to` say.
fn connector(from: End, to: End) -> Result<ConnectorOptions, String> {
    let mut options = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    match from {
        End::Auto => {},
        End::Clock(m) => options = options.with_from_clock(Some(ClockPosition::try_from(m).map_err(stringify)?)),
        End::Pinned(side, at) => options = options.with_from_side(Some(side)).with_from_position(Some(at)),
    }
    match to {
        End::Auto => {},
        End::Clock(m) => options = options.with_to_clock(Some(ClockPosition::try_from(m).map_err(stringify)?)),
        End::Pinned(side, at) => options = options.with_to_side(Some(side)).with_to_position(Some(at)),
    }
    Ok(options)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn text_at(
    document: &web_sys::Document,
    class: &str,
    x: f64,
    y: f64,
    anchor: &str,
    text: &str,
) -> Result<web_sys::Element, String> {
    let element = make(
        document,
        "text",
        &[("class", class), ("x", &n(x)), ("y", &n(y)), ("text-anchor", anchor)],
    )?;
    element.set_text_content(Some(text));
    Ok(element)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the scene of the net in a nested `<svg>` of `host`, and its overlay. Every element is as empty as a
/// presentation leaves it, and [`apply_writes`] draws a snapshot.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added, or the scene did not draw what the net has.
pub(super) fn build_scene(
    document: &web_sys::Document,
    host: &web_sys::Element,
    net: &Net,
    instance: u32,
) -> Result<Built, String> {
    let scene_id = format!("pn-{instance}-scene");
    let (width, height) = (n(VIEW_WIDTH), n(VIEW_HEIGHT));
    let svg = make(
        document,
        "svg",
        &[
            ("id", &scene_id),
            ("class", "pn-scene"),
            ("x", "0"),
            ("y", &n(TOP)),
            ("width", &width),
            ("height", &height),
            ("viewBox", &format!("0 0 {width} {height}")),
        ],
    )?;
    add(host, &svg)?;
    let scene = Scene::new(SvgRoot::attach(&scene_id).map_err(stringify)?).map_err(stringify)?;
    // The west edge is empty, and the east edge has the consumers' labels.
    scene.show_toolbar(ToolbarOptions::new(Side::West)).map_err(stringify)?;

    let mut places = Vec::with_capacity(PlaceId::all().count());
    for p in PlaceId::all() {
        let hub = matches!(p.kind(), PlaceKind::Free | PlaceKind::Used);
        let options = EllipseOptions::default()
            .with_fill(Some(if hub { HUB_FILL } else { ACTOR_FILL }))
            .with_font_size(Some(if hub { 13.0 } else { 10.0 }));
        let radius = if hub { HUB_RADIUS } else { RADIUS };
        places.push(
            scene
                .add_circle_node_with(place_centre(p), radius, place_word(p), options)
                .map_err(stringify)?,
        );
    }
    let mut gates = Vec::with_capacity(TransitionId::all().count());
    for t in TransitionId::all() {
        let rect = gate_rect(t);
        gates.push(
            scene
                .add_node_with(rect.origin, rect.size, " ", NodeOptions::default().with_fill(Some(GATE_FILL)))
                .map_err(stringify)?,
        );
    }
    for arc in net.arcs() {
        let (p, t) = (places[arc.place.index()], gates[arc.transition.index()]);
        let ends = arc_ends(&arc);
        let (from, to) = if arc.input { (p, t) } else { (t, p) };
        scene
            .add_edge_with(from, to, connector(ends.from, ends.to)?)
            .map_err(stringify)?;
    }

    // The library's nodes and connectors are the picture. The overlay is what is read, so they are left out of the
    // tree, and each connector is given the index of its arc, which is what a test and the stylesheet find it by.
    let content = svg
        .query_selector(".svg-dom-graph-content")
        .map_err(|e| format!("could not look for the content group: {e:?}"))?
        .ok_or_else(|| "the scene has no content group".to_owned())?;
    let mut paths = Vec::with_capacity(ARCS);
    let mut child = content.first_element_child();
    let mut node_at = 0;
    while let Some(element) = child {
        child = element.next_element_sibling();
        set(&element, "aria-hidden", "true")?;
        // The places were added first, in order. A hub's label is moved up, so that the count is not drawn over it.
        if !element.tag_name().eq_ignore_ascii_case("path") {
            if node_at == PlaceId::FREE.index() || node_at == PlaceId::USED.index() {
                if let Some(label) = element.query_selector("text").ok().flatten() {
                    set(&label, "y", &n(HUB_RADIUS - HUB_LABEL_LIFT))?;
                }
            }
            node_at += 1;
        }
        if element.tag_name().eq_ignore_ascii_case("path") {
            paths.push(element);
        }
    }
    if paths.len() != ARCS {
        return Err(format!("the scene drew {} connectors, not {ARCS}", paths.len()));
    }
    for (index, path) in paths.iter().enumerate() {
        set(path, "data-pn-arc", &index.to_string())?;
        set(path, "id", &format!("pn-{instance}-a-{index}"))?;
    }

    let mut slots: Vec<Option<SlotEl>> = (0..SLOTS).map(|_| None).collect();
    for (index, path) in paths.into_iter().enumerate() {
        slots[arc_slot(index)] = Some(SlotEl { target: path, text: None });
    }
    let overlay = make(
        document,
        "g",
        &[
            ("class", "pn-overlay"),
            ("role", "group"),
            ("aria-label", "Petri net of a bounded buffer: 20 places and 18 transitions"),
        ],
    )?;
    add(&content, &overlay)?;
    decorate(document, &overlay)?;
    for p in PlaceId::all() {
        slots[place_slot(p)] = Some(place_target(document, &overlay, p, instance)?);
    }
    for t in TransitionId::all() {
        slots[gate_slot(t)] = Some(gate_target(document, &overlay, t, instance)?);
    }
    let cells = make(document, "g", &[("class", "pn-cells"), ("aria-hidden", "true")])?;
    add(&overlay, &cells)?;
    for cell in 0..usize::from(CAPACITY) {
        let x = CELL_ORIGIN.0 + cell as f64 * (CELL_SIZE.0 + CELL_GAP);
        let rect = make(
            document,
            "rect",
            &[
                ("class", "pn-cell"),
                ("data-pn-cell", &cell.to_string()),
                ("x", &n(x)),
                ("y", &n(CELL_ORIGIN.1)),
                ("width", &n(CELL_SIZE.0)),
                ("height", &n(CELL_SIZE.1)),
                ("rx", "3"),
            ],
        )?;
        add(&cells, &rect)?;
        slots[cell_slot(cell)] = Some(SlotEl { target: rect, text: None });
    }
    let caption = text_at(document, "pn-caption", CAPTION_AT.0, CAPTION_AT.1, "middle", "")?;
    add(&overlay, &caption)?;
    slots[super::projection::CAPTION_SLOT] = Some(SlotEl {
        target: caption.clone(),
        text: Some(caption),
    });

    let slots = slots
        .into_iter()
        .map(|slot| slot.ok_or_else(|| "a slot of the scene was not made".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Built { scene, slots })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The things that never change: the names of the three zones and of each worker's ring.
fn decorate(document: &web_sys::Document, overlay: &web_sys::Element) -> Result<(), String> {
    let layer = make(document, "g", &[("class", "pn-decor"), ("aria-hidden", "true")])?;
    add(overlay, &layer)?;
    for (x, word) in [(190.0, "PRODUCERS"), (FREE_CENTRE.0, "BUFFER"), (900.0, "CONSUMERS")] {
        add(&layer, &text_at(document, "pn-zone", x, 22.0, "middle", word)?)?;
    }
    for i in 0..PRODUCERS {
        let c = ring_title_at(i, false);
        add(
            &layer,
            &text_at(document, "pn-ring-title", c.x, c.y, "middle", &format!("Producer {i}"))?,
        )?;
    }
    for j in 0..CONSUMERS {
        let c = ring_title_at(j, true);
        add(
            &layer,
            &text_at(document, "pn-ring-title", c.x, c.y, "middle", &format!("Consumer {j}"))?,
        )?;
    }
    add(
        &layer,
        &text_at(
            document,
            "pn-hub-note",
            USED_CENTRE.0,
            USED_CENTRE.1 + HUB_RADIUS + 34.0,
            "middle",
            "Free + Used = 8",
        )?,
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The target of a place: its rings, its hit area, its token and, for a hub, its count.
fn place_target(
    document: &web_sys::Document,
    overlay: &web_sys::Element,
    p: PlaceId,
    instance: u32,
) -> Result<SlotEl, String> {
    let c = place_centre(p);
    let hub = matches!(p.kind(), PlaceKind::Free | PlaceKind::Used);
    let r = if hub { HUB_RADIUS } else { RADIUS };
    let group = make(
        document,
        "g",
        &[
            ("id", &format!("pn-{instance}-p-{}", p.index())),
            ("class", "pn-place"),
            ("data-pn-place", &p.name()),
            ("role", "button"),
            ("tabindex", "-1"),
        ],
    )?;
    add(
        &group,
        &make(
            document,
            "circle",
            &[("class", "pn-ring"), ("cx", &n(c.x)), ("cy", &n(c.y)), ("r", &n(r + 6.0))],
        )?,
    )?;
    add(
        &group,
        &make(
            document,
            "circle",
            &[("class", "pn-hit"), ("cx", &n(c.x)), ("cy", &n(c.y)), ("r", &n(r))],
        )?,
    )?;
    let text = if hub {
        let count = text_at(document, "pn-count", c.x, c.y + 20.0, "middle", "")?;
        add(&group, &count)?;
        Some(count)
    } else {
        add(
            &group,
            &make(
                document,
                "circle",
                &[("class", "pn-dot"), ("cx", &n(c.x)), ("cy", &n(c.y + 12.0)), ("r", "5")],
            )?,
        )?;
        None
    };
    add(overlay, &group)?;
    Ok(SlotEl { target: group, text })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The target of a gate: its ring, its hit area, and its name and status beside it.
fn gate_target(
    document: &web_sys::Document,
    overlay: &web_sys::Element,
    t: TransitionId,
    instance: u32,
) -> Result<SlotEl, String> {
    let c = gate_centre(t);
    let (w, h) = (GATE.width, GATE.height);
    let group = make(
        document,
        "g",
        &[
            ("id", &format!("pn-{instance}-t-{}", t.index())),
            ("class", "pn-gate"),
            ("data-pn-transition", &t.name()),
            ("role", "button"),
            ("tabindex", "-1"),
        ],
    )?;
    add(
        &group,
        &make(
            document,
            "rect",
            &[
                ("class", "pn-bar"),
                ("x", &n(c.x - w / 2.0)),
                ("y", &n(c.y - h / 2.0)),
                ("width", &n(w)),
                ("height", &n(h)),
                ("rx", "2"),
            ],
        )?,
    )?;
    add(
        &group,
        &make(
            document,
            "rect",
            &[
                ("class", "pn-ring"),
                ("x", &n(c.x - w / 2.0 - 6.0)),
                ("y", &n(c.y - h / 2.0 - 6.0)),
                ("width", &n(w + 12.0)),
                ("height", &n(h + 12.0)),
                ("rx", "4"),
            ],
        )?,
    )?;
    add(
        &group,
        &make(
            document,
            "rect",
            &[
                ("class", "pn-hit"),
                ("x", &n(c.x - 12.0)),
                ("y", &n(c.y - h / 2.0 - 4.0)),
                ("width", "24"),
                ("height", &n(h + 8.0)),
            ],
        )?,
    )?;
    let at = gate_label(t);
    let name = format!("{} {}", t.name().split('[').next().unwrap_or(""), t.worker());
    let status = make(document, "tspan", &[("class", "pn-state")])?;
    if at.inline {
        let line = text_at(document, "pn-name", at.x, at.y, at.anchor, &format!("{name} "))?;
        add(&line, &status)?;
        add(&group, &line)?;
    } else {
        add(&group, &text_at(document, "pn-name", at.x, at.y, at.anchor, &name)?)?;
        let line = text_at(document, "pn-name pn-line2", at.x, at.y + 12.0, at.anchor, "")?;
        add(&line, &status)?;
        add(&group, &line)?;
    }
    add(overlay, &group)?;
    Ok(SlotEl {
        target: group,
        text: Some(status),
    })
}
