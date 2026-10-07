//! Internal unit tests for `Scene::node`: construction rollback guards, node labelling, nested scenes, and the
//! container-node interactions built on top of them.
//!
//! - [`guards`] — `RenderGuard`/`OperatorConstructionGuard`: transactional rollback on drop, and no rollback once
//!   disarmed.
//! - [`ref_name`] — `ref_name`/`current_ref_name`: the label a node is called by, and how a live `Selection` extends
//!   it.
//! - [`nested_scenes`] — `Scene::add_container_node`, `enter`/`exit`, and the navigation invariants around them.
//! - [`replace_container_child`] — `Scene::replace_container_child`: swapping a nested `Scene`, its rejections, its
//!   fresh `NavigationState` for the detached child, and its transactional rollback on a failed DOM write.
//! - [`make_enterable`] — `Scene::make_enterable`: a container node made clickable/keyboard-activatable, and its
//!   rollback on a failed DOM write.

mod guards;
mod make_enterable;
mod nested_scenes;
mod ref_name;
mod replace_container_child;
mod support;
