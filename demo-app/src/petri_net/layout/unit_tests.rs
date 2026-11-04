use super::*;
use crate::petri_net::net::{Net, PLACES, TRANSITIONS};

fn circle_radius(p: PlaceId) -> f64 {
    if matches!(p.kind(), PlaceKind::Free | PlaceKind::Used) {
        HUB_RADIUS
    } else {
        RADIUS
    }
}

fn gate_circle_gap(t: TransitionId, p: PlaceId) -> f64 {
    let (g, c) = (gate_rect(t), place_centre(p));
    let nearest_x = c.x.clamp(g.origin.x, g.origin.x + g.size.width);
    let nearest_y = c.y.clamp(g.origin.y, g.origin.y + g.size.height);
    ((c.x - nearest_x).powi(2) + (c.y - nearest_y).powi(2)).sqrt() - circle_radius(p)
}

#[test]
fn everything_is_inside_the_graph_with_room_for_its_label() {
    for p in PlaceId::all() {
        let (c, r) = (place_centre(p), circle_radius(p));
        assert!(
            c.x - r >= 0.0 && c.x + r <= VIEW_WIDTH && c.y - r >= 0.0 && c.y + r <= VIEW_HEIGHT,
            "{}",
            p.name()
        );
    }
    for t in TransitionId::all() {
        let g = gate_rect(t);
        assert!(g.origin.x >= 0.0 && g.origin.x + g.size.width <= VIEW_WIDTH, "{}", t.name());
        assert!(g.origin.y >= 0.0 && g.origin.y + g.size.height <= VIEW_HEIGHT, "{}", t.name());
    }
}

#[test]
fn no_two_places_overlap() {
    let places: Vec<PlaceId> = PlaceId::all().collect();
    assert_eq!(places.len(), PLACES);
    for (n, &a) in places.iter().enumerate() {
        for &b in &places[n + 1..] {
            let (ca, cb) = (place_centre(a), place_centre(b));
            let distance = ((ca.x - cb.x).powi(2) + (ca.y - cb.y).powi(2)).sqrt();
            assert!(
                distance > circle_radius(a) + circle_radius(b) + 8.0,
                "{} and {}",
                a.name(),
                b.name()
            );
        }
    }
}

#[test]
fn no_gate_touches_a_place_or_another_gate() {
    for t in TransitionId::all() {
        for p in PlaceId::all() {
            assert!(gate_circle_gap(t, p) > 8.0, "{} and {}", t.name(), p.name());
        }
        for u in TransitionId::all().filter(|u| u.index() > t.index()) {
            let (a, b) = (gate_centre(t), gate_centre(u));
            assert!(
                (a.x - b.x).abs() > GATE.width || (a.y - b.y).abs() > GATE.height,
                "{} and {}",
                t.name(),
                u.name()
            );
        }
    }
    assert_eq!(TransitionId::all().count(), TRANSITIONS);
}

#[test]
fn every_arc_leaves_room_for_its_nine_pixel_arrowhead() {
    for arc in Net::bounded_buffer().arcs() {
        let gap = gate_circle_gap(arc.transition, arc.place);
        assert!(
            gap >= 20.0,
            "{} and {} are only {gap:.1} apart",
            arc.transition.name(),
            arc.place.name()
        );
    }
}

#[test]
fn free_is_above_used_and_the_producers_are_left_of_the_hubs_and_the_consumers_right() {
    assert!(FREE_CENTRE.1 < USED_CENTRE.1);
    for t in TransitionId::all() {
        match t.kind() {
            TransitionKind::Put => assert!(gate_centre(t).x < FREE_CENTRE.0 - HUB_RADIUS),
            TransitionKind::Take => assert!(gate_centre(t).x > FREE_CENTRE.0 + HUB_RADIUS),
            _ => {},
        }
    }
}

#[test]
fn a_gate_never_gets_a_clock_and_a_circle_never_gets_a_side() {
    for arc in Net::bounded_buffer().arcs() {
        let ends = arc_ends(&arc);
        let (gate, place) = if arc.input { (ends.to, ends.from) } else { (ends.from, ends.to) };
        assert!(!matches!(gate, End::Clock(_)), "{}", arc.transition.name());
        assert!(!matches!(place, End::Pinned(..)), "{}", arc.place.name());
    }
}

#[test]
fn each_hub_has_seven_distinct_minutes_and_the_producer_and_consumer_sides_do_not_meet() {
    let arcs = Net::bounded_buffer().arcs();
    for hub in [PlaceId::FREE, PlaceId::USED] {
        let mut minutes: Vec<u8> = arcs
            .iter()
            .filter(|a| a.place == hub)
            .filter_map(|a| {
                let ends = arc_ends(a);
                match if a.input { ends.from } else { ends.to } {
                    End::Clock(m) => Some(m),
                    _ => None,
                }
            })
            .collect();
        assert_eq!(minutes.len(), 7);
        minutes.sort_unstable();
        minutes.dedup();
        assert_eq!(minutes.len(), 7, "{}", hub.name());
        assert!(minutes.iter().all(|&m| m < 60));
        assert!(
            minutes.windows(2).all(|w| w[1] - w[0] >= 3),
            "spokes at least three minutes apart"
        );
    }
}

const _: () = assert!(FREE_POSITION < USED_POSITION);

#[test]
fn the_free_arc_is_always_above_the_used_arc_on_a_shared_side() {
    for arc in Net::bounded_buffer().arcs() {
        let (hub, gate) = (arc.place, arc.transition);
        if !matches!(hub.kind(), PlaceKind::Free | PlaceKind::Used) {
            continue;
        }
        let ends = arc_ends(&arc);
        let End::Pinned(_, position) = (if arc.input { ends.to } else { ends.from }) else {
            panic!("{} has no side", gate.name());
        };
        let expected = if hub.kind() == PlaceKind::Free { FREE_POSITION } else { USED_POSITION };
        assert_eq!(position, expected, "{} with {}", gate.name(), hub.name());
    }
}

#[test]
fn a_put_and_a_take_use_four_different_anchors() {
    let arcs = Net::bounded_buffer().arcs();
    for t in TransitionId::all().filter(|t| matches!(t.kind(), TransitionKind::Put | TransitionKind::Take)) {
        let mut anchors: Vec<(Side, u32)> = arcs
            .iter()
            .filter(|a| a.transition == t)
            .map(|a| {
                let ends = arc_ends(a);
                match if a.input { ends.to } else { ends.from } {
                    End::Pinned(side, position) => (side, (position * 100.0) as u32),
                    other => panic!("{other:?}"),
                }
            })
            .collect();
        assert_eq!(anchors.len(), 4);
        anchors.sort_by_key(|&(side, position)| (side as u8, position));
        anchors.dedup();
        assert_eq!(anchors.len(), 4, "{}", t.name());
    }
}

#[test]
fn the_buffer_picture_is_inside_the_graph_and_has_eight_cells() {
    let right = CELL_ORIGIN.0 + 8.0 * (CELL_SIZE.0 + CELL_GAP) - CELL_GAP;
    assert!(right <= VIEW_WIDTH && CELL_ORIGIN.1 + CELL_SIZE.1 < CAPTION_AT.1);
    assert!(
        ((CELL_ORIGIN.0 + right) / 2.0 - FREE_CENTRE.0).abs() < 1.0,
        "centred over the hubs"
    );
}

#[test]
fn the_buttons_are_in_one_row_and_do_not_overlap() {
    let mut right = 0.0;
    for index in 0..BUTTONS.len() {
        let area = button_area(index);
        assert!(area.origin.x >= right && area.origin.y == BUTTON_ROW);
        right = area.origin.x + area.size.width;
    }
    assert!(right <= VIEW_WIDTH);
    assert_eq!(button_area(BUTTONS.len()).size.width, 0.0);
}

#[test]
fn the_selectors_fit_in_their_row() {
    let places = [SCENARIO_SELECTOR, CHECKPOINT_SELECTOR, SPEED_SELECTOR];
    let mut right = 0.0;
    for place in places {
        assert!(place.name_left >= right);
        assert!(place.area.origin.x > place.name_left);
        right = place.area.origin.x + place.area.size.width;
    }
    assert!(right <= VIEW_WIDTH);
}

#[test]
fn every_gate_label_is_inside_the_graph_and_off_the_side_that_has_arcs() {
    for t in TransitionId::all() {
        let (c, label) = (gate_centre(t), gate_label(t));
        assert!(label.x > 40.0 && label.x < VIEW_WIDTH - 40.0, "{}", t.name());
        assert!(label.y > 10.0 && label.y < VIEW_HEIGHT, "{}", t.name());
        match t.kind() {
            TransitionKind::Put => assert!(label.x < c.x && label.anchor == "end"),
            TransitionKind::Take | TransitionKind::Complete => assert!(label.x > c.x && label.anchor == "start"),
            TransitionKind::Start => assert!(label.x < c.x && label.y > c.y && label.anchor == "end"),
            TransitionKind::Finish => assert!(label.x == c.x && label.y < c.y),
        }
    }
}

#[test]
fn a_selector_text_shrinks_to_fit_and_never_grows_past_the_usual_size() {
    assert_eq!(selector_font_px(0, 100.0), 14.0);
    assert_eq!(selector_font_px(5, 400.0), 14.0, "room to spare");
    let (chars, width) = (41, SCENARIO_SELECTOR.area.size.width);
    let size = selector_font_px(chars, width);
    assert!((8.0..14.0).contains(&size));
    assert!(chars as f64 * size * 0.62 <= width - 30.0, "{size}");
    assert_eq!(selector_font_px(500, 100.0), 8.0, "there is a smallest size");
}

#[test]
fn every_selector_reads_at_ten_pixels_or_more() {
    for (chars, place) in [(41, SCENARIO_SELECTOR), (32, CHECKPOINT_SELECTOR), (11, SPEED_SELECTOR)] {
        assert!(selector_font_px(chars, place.area.size.width) >= 10.0);
    }
}

#[test]
fn a_button_text_shrinks_to_fit_and_the_two_labels_of_the_manual_button_both_fit() {
    let width = BUTTONS.iter().find(|b| b.0 == "manual").map(|b| b.2).unwrap_or(0.0);
    assert_eq!(button_font_px("Enter manual".len(), width), 14.0);
    let size = button_font_px("Return to script".len(), width);
    assert!((8.0..14.0).contains(&size) && size >= 10.0, "{size}");
    assert!("Return to script".len() as f64 * size * 0.62 <= width - 16.0);
}
