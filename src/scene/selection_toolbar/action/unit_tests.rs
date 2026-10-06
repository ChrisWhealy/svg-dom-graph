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
    check_eq(SelectionToolbarAction::Next.next_position(None, 4, 1), Some(Some(0)))
}

#[test]
fn next_from_unstarted_on_an_empty_node_is_disabled() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.next_position(None, 0, 1), None)
}

#[test]
fn next_from_the_last_element_is_disabled() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.next_position(Some(3), 4, 1), None)
}

#[test]
fn next_from_a_middle_element_advances_by_one() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.next_position(Some(1), 4, 1), Some(Some(2)))
}

#[test]
fn prev_from_element_zero_returns_to_unstarted() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Prev.next_position(Some(0), 4, 1), Some(None))
}

#[test]
fn prev_from_a_middle_element_steps_back_by_one() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Prev.next_position(Some(2), 4, 1), Some(Some(1)))
}

#[test]
fn prev_from_unstarted_is_disabled() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Prev.next_position(None, 4, 1), None)
}

#[test]
fn restart_from_any_started_position_returns_to_unstarted() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Restart.next_position(Some(0), 4, 1), Some(None))?;
    check_eq(SelectionToolbarAction::Restart.next_position(Some(3), 4, 1), Some(None))
}

#[test]
fn restart_from_unstarted_is_disabled() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Restart.next_position(None, 4, 1), None)
}

#[test]
fn is_enabled_agrees_with_next_position() -> Result<(), String> {
    check_eq(SelectionToolbarAction::Next.is_enabled(Some(3), 4, 1), false)?;
    check_eq(SelectionToolbarAction::Next.is_enabled(Some(2), 4, 1), true)?;
    check_eq(SelectionToolbarAction::Prev.is_enabled(None, 4, 1), false)?;
    check_eq(SelectionToolbarAction::Prev.is_enabled(Some(0), 4, 1), true)?;
    check_eq(SelectionToolbarAction::Restart.is_enabled(None, 4, 1), false)?;
    check_eq(SelectionToolbarAction::Restart.is_enabled(Some(0), 4, 1), true)
}

#[test]
fn next_stride_moves_a_whole_group_and_clamps_at_the_end() -> Result<(), String> {
    check_eq(SelectionToolbarAction::NextStride.next_position(Some(2), 20, 5), Some(Some(7)))?;
    check_eq(
        SelectionToolbarAction::NextStride.next_position(Some(17), 20, 5),
        Some(Some(19)),
    )?;
    check_eq(SelectionToolbarAction::NextStride.next_position(Some(19), 20, 5), None)
}

#[test]
fn next_stride_from_unstarted_lands_on_the_end_of_the_first_group() -> Result<(), String> {
    check_eq(SelectionToolbarAction::NextStride.next_position(None, 20, 5), Some(Some(4)))?;
    check_eq(SelectionToolbarAction::NextStride.next_position(None, 0, 5), None)
}

#[test]
fn prev_stride_moves_back_a_group_clamps_at_zero_and_never_goes_unstarted() -> Result<(), String> {
    check_eq(SelectionToolbarAction::PrevStride.next_position(Some(7), 20, 5), Some(Some(2)))?;
    check_eq(SelectionToolbarAction::PrevStride.next_position(Some(2), 20, 5), Some(Some(0)))?;
    check_eq(SelectionToolbarAction::PrevStride.next_position(Some(0), 20, 5), None)?;
    check_eq(SelectionToolbarAction::PrevStride.next_position(None, 20, 5), None)
}

#[test]
fn the_toolbar_holds_five_buttons_with_a_stride_and_three_without() -> Result<(), String> {
    check_eq(SelectionToolbarAction::buttons(false).len(), 3)?;
    check_eq(
        SelectionToolbarAction::buttons(true),
        vec![
            SelectionToolbarAction::PrevStride,
            SelectionToolbarAction::Prev,
            SelectionToolbarAction::Next,
            SelectionToolbarAction::NextStride,
            SelectionToolbarAction::Restart,
        ],
    )?;
    check_eq(SelectionToolbarAction::NextStride.label("Round"), "Next Round".to_owned())?;
    check_eq(
        SelectionToolbarAction::PrevStride.aria_label("Round"),
        "Previous round".to_owned(),
    )
}
