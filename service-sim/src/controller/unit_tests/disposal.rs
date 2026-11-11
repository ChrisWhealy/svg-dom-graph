//! A controller that has been disposed, and callbacks that outlive it.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_disposed_controller_draws_nothing_and_late_callbacks_are_inert() {
    let (c, log) = start(e0());
    c.scheduler().keep_cancelled(true);
    c.set_speed(10);
    c.play();
    c.seek(5);
    c.dispose();
    let draws = log.borrow().draws.len();
    c.scheduler().advance_by(1_000);
    run_frames(&c);
    assert_eq!(log.borrow().draws.len(), draws);
    assert_eq!(c.committed_slice(), 0);
    assert!(!c.is_playing());
    assert_eq!(c.next(), Outcome::Unavailable);
    assert_eq!(c.play(), Outcome::Unavailable);
}

#[test]
fn scheduled_callbacks_hold_no_strong_reference_to_the_controller() {
    let (c, _) = start(e0());
    c.set_speed(10);
    c.play();
    assert_eq!(c.scheduler().held(), 1);
    assert_eq!(
        Rc::strong_count(&c.inner),
        1,
        "a timer is waiting, and it does not hold the controller"
    );
    // A seek takes the timer back and waits for a frame, which does not hold the controller either.
    c.seek(4);
    assert_eq!((c.scheduler().held(), c.scheduler().pending_frames()), (1, 1));
    assert_eq!(
        Rc::strong_count(&c.inner),
        1,
        "a frame is waiting, and it does not hold the controller"
    );
    // Dropping the controller while its callback is held lets it go, and the callback finds nothing to act on.
    let probe = Rc::downgrade(&c.inner);
    drop(c);
    assert!(probe.upgrade().is_none(), "the controller was released");
}
