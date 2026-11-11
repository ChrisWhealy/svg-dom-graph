//! A tab that holds the lock every test shares.

use crate::common::timing::Timing;
use headless_chrome::Tab;
use std::{
    ops::Deref,
    sync::{Arc, MutexGuard},
};

/// A tab that holds the lock every test shares, until it is dropped.
///
/// A click through the toolbar needs its tab in front, and Chrome has one foreground tab at a time. Tests that ran side
/// by side would each take the front from the others and see their clicks fail. The lock makes them take turns.
///
/// It also keeps the run to one active tab. Running tabs in parallel has twice been followed by a macOS kernel panic (a
/// kernel spinlock timeout between two Chrome threads), which taking turns avoids.
pub(crate) struct DemoTab {
    tab: Arc<Tab>,
    // Declared before the turn, so the time is recorded while the test still has the browser to itself.
    _timing: Timing,
    _turn: MutexGuard<'static, ()>,
}

impl DemoTab {
    /// A tab that holds `turn` until it is dropped, and times how long it holds the browser.
    pub(crate) fn new(tab: Arc<Tab>, timing: Timing, turn: MutexGuard<'static, ()>) -> Self {
        Self {
            tab,
            _timing: timing,
            _turn: turn,
        }
    }
}

impl Deref for DemoTab {
    type Target = Tab;

    fn deref(&self) -> &Tab {
        &self.tab
    }
}
