use super::*;

#[derive(Clone, Debug)]
enum Cmd {
    Frame(u32),
    Reset,
    Pause(bool),
    /// Runs another dispatch from inside `apply`, as a library notification does.
    Chain(Vec<Cmd>),
    /// Refused by the controller.
    Refused,
    /// Stale once the lease behind it has been dropped.
    Lease(Weak<()>, u32),
    /// Drops the lease it holds when it runs.
    DropLease(Rc<RefCell<Option<Rc<()>>>>),
}

/// `Weak` has no equality, so a lease is compared by its tag, and a shared holder by identity.
impl PartialEq for Cmd {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Cmd::Frame(a), Cmd::Frame(b)) => a == b,
            (Cmd::Reset, Cmd::Reset) | (Cmd::Refused, Cmd::Refused) => true,
            (Cmd::Pause(a), Cmd::Pause(b)) => a == b,
            (Cmd::Chain(a), Cmd::Chain(b)) => a == b,
            (Cmd::Lease(_, a), Cmd::Lease(_, b)) => a == b,
            (Cmd::DropLease(a), Cmd::DropLease(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl BridgeCommand for Cmd {
    fn merge_into_tail(tail: &mut Self, incoming: &Self) -> bool {
        if let (Cmd::Frame(old), Cmd::Frame(new)) = (tail, incoming) {
            *old = *new;
            return true;
        }
        false
    }

    fn is_stale(&self) -> bool {
        matches!(self, Cmd::Lease(lease, _) if lease.upgrade().is_none())
    }
}

struct Controller {
    bridge: Weak<CommandBridge<Cmd, Controller>>,
    applied: Vec<Cmd>,
    reconciles: u32,
    /// Whether `reconcile` could take the controller's own borrow, which it must be able to.
    nested_dispatch_outcomes: Vec<DispatchOutcome>,
}

impl CommandTarget<Cmd> for Controller {
    fn apply(&mut self, command: Cmd) -> ApplyOutcome {
        if let Cmd::Chain(next) = &command {
            let bridge = self.bridge.upgrade().expect("the bridge");
            for cmd in next.clone() {
                // The controller is borrowed here, so this must queue and not panic.
                self.nested_dispatch_outcomes.push(bridge.dispatch(cmd));
            }
        }
        if let Cmd::DropLease(holder) = &command {
            holder.borrow_mut().take();
        }
        let outcome = if command == Cmd::Refused {
            ApplyOutcome::Rejected
        } else {
            ApplyOutcome::Applied
        };
        self.applied.push(command);
        outcome
    }

    fn reconcile(&mut self) {
        self.reconciles += 1;
    }
}

fn setup() -> (Rc<RefCell<Controller>>, Rc<CommandBridge<Cmd, Controller>>) {
    let controller = Rc::new(RefCell::new(Controller {
        bridge: Weak::new(),
        applied: Vec::new(),
        reconciles: 0,
        nested_dispatch_outcomes: Vec::new(),
    }));
    let bridge = Rc::new(CommandBridge::new(&controller));
    controller.borrow_mut().bridge = Rc::downgrade(&bridge);
    (controller, bridge)
}

#[test]
fn a_command_to_an_idle_controller_runs_at_once_and_the_controller_reconciles_after() {
    let (controller, bridge) = setup();
    assert_eq!(bridge.dispatch(Cmd::Reset), DispatchOutcome::Executed);
    assert_eq!(controller.borrow().applied, vec![Cmd::Reset]);
    assert_eq!(controller.borrow().reconciles, 1);
    assert_eq!(bridge.pending(), 0);
}

#[test]
fn commands_run_in_strict_fifo_order() {
    let (controller, bridge) = setup();
    for cmd in [Cmd::Reset, Cmd::Pause(true), Cmd::Frame(1), Cmd::Pause(false)] {
        bridge.dispatch(cmd);
    }
    assert_eq!(
        controller.borrow().applied,
        vec![Cmd::Reset, Cmd::Pause(true), Cmd::Frame(1), Cmd::Pause(false)]
    );
}

#[test]
fn a_command_dispatched_while_the_controller_is_busy_is_queued_not_lost_and_runs_after_in_order() {
    let (controller, bridge) = setup();
    let outcome = bridge.dispatch(Cmd::Chain(vec![Cmd::Reset, Cmd::Pause(true)]));
    assert_eq!(outcome, DispatchOutcome::Executed);
    // The nested ones were queued, and ran after the chain that issued them, in order.
    assert_eq!(
        controller.borrow().nested_dispatch_outcomes,
        vec![DispatchOutcome::Queued, DispatchOutcome::Queued]
    );
    assert_eq!(
        controller.borrow().applied,
        vec![Cmd::Chain(vec![Cmd::Reset, Cmd::Pause(true)]), Cmd::Reset, Cmd::Pause(true)]
    );
    assert_eq!(bridge.pending(), 0);
}

#[test]
fn a_nested_dispatch_that_itself_chains_is_still_consumed_by_the_outer_loop() {
    let (controller, bridge) = setup();
    bridge.dispatch(Cmd::Chain(vec![Cmd::Chain(vec![Cmd::Reset]), Cmd::Pause(true)]));
    let applied = controller.borrow().applied.clone();
    // The outer chain, then the inner chain and the pause it queued, then the reset the inner chain queued.
    assert_eq!(applied.len(), 4);
    assert_eq!(applied[3], Cmd::Reset);
    assert_eq!(applied[2], Cmd::Pause(true));
}

#[test]
fn the_drain_flag_is_reset_even_when_the_controller_is_dropped_mid_way() {
    let (controller, bridge) = setup();
    drop(controller);
    assert_eq!(bridge.dispatch(Cmd::Reset), DispatchOutcome::Discarded);
    assert_eq!(bridge.pending(), 0);
    // A later dispatch is not stuck behind a drain flag that never came down.
    assert_eq!(bridge.dispatch(Cmd::Reset), DispatchOutcome::Discarded);
}

#[test]
fn adjacent_frames_merge_into_the_latest_and_a_barrier_between_them_prevents_it() {
    let (controller, bridge) = setup();
    // Held back by a chain, so that these three are all queued at the same time.
    bridge.dispatch(Cmd::Chain(vec![Cmd::Frame(1), Cmd::Frame(2), Cmd::Frame(3)]));
    let applied = controller.borrow().applied.clone();
    assert_eq!(applied[1..], [Cmd::Frame(3)]);

    let (controller, bridge) = setup();
    bridge.dispatch(Cmd::Chain(vec![Cmd::Frame(1), Cmd::Reset, Cmd::Frame(2)]));
    let applied = controller.borrow().applied.clone();
    assert_eq!(applied[1..], [Cmd::Frame(1), Cmd::Reset, Cmd::Frame(2)]);
}

#[test]
fn a_command_the_controller_refuses_is_reported_as_rejected_and_not_run_again() {
    let (controller, bridge) = setup();
    assert_eq!(bridge.dispatch(Cmd::Refused), DispatchOutcome::Rejected);
    assert_eq!(controller.borrow().applied, vec![Cmd::Refused]);
    assert_eq!(bridge.dispatch(Cmd::Reset), DispatchOutcome::Executed);
    assert_eq!(controller.borrow().applied, vec![Cmd::Refused, Cmd::Reset]);
}

#[test]
fn a_stale_command_is_dropped_without_running_and_a_live_one_runs() {
    let (controller, bridge) = setup();
    let lease = Rc::new(());
    bridge.dispatch(Cmd::Lease(Rc::downgrade(&lease), 1));
    drop(lease);
    bridge.dispatch(Cmd::Lease(Weak::new(), 2));
    assert_eq!(controller.borrow().applied.len(), 1);
    assert!(matches!(controller.borrow().applied[0], Cmd::Lease(_, 1)));
}

#[test]
fn a_queued_command_whose_lease_is_dropped_by_an_earlier_command_is_discarded_unrun() {
    let (controller, bridge) = setup();
    let lease = Rc::new(());
    let holder = Rc::new(RefCell::new(Some(lease.clone())));
    let weak = Rc::downgrade(&lease);
    drop(lease);
    // The first command drops the last lease while the second is already queued behind it.
    bridge.dispatch(Cmd::Chain(vec![Cmd::DropLease(holder), Cmd::Lease(weak, 7)]));
    let applied = controller.borrow().applied.clone();
    assert_eq!(applied.len(), 2, "{applied:?}");
    assert!(matches!(applied[1], Cmd::DropLease(_)));
}

#[test]
fn shutting_down_discards_the_queue_and_every_later_command() {
    let (controller, bridge) = setup();
    bridge.shut_down();
    assert!(bridge.is_shut_down());
    assert_eq!(bridge.dispatch(Cmd::Reset), DispatchOutcome::Discarded);
    assert!(controller.borrow().applied.is_empty());
    assert_eq!(controller.borrow().reconciles, 0);
}

#[test]
fn a_command_queued_by_a_running_command_after_a_shutdown_does_not_run() {
    // A running command shuts the bridge down, then dispatches. The second is discarded.
    struct Stopper {
        bridge: Weak<CommandBridge<Cmd, Stopper>>,
        applied: u32,
    }
    impl CommandTarget<Cmd> for Stopper {
        fn apply(&mut self, _command: Cmd) -> ApplyOutcome {
            self.applied += 1;
            let bridge = self.bridge.upgrade().expect("the bridge");
            bridge.shut_down();
            assert_eq!(bridge.dispatch(Cmd::Reset), DispatchOutcome::Discarded);
            ApplyOutcome::Applied
        }

        fn reconcile(&mut self) {}
    }
    let controller = Rc::new(RefCell::new(Stopper {
        bridge: Weak::new(),
        applied: 0,
    }));
    let bridge = Rc::new(CommandBridge::new(&controller));
    controller.borrow_mut().bridge = Rc::downgrade(&bridge);
    bridge.dispatch(Cmd::Reset);
    assert_eq!(controller.borrow().applied, 1);
    assert_eq!(bridge.pending(), 0);
}

#[test]
fn reconcile_runs_after_the_whole_drain_including_commands_queued_during_it() {
    let (controller, bridge) = setup();
    bridge.dispatch(Cmd::Chain(vec![Cmd::Reset, Cmd::Pause(false)]));
    // One drain, so one reconcile, after all three commands.
    assert_eq!(controller.borrow().applied.len(), 3);
    assert_eq!(controller.borrow().reconciles, 1);
}
