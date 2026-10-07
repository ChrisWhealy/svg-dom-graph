// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a selection toolbar button does when activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionToolbarAction {
    Prev,
    Next,
    Restart,
    /// Steps back by the toolbar's own [`super::SelectionStride`].
    PrevStride,
    /// Steps forward by the toolbar's own [`super::SelectionStride`].
    NextStride,
}

impl SelectionToolbarAction {
    /// The buttons a selection toolbar holds, in display order: with a stride, `PrevStride` and `NextStride` go on
    /// either side of `Prev`/`Next`'s own pair; without one, just `Prev`, `Next` and `Restart`.
    pub(super) fn buttons(has_stride: bool) -> Vec<Self> {
        if has_stride {
            vec![Self::PrevStride, Self::Prev, Self::Next, Self::NextStride, Self::Restart]
        } else {
            vec![Self::Prev, Self::Next, Self::Restart]
        }
    }

    /// The visible word on the button. `stride_label` is the toolbar's own [`super::SelectionStride::label`].
    pub(super) fn label(self, stride_label: &str) -> String {
        match self {
            Self::Prev => "Prev".to_owned(),
            Self::Next => "Next".to_owned(),
            Self::Restart => "Restart".to_owned(),
            Self::PrevStride => format!("Prev {stride_label}"),
            Self::NextStride => format!("Next {stride_label}"),
        }
    }

    /// The button's accessible name — spelled out, since "Prev" is not a word assistive technology should have to guess
    /// the meaning of.
    pub(super) fn aria_label(self, stride_label: &str) -> String {
        match self {
            Self::Prev => "Previous selection".to_owned(),
            Self::Next => "Next selection".to_owned(),
            Self::Restart => "Restart selection".to_owned(),
            Self::PrevStride => format!("Previous {}", stride_label.to_lowercase()),
            Self::NextStride => format!("Next {}", stride_label.to_lowercase()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The flat position this action moves to from `current`, for a data node holding `len` values. It is `None` if
    /// activating it right now would change nothing. That is also this button's own disabled/enabled test (see
    /// [`is_enabled`](Self::is_enabled)).
    ///
    /// `current` is `None` for the unstarted state. That is before element `0` has ever been processed, or after `Prev`
    /// or `Restart` has walked back to it. It is never a separate cursor kept alongside the node's own `Selection`. See
    /// [`crate::scene::Scene::show_selection_toolbar`]'s own doc comment for why there is only ever this one value.
    ///
    /// - [`Prev`](Self::Prev)/[`Restart`](Self::Restart) never move past the unstarted state — there is nothing before
    ///   it to walk back to, so both are disabled once `current` is already `None`.
    /// - [`Next`](Self::Next) never moves past the last element, and is disabled outright for an empty node (`len ==
    ///   0`), which also holds `current` at `None` forever.
    pub(super) fn next_position(self, current: Option<usize>, len: usize, stride: usize) -> Option<Option<usize>> {
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
            // A stride moves `stride` cells, clamped to the first/last cell, and is disabled when that would not move
            // at all. Unlike `Prev`, `PrevStride` never goes back to the unstarted state. From unstarted, `NextStride`
            // moves as far as the first group's last cell, as if the walk stood just before cell `0`.
            Self::NextStride => match current {
                None if len > 0 => Some(Some((stride - 1).min(len - 1))),
                Some(i) if i + 1 < len => Some(Some((i + stride).min(len - 1))),
                _ => None,
            },
            Self::PrevStride => match current {
                Some(i) if i > 0 => Some(Some(i.saturating_sub(stride))),
                _ => None,
            },
        }
    }

    /// Whether activating this action right now would change anything — see [`next_position`](Self::next_position),
    /// whose `None` this is exactly the test for.
    pub(super) fn is_enabled(self, current: Option<usize>, len: usize, stride: usize) -> bool {
        self.next_position(current, len, stride).is_some()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
