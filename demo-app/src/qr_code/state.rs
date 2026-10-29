//! The position in the walk, and what drawing it takes.
//!
//! [`State`] is the one place the position is kept, so that the graph, the matrix, the buttons and the text are all
//! drawn from it. Every draw is of the whole state, and the parts skip what is already so, so a draw does not depend on
//! the one before it.

use super::{
    ANNOUNCEMENT_ID, CHOSEN, Display, EXPORT_ACTION, JUMP_ACTION, NO_ROLE_EXPLANATION, NOT_CHOSEN,
    UNSTARTED_EXPLANATION, caption_text,
    grid::QrModuleKind,
    retry::Retry,
    role_action,
    snapshots::{COUNT, JUMPS, SnapshotSet, Stage},
    stats_text,
    viewport::Frame,
};
use demo_common::{
    svg_buttons,
    util::{announce, stringify},
};
use svg_dom_graph::scene::{Scene, Selection};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that decides what is drawn.
pub(super) struct State {
    /// Every snapshot of the current input.
    pub(super) set: SnapshotSet,
    /// `None` is the unstarted state, which shows an empty matrix and nothing but the input.
    pub(super) at: Option<usize>,
    /// The role whose modules are outlined.
    pub(super) role: Option<QrModuleKind>,
    /// Whether the nodes and the anchor's selection still have to be brought into line with `set` and `at`, which is so
    /// from new text until that has worked, see [`State::apply`].
    pub(super) stale: Retry,
    pub(super) display: Display,
}

impl State {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the whole of the current state in `scene`.
    ///
    /// If new text has been taken in, the nodes are first given its values, and the anchor's selection is put where the
    /// position is. That is several writes, and it is done again at every draw until all of it has worked, since the
    /// writes cannot be undone and doing them twice is harmless. See [`Retry`].
    ///
    /// # Errors
    ///
    /// Returns `Err` if the DOM or the library refuses a write, or a text does not fit its block. The next call writes
    /// whatever is still out of date.
    pub(super) fn apply(&self, scene: &Scene) -> Result<(), String> {
        let display = &self.display;
        self.stale.run(|| {
            display.graph.write_content(scene, &self.set.content)?;
            let selection = self.at.map_or(Selection::None, Selection::Cell);
            scene.set_selection(display.step, selection).map_err(stringify)
        })?;
        let snapshot = self.set.at(self.at);
        let caption = caption_text(&snapshot.label, &snapshot.grid);
        let writes = display.viewport.render(&Frame {
            grid: &snapshot.grid,
            highlights: &snapshot.highlights,
            role: self.role,
            summary: &caption,
        })?;
        display.graph.show(scene, snapshot)?;

        // The finished symbol can be saved only once the walk has reached it.
        svg_buttons::set_disabled(&display.layer, EXPORT_ACTION, self.at != Some(COUNT - 1))?;

        // The button of the stage that the position is in is pressed, and none is before the walk starts.
        for &(label, index) in &JUMPS {
            let current = self.at.is_some() && Stage::of(index) == snapshot.stage;
            let action = format!("{JUMP_ACTION}{index}");
            svg_buttons::set_pressed(&display.layer, &action, current, if current { CHOSEN } else { NOT_CHOSEN })?;
            debug_assert!(!label.is_empty());
        }
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
            .map_or("Not started".to_owned(), |index| format!("Step {} of {COUNT}", index + 1));
        display.caption.set(&caption)?;
        display
            .stats
            .set(&stats_text(&position, display.viewport.element_count()?, writes))?;
        let (stage_name, stage_text) = if self.at.is_some() {
            (snapshot.stage.name(), snapshot.stage.explanation())
        } else {
            ("Not started", UNSTARTED_EXPLANATION)
        };
        display.stage_name.set(stage_name)?;
        display.stage_text.set(stage_text)?;
        let (note_name, note_text) = snapshot.note.as_ref().map_or(("", ""), |(name, text)| (name, text));
        display.note_name.set(note_name)?;
        display.note_text.set(note_text)?;
        display.role_name.set(self.role.map_or("No role chosen", QrModuleKind::label))?;
        display
            .role_text
            .set(self.role.map_or(NO_ROLE_EXPLANATION, QrModuleKind::explanation))?;

        let outlined = self.role.map_or(String::new(), |role| {
            let count = snapshot
                .grid
                .cells()
                .filter(|&(row, col)| snapshot.grid.role(row, col) == Some(role))
                .count();
            format!(" Outlining {count} {} modules.", role.label().to_lowercase())
        });
        // The unstarted label already says so, and saying it twice reads badly aloud.
        let spoken = if self.at.is_some() {
            format!("{position}. {}", snapshot.label)
        } else {
            snapshot.label.clone()
        };
        announce(ANNOUNCEMENT_ID, &format!("{spoken}{outlined}"));
        Ok(())
    }
}
