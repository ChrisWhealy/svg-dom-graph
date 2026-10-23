//! Internal unit tests for the spring solver, in groups.
//!
//! - [`forces`] — a spring at its length, force symmetry, linear stiffness, energy and damping, and determinism.
//! - [`contact_and_proxy`] — bounds, contact and radius, and a pinned proxy's pursuit of its target.
//! - [`pins`] — release, cancel and a second press, several pins at once, and held-node quiescence.
//! - [`lifecycle`] — sleep, the watchdog, pause, reset, settling at once, and a step that goes wrong.
//! - [`validation`] — every refusal to build or change a system, and resyncing to the scene's centres.
//! - [`overflow`] — derived numbers that overflow although everything they came from is finite.
//! - [`support`] — the small systems and helpers they share.

use super::*;
use support::*;

mod contact_and_proxy;
mod forces;
mod lifecycle;
mod overflow;
mod pins;
mod support;
mod validation;
