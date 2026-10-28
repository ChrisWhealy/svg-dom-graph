use super::{Cursor, Outcome, apply_pair, apply_transition, retry, selection_of, shown_stage, stage_of};
use crate::fft::algorithm::{FftTrace, Preset, trace};

fn run(preset: Preset) -> FftTrace {
    trace(&preset.samples(8)).expect("a valid input")
}

/// A display that records what it was asked to draw, and fails when told to.
#[derive(Default)]
struct Display {
    drawn: Vec<(usize, usize)>,
    driver: Vec<usize>,
    fail_renders: usize,
    fail_driver: bool,
}

impl Display {
    fn render(&mut self, completed: usize, pair: usize) -> Result<(), String> {
        if self.fail_renders > 0 {
            self.fail_renders -= 1;
            return Err(format!("render {completed}/{pair} failed"));
        }
        self.drawn.push((completed, pair));
        Ok(())
    }

    fn set_driver(&mut self, completed: usize) -> Result<(), String> {
        if self.fail_driver {
            return Err("driver failed".to_owned());
        }
        self.driver.push(completed);
        Ok(())
    }
}

fn step(
    cursor: &mut Cursor,
    trace: &FftTrace,
    display: &mut Display,
    from: Option<usize>,
    to: Option<usize>,
) -> Outcome {
    let display = std::cell::RefCell::new(display);
    apply_transition(
        cursor,
        trace,
        from,
        to,
        &mut |c, p| display.borrow_mut().render(c, p),
        &mut |c| display.borrow_mut().set_driver(c),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_toolbar_selection_and_the_completed_stages_map_both_ways() {
    assert_eq!(stage_of(None), 0);
    assert_eq!(stage_of(Some(0)), 1);
    assert_eq!(stage_of(Some(2)), 3);
    assert_eq!(selection_of(0), None);
    assert_eq!(selection_of(3), Some(2));
    for completed in 0..=4 {
        assert_eq!(stage_of(selection_of(completed)), completed);
    }
}

#[test]
fn before_any_stage_is_complete_the_inspector_previews_stage_one() {
    assert_eq!(shown_stage(0), 1);
    assert_eq!(shown_stage(1), 1);
    assert_eq!(shown_stage(3), 3);
}

#[test]
fn a_new_cursor_is_unstarted_and_does_not_need_repair() {
    let cursor = Cursor::new(3);
    assert_eq!((cursor.completed(), cursor.pair(), cursor.needs_repair()), (0, 0, false));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_default_pair_follows_the_first_nonzero_rule_at_every_stage() {
    let trace = run(Preset::Impulse);
    let cursor = Cursor::new(3);
    assert_eq!(
        cursor.pair_for(&trace, 0),
        2,
        "the preview is stage 1, whose impulse is in lanes 4 and 5"
    );
    assert_eq!(cursor.pair_for(&trace, 1), 2);
    assert_eq!(cursor.pair_for(&trace, 2), 2);
    assert_eq!(cursor.pair_for(&trace, 3), 0);
}

#[test]
fn a_pair_chosen_at_a_stage_is_remembered_there_and_nowhere_else() {
    let trace = run(Preset::Mixed);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    let display = std::cell::RefCell::new(display);
    assert_eq!(
        apply_pair(&mut cursor, 3, &mut |c, p| display.borrow_mut().render(c, p)),
        Outcome::Committed
    );
    assert_eq!(cursor.pair_for(&trace, 1), 3);
    assert_eq!(cursor.pair_for(&trace, 2), 0, "stage 2 still has its own default");
    // Stepping to stage 2 and back finds the choice again, and also at the unstarted preview, which is stage 1.
    assert_eq!(cursor.pair_for(&trace, 0), 3);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn stepping_forward_draws_the_next_stage_and_moves_the_cursor() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    assert_eq!(step(&mut cursor, &trace, &mut display, None, Some(0)), Outcome::Committed);
    assert_eq!(step(&mut cursor, &trace, &mut display, Some(0), Some(1)), Outcome::Committed);
    assert_eq!(cursor.completed(), 2);
    assert_eq!(display.drawn, [(1, 2), (2, 2)]);
    assert!(display.driver.is_empty(), "the toolbar already moved, so it is left alone");
}

#[test]
fn stepping_back_to_the_start_is_unstarted() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    assert_eq!(step(&mut cursor, &trace, &mut display, Some(0), None), Outcome::Committed);
    assert_eq!(cursor.completed(), 0);
    assert_eq!(display.drawn.last(), Some(&(0, 2)));
}

#[test]
fn a_failed_step_puts_the_toolbar_back_and_draws_the_last_good_state_again() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    display.fail_renders = 1;
    let outcome = step(&mut cursor, &trace, &mut display, Some(0), Some(1));
    assert_eq!(outcome, Outcome::RolledBack("render 2/2 failed".to_owned()));
    assert_eq!(cursor.completed(), 1, "the cursor never moved");
    assert_eq!(display.driver, [1], "the toolbar went back to stage 1");
    assert_eq!(display.drawn.last(), Some(&(1, 2)), "and the stage-1 display was drawn again");
    assert!(!cursor.needs_repair());
}

#[test]
fn after_a_rollback_the_same_step_works() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    display.fail_renders = 1;
    step(&mut cursor, &trace, &mut display, Some(0), Some(1));
    assert_eq!(step(&mut cursor, &trace, &mut display, Some(0), Some(1)), Outcome::Committed);
    assert_eq!(cursor.completed(), 2);
}

#[test]
fn a_failed_step_whose_undo_also_fails_needs_repair_and_refuses_more() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    display.fail_renders = 2;
    let outcome = step(&mut cursor, &trace, &mut display, Some(0), Some(1));
    assert!(matches!(outcome, Outcome::NeedsRepair(ref message) if message.contains("then")));
    assert!(cursor.needs_repair());
    assert_eq!(cursor.completed(), 1, "the last good state is still the logical state");

    assert_eq!(step(&mut cursor, &trace, &mut display, Some(0), Some(1)), Outcome::Refused);
    assert_eq!(cursor.completed(), 1);
    let display = std::cell::RefCell::new(display);
    assert_eq!(
        apply_pair(&mut cursor, 1, &mut |c, p| display.borrow_mut().render(c, p)),
        Outcome::Refused
    );
}

#[test]
fn a_toolbar_that_cannot_be_put_back_needs_repair() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    display.fail_renders = 1;
    display.fail_driver = true;
    assert!(matches!(
        step(&mut cursor, &trace, &mut display, Some(0), Some(1)),
        Outcome::NeedsRepair(_)
    ));
    assert!(cursor.needs_repair());
}

#[test]
fn a_retry_draws_the_last_good_state_and_ends_the_repair() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    display.fail_renders = 2;
    step(&mut cursor, &trace, &mut display, Some(0), Some(1));
    assert!(cursor.needs_repair());

    let display = std::cell::RefCell::new(display);
    let result = retry(&mut cursor, &mut |c, p| display.borrow_mut().render(c, p), &mut |c| {
        display.borrow_mut().set_driver(c)
    });
    assert_eq!(result, Ok(()));
    assert!(!cursor.needs_repair());
    assert_eq!(display.borrow().drawn.last(), Some(&(1, 2)));
    assert_eq!(display.borrow().driver.last(), Some(&1));
}

#[test]
fn a_retry_that_fails_leaves_the_repair_outstanding() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    display.fail_renders = 3;
    step(&mut cursor, &trace, &mut display, Some(0), Some(1));
    let display = std::cell::RefCell::new(display);
    let result = retry(&mut cursor, &mut |c, p| display.borrow_mut().render(c, p), &mut |c| {
        display.borrow_mut().set_driver(c)
    });
    assert!(result.is_err());
    assert!(cursor.needs_repair());
}

#[test]
fn a_callback_that_starts_from_the_wrong_place_is_not_applied() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    // The cursor says stage 1, but the toolbar reports a move from stage 3.
    let outcome = step(&mut cursor, &trace, &mut display, Some(2), None);
    assert_eq!(outcome, Outcome::Resynchronised);
    assert_eq!(cursor.completed(), 1, "no stage was skipped or applied");
    assert_eq!(display.driver, [1]);
    assert_eq!(display.drawn.last(), Some(&(1, 2)));
}

#[test]
fn a_step_beyond_the_last_stage_is_held_at_the_last_stage() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    assert_eq!(step(&mut cursor, &trace, &mut display, Some(0), Some(9)), Outcome::Committed);
    assert_eq!(cursor.completed(), 3);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_failed_pair_change_draws_the_old_pair_again() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    display.fail_renders = 1;
    let display = std::cell::RefCell::new(display);
    let outcome = apply_pair(&mut cursor, 3, &mut |c, p| display.borrow_mut().render(c, p));
    assert_eq!(outcome, Outcome::RolledBack("render 1/3 failed".to_owned()));
    assert_eq!(cursor.pair(), 2, "the committed pair is unchanged");
    assert_eq!(cursor.pair_for(&trace, 1), 2, "and the failed choice is not cached");
    assert_eq!(display.borrow().drawn.last(), Some(&(1, 2)));
}

#[test]
fn a_pair_change_that_cannot_be_undone_needs_repair() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    display.fail_renders = 2;
    let display = std::cell::RefCell::new(display);
    assert!(matches!(
        apply_pair(&mut cursor, 3, &mut |c, p| display.borrow_mut().render(c, p)),
        Outcome::NeedsRepair(_)
    ));
    assert!(cursor.needs_repair());
}

#[test]
fn a_pair_chosen_before_the_first_step_is_the_preview_pair() {
    let trace = run(Preset::Mixed);
    let mut cursor = Cursor::new(3);
    let display = std::cell::RefCell::new(Display::default());
    assert_eq!(
        apply_pair(&mut cursor, 2, &mut |c, p| display.borrow_mut().render(c, p)),
        Outcome::Committed
    );
    assert_eq!(display.borrow().drawn, [(0, 2)]);
    // Stepping to stage 1 shows the pair that was previewed.
    let mut display = display.into_inner();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    assert_eq!(display.drawn.last(), Some(&(1, 2)));
}

#[test]
fn forgetting_the_choices_keeps_the_position() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::new(3);
    let display = std::cell::RefCell::new(Display::default());
    apply_pair(&mut cursor, 0, &mut |c, p| display.borrow_mut().render(c, p));
    assert_eq!(cursor.pair_for(&trace, 0), 0);
    cursor.forget_choices();
    assert_eq!(cursor.pair_for(&trace, 0), 2);
    assert_eq!((cursor.completed(), cursor.pair()), (0, 0));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The first butterfly is not the first pair
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The impulse at `x[1]` sits in lane 4 once the input is in bit-reversed order, so the first butterfly with a nonzero
/// input is pair 2, lanes 4 and 5, and not pair 0.
#[test]
fn an_unstarted_cursor_starts_on_the_pair_that_is_drawn_first_and_remembers_it_for_stage_one() {
    let trace = run(Preset::Impulse);
    assert_ne!(
        Cursor::new(trace.stages()).pair_for(&trace, 0),
        0,
        "the default is not pair 0 for this signal"
    );

    let cursor = Cursor::unstarted(&trace);
    assert_eq!((cursor.completed(), cursor.pair(), cursor.needs_repair()), (0, 2, false));
    assert_eq!(cursor.pair_for(&trace, 0), 2);
    assert_eq!(
        cursor.pair_for(&trace, 1),
        2,
        "stage 1 is the stage that is previewed, and its pair is remembered"
    );
}

#[test]
fn a_failed_first_step_puts_back_the_pair_that_was_drawn_and_not_pair_zero() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::unstarted(&trace);
    let mut display = Display {
        fail_renders: 2,
        ..Display::default()
    };

    // The step fails, and so does drawing the unstarted state again, so a repair is outstanding.
    let outcome = step(&mut cursor, &trace, &mut display, None, Some(0));
    assert!(matches!(outcome, Outcome::NeedsRepair(_)), "{outcome:?}");
    assert!(cursor.needs_repair());
    assert_eq!((cursor.completed(), cursor.pair()), (0, 2));

    // A retry draws the last good state, which is the pair that was on show.
    let mut display = Display::default();
    let display = std::cell::RefCell::new(&mut display);
    retry(&mut cursor, &mut |c, p| display.borrow_mut().render(c, p), &mut |c| {
        display.borrow_mut().set_driver(c)
    })
    .expect("the retry is drawn");
    assert_eq!(display.borrow().drawn, vec![(0, 2)]);
}

#[test]
fn a_rolled_back_step_draws_the_pair_that_was_shown_first() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::unstarted(&trace);
    let mut display = Display {
        fail_renders: 1,
        ..Display::default()
    };

    let outcome = step(&mut cursor, &trace, &mut display, None, Some(0));
    assert!(matches!(outcome, Outcome::RolledBack(_)), "{outcome:?}");
    assert_eq!(display.drawn, vec![(0, 2)], "the old display is drawn again with its own pair");
}

#[test]
fn forgetting_the_choices_returns_to_the_default_pair() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::unstarted(&trace);
    cursor.forget_choices();
    assert_eq!(cursor.pair_for(&trace, 0), 2, "the default is found again, and is the same");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   A restart forgets the chosen pairs only once it is drawn
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A cursor at stage 2 whose stage 1 has a chosen pair that is not the default one, and the display it drew on.
fn at_stage_two_with_a_choice_at_stage_one(trace: &FftTrace) -> (Cursor, Display) {
    let mut cursor = Cursor::unstarted(trace);
    let mut display = Display::default();
    step(&mut cursor, trace, &mut display, None, Some(0));
    let shown = std::cell::RefCell::new(&mut display);
    // The default pair at stage 1 is 2, so choosing 0 is a choice.
    apply_pair(&mut cursor, 0, &mut |c, p| shown.borrow_mut().render(c, p));
    step(&mut cursor, trace, &mut display, Some(0), Some(1));
    assert_eq!((cursor.completed(), cursor.pair_for(trace, 1)), (2, 0), "set up");
    (cursor, display)
}

#[test]
fn a_restart_that_fails_leaves_the_pair_chosen_at_stage_one() {
    let trace = run(Preset::Impulse);
    let (mut cursor, mut display) = at_stage_two_with_a_choice_at_stage_one(&trace);
    display.fail_renders = 1;

    let outcome = step(&mut cursor, &trace, &mut display, Some(1), None);

    assert!(matches!(outcome, Outcome::RolledBack(_)), "{outcome:?}");
    assert_eq!(cursor.completed(), 2, "the display is where it was");
    assert_eq!(cursor.pair_for(&trace, 1), 0, "and the choice at stage 1 was not forgotten");
    // Stepping back to stage 1 afterwards finds it.
    step(&mut cursor, &trace, &mut display, Some(1), Some(0));
    assert_eq!(display.drawn.last(), Some(&(1, 0)));
}

#[test]
fn a_restart_that_fails_and_cannot_be_undone_still_keeps_the_choice_for_the_retry() {
    let trace = run(Preset::Impulse);
    let (mut cursor, mut display) = at_stage_two_with_a_choice_at_stage_one(&trace);
    display.fail_renders = 2;

    let outcome = step(&mut cursor, &trace, &mut display, Some(1), None);

    assert!(matches!(outcome, Outcome::NeedsRepair(_)), "{outcome:?}");
    assert_eq!(cursor.pair_for(&trace, 1), 0);
}

#[test]
fn a_move_to_unstarted_that_is_out_of_step_is_only_resynchronised_and_forgets_nothing() {
    let trace = run(Preset::Impulse);
    let (mut cursor, mut display) = at_stage_two_with_a_choice_at_stage_one(&trace);

    // The toolbar says it was at stage 3, but the display is at stage 2.
    let outcome = step(&mut cursor, &trace, &mut display, Some(2), None);

    assert_eq!(outcome, Outcome::Resynchronised);
    assert_eq!(cursor.pair_for(&trace, 1), 0);
}

#[test]
fn a_restart_that_succeeds_draws_the_default_pair_and_forgets_every_choice() {
    let trace = run(Preset::Impulse);
    let (mut cursor, mut display) = at_stage_two_with_a_choice_at_stage_one(&trace);

    let outcome = step(&mut cursor, &trace, &mut display, Some(1), None);

    assert_eq!(outcome, Outcome::Committed);
    assert_eq!(display.drawn.last(), Some(&(0, 2)), "the default pair, whatever was chosen");
    assert_eq!((cursor.completed(), cursor.pair()), (0, 2));
    assert_eq!(cursor.pair_for(&trace, 1), 2, "the choice at stage 1 is forgotten");
}

#[test]
fn a_step_back_from_the_first_stage_is_a_restart_and_forgets_the_choices_when_it_is_drawn() {
    let trace = run(Preset::Impulse);
    let mut cursor = Cursor::unstarted(&trace);
    let mut display = Display::default();
    step(&mut cursor, &trace, &mut display, None, Some(0));
    {
        let shown = std::cell::RefCell::new(&mut display);
        apply_pair(&mut cursor, 1, &mut |c, p| shown.borrow_mut().render(c, p));
    }
    display.fail_renders = 1;

    let failed = step(&mut cursor, &trace, &mut display, Some(0), None);
    assert!(matches!(failed, Outcome::RolledBack(_)));
    assert_eq!(
        (cursor.completed(), cursor.pair()),
        (1, 1),
        "still on stage 1, and the pair chosen"
    );

    let drawn = step(&mut cursor, &trace, &mut display, Some(0), None);
    assert_eq!(drawn, Outcome::Committed);
    assert_eq!((cursor.completed(), cursor.pair()), (0, 2));
}
