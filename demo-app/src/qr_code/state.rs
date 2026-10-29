//! The spike's position, and what drawing it takes.
//!
//! [`State`] is the one place the position is kept, so that the matrix, the buttons and the text are all drawn from it.

use super::{
    ANNOUNCEMENT_ID, CHOSEN, Display, NEXT, NO_ROLE_EXPLANATION, NOT_CHOSEN, PREV, RESTART, caption_text,
    grid::{BitGrid, QrModuleKind},
    role_action,
    spike::SpikeStep,
    stats_text,
    viewport::Frame,
};
use demo_common::{svg_buttons, util::announce};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The one place the spike keeps its position, so that every display is drawn from it.
pub(super) struct State {
    pub(super) steps: Vec<SpikeStep>,
    /// `None` is the unstarted state, which shows an empty grid.
    pub(super) at: Option<usize>,
    pub(super) role: Option<QrModuleKind>,
    pub(super) empty: BitGrid,
    pub(super) display: Display,
}

impl State {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the whole of the current state, and brings the buttons and the text into line with it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the DOM refuses a write, or a text does not fit its block. The next call writes whatever is
    /// still out of date.
    pub(super) fn apply(&self) -> Result<(), String> {
        let display = &self.display;
        let (label, grid, highlights): (&str, &BitGrid, &[(usize, usize)]) = match self.at {
            None => ("Not started. Every module is empty.", &self.empty, &[]),
            Some(index) => {
                let step = &self.steps[index];
                (step.label, &step.grid, &step.highlights)
            },
        };
        let caption = caption_text(label, grid);
        let writes = display.viewport.render(&Frame {
            grid,
            highlights,
            role: self.role,
            summary: &caption,
        })?;

        let last = self.steps.len() - 1;
        svg_buttons::set_disabled(&display.layer, PREV, self.at.is_none())?;
        svg_buttons::set_disabled(&display.layer, RESTART, self.at.is_none())?;
        svg_buttons::set_disabled(&display.layer, NEXT, self.at == Some(last))?;
        for role in std::iter::once(None).chain(QrModuleKind::ALL.map(Some)) {
            let chosen = role == self.role;
            svg_buttons::set_pressed(
                &display.layer,
                &role_action(role),
                chosen,
                if chosen { CHOSEN } else { NOT_CHOSEN },
            )?;
        }

        let position = self
            .at
            .map_or("Not started".to_owned(), |index| format!("Step {} of {}", index + 1, last + 1));
        display.caption.set(&caption)?;
        display
            .stats
            .set(&stats_text(&position, display.viewport.element_count()?, writes))?;
        display.role_name.set(self.role.map_or("No role chosen", QrModuleKind::label))?;
        display
            .role_text
            .set(self.role.map_or(NO_ROLE_EXPLANATION, QrModuleKind::explanation))?;

        let outlined = self.role.map_or(String::new(), |role| {
            let count = grid.cells().filter(|&(row, col)| grid.role(row, col) == Some(role)).count();
            format!(" Outlining {count} {} modules.", role.label().to_lowercase())
        });
        // The unstarted label already says so, and saying it twice reads badly aloud.
        let spoken = if self.at.is_some() {
            format!("{position}. {label}")
        } else {
            label.to_owned()
        };
        announce(ANNOUNCEMENT_ID, &format!("{spoken}{outlined}"));
        Ok(())
    }
}
