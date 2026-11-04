//! Where everything is: the places, the gates, the hubs and the controls. Pure: nothing here touches the DOM.
//!
//! Each worker is a ring, so that its last arc returns round the ring to its first place and no arc crosses from one
//! lane to another. The four producer rings are on the left, the two buffer places are in the middle with Free above
//! Used, and the three consumer rings are on the right. These coordinates are fixed for every snapshot.

use super::net::{Arc, PRODUCERS, PlaceId, PlaceKind, TransitionId, TransitionKind};
use svg_dom::root::utils::{Point, Rect, Size};

/// The size of the graph, in the units of the scene.
pub(crate) const VIEW_WIDTH: f64 = 1040.0;
pub(crate) const VIEW_HEIGHT: f64 = 850.0;

/// How tall the controls above the graph are, and the size of the scene that holds both.
pub(crate) const TOP: f64 = 168.0;
pub(crate) const SCENE: Size = Size {
    width: VIEW_WIDTH,
    height: TOP + VIEW_HEIGHT,
};

/// The radius of the place of a worker, and of a hub.
pub(crate) const RADIUS: f64 = 24.0;
pub(crate) const HUB_RADIUS: f64 = 34.0;

/// How far above the centre of a hub the library's label ("Free" or "Used") is moved, to leave room for the count under
/// it.
pub(crate) const HUB_LABEL_LIFT: f64 = 14.0;

/// The width and height of every gate, which is a narrow upright bar.
pub(crate) const GATE: Size = Size { width: 12.0, height: 36.0 };

/// Where the first producer ring and the first consumer ring are, and how far apart the rings are.
const PRODUCER_ORIGIN: (f64, f64) = (30.0, 40.0);
const PRODUCER_PITCH: f64 = 200.0;
const CONSUMER_ORIGIN: (f64, f64) = (800.0, 60.0);
const CONSUMER_PITCH: f64 = 250.0;

/// The centres of the two hubs.
pub(crate) const FREE_CENTRE: (f64, f64) = (590.0, 355.0);
pub(crate) const USED_CENTRE: (f64, f64) = (590.0, 515.0);

/// The minute round a hub where its spoke to each producer's Put leaves, and to each consumer's Take. The minutes run
/// clockwise from north, so 45 is west and 15 is east.
pub(crate) const PRODUCER_MINUTES: [u8; 4] = [52, 48, 44, 40];
pub(crate) const CONSUMER_MINUTES: [u8; 3] = [21, 15, 9];

/// Where along the side of a gate its two buffer arcs are fixed: the one to or from Free is above the one to or from
/// Used, as the hubs are.
pub(crate) const FREE_POSITION: f64 = 0.25;
pub(crate) const USED_POSITION: f64 = 0.75;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The centre of a place, in the units of the graph.
pub(crate) fn place_centre(p: PlaceId) -> Point {
    let (x, y) = match p.kind() {
        PlaceKind::Free => FREE_CENTRE,
        PlaceKind::Used => USED_CENTRE,
        kind => {
            let worker = p.worker().unwrap_or(0);
            let (ox, oy) = ring_origin(worker, matches!(kind, PlaceKind::ConsumerIdle | PlaceKind::ConsumerProcessing));
            let (dx, dy) = match kind {
                PlaceKind::ProducerIdle => (330.0, 160.0),
                PlaceKind::ProducerBusy => (110.0, 95.0),
                PlaceKind::ProducerReady => (330.0, 30.0),
                PlaceKind::ConsumerIdle => (50.0, 30.0),
                PlaceKind::ConsumerProcessing => (50.0, 160.0),
                PlaceKind::Free | PlaceKind::Used => (0.0, 0.0),
            };
            (ox + dx, oy + dy)
        },
    };
    Point::new(x, y)
}

/// The centre of a gate.
pub(crate) fn gate_centre(t: TransitionId) -> Point {
    let consumer = matches!(t.kind(), TransitionKind::Take | TransitionKind::Complete);
    let (ox, oy) = ring_origin(t.worker(), consumer);
    let (dx, dy) = match t.kind() {
        TransitionKind::Start => (220.0, 160.0),
        TransitionKind::Finish => (220.0, 30.0),
        TransitionKind::Put => (330.0, 95.0),
        TransitionKind::Take => (50.0, 95.0),
        TransitionKind::Complete => (150.0, 95.0),
    };
    Point::new(ox + dx, oy + dy)
}

/// Where the label of a gate is written: the baseline of its first line, and which way the text runs from there. The
/// label is on the side of the gate that has no arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LabelAt {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) anchor: &'static str,
    /// Whether the status follows the name on one line, and not under it.
    pub(crate) inline: bool,
}

pub(crate) fn gate_label(t: TransitionId) -> LabelAt {
    let c = gate_centre(t);
    let (half_w, half_h) = (GATE.width / 2.0, GATE.height / 2.0);
    match t.kind() {
        TransitionKind::Put => LabelAt {
            x: c.x - half_w - 8.0,
            y: c.y - 2.0,
            anchor: "end",
            inline: false,
        },
        TransitionKind::Take | TransitionKind::Complete => LabelAt {
            x: c.x + half_w + 8.0,
            y: c.y - 2.0,
            anchor: "start",
            inline: false,
        },
        TransitionKind::Start => LabelAt {
            x: c.x - half_w - 8.0,
            y: c.y + half_h - 2.0,
            anchor: "end",
            inline: true,
        },
        TransitionKind::Finish => LabelAt {
            x: c.x,
            y: c.y - half_h - 8.0,
            anchor: "middle",
            inline: true,
        },
    }
}

/// The rectangle of a gate.
pub(crate) fn gate_rect(t: TransitionId) -> Rect {
    let c = gate_centre(t);
    Rect {
        origin: Point::new(c.x - GATE.width / 2.0, c.y - GATE.height / 2.0),
        size: GATE,
    }
}

/// The top left corner of the ring of a worker.
fn ring_origin(worker: usize, consumer: bool) -> (f64, f64) {
    if consumer {
        (CONSUMER_ORIGIN.0, CONSUMER_ORIGIN.1 + CONSUMER_PITCH * worker as f64)
    } else {
        (PRODUCER_ORIGIN.0, PRODUCER_ORIGIN.1 + PRODUCER_PITCH * worker as f64)
    }
}

/// Where a worker's ring is named, which is clear of every arc and every label.
pub(crate) fn ring_title_at(worker: usize, consumer: bool) -> Point {
    let (x, y) = ring_origin(worker, consumer);
    if consumer {
        Point::new(x + 130.0, y + 2.0)
    } else {
        Point::new(x + 220.0, y + 99.0)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How one end of an arc is fixed to its node, when the node's own rule is not what is wanted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum End {
    /// The library chooses, from a ray between the centres.
    Auto,
    /// At this minute round a circle, 0 to 59.
    Clock(u8),
    /// On this side of a gate, at this fraction of the way along it.
    Side(GateSide, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GateSide {
    North,
    South,
    East,
    West,
}

/// How both ends of an arc are fixed. The `from` end is the place for an input arc and the gate for an output arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ArcEnds {
    pub(crate) from: End,
    pub(crate) to: End,
}

/// How the ends of `arc` are fixed. A gate never gets a clock position, since only a circle can have one.
pub(crate) fn arc_ends(arc: &Arc) -> ArcEnds {
    let (place, t) = (arc.place, arc.transition);
    let hub = matches!(place.kind(), PlaceKind::Free | PlaceKind::Used);
    let (gate_end, place_end) = match (t.kind(), hub) {
        // A Put takes its prepared item from the north, returns the worker to the south, and has the buffer to the
        // east.
        (TransitionKind::Put, false) if arc.input => (End::Side(GateSide::North, 0.5), End::Auto),
        (TransitionKind::Put, false) => (End::Side(GateSide::South, 0.5), End::Auto),
        (TransitionKind::Put, true) => (
            End::Side(
                GateSide::East,
                if place.kind() == PlaceKind::Free { FREE_POSITION } else { USED_POSITION },
            ),
            End::Clock(PRODUCER_MINUTES[t.worker() % PRODUCERS]),
        ),
        // A Take mirrors it: the idle consumer is to the north, the processing consumer to the south, the buffer to the
        // west.
        (TransitionKind::Take, false) if arc.input => (End::Side(GateSide::North, 0.5), End::Auto),
        (TransitionKind::Take, false) => (End::Side(GateSide::South, 0.5), End::Auto),
        (TransitionKind::Take, true) => (
            End::Side(
                GateSide::West,
                if place.kind() == PlaceKind::Free { FREE_POSITION } else { USED_POSITION },
            ),
            End::Clock(CONSUMER_MINUTES[t.worker()]),
        ),
        _ => (End::Auto, End::Auto),
    };
    if arc.input {
        ArcEnds { from: place_end, to: gate_end }
    } else {
        ArcEnds { from: gate_end, to: place_end }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cells of the picture of the buffer, which are filled for each used slot: the top left of the first and the size
/// of each, and the gap between them.
pub(crate) const CELL_ORIGIN: (f64, f64) = (464.0, 60.0);
pub(crate) const CELL_SIZE: (f64, f64) = (28.0, 28.0);
pub(crate) const CELL_GAP: f64 = 4.0;

/// Where the buffer's caption is written, which is centred.
pub(crate) const CAPTION_AT: (f64, f64) = (590.0, 116.0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far from the left edge of the scene the controls are, and the height of a selector or a button.
pub(crate) const MARGIN: f64 = 12.0;
pub(crate) const ROW_HEIGHT: f64 = 28.0;

/// How far down the scene the three rows of controls are.
pub(crate) const SELECT_ROW: f64 = 10.0;
pub(crate) const BUTTON_ROW: f64 = 48.0;
/// The height of the middle of the range's track. Its name and its value are written 13 pixels above.
pub(crate) const RANGE_MIDDLE: f64 = 124.0;

const GAP: f64 = 6.0;

/// The buttons: what each does, what is written on it, how wide it is, and whether it is a toggle.
pub(crate) const BUTTONS: [(&str, &str, f64, bool); 7] = [
    ("restart", "Restart", 72.0, false),
    ("previous", "Previous", 84.0, false),
    ("next", "Next", 64.0, false),
    ("play", "Play", 72.0, false),
    ("manual", "Enter manual", 132.0, true),
    ("fire", "Fire selected", 120.0, false),
    ("toggle-focused", "Focused wiring", 130.0, true),
];

/// A selector and its name: where the name starts, and the area of the selector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SelectorPlace {
    pub(crate) name_left: f64,
    pub(crate) area: Rect,
}

const fn selector(name_left: f64, left: f64, width: f64) -> SelectorPlace {
    SelectorPlace {
        name_left,
        area: Rect {
            origin: Point { x: left, y: SELECT_ROW },
            size: Size { width, height: ROW_HEIGHT },
        },
    }
}

pub(crate) const SCENARIO_SELECTOR: SelectorPlace = selector(MARGIN, 82.0, 330.0);
pub(crate) const CHECKPOINT_SELECTOR: SelectorPlace = selector(430.0, 520.0, 240.0);
pub(crate) const SPEED_SELECTOR: SelectorPlace = selector(780.0, 840.0, 130.0);

/// The size of text, in pixels, that is the usual size, or smaller if the longest of its `chars` characters would not
/// fit across `width` once `spare` pixels are taken off for an arrow or a margin. A glyph is taken to be 0.62 of its
/// size wide, as the widest of the fonts that the page may use is.
fn fitted_font_px(chars: usize, width: f64, spare: f64) -> f64 {
    const USUAL: f64 = 14.0;
    const GLYPH: f64 = 0.62;
    if chars == 0 {
        return USUAL;
    }
    ((width - spare) / (chars as f64 * GLYPH)).clamp(8.0, USUAL).floor()
}

/// The size of the text of a selector, which leaves room for the arrow.
pub(crate) fn selector_font_px(chars: usize, width: f64) -> f64 {
    fitted_font_px(chars, width, 30.0)
}

/// The size of the text of a button, which leaves a margin on each side.
pub(crate) fn button_font_px(chars: usize, width: f64) -> f64 {
    fitted_font_px(chars, width, 16.0)
}

/// The range: the left end of its track and how long it is. The thumb is 16 pixels wide, half of it past either end.
pub(crate) const RANGE_LEFT: f64 = MARGIN + 8.0;
pub(crate) const RANGE_LENGTH: f64 = VIEW_WIDTH - 2.0 * RANGE_LEFT;

/// The area of the button at `index` of [`BUTTONS`]. The row goes from left to right, from the left margin.
pub(crate) fn button_area(index: usize) -> Rect {
    let Some(&(_, _, width, _)) = BUTTONS.get(index) else {
        return Rect {
            origin: Point::new(0.0, 0.0),
            size: Size::new(0.0, 0.0),
        };
    };
    let left = MARGIN + BUTTONS[..index].iter().map(|b| b.2 + GAP).sum::<f64>();
    Rect {
        origin: Point::new(left, BUTTON_ROW),
        size: Size::new(width, ROW_HEIGHT),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
