use super::*;
use crate::tarjan::{fixture, view};

const VIEWPORT: (f64, f64) = (400.0, 360.0);

struct Rig {
    graph: Graph,
    edges: Vec<(usize, usize)>,
    trace: Trace,
    layout: Layout,
}

fn rig() -> Rig {
    let graph = fixture::graph();
    Rig {
        edges: graph.edges(),
        trace: Trace::build(&graph).unwrap(),
        layout: Layout::build(101.3, 91.8),
        graph,
    }
}

impl Rig {
    fn present(&self, index: usize, choices: &Choices) -> Presentation {
        let camera = view::zoom_to(VIEWPORT, &self.layout, 0).camera;
        present(&self.context(camera), index, choices).expect("a snapshot")
    }

    fn context(&self, camera: Camera) -> Context<'_> {
        Context {
            graph: &self.graph,
            edges: &self.edges,
            trace: &self.trace,
            layout: &self.layout,
            camera,
            viewport: VIEWPORT,
        }
    }
}

#[test]
fn a_presentation_has_every_vertex_every_edge_and_every_field_and_nothing_is_empty_but_the_view_notice() {
    let r = rig();
    for k in [0, 1, 74, 150, 252] {
        let p = r.present(k, &Choices::first());
        assert_eq!((p.vertices.len(), p.edges.len(), p.panel.len()), (64, 114, PANEL.len()));
        for (i, field) in p.panel.iter().enumerate() {
            assert!(i == 3 || !field.is_empty(), "snapshot {k}: the field {} is empty", PANEL[i].0);
        }
    }
}

#[test]
fn there_is_no_presentation_beyond_the_end() {
    let r = rig();
    assert!(present(&r.context(view::zoom_to(VIEWPORT, &r.layout, 0).camera), 253, &Choices::first()).is_none());
}

#[test]
fn before_the_search_nothing_is_discovered_and_every_cell_says_two_dashes() {
    let p = rig().present(0, &Choices::first());
    assert!(
        p.vertices
            .iter()
            .all(|v| v.values == ["--", "--"] && v.state == VertexState::Undiscovered)
    );
    assert!(p.stack.is_empty() && p.roster.is_empty() && p.outlines.is_empty());
    assert_eq!(p.condensation, Condensation::default());
    assert_eq!(p.panel[0], "0 of 252");
    assert!(p.vertices[0].current, "the first attention is vertex 0");
}

#[test]
fn the_cells_are_two_digit_strings_and_the_count_and_type_never_change() {
    let r = rig();
    for k in (0..=252).step_by(7) {
        for v in &r.present(k, &Choices::first()).vertices {
            assert!(
                v.values
                    .iter()
                    .all(|s| s == "--" || (s.len() == 2 && s.chars().all(|c| c.is_ascii_digit()))),
                "{v:?}"
            );
        }
    }
}

#[test]
fn the_stack_panel_has_a_row_for_every_entry_up_to_27_and_the_roster_one_for_every_component() {
    let r = rig();
    let peak = (0..=252)
        .map(|k| (k, r.present(k, &Choices::first()).stack.len()))
        .max_by_key(|&(_, n)| n)
        .unwrap();
    assert_eq!(peak.1, 27);
    for k in 0..=252 {
        let p = r.present(k, &Choices::first());
        let s = r.trace.snapshot(k).unwrap();
        assert_eq!(p.stack.len(), s.scc_stack.len(), "snapshot {k}");
        assert_eq!(p.roster.len(), s.components.len());
        assert_eq!(p.outlines.len(), s.components.len());
        assert!(
            p.stack.iter().zip(&s.scc_stack).all(|(row, &v)| row.vertex == v),
            "bottom to top, in push order"
        );
    }
}

#[test]
fn a_vertex_is_undiscovered_on_the_stack_or_closed_and_a_closed_one_names_its_component() {
    let r = rig();
    let p = r.present(80, &Choices::first());
    let s = r.trace.snapshot(80).unwrap();
    for v in 0..64 {
        let view = &p.vertices[v];
        match view.state {
            VertexState::Undiscovered => assert!(s.index[v].is_none() && view.component.is_none()),
            VertexState::OnStack => assert!(s.on_stack[v] && view.component.is_none()),
            VertexState::Closed => assert_eq!(view.component, s.membership[v].map(|c| c + 1)),
        }
    }
    assert!(p.vertices[63].label.contains("in component C1"), "{}", p.vertices[63].label);
}

#[test]
fn only_the_attention_vertex_is_current_and_its_name_says_so() {
    let r = rig();
    let p = r.present(35, &Choices::first());
    let current: Vec<usize> = (0..64).filter(|&v| p.vertices[v].current).collect();
    assert_eq!(current, vec![33]);
    assert!(p.vertices[33].label.ends_with("the vertex in focus"));
}

#[test]
fn the_inspected_edge_is_marked_with_its_kind_and_only_it() {
    let r = rig();
    let p = r.present(34, &Choices::first());
    let marked: Vec<(usize, Kind)> = p
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.inspected.map(|k| (i, k)))
        .collect();
    let e = edge_index(&r.edges, 3, 33).unwrap();
    assert_eq!(marked, vec![(e, Kind::Tree)]);
    assert!(p.edges[e].label.contains("being inspected now: a tree edge"));
    assert!(r.present(35, &Choices::first()).edges.iter().all(|e| e.inspected.is_none()));
}

#[test]
fn the_sentences_say_what_each_kind_of_event_did() {
    let r = rig();
    let says = |k: usize| r.present(k, &Choices::first()).panel[1].clone();
    assert!(says(1).starts_with("Enter v00: index and lowlink are 00"), "{}", says(1));
    assert!(
        says(34).contains("a tree edge: v33 is undiscovered, so the search goes down to it"),
        "{}",
        says(34)
    );
    assert!(says(74).starts_with("Component C1 closes at v63"), "{}", says(74));
    assert!(says(252).starts_with("Complete: all 10 components are found"), "{}", says(252));
    let stack = (0..=252)
        .map(&says)
        .find(|t| t.contains("is on the stack, so lowlink"))
        .expect("a stack edge that lowers a lowlink");
    assert!(stack.contains("using the index of"), "{stack}");
    assert!(says(0).starts_with("Before the search"));
    assert!(
        (0..=252)
            .map(says)
            .any(|t| t.contains("is already in a component, so it is ignored"))
    );
    assert!(
        (0..=252)
            .map(says)
            .any(|t| t.starts_with("Return from") && t.contains("goes from"))
    );
}

#[test]
fn the_two_stacks_are_named_apart_in_the_panel() {
    let r = rig();
    let p = r.present(100, &Choices::first());
    assert!(PANEL[4].1.contains("SCC stack") && PANEL[5].1.contains("DFS chain"));
    assert!(p.panel[4].starts_with("depth ") && p.panel[5].contains("calls:"));
}

#[test]
fn a_component_that_is_selected_is_picked_out_in_every_view_and_cleared_when_it_is_not_emitted() {
    let r = rig();
    let mut choices = Choices::first();
    choices.component = Some(3);
    let gone = r.present(80, &choices);
    assert!(gone.outlines.iter().all(|o| !o.selected) && gone.roster.iter().all(|row| !row.selected));
    assert_eq!(gone.dag_selected, None, "C3 is not emitted until step 81");
    let there = r.present(81, &choices);
    assert!(there.outlines.iter().any(|o| o.component == 3 && o.selected));
    assert!(there.roster.iter().any(|row| row.component == 3 && row.selected));
    assert_eq!(there.dag_selected, Some(3));
}

#[test]
fn selecting_a_vertex_or_an_edge_describes_it_and_marks_it() {
    let r = rig();
    let mut choices = Choices::first();
    choices.selected = Some(Target::Vertex(33));
    let p = r.present(36, &choices);
    assert!(
        p.vertices[33].selected && p.panel[8].starts_with("v33: index"),
        "{}",
        p.panel[8]
    );
    let e = edge_index(&r.edges, 3, 33).unwrap();
    choices.selected = Some(Target::Edge(e));
    let p = r.present(36, &choices);
    assert!(p.edges[e].selected && p.panel[8].starts_with("e"), "{}", p.panel[8]);
    assert!(p.panel[8].contains("from v03 to v33"));
    choices.selected = None;
    assert!(r.present(36, &choices).panel[8].starts_with("Nothing selected"));
}

#[test]
fn exactly_one_target_is_a_tab_stop() {
    let r = rig();
    let mut choices = Choices::first();
    choices.roving = Target::Edge(17);
    let p = r.present(10, &choices);
    let stops = p.vertices.iter().filter(|v| v.tab_stop).count() + p.edges.iter().filter(|e| e.tab_stop).count();
    assert_eq!(stops, 1);
    assert!(p.edges[17].tab_stop);
}

#[test]
fn the_view_notice_is_empty_in_view_and_says_near_the_edge_or_off_screen_whatever_following_is() {
    let r = rig();
    let on = |k: usize, camera: Camera, follow: bool| {
        let mut choices = Choices::first();
        choices.follow = follow;
        present(&r.context(camera), k, &choices).unwrap().panel[3].clone()
    };
    let open = view::zoom_to(VIEWPORT, &r.layout, 0).camera;
    assert_eq!(on(0, open, true), "");
    // Seeking to the closure of B9 leaves the view where it was, far from vertex 63.
    for follow in [true, false] {
        assert_eq!(
            on(74, open, follow),
            "Active vertex v63 is off-screen; use Zoom to active vertex."
        );
    }
    let near = Camera { tx: open.tx + 330.0, ..open };
    let text = on(0, near, true);
    assert!(text.contains("near the edge") || text.contains("off-screen") || text.is_empty());
    assert_eq!(view_notice(Some(Visibility::Inside), Some(3)), "");
    assert_eq!(
        view_notice(Some(Visibility::NearEdge), Some(3)),
        "Active vertex v03 is near the edge of the view; use Zoom to active vertex."
    );
    assert_eq!(view_notice(None, None), "");
}

#[test]
fn an_inspected_edge_whose_far_end_is_off_screen_names_it() {
    let r = rig();
    let open = view::zoom_to(VIEWPORT, &r.layout, 0).camera;
    let p = present(&r.context(open), 34, &Choices::first()).unwrap();
    assert!(p.panel[1].ends_with("The target v33 is off-screen."), "{}", p.panel[1]);
    let near = r.layout.centre(33);
    let there = open.centred_on(near, VIEWPORT);
    let q = present(&r.context(there), 34, &Choices::first()).unwrap();
    assert!(!q.panel[1].contains("off-screen"), "{}", q.panel[1]);
}

#[test]
fn the_condensation_part_is_the_condensation_of_the_snapshot_and_grows_one_node_at_a_closure() {
    let r = rig();
    assert_eq!(r.present(74, &Choices::first()).condensation.nodes.len(), 1);
    assert_eq!(r.present(73, &Choices::first()).condensation.nodes.len(), 0);
    let last = r.present(252, &Choices::first());
    assert_eq!((last.condensation.nodes.len(), last.condensation.arcs.len()), (10, 17));
    assert_eq!(last.panel[7], "10 components and 17 arrows between them");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A page, as far as the tests need to know it, which writes are made on.
fn apply(mut page: Presentation, writes: &[Write]) -> Presentation {
    for write in writes {
        match write {
            Write::Vertex(i, v) => page.vertices[*i] = v.clone(),
            Write::Edge(i, e) => page.edges[*i] = e.clone(),
            Write::Outlines(o) => page.outlines = o.clone(),
            Write::Stack(s) => page.stack = s.clone(),
            Write::Roster(r) => page.roster = r.clone(),
            Write::Dag(c, selected) => {
                page.condensation = c.clone();
                page.dag_selected = *selected;
            },
            Write::Panel(i, t) => page.panel[*i] = t.clone(),
        }
    }
    page
}

fn blank(like: &Presentation) -> Presentation {
    let mut b = like.clone();
    b.vertices.iter_mut().for_each(|v| v.label.clear());
    b.edges.iter_mut().for_each(|e| e.label.clear());
    b.outlines.clear();
    b.stack.clear();
    b.roster.clear();
    b.condensation = Condensation::default();
    b.dag_selected = None;
    b.panel.iter_mut().for_each(String::clear);
    b
}

#[test]
fn a_diff_gives_the_same_page_as_a_full_draw_for_every_snapshot_after_every_other() {
    let r = rig();
    let mut choices = Choices::first();
    choices.selected = Some(Target::Vertex(5));
    for k in 0..=252usize {
        let after = r.present(k, &choices);
        // The status is said aloud and never written, so the pages are compared without it.
        let mut drawn_whole = apply(blank(&after), &full(&after));
        drawn_whole.status = after.status.clone();
        for from in [0usize, k.saturating_sub(1), (k * 7 + 3) % 253, 252 - k] {
            let before = r.present(from, &choices);
            let mut patched = apply(apply(blank(&before), &full(&before)), &diff(&before, &after));
            patched.status = after.status.clone();
            assert_eq!(patched, drawn_whole, "{from} to {k}");
        }
    }
}

#[test]
fn a_diff_between_equal_presentations_writes_nothing_and_between_neighbours_writes_little() {
    let r = rig();
    let p = r.present(40, &Choices::first());
    assert!(diff(&p, &p).is_empty());
    let q = r.present(41, &Choices::first());
    assert!(diff(&p, &q).len() < full(&q).len() / 10);
}

#[test]
fn a_change_of_selection_diffs_to_the_view_alone() {
    let r = rig();
    let before = r.present(40, &Choices::first());
    let mut choices = Choices::first();
    choices.selected = Some(Target::Vertex(3));
    choices.roving = Target::Vertex(3);
    let writes = diff(&before, &r.present(40, &choices));
    assert!(
        writes.iter().all(|w| matches!(w, Write::Vertex(..) | Write::Panel(8, _))),
        "{writes:?}"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_buttons_have_nowhere_to_go_at_the_ends_and_between_the_closures() {
    let t = rig().trace;
    let b = |k: usize| bounds(&t, k);
    assert!(b(0).restart && b(0).previous && !b(0).next && b(0).previous_component && !b(0).next_component);
    assert!(b(252).next && b(252).play && b(252).last && !b(252).previous && b(252).next_component);
    assert!(!b(100).previous_component && !b(100).next_component);
    assert!(b(251).next_component, "nothing closes after the last closure");
    assert!(!b(247).next_component && !b(251).previous_component);
}

#[test]
fn only_a_closure_and_the_end_are_said_aloud_during_playback() {
    let t = rig().trace;
    assert!(!announced_while_playing(&t, 0) && !announced_while_playing(&t, 34));
    assert!(announced_while_playing(&t, 74) && announced_while_playing(&t, 252));
    assert_eq!((0..=252).filter(|&k| announced_while_playing(&t, k)).count(), 11);
}

#[test]
fn pausing_says_where_and_how_many_are_found() {
    assert_eq!(
        pause_summary(&rig().trace, 100),
        "Paused at step 100 of 252. 3 components found."
    );
}

#[test]
fn the_speeds_are_the_four_of_the_design_with_400_the_normal_one() {
    let ms: Vec<u32> = Speed::ALL.iter().map(|s| s.interval_ms()).collect();
    assert_eq!(ms, vec![800, 400, 200, 100]);
    assert_eq!(Choices::first().speed, Speed::Normal);
    for s in Speed::ALL {
        assert_eq!(Speed::from_key(s.key()), Some(s));
    }
    assert_eq!(Speed::from_key("300"), None);
}

#[test]
fn edge_names_and_places_are_stable() {
    let r = rig();
    assert_eq!(edge_name(0), "e000");
    assert_eq!(edge_name(113), "e113");
    assert_eq!(edge_index(&r.edges, 0, 1), Some(0));
    assert_eq!(edge_index(&r.edges, 0, 63), None);
}

#[test]
fn the_keyboard_visits_the_64_vertices_and_then_the_114_edges_and_does_not_wrap() {
    let order = target_order(64, 114);
    assert_eq!(order.len(), 178);
    assert_eq!(
        (order[0], order[63], order[64], order[177]),
        (Target::Vertex(0), Target::Vertex(63), Target::Edge(0), Target::Edge(113))
    );
    assert_eq!(move_roving(&order, Target::Vertex(63), "ArrowRight"), Some(Target::Edge(0)));
    assert_eq!(move_roving(&order, Target::Edge(0), "ArrowLeft"), Some(Target::Vertex(63)));
    assert_eq!(move_roving(&order, Target::Vertex(0), "ArrowUp"), None);
    assert_eq!(move_roving(&order, Target::Edge(113), "ArrowDown"), None);
    assert_eq!(move_roving(&order, Target::Edge(50), "Home"), Some(Target::Vertex(0)));
    assert_eq!(move_roving(&order, Target::Vertex(5), "End"), Some(Target::Edge(113)));
    assert_eq!(move_roving(&order, Target::Vertex(0), "Home"), None);
    assert_eq!(move_roving(&order, Target::Vertex(0), "x"), None);
}
