use super::*;
use crate::fft::waves::initial;
use svg_dom_graph::scene::{PlotLabel, PlotPlane, TickLabelPlacement};

fn lines(items: &[PlotItem]) -> Vec<&PlotLine> {
    items
        .iter()
        .filter_map(|item| if let PlotItem::Line(line) = item { Some(line) } else { None })
        .collect()
}

#[test]
fn the_vertical_axis_is_at_the_left_edge_and_shows_negative_and_positive_values() {
    let config = config();

    assert_eq!(config.y.position, AxisPosition::AtMinimum);
    assert_eq!((config.y.min, config.y.max), (-RANGE, RANGE));
    assert_eq!(config.y.major_interval, Some(1.0));
    assert_eq!(config.y.title.as_deref(), Some("value"));
}

#[test]
fn time_runs_from_zero_to_one_so_only_positive_values_are_shown_along_it() {
    let config = config();

    assert_eq!((config.x.min, config.x.max), (0.0, 1.0));
    assert_eq!(
        config.x.position,
        AxisPosition::AtZero,
        "the horizontal axis line is at a value of zero"
    );
    assert_eq!(config.x.title.as_deref(), Some("time, over one window"));
    let TickLabels::Custom(ticks) = &config.x.tick_labels else {
        panic!("fractions are written for the ticks")
    };
    assert_eq!(ticks.len(), 5);
    assert_eq!(ticks[2], (0.5, "\u{00BD}".to_owned()));
}

#[test]
fn the_graph_is_a_real_plot_with_its_labels_at_the_edge() {
    let config = config();

    assert_eq!(config.plane, PlotPlane::Real);
    assert_eq!(config.label_placement, None, "the margin is the default of a real plot");
    let _ = TickLabelPlacement::Edge;
    config.validate().expect("the graph can be drawn");
}

#[test]
fn the_key_names_each_wave_the_sum_and_the_samples() {
    let key = config().legend;
    let names: Vec<&str> = key.iter().map(|entry| entry.label.as_str()).collect();

    assert_eq!(names, ["A (1 cycle)", "B (2 cycles)", "C (3 cycles)", "A + B + C", "samples"]);
    assert_eq!(key[0].dash, None);
    assert_eq!(key[1].dash.as_deref(), Some("7 4"));
    assert_eq!(key[2].dash.as_deref(), Some("2 4"));
}

#[test]
fn a_graph_has_four_curves_and_one_mark_for_each_sample() {
    for samples in [8, 16] {
        let items = items(&initial(), samples);

        assert_eq!(lines(&items).len(), 4);
        let PlotItem::Points(marks) = items.last().expect("the marks are last") else {
            panic!("the last item is the marks")
        };
        assert_eq!(marks.points.len(), samples);
    }
}

#[test]
fn the_curves_differ_by_more_than_colour() {
    let items = items(&initial(), 8);
    let drawn = lines(&items);

    assert_eq!(drawn[0].dash, None);
    assert_eq!(drawn[1].dash.as_deref(), Some("7 4"));
    assert_eq!(drawn[2].dash.as_deref(), Some("2 4"));
    assert!(drawn[3].width > 1.5 * drawn[0].width, "the sum is thick");
}

#[test]
fn a_curve_has_a_point_for_each_end_of_each_segment_and_runs_over_the_whole_window() {
    let items = items(&initial(), 8);
    let curve = &lines(&items)[3].points;

    assert_eq!(curve.len(), SEGMENTS + 1);
    assert_eq!((curve[0].x, curve[SEGMENTS].x), (0.0, 1.0));
}

#[test]
fn the_marks_are_on_the_sum_at_even_times() {
    let waves = initial();
    let items = items(&waves, 8);
    let PlotItem::Points(marks) = items.last().expect("marks") else {
        panic!("marks")
    };

    for (i, mark) in marks.points.iter().enumerate() {
        let t = i as f64 / 8.0;
        assert_eq!(mark.x, t);
        assert!((mark.y - sum_at(&waves, t)).abs() < 1e-12);
    }
}

#[test]
fn the_sum_can_never_leave_the_graph() {
    let waves = [Wave::new(1.0, 90.0); 3];
    let (low, high) = extremes(&waves);

    assert!(high <= RANGE + 1e-9 && low >= -RANGE - 1e-9);
}

#[test]
fn the_label_names_every_wave_and_the_range_of_the_sum() {
    let said = label(&initial(), 8);

    assert!(said.contains("wave A, 1 cycle, amplitude 1.00, phase 0 degrees"), "{said}");
    assert!(said.contains("wave C, 3 cycles, amplitude 0.40, phase 210 degrees"), "{said}");
    assert!(said.contains("8 evenly spaced points"));
}

#[test]
fn no_label_is_drawn_on_the_graph_itself() {
    // The names are in the key and on the axes, so that nothing on the curves has to be read round.
    assert!(
        !items(&initial(), 8)
            .iter()
            .any(|item| matches!(item, PlotItem::Label(PlotLabel { .. })))
    );
}
