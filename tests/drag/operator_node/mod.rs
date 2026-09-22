//! `Scene::add_unary_operator_node`/`add_binary_operator_node`/`add_arithmetic_operator_node`: a node labelled with
//! the operation that produced its single value, wired with an auto-drawn edge from each of its operand(s).
//!
//! - [`construction`] — rendering (label row plus an inset value cell), auto-wired input edge(s), and
//!   operand/result validation, for a unary, binary, or arithmetic operator node.
//! - [`anchor_routing`] — the same-side anti-crossing split, `EdgeAnchors` honoured on auto-wired operand edges,
//!   and dragging either an operand or the operator itself.
//! - [`port_markers`] — the non-commutative `SUB`/`DIV`/`MOD` "L"/"R" port marker: which operators draw one, its
//!   accessible name, and that its own identity survives a near/far reassignment or a full operand-position
//!   exchange.
//! - [`chaining`] — an operator node's own result feeding a further operator node as one of its own operands.

mod anchor_routing;
mod chaining;
mod construction;
mod port_markers;
