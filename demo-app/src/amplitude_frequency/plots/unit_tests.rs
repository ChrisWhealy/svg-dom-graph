use super::*;

const TOLERANCE: f64 = 1e-9;

fn wave(amplitude: f64, frequency: f64, phase: f64) -> Wave {
    Wave { amplitude, frequency, phase }
}

fn lines(items: &[PlotItem]) -> Vec<&PlotLine> {
    items
        .iter()
        .filter_map(|item| if let PlotItem::Line(line) = item { Some(line) } else { None })
        .collect()
}

fn labels(items: &[PlotItem]) -> Vec<&PlotLabel> {
    items
        .iter()
        .filter_map(|item| if let PlotItem::Label(label) = item { Some(label) } else { None })
        .collect()
}

/// The wave that `items` draws: the one line that is not dotted.
fn drawn(items: &[PlotItem]) -> &PlotLine {
    lines(items).into_iter().find(|line| line.dash.is_none()).expect("a wave")
}

/// How many times the points of `line` turn round: the tops and the bottoms of the wave that are not at its ends.
fn turns(line: &PlotLine) -> usize {
    let rises: Vec<bool> = line.points.windows(2).map(|pair| pair[1].y > pair[0].y).collect();
    rises.windows(2).filter(|pair| pair[0] != pair[1]).count()
}

#[test]
fn the_scene_is_800_wide_and_400_high_with_a_strip_for_the_sliders_at_the_top() {
    let config = config(&Wave::first());

    assert_eq!((config.size.width, config.size.height), (800.0, 400.0));
    assert_eq!(config.insets, Some(Insets::new(LEFT_MARGIN, STRIP, END_MARGIN, 0.0)));
    config.validate().expect("the configuration can be drawn");
}

#[test]
fn time_runs_from_0_to_1_second_whatever_the_wave() {
    for wave in [wave(1.0, 1.0, 0.0), wave(0.5, 8.0, 270.0), Wave::first()] {
        let config = config(&wave);
        assert_eq!((config.x.min, config.x.max), (0.0, 1.0));
    }
}

#[test]
fn the_time_axis_is_ticked_every_tenth_of_a_second_and_written_in_seconds() {
    let config = config(&Wave::first());
    let TickLabels::Custom(ticks) = &config.x.tick_labels else {
        panic!("the ticks are given")
    };

    assert_eq!(ticks.len(), 10);
    assert_eq!(ticks[0], (0.1, "0.1 s".to_owned()));
    assert_eq!(ticks[9], (1.0, "1.0 s".to_owned()));
    assert!(ticks.windows(2).all(|pair| pair[1].0 > pair[0].0));
}

#[test]
fn the_height_axis_is_the_one_of_the_circle_and_stops_its_labels_at_1() {
    let config = config(&Wave::first());

    assert_eq!((config.y.min, config.y.max), (-1.2, 1.2));
    assert_eq!(config.y.position, AxisPosition::AtMinimum);
    let TickLabels::Custom(ticks) = &config.y.tick_labels else {
        panic!("the ticks are given")
    };
    assert!(ticks.iter().all(|(value, _)| value.abs() <= 1.0 + TOLERANCE));
}

#[test]
fn the_sliders_take_the_ranges_that_were_asked_for() {
    assert_eq!(AMPLITUDES, (0.0, 1.0));
    assert_eq!(FREQUENCIES, (1.0, 8.0));
    assert_eq!(PHASES, (0.0, 360.0));
}

#[test]
fn the_first_wave_is_within_every_range() {
    let first = Wave::first();

    assert!((AMPLITUDES.0..=AMPLITUDES.1).contains(&first.amplitude));
    assert!((FREQUENCIES.0..=FREQUENCIES.1).contains(&first.frequency));
    assert!((PHASES.0..=PHASES.1).contains(&first.phase));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_wave_is_the_amplitude_times_the_sine_of_the_frequency_and_the_time_and_the_phase() {
    assert!(wave(1.0, 1.0, 0.0).at(0.0).abs() < TOLERANCE);
    assert!(
        (wave(1.0, 1.0, 0.0).at(0.25) - 1.0).abs() < TOLERANCE,
        "a crest a quarter of a cycle in"
    );
    assert!((wave(0.5, 1.0, 0.0).at(0.25) - 0.5).abs() < TOLERANCE, "half as tall");
    assert!((wave(1.0, 2.0, 0.0).at(0.125) - 1.0).abs() < TOLERANCE, "twice as fast");
    assert!(
        (wave(1.0, 1.0, 90.0).at(0.0) - 1.0).abs() < TOLERANCE,
        "90 degrees on, it starts at its crest"
    );
    assert!((wave(1.0, 1.0, 180.0).at(0.25) + 1.0).abs() < TOLERANCE, "upside down");
}

#[test]
fn the_wave_never_goes_beyond_the_amplitude() {
    for (a, f, p) in [(1.0, 8.0, 0.0), (0.37, 3.3, 123.0), (0.8, 1.0, 359.0), (0.0, 5.0, 10.0)] {
        let wave = wave(a, f, p);
        for line in lines(&items(&wave)).into_iter().filter(|line| line.dash.is_none()) {
            assert!(line.points.iter().all(|point| point.y.abs() <= a + TOLERANCE), "{wave:?}");
        }
    }
}

#[test]
fn a_wave_of_no_amplitude_is_flat_on_the_axis() {
    let items = items(&wave(0.0, 4.0, 90.0));

    assert!(drawn(&items).points.iter().all(|point| point.y == 0.0));
    assert_eq!(turns(drawn(&items)), 0);
}

#[test]
fn a_wave_of_one_whole_cycle_has_two_turns_and_every_cycle_adds_two() {
    for cycles in 1..=8 {
        let items = items(&wave(1.0, f64::from(cycles), 90.0));
        // A phase of 90 degrees starts it at a crest, so one cycle goes down to a trough and up again: one turn.
        assert_eq!(turns(drawn(&items)), 2 * cycles as usize - 1, "{cycles} cycles");
    }
}

#[test]
fn the_wave_has_the_same_number_of_points_whatever_it_is() {
    for wave in [wave(1.0, 1.0, 0.0), wave(0.2, 8.0, 300.0), wave(0.0, 3.0, 0.0)] {
        let items = items(&wave);
        let points = &drawn(&items).points;
        assert_eq!(points.len(), 513);
        assert_eq!(points[0].x, 0.0);
        assert_eq!(points[512].x, 1.0);
    }
}

#[test]
fn a_higher_frequency_has_a_shorter_cycle() {
    let (slow, quick) = (wave(1.0, 1.0, 0.0), wave(1.0, 4.0, 0.0));

    assert!((slow.at(1.0) - slow.at(0.0)).abs() < TOLERANCE, "one cycle fits in a second");
    assert!((quick.at(0.25) - quick.at(0.0)).abs() < TOLERANCE, "four cycles fit in it");
}

#[test]
fn a_phase_moves_the_wave_to_the_left() {
    let (plain, shifted) = (wave(1.0, 1.0, 0.0), wave(1.0, 1.0, 90.0));

    // The shifted wave is where the plain one will be a quarter of a cycle later.
    assert!((shifted.at(0.0) - plain.at(0.25)).abs() < TOLERANCE);
    assert!((shifted.at(0.5) - plain.at(0.75)).abs() < TOLERANCE);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn dotted_lines_mark_the_top_and_the_bottom_of_the_wave() {
    let items = items(&wave(0.6, 2.0, 0.0));
    let dotted: Vec<&PlotLine> = lines(&items).into_iter().filter(|line| line.dash.is_some()).collect();

    assert_eq!(dotted.len(), 2);
    let heights: Vec<f64> = dotted.iter().map(|line| line.points[0].y).collect();
    assert_eq!(heights, vec![0.6, -0.6]);
    for line in dotted {
        assert_eq!((line.points[0].x, line.points[1].x), (0.0, 1.0), "all the way across");
        assert_eq!(line.points[0].y, line.points[1].y);
    }
}

#[test]
fn a_wave_of_no_amplitude_has_no_dotted_lines_since_they_would_be_on_the_axis() {
    let items = items(&wave(0.0, 2.0, 0.0));

    assert!(lines(&items).iter().all(|line| line.dash.is_none()));
}

#[test]
fn the_wave_is_not_drawn_in_the_colour_of_the_dotted_lines() {
    let items = items(&Wave::first());

    assert_eq!(drawn(&items).colour, HEIGHT);
    assert!(
        lines(&items)
            .iter()
            .filter(|line| line.dash.is_some())
            .all(|line| line.colour == GUIDE)
    );
    assert_ne!(HEIGHT, GUIDE);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_values_are_written_as_they_are_beside_the_sliders() {
    assert_eq!(frequency_text(2.0), "2.0 Hz");
    assert_eq!(frequency_text(7.5), "7.5 Hz");
    assert_eq!(amplitude_text(1.0), "1.00");
    assert_eq!(amplitude_text(0.8), "0.80");
    assert_eq!(phase_text(0.0), "0\u{b0}");
    assert_eq!(phase_text(359.0), "359\u{b0}");
}

#[test]
fn the_formula_has_the_three_numbers_of_the_wave_in_it() {
    assert_eq!(
        formula(&Wave::first()),
        "y = 1.00 \u{b7} sin(2\u{3c0} \u{b7} 2.0 \u{b7} t + 0\u{b0})"
    );
    assert_eq!(
        formula(&wave(0.8, 3.5, 90.0)),
        "y = 0.80 \u{b7} sin(2\u{3c0} \u{b7} 3.5 \u{b7} t + 90\u{b0})"
    );
}

#[test]
fn the_formula_is_written_in_the_plot_below_where_the_wave_ever_goes() {
    let items = items(&wave(1.0, 3.0, 40.0));
    let formulas: Vec<&PlotLabel> = labels(&items);
    let up = (SIZE.1 - STRIP) / 2.4;

    assert_eq!(formulas.len(), 1);
    let label = formulas[0];
    assert_eq!(label.anchor, LabelAnchor::Start);
    assert_eq!(label.at.x, 0.0, "at the left");
    let top = label.at.y - label.offset.1 / up + 0.4 * TITLE_SIZE / up;
    let bottom = label.at.y - label.offset.1 / up - 0.4 * TITLE_SIZE / up;
    assert!(top < -1.0 - 0.02, "below the lowest that a wave of amplitude 1 goes: {top}");
    assert!(bottom > -1.2, "inside the plot: {bottom}");
}

#[test]
fn the_formula_is_plain_text() {
    for wave in [Wave::first(), wave(0.5, 7.7, 359.0)] {
        assert!(!formula(&wave).contains(['<', '>', '&']));
    }
}

#[test]
fn what_assistive_technology_reads_has_the_three_numbers_in_it() {
    let text = description(&wave(0.8, 3.5, 90.0));

    assert!(
        text.contains("0.80") && text.contains("3.5 Hz") && text.contains("90 degrees"),
        "{text}"
    );
    assert!(config(&Wave::first()).description.is_some_and(|text| text.contains("2.0 Hz")));
}

#[test]
fn every_wave_draws_the_same_items_but_the_dotted_lines() {
    for wave in [wave(1.0, 1.0, 0.0), wave(0.3, 6.0, 200.0)] {
        assert_eq!(items(&wave).len(), 4, "two dotted lines, the wave and the formula");
    }
    assert_eq!(items(&wave(0.0, 1.0, 0.0)).len(), 2, "the wave and the formula");
}
