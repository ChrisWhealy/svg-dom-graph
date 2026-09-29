use super::SelectionToolbarAction;

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

#[test]
fn next_from_unstarted_on_a_non_empty_node_goes_to_element_zero() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.next_position(None, 4), Some(Some(0)))
}

#[test]
fn next_from_unstarted_on_an_empty_node_is_disabled() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.next_position(None, 0), None)
}

#[test]
fn next_from_the_last_element_is_disabled() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.next_position(Some(3), 4), None)
}

#[test]
fn next_from_a_middle_element_advances_by_one() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.next_position(Some(1), 4), Some(Some(2)))
}

#[test]
fn prev_from_element_zero_returns_to_unstarted() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Prev.next_position(Some(0), 4), Some(None))
}

#[test]
fn prev_from_a_middle_element_steps_back_by_one() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Prev.next_position(Some(2), 4), Some(Some(1)))
}

#[test]
fn prev_from_unstarted_is_disabled() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Prev.next_position(None, 4), None)
}

#[test]
fn restart_from_any_started_position_returns_to_unstarted() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Restart.next_position(Some(0), 4), Some(None))?;
    check_eq(SelectionToolbarAction::Restart.next_position(Some(3), 4), Some(None))
}

#[test]
fn restart_from_unstarted_is_disabled() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Restart.next_position(None, 4), None)
}

#[test]
fn is_enabled_agrees_with_next_position() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.is_enabled(Some(3), 4), false)?;
    check_eq(SelectionToolbarAction::Next.is_enabled(Some(2), 4), true)?;
    check_eq(SelectionToolbarAction::Prev.is_enabled(None, 4), false)?;
    check_eq(SelectionToolbarAction::Prev.is_enabled(Some(0), 4), true)?;
    check_eq(SelectionToolbarAction::Restart.is_enabled(None, 4), false)?;
    check_eq(SelectionToolbarAction::Restart.is_enabled(Some(0), 4), true)
}
