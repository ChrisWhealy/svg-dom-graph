//! The SVG scene of one graph: its layers, its elements in the order of the slots, and the writes that make it match a
//! presentation.
//!
//! Every ID carries the instance, the generation and the key of the graph, so that a scene that is built apart from the
//! page never has an ID that the live one has. Every vertex and edge also carries a `data-ek-*` attribute that does not
//! change, which is what a test finds it by.

use super::{
    chrome::Chrome,
    dom::{add, make, set},
    fixtures::{Fixture, NAMES, VIEW_HEIGHT, VIEW_WIDTH},
    geometry::Geometry,
    projection::{NODE_RADIUS, Slots, Target, Write},
    solver::{Direction, Trace},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One element of the list of slots: the element that the class and the attributes are written to, and the one that the
/// text is written to, if the slot has text.
pub(super) struct SlotEl {
    target: web_sys::Element,
    text: Option<web_sys::Element>,
}

/// The scene of one graph, built and not yet in the page, or in it.
pub(super) struct Scene {
    pub(super) root: web_sys::Element,
    slots: Vec<SlotEl>,
    layout: Slots,
}

impl Scene {
    /// The element of a vertex or an edge, which is what the focus goes to.
    pub(super) fn element_of(&self, target: Target) -> Option<web_sys::Element> {
        let at = match target {
            Target::Vertex(v) => self.layout.vertex(v),
            Target::Edge(e) => self.layout.edge(e),
        };
        self.slots.get(at).map(|slot| slot.target.clone())
    }

    /// Whether `element` is somewhere in this scene.
    pub(super) fn contains(&self, element: &web_sys::Element) -> bool {
        self.root.contains(Some(element))
    }
}

/// Writes the writes that concern the scene, and not the text panel, to `scene`.
///
/// # Errors
///
/// Returns `Err` if an attribute could not be written, part way through, which leaves the scene partly written.
pub(super) fn apply_scene_writes(scene: &Scene, writes: &[Write]) -> Result<(), String> {
    for write in writes {
        match write {
            Write::Class(at, class) => set(&scene.slots[*at].target, "class", class)?,
            Write::Attr(at, name, value) => set(&scene.slots[*at].target, name, value)?,
            Write::RemoveAttr(at, name) => {
                let _ = scene.slots[*at].target.remove_attribute(name);
            },
            Write::Text(at, text) => {
                if let Some(element) = &scene.slots[*at].text {
                    element.set_text_content(Some(text));
                }
            },
            Write::Panel(..) => {},
        }
    }
    Ok(())
}

/// Writes all of `writes`, the text panel's too.
///
/// # Errors
///
/// Returns `Err` if an attribute could not be written, part way through.
pub(super) fn apply_writes(scene: &Scene, chrome: &Chrome, writes: &[Write]) -> Result<(), String> {
    apply_scene_writes(scene, writes)?;
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
fn number(x: f64) -> String {
    format!("{x:.1}")
}

fn name_of(v: usize) -> &'static str {
    NAMES.get(v).copied().unwrap_or("?")
}

/// Makes the markers that the arrowheads are, one for each kind of line, which differ in shape and not only in colour:
/// a solid triangle for an original edge, an open chevron for a forward residual transition, and an open diamond for a
/// reverse one.
fn markers(document: &web_sys::Document, prefix: &str) -> Result<web_sys::Element, String> {
    let defs = make(document, "defs", &[])?;
    for (kind, shape) in [
        ("edge", "M0 0L8 4L0 8Z"),
        ("res-f", "M0 0L8 4L0 8"),
        ("res-r", "M4 0L8 4L4 8L0 4Z"),
    ] {
        let marker = make(
            document,
            "marker",
            &[
                ("id", &format!("{prefix}-mk-{kind}")),
                ("class", &format!("ek-mk ek-mk-{kind}")),
                ("viewBox", "0 0 8 8"),
                ("markerWidth", "9"),
                ("markerHeight", "9"),
                ("refX", "8"),
                ("refY", "4"),
                ("orient", "auto"),
                ("markerUnits", "userSpaceOnUse"),
            ],
        )?;
        add(&marker, &make(document, "path", &[("d", shape)])?)?;
        add(&defs, &marker)?;
    }
    Ok(defs)
}

/// Builds the scene of `fixture` for `trace`, not yet in the page. Every element is as empty as a presentation leaves
/// it, and [`apply_writes`] draws a snapshot.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
pub(super) fn build_scene(
    document: &web_sys::Document,
    fixture: &'static Fixture,
    trace: &Trace,
    geometry: &Geometry,
    instance: u32,
    generation: u64,
) -> Result<Scene, String> {
    let prefix = format!("ek-{instance}-g{generation}-{}", fixture.key);
    let (nv, ne) = (trace.vertex_count(), trace.edges().len());
    let layout = Slots::new(nv, ne);
    let root = make(
        document,
        "g",
        &[
            ("id", &format!("{prefix}-root")),
            ("class", "ek-root"),
            ("data-ek-fixture", fixture.key),
        ],
    )?;
    add(&root, &markers(document, &prefix)?)?;
    let lanes = make(document, "g", &[("class", "ek-lanes")])?;
    let edges = make(
        document,
        "g",
        &[("class", "ek-edges"), ("role", "list"), ("aria-label", "Original edges")],
    )?;
    let labels = make(document, "g", &[("class", "ek-labels"), ("aria-hidden", "true")])?;
    let nodes = make(
        document,
        "g",
        &[("class", "ek-nodes"), ("role", "list"), ("aria-label", "Vertices")],
    )?;
    let badges = make(document, "g", &[("class", "ek-badges"), ("aria-hidden", "true")])?;
    let mut slots: Vec<Option<SlotEl>> = (0..layout.count()).map(|_| None).collect();
    let text_at = |document: &web_sys::Document, class: &str, at: (f64, f64)| -> Result<web_sys::Element, String> {
        make(
            document,
            "text",
            &[
                ("class", class),
                ("x", &number(at.0)),
                ("y", &number(at.1)),
                ("text-anchor", "middle"),
                ("dominant-baseline", "central"),
            ],
        )
    };
    for v in 0..nv {
        let (x, y) = geometry.vertices[v];
        let group = make(
            document,
            "g",
            &[
                ("id", &format!("{prefix}-vertex-{}", name_of(v))),
                ("class", "ek-node"),
                ("role", "listitem"),
                ("tabindex", "-1"),
                ("data-ek-vertex", name_of(v)),
            ],
        )?;
        add(
            &group,
            &make(
                document,
                "circle",
                &[
                    ("class", "ek-node-circle"),
                    ("cx", &number(x)),
                    ("cy", &number(y)),
                    ("r", &number(NODE_RADIUS)),
                ],
            )?,
        )?;
        let name = text_at(document, "ek-node-text", (x, y))?;
        name.set_text_content(Some(name_of(v)));
        add(&group, &name)?;
        add(&nodes, &group)?;
        slots[layout.vertex(v)] = Some(SlotEl { target: group, text: None });
        let (bx, by) = geometry.badges[v];
        let badge = make(document, "g", &[("class", "ek-badge ek-off")])?;
        add(
            &badge,
            &make(
                document,
                "circle",
                &[
                    ("class", "ek-badge-circle"),
                    ("cx", &number(bx)),
                    ("cy", &number(by)),
                    ("r", "9"),
                ],
            )?,
        )?;
        let number_text = text_at(document, "ek-badge-text", (bx, by))?;
        add(&badge, &number_text)?;
        add(&badges, &badge)?;
        slots[layout.badge(v)] = Some(SlotEl {
            target: badge,
            text: Some(number_text),
        });
    }
    for e in 0..ne {
        let route = &geometry.edges[e];
        let group = make(
            document,
            "g",
            &[
                ("id", &format!("{prefix}-edge-e{e}")),
                ("class", "ek-edge"),
                ("role", "listitem"),
                ("tabindex", "-1"),
                ("data-ek-edge", &format!("e{e}")),
            ],
        )?;
        add(
            &group,
            &make(
                document,
                "path",
                &[
                    ("class", "ek-edge-line"),
                    ("d", &route.d),
                    ("marker-end", &format!("url(#{prefix}-mk-edge)")),
                ],
            )?,
        )?;
        add(&group, &make(document, "path", &[("class", "ek-edge-hit"), ("d", &route.d)])?)?;
        add(&edges, &group)?;
        slots[layout.edge(e)] = Some(SlotEl { target: group, text: None });
        let label = text_at(document, "ek-label", route.label)?;
        add(&labels, &label)?;
        slots[layout.edge_label(e)] = Some(SlotEl {
            target: label.clone(),
            text: Some(label),
        });
        for direction in [Direction::Forward, Direction::Reverse] {
            let (letter, kind) = if direction == Direction::Forward { ("f", "res-f") } else { ("r", "res-r") };
            let lane = geometry.lane(e, direction);
            let group = make(
                document,
                "g",
                &[
                    ("id", &format!("{prefix}-res-e{e}-{letter}")),
                    ("class", "ek-res ek-off"),
                    ("data-ek-edge", &format!("e{e}")),
                    ("data-ek-direction", letter),
                ],
            )?;
            add(
                &group,
                &make(
                    document,
                    "path",
                    &[
                        ("class", "ek-res-line"),
                        ("d", &lane.d),
                        ("marker-end", &format!("url(#{prefix}-mk-{kind})")),
                    ],
                )?,
            )?;
            add(&group, &make(document, "path", &[("class", "ek-res-hit"), ("d", &lane.d)])?)?;
            add(&lanes, &group)?;
            slots[layout.lane(e, direction)] = Some(SlotEl { target: group, text: None });
            let label = text_at(document, "ek-res-label ek-off", lane.label)?;
            add(&labels, &label)?;
            slots[layout.lane_label(e, direction)] = Some(SlotEl {
                target: label.clone(),
                text: Some(label),
            });
        }
    }
    for layer in [&lanes, &edges, &labels, &nodes, &badges] {
        add(&root, layer)?;
    }
    let _ = (VIEW_WIDTH, VIEW_HEIGHT);
    Ok(Scene {
        root,
        slots: slots.into_iter().flatten().collect(),
        layout,
    })
}
