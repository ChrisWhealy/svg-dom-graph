// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a selection toolbar button does when activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionToolbarAction {
    Prev,
    Next,
    Restart,
}

impl SelectionToolbarAction {
    /// The buttons a selection toolbar holds, in display order.
    pub(super) const ALL: [Self; 3] = [Self::Prev, Self::Next, Self::Restart];

    /// The visible word on the button.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Prev => "Prev",
            Self::Next => "Next",
            Self::Restart => "Restart",
        }
    }

    /// The button's accessible name — spelled out, since "Prev" is not a word assistive technology should have to
    /// guess the meaning of.
    pub(super) fn aria_label(self) -> &'static str {
        match self {
            Self::Prev => "Previous selection",
            Self::Next => "Next selection",
            Self::Restart => "Restart selection",
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The flat position this action moves to from `current`, for a data node holding `len` values — `None` if
    /// activating it right now would change nothing, which is also this button's own disabled/enabled test (see
    /// [`is_enabled`](Self::is_enabled)).
    ///
    /// `current` is `None` for the unstarted state — before element `0` has ever been processed, or after `Prev`
    /// or `Restart` has walked back to it — never a separate cursor kept alongside the node's own `Selection`; see
    /// [`crate::scene::Scene::show_selection_toolbar`]'s own doc comment for why there is only ever this one value.
    ///
    /// - [`Prev`](Self::Prev)/[`Restart`](Self::Restart) never move past the unstarted state — there is nothing
    ///   before it to walk back to, so both are disabled once `current` is already `None`.
    /// - [`Next`](Self::Next) never moves past the last element, and is disabled outright for an empty node
    ///   (`len == 0`), which also holds `current` at `None` forever.
    pub(super) fn next_position(self, current: Option<usize>, len: usize) -> Option<Option<usize>> {
        match self {
            Self::Next => match current {
                None if len > 0 => Some(Some(0)),
                Some(i) if i + 1 < len => Some(Some(i + 1)),
                _ => None,
            },
            Self::Prev => match current {
                Some(0) => Some(None),
                Some(i) => Some(Some(i - 1)),
                None => None,
            },
            Self::Restart => current.map(|_| None),
        }
    }

    /// Whether activating this action right now would change anything — see
    /// [`next_position`](Self::next_position), whose `None` this is exactly the test for.
    pub(super) fn is_enabled(self, current: Option<usize>, len: usize) -> bool {
        self.next_position(current, len).is_some()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
