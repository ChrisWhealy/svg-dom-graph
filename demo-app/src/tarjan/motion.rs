//! The condensation as springs, in the page: once the last step is shown its arrows become springs and its nodes are
//! let go, and the springs of [`relax`](super::relax) move them. Going back to any other step puts them where the table
//! of places had them.
//!
//! The library draws the arrows, so they follow their nodes without any help from here.

use super::{
    condensation::BOX,
    dom::set,
    layout::LEFT_PANEL,
    relax::{self, RUN_SECONDS},
    scene::Board,
};
use demo_common::force::{
    animation::AnimationLoop,
    clock::FrameClock,
    solver::{Bounds, FIXED_DT, SpringSystem},
};
use std::rc::Rc;
use svg_dom::root::utils::Point;

/// What the springs are doing.
pub(super) enum Motion {
    /// Nothing: the nodes are where the table of places put them.
    Idle,
    /// The springs are moving the nodes.
    Running(Box<Running>),
    /// The springs have run their time, and the nodes stay where they are until another step is shown.
    Done,
}

pub(super) struct Running {
    system: SpringSystem,
    clock: FrameClock,
    animation: Rc<AnimationLoop>,
    /// How much of the physics' time has been used, in seconds.
    elapsed: f64,
}

/// Whether the page has asked for less motion, in which case the springs are not run and the nodes stay where they are.
fn reduced_motion() -> bool {
    web_sys::window()
        .and_then(|window| window.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .is_some_and(|query| query.matches())
}

impl Board {
    /// Says what the springs are doing, in the `data-tj-springs` attribute of the scene, which is `idle`, `running` or
    /// `done`.
    fn say_motion(&self) {
        let word = match &*self.motion.borrow() {
            Motion::Idle => "idle",
            Motion::Running(_) => "running",
            Motion::Done => "done",
        };
        let _ = set(self.svg(), "data-tj-springs", word);
    }

    /// The area that the nodes of the condensation stay in: under the roster, and no further right than the graph.
    fn dag_bounds(&self) -> Bounds {
        let top = self.layout.dag_origin.1 - 20.0;
        Bounds::from_origin_and_size(190.0, top, LEFT_PANEL - 5.0 - 190.0, self.layout.size.1 - top)
    }

    /// Lets the springs move the nodes of the condensation, if they are not moving or have not already. Does nothing
    /// until every component is in the condensation, since a spring needs both its ends.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the springs could not be made. The nodes stay where they are.
    pub(super) fn relax(self: &Rc<Self>) -> Result<(), String> {
        if !matches!(*self.motion.borrow(), Motion::Idle) {
            return Ok(());
        }
        let (centres, arcs) = {
            let dag = self.dag.borrow();
            let homes: Vec<(f64, f64)> = dag.home.iter().flatten().copied().collect();
            if homes.len() < 10 || dag.shown.is_empty() {
                return Ok(());
            }
            (
                homes
                    .iter()
                    .map(|&(x, y)| (x + BOX.0 / 2.0, y + BOX.1 / 2.0))
                    .collect::<Vec<_>>(),
                dag.shown.clone(),
            )
        };
        if reduced_motion() {
            *self.motion.borrow_mut() = Motion::Done;
            self.say_motion();
            return Ok(());
        }
        let system = relax::build(&centres, &arcs, self.dag_bounds())?;
        let weak = Rc::downgrade(self);
        let animation = AnimationLoop::new(move |timestamp| {
            if let Some(board) = weak.upgrade() {
                board.relax_frame(timestamp);
            }
        })?;
        animation.request();
        *self.motion.borrow_mut() = Motion::Running(Box::new(Running {
            system,
            clock: FrameClock::new(),
            animation,
            elapsed: 0.0,
        }));
        self.say_motion();
        Ok(())
    }

    /// Runs the steps that one frame is owed, and draws where they leave the nodes.
    fn relax_frame(&self, timestamp: f64) {
        let centres = {
            let mut motion = self.motion.borrow_mut();
            let Motion::Running(run) = &mut *motion else {
                return;
            };
            let steps = run.clock.advance(timestamp);
            for _ in 0..steps {
                if !run.system.wants_frames() {
                    break;
                }
                run.system.step(FIXED_DT);
                run.elapsed += FIXED_DT;
            }
            (0..run.system.len())
                .filter_map(|i| run.system.centre(i).map(|c| (c.x - BOX.0 / 2.0, c.y - BOX.1 / 2.0)))
                .collect::<Vec<_>>()
        };
        let drawn = self.place_nodes(&centres);
        let mut motion = self.motion.borrow_mut();
        let finished = match &*motion {
            Motion::Running(run) => drawn.is_err() || !run.system.wants_frames() || run.elapsed >= RUN_SECONDS,
            _ => return,
        };
        if finished {
            *motion = Motion::Done;
        } else if let Motion::Running(run) = &*motion {
            run.animation.request();
        }
        drop(motion);
        self.say_motion();
    }

    /// Stops the springs and puts every node back where the table of places has it. Does nothing if they have not
    /// moved.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a node could not be moved back.
    pub(super) fn rest(&self) -> Result<(), String> {
        if matches!(*self.motion.borrow(), Motion::Idle) {
            return Ok(());
        }
        // Dropping the running state cancels its pending frame.
        *self.motion.borrow_mut() = Motion::Idle;
        self.say_motion();
        let home: Vec<(f64, f64)> = self.dag.borrow().home.iter().flatten().copied().collect();
        self.place_nodes(&home)
    }

    /// Moves node `i + 1` of the condensation to the top left `origins[i]`, for each. A node whose component is not in
    /// the condensation yet is left alone.
    fn place_nodes(&self, origins: &[(f64, f64)]) -> Result<(), String> {
        let dag = self.dag.borrow();
        for (slot, &(x, y)) in dag.nodes.iter().flatten().zip(origins) {
            self.scene
                .move_node(slot.0, Point::new(x, y))
                .map_err(|e| format!("could not move a node of the condensation: {e}"))?;
        }
        Ok(())
    }
}
