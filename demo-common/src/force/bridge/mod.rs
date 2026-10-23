//! A lossless, ordered way into a controller that must never be borrowed twice.
//!
//! A controller is touched from many places: an animation frame, a button, and notifications that come from the library
//! in the middle of a move. Some of these happen while the controller is already being worked on, and a second
//! `borrow_mut` there is a panic. Dropping the second one is no answer either, since a Reset or a weight change that
//! quietly vanishes is a bug.
//!
//! So nothing borrows the controller directly. Every access goes through [`CommandBridge::dispatch`], which puts an
//! owned command in a FIFO first and then tries to drain it. If the controller is busy, the command stays in the FIFO
//! and runs when the operation in progress ends. Commands run strictly in the order they arrived, except that adjacent
//! animation frames merge into the latest one, and a command that has gone stale is dropped, never run.
//!
//! The bridge holds only a [`Weak`] reference to the controller, so the controller can own whatever it needs, a scene
//! included, without a cycle.

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::{Rc, Weak},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A command the bridge can queue.
pub trait BridgeCommand: Sized {
    /// Merges `incoming` into `tail`, the newest command already queued, and returns `true` if it did, so that
    /// `incoming` is then not queued. This is how adjacent animation frames collapse into the latest. A command that is
    /// an ordering barrier must return `false`.
    fn merge_into_tail(tail: &mut Self, incoming: &Self) -> bool;

    /// Whether the command has gone stale since it was queued and must be dropped unrun.
    fn is_stale(&self) -> bool;
}

/// What a controller makes of one command it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// The command ran.
    Applied,
    /// The command was refused because it was not allowed in the state the controller was then in. It changed nothing,
    /// and it is not run again later.
    Rejected,
}

/// What the bridge asks of a controller.
pub trait CommandTarget<C> {
    /// Runs one command. The queue is not borrowed while it runs, so it may cause more commands to be dispatched, and
    /// those are queued behind it.
    fn apply(&mut self, command: C) -> ApplyOutcome;

    /// Called once after a drain has finished, with the controller's borrow released by the bridge first. A controller
    /// uses it to work out whether it needs a frame, so a command that arrived while it was busy cannot leave it asleep.
    fn reconcile(&mut self);
}

/// What happened to a command given to [`CommandBridge::dispatch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchOutcome {
    /// The command ran during this call.
    Executed,
    /// The controller was busy. The command is in the queue and will run when the operation in progress ends.
    Queued,
    /// The command ran during this call and the controller refused it.
    Rejected,
    /// The bridge has been shut down. The command was discarded and nothing ran.
    Discarded,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The queue, and the way into one controller.
pub struct CommandBridge<C, T> {
    controller: Weak<RefCell<T>>,
    queue: RefCell<VecDeque<(u64, C)>>,
    draining: Cell<bool>,
    shut_down: Cell<bool>,
    next_ticket: Cell<u64>,
}

/// Puts the drain flag back, and then reconciles, when a drain ends by any route, including an early return.
struct DrainGuard<'a, C: BridgeCommand, T: CommandTarget<C>> {
    bridge: &'a CommandBridge<C, T>,
}

impl<C: BridgeCommand, T: CommandTarget<C>> Drop for DrainGuard<'_, C, T> {
    fn drop(&mut self) {
        self.bridge.draining.set(false);
        // The controller's borrow was released before this guard went, since the guard was made first. If it cannot be
        // taken now, the controller is being worked on by something that will reconcile when it finishes.
        if self.bridge.shut_down.get() {
            return;
        }
        if let Some(controller) = self.bridge.controller.upgrade() {
            if let Ok(mut controller) = controller.try_borrow_mut() {
                controller.reconcile();
            }
        }
    }
}

impl<C: BridgeCommand, T: CommandTarget<C>> CommandBridge<C, T> {
    /// A bridge to `controller`, which it holds only weakly.
    pub fn new(controller: &Rc<RefCell<T>>) -> Self {
        Self {
            controller: Rc::downgrade(controller),
            queue: RefCell::new(VecDeque::new()),
            draining: Cell::new(false),
            shut_down: Cell::new(false),
            next_ticket: Cell::new(0),
        }
    }

    /// Queues `command`, merging it into the newest queued command if they merge, and then drains the queue if nothing
    /// else is. Every way into the controller is this function.
    pub fn dispatch(&self, command: C) -> DispatchOutcome {
        if self.shut_down.get() {
            return DispatchOutcome::Discarded;
        }
        let ticket = self.next_ticket.get();
        self.next_ticket.set(ticket + 1);
        {
            let mut queue = self.queue.borrow_mut();
            if let Some((_, tail)) = queue.back_mut() {
                if C::merge_into_tail(tail, &command) {
                    drop(queue);
                    return self.drain(None);
                }
            }
            queue.push_back((ticket, command));
        }
        self.drain(Some(ticket))
    }

    /// Runs whatever is queued, if nothing is already running it. `watch` is the ticket of the command whose outcome is
    /// wanted.
    fn drain(&self, watch: Option<u64>) -> DispatchOutcome {
        // A nested call, from a command that is running, appends to the queue and leaves the outer loop to run it.
        if self.draining.replace(true) {
            return DispatchOutcome::Queued;
        }
        // Made before the controller is borrowed, so it is dropped after it.
        let _guard = DrainGuard { bridge: self };
        let Some(controller) = self.controller.upgrade() else {
            self.queue.borrow_mut().clear();
            return DispatchOutcome::Discarded;
        };
        let Ok(mut controller) = controller.try_borrow_mut() else {
            return DispatchOutcome::Queued;
        };
        let mut mine: Option<ApplyOutcome> = None;
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some((ticket, command)) = next else {
                break;
            };
            if self.shut_down.get() || command.is_stale() {
                continue;
            }
            let outcome = controller.apply(command);
            if Some(ticket) == watch {
                mine = Some(outcome);
            }
        }
        match mine {
            Some(ApplyOutcome::Applied) => DispatchOutcome::Executed,
            Some(ApplyOutcome::Rejected) => DispatchOutcome::Rejected,
            // The command was merged into an earlier one, or dropped as stale: it is handled, and not pending.
            None => DispatchOutcome::Executed,
        }
    }

    /// How many commands are waiting.
    pub fn pending(&self) -> usize {
        self.queue.borrow().len()
    }

    /// Discards every queued command and refuses every later one. A command that is running may finish. Called first when
    /// a demo is torn down.
    pub fn shut_down(&self) {
        self.shut_down.set(true);
        self.queue.borrow_mut().clear();
    }

    /// Whether [`shut_down`](Self::shut_down) has been called.
    pub fn is_shut_down(&self) -> bool {
        self.shut_down.get()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
