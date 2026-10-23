//! The weighted springs controller: the one object that touches the scene, the clock and the panel.
//!
//! Everything that can be decided without a browser is decided in [`super::core::Core`], and everything the controller
//! is asked to do arrives as a [`ControllerCommand`] through a [`CommandBridge`]. It is never borrowed any other way. A
//! command that comes while the controller is busy waits its turn in the bridge's queue and is not lost.
//!
//! The controller is the *only* writer of a free circle. A circle a pointer holds is written by the library's own drag,
//! and one whose proxy is catching up after a release is written by neither until it has arrived.

use super::core::Core;
use crate::dijkstra::{NAMES, random_weights};
use demo_common::{
    force::{
        animation::AnimationLoop,
        bridge::{ApplyOutcome, BridgeCommand, CommandTarget},
        solver::{Status, Vec2},
    },
    util::{clear_error, report_error},
};
use std::rc::{Rc, Weak};
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    EdgeId, NodeId,
    scene::{DragPhase, Scene},
};

/// The ids of the panel's own elements.
const PANEL_ID: &str = "panel-springs";
const STATUS_ID: &str = "springs-status";
const FEEDBACK_ID: &str = "springs-feedback";
const ANNOUNCEMENT_ID: &str = "springs-announcement";
pub(super) const ERROR_ID: &str = "springs-error";

/// The actions of the buttons drawn in the scene, as `data-action` says them.
pub(super) const RESET_ACTION: &str = "reset";
pub(super) const PAUSE_ACTION: &str = "pause";
pub(super) const RANDOMISE_ACTION: &str = "randomise";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the controller is asked to do. Each is an owned value, queued before it is run.
pub(super) enum ControllerCommand {
    /// The library reported a step of a drag of a node. `lease` is the token of the observer registration it came from:
    /// if that registration has been torn down by the time this runs, the event is stale and is dropped unrun.
    Drag { id: NodeId, phase: DragPhase, lease: Weak<()> },
    /// An animation frame arrived at this browser timestamp.
    Frame { timestamp_ms: f64 },
    /// The user asked to put the circles back where they started.
    Reset,
    /// The user pressed Pause or Resume. Whether that pauses or resumes is decided when the command runs, from the
    /// state then, never from the state when the button was pressed: two presses queued together are a pause and a
    /// resume.
    TogglePaused,
    /// The page was hidden or shown.
    SetHidden(bool),
    /// The page's preference for reduced motion changed.
    SetReducedMotion(bool),
    /// The user asked for new random weights and tension.
    RandomizeWeights,
}

impl BridgeCommand for ControllerCommand {
    /// Adjacent frames collapse into the latest. Every other command is an ordering barrier.
    fn merge_into_tail(tail: &mut Self, incoming: &Self) -> bool {
        if let (ControllerCommand::Frame { timestamp_ms: old }, ControllerCommand::Frame { timestamp_ms: new }) =
            (tail, incoming)
        {
            *old = *new;
            return true;
        }
        false
    }

    /// A drag event whose registration is gone. A node id alone cannot tell, so the lease does.
    fn is_stale(&self) -> bool {
        matches!(self, ControllerCommand::Drag { lease, .. } if lease.upgrade().is_none())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel's elements that the controller writes to: the panel's own text, and the buttons drawn in the scene.
pub(super) struct PanelView {
    pub(super) panel: web_sys::Element,
    pub(super) status: web_sys::Element,
    pub(super) feedback: web_sys::Element,
    pub(super) announcement: web_sys::Element,
    /// The group of the Reset button in the scene.
    pub(super) reset: web_sys::Element,
    /// The group of the Pause button in the scene, which says Resume while paused.
    pub(super) pause: web_sys::Element,
}

impl PanelView {
    /// Finds every element, or says which one is missing. The buttons must already have been drawn in the scene
    /// `diagram_id`.
    pub(super) fn find(document: &web_sys::Document, diagram_id: &str) -> Result<Self, String> {
        let get = |id: &str| demo_common::util::required_element(document, id);
        let button = |action: &str| -> Result<web_sys::Element, String> {
            document
                .query_selector(&format!("#{diagram_id} [data-action=\"{action}\"]"))
                .map_err(|e| format!("could not look for the {action} button: {e:?}"))?
                .ok_or_else(|| format!("the {action} button is missing from the scene"))
        };
        Ok(Self {
            panel: get(PANEL_ID)?,
            status: get(STATUS_ID)?,
            feedback: get(FEEDBACK_ID)?,
            announcement: get(ANNOUNCEMENT_ID)?,
            reset: button(RESET_ACTION)?,
            pause: button(PAUSE_ACTION)?,
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a button of the scene as unavailable, or available again. An SVG group has no `disabled` attribute, so the
/// state is `aria-disabled` for assistive technology, and a faded look for everyone else. The button still takes the
/// click, which the demo then refuses with a reason.
fn set_available(button: &web_sys::Element, available: bool) {
    if available {
        let _ = button.remove_attribute("aria-disabled");
    } else {
        let _ = button.set_attribute("aria-disabled", "true");
    }
    let _ = button.set_attribute(
        "style",
        if available {
            "cursor: pointer;"
        } else {
            "cursor: not-allowed; opacity: 0.45;"
        },
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The controller.
pub(super) struct Controller {
    core: Core,
    scene: Scene,
    nodes: Vec<NodeId>,
    edges: Vec<EdgeId>,
    animation: Option<Rc<AnimationLoop>>,
    view: PanelView,
    /// The state last published, so only a change of it is announced.
    announced: Option<Status>,
    /// The vertex a pointer is holding, for the announcement.
    holding: Option<usize>,
}

impl Controller {
    pub(super) fn new(core: Core, scene: Scene, nodes: Vec<NodeId>, edges: Vec<EdgeId>, view: PanelView) -> Self {
        Self {
            core,
            scene,
            nodes,
            edges,
            animation: None,
            view,
            announced: None,
            holding: None,
        }
    }

    /// Gives the controller the loop that calls it back each frame. It cannot be given at construction, since the
    /// loop's callback needs the bridge, and the bridge needs the controller.
    pub(super) fn set_animation(&mut self, animation: Rc<AnimationLoop>) {
        self.animation = Some(animation);
    }

    /// Cancels the pending frame, if there is one. Called first when the demo is torn down.
    ///
    /// The panel then says no frame is pending, since it is the last thing published: nothing can publish after the
    /// demo has been torn down.
    pub(super) fn cancel_animation(&self) {
        if let Some(animation) = &self.animation {
            animation.cancel();
        }
        let _ = self.view.panel.set_attribute("data-physics-pending", "0");
    }

    pub(super) fn is_animation_pending(&self) -> bool {
        self.animation.as_ref().is_some_and(|a| a.is_pending())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The index of node `id`, and its centre when its top-left corner is `origin`. The centre uses the size the
    /// scene reports for the node, never a radius written down here.
    fn centre_of(&self, id: NodeId, origin: Point) -> Option<(usize, Vec2)> {
        let index = self.nodes.iter().position(|node| *node == id)?;
        let size = self.scene.node_rect(id).ok()?.size;
        Some((index, Vec2::new(origin.x + size.width / 2.0, origin.y + size.height / 2.0)))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn on_drag(&mut self, id: NodeId, phase: DragPhase) {
        match phase {
            DragPhase::Start { origin } => {
                if let Some((index, centre)) = self.centre_of(id, origin) {
                    self.core.drag_start(index, centre);
                    self.holding = Some(index);
                }
            },
            DragPhase::Applied { origin } => {
                if let Some((index, centre)) = self.centre_of(id, origin) {
                    self.core.drag_applied(index, centre);
                }
            },
            DragPhase::End { origin } => {
                if let Some((index, centre)) = self.centre_of(id, origin) {
                    self.core.drag_end(index, centre);
                    self.holding = None;
                }
            },
            DragPhase::Cancel { origin } => {
                if let Some((index, centre)) = self.centre_of(id, origin) {
                    self.core.drag_cancel(index, centre);
                    self.holding = None;
                }
            },
            // A phase added later is not one this demo knows about. It changes nothing.
            _ => {},
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes each of `moves` to the page, and stops at the first that fails.
    ///
    /// A failure leaves the solver and the scene possibly disagreeing, and the scene is right. So the solver is put
    /// where the page really has every circle, the physics is paused, and the reason is shown. Nothing is retried
    /// quietly.
    fn draw(&mut self, moves: Vec<(usize, Vec2)>) {
        for (index, centre) in moves {
            let node = self.nodes[index];
            let size = match self.scene.node_rect(node) {
                Ok(rect) => rect.size,
                Err(error) => return self.fail(&error),
            };
            let origin = Point::new(centre.x - size.width / 2.0, centre.y - size.height / 2.0);
            match self.scene.move_node(node, origin) {
                Ok(()) => self.core.confirm_drawn(index),
                Err(error) => return self.fail(&error),
            }
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn fail(&mut self, error: &dyn std::fmt::Display) {
        let centres: Vec<Vec2> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                self.scene.node_rect(*node).map_or_else(
                    |_| self.core.system().centre(index).unwrap_or(Vec2::ZERO),
                    |rect| Vec2::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0),
                )
            })
            .collect();
        self.core.fail(&centres, error.to_string());
        report_error(
            ERROR_ID,
            &format!("The physics stopped because a circle could not be drawn: {error}"),
        );
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// New random weights for the springs, in the order of the edges.
    fn new_weights(&self) -> Vec<f64> {
        random_weights(self.edges.len(), js_sys::Math::random)
            .into_iter()
            .map(|w| w as f64)
            .collect()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows `weights` on the edge labels.
    fn show_weights(&self, weights: &[f64]) {
        for (edge, weight) in self.edges.iter().zip(weights) {
            if let Err(error) = self.scene.set_edge_label(*edge, Some(&(*weight as i64).to_string())) {
                report_error(ERROR_ID, &format!("An edge label could not be changed: {error}"));
            }
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Gives the springs new random weights and a random tension that is known to settle, and shows the weights on the
    /// edge labels. The circles stay where they are, and the network starts to move and comes to rest by itself.
    fn randomise(&mut self) -> ApplyOutcome {
        let weights = self.new_weights();
        let tension = self.core.choose_tension(&weights, js_sys::Math::random);
        let outcome = self.core.randomise(&weights, &tension);
        if outcome == ApplyOutcome::Applied {
            self.show_weights(&weights);
        }
        outcome
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts the circles back where they started, with every spring at its starting length. The weights are kept.
    fn reset(&mut self) -> ApplyOutcome {
        let (outcome, moves) = self.core.reset();
        if outcome == ApplyOutcome::Applied {
            self.draw(moves);
        }
        outcome
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Asks for a frame if the physics wants one, and cancels any that is pending if it does not.
    fn schedule(&self) {
        let Some(animation) = &self.animation else {
            return;
        };
        if self.core.wants_frame() {
            animation.request();
        } else {
            animation.cancel();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Publishes the state: the two attributes the tests read, the status line, the controls, and a sparing
    /// announcement.
    fn publish(&mut self) {
        let status = self.core.status();
        let _ = self.view.panel.set_attribute("data-physics", status.as_str());
        let _ = self
            .view
            .panel
            .set_attribute("data-physics-pending", if self.is_animation_pending() { "1" } else { "0" });
        self.view.status.set_text_content(Some(&self.core.status_text()));
        self.view
            .feedback
            .set_text_content(Some(self.core.feedback().map_or("", |f| f.message())));
        set_available(&self.view.reset, self.core.reset_enabled());
        let paused = self.core.is_user_paused();
        let name = if paused { "Resume" } else { "Pause" };
        let _ = self.view.pause.set_attribute("aria-label", name);
        let _ = self
            .view
            .pause
            .set_attribute("aria-pressed", if paused { "true" } else { "false" });
        if let Ok(Some(text)) = self.view.pause.query_selector("text") {
            text.set_text_content(Some(name));
        }
        if self.core.fault().is_none() {
            clear_error(ERROR_ID);
        }
        self.announce(status);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Announces a major change of state, and nothing between: one line when a drag starts, when the graph is paused,
    /// when it comes to rest and when it fails to. Never one for each frame.
    fn announce(&mut self, status: Status) {
        if self.announced == Some(status) {
            return;
        }
        let before = self.announced.replace(status);
        let message = match (before, status) {
            (_, Status::Dragging) if before != Some(Status::Dragging) => {
                self.holding.map(|index| format!("Dragging vertex {}.", NAMES[index]))
            },
            (_, Status::Paused) => Some("Paused.".to_owned()),
            (Some(Status::Running | Status::Dragging | Status::Paused), Status::Sleeping) => {
                Some("The graph has settled.".to_owned())
            },
            (_, Status::Unsettled) => Some("The graph did not settle.".to_owned()),
            _ => None,
        };
        if let Some(message) = message {
            self.view.announcement.set_text_content(Some(&message));
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl CommandTarget<ControllerCommand> for Controller {
    fn apply(&mut self, command: ControllerCommand) -> ApplyOutcome {
        match command {
            ControllerCommand::Drag { id, phase, .. } => {
                self.on_drag(id, phase);
                ApplyOutcome::Applied
            },
            ControllerCommand::Frame { timestamp_ms } => {
                let moves = self.core.frame(timestamp_ms);
                self.draw(moves);
                ApplyOutcome::Applied
            },
            ControllerCommand::Reset => self.reset(),
            ControllerCommand::TogglePaused => {
                self.core.toggle_user_paused();
                ApplyOutcome::Applied
            },
            ControllerCommand::SetHidden(hidden) => {
                self.core.set_hidden(hidden);
                ApplyOutcome::Applied
            },
            ControllerCommand::SetReducedMotion(reduced) => {
                self.core.set_reduced_motion(reduced);
                ApplyOutcome::Applied
            },
            ControllerCommand::RandomizeWeights => self.randomise(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs once after every drain, with the controller's borrow already released by the bridge and any command that
    /// arrived in the meantime already run. It draws what reduced motion settled, asks for the next frame or cancels
    /// the pending one, and publishes.
    fn reconcile(&mut self) {
        let moves = self.core.settle();
        self.draw(moves);
        self.core.settled_down();
        self.schedule();
        self.publish();
    }
}
