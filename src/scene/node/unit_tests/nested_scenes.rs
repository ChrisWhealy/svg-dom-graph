//! `Scene::add_container_node`, `enter`/`exit`, and the navigation invariants around them.

use super::support::{make_svg, visibility};
use crate::{error::Error, scene::Scene, test_support::check};
use svg_dom::root::utils::{Point, Size};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Grafting a focused, unowned child succeeds: the parent stays focused and visible, and the child becomes nested,
/// unfocused, and hidden.
#[wasm_bindgen_test]
fn add_container_node_grafts_a_focused_unowned_child() -> Result<(), String> {
    let parent = Scene::new(make_svg("container-graft-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("container-graft-child")).map_err(|e| e.to_string())?;

    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "Round", child.clone())
        .map_err(|e| e.to_string())?;

    check(parent.is_focused(), "parent stopped being focused after grafting a child")?;
    check(!child.is_focused(), "child was still focused after being grafted")?;
    check(child.is_nested(), "child did not report itself as nested after being grafted")?;
    let (reported_parent, reported_node) = child.parent().ok_or("child.parent() was None after grafting")?;
    check(
        reported_parent.is_focused(),
        "the parent Scene reported by child.parent() was not the focused one",
    )?;
    check(reported_node == node, "child.parent() reported the wrong container NodeId")?;
    check(
        visibility(&parent.inner.borrow().svg).as_deref() != Some("hidden"),
        "parent's own <svg> was hidden after grafting a child",
    )?;
    check(
        visibility(&child.inner.borrow().svg).as_deref() == Some("hidden"),
        "child's own <svg> was not hidden after being grafted",
    )
}

/// `enter`/`exit` round-trip: `enter` moves focus and DOM visibility onto the child, `exit` moves both back.
#[wasm_bindgen_test]
fn enter_and_exit_round_trip_focus_and_visibility() -> Result<(), String> {
    let parent = Scene::new(make_svg("container-round-trip-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("container-round-trip-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "Round", child.clone())
        .map_err(|e| e.to_string())?;

    let entered = parent.enter(node).map_err(|e| e.to_string())?;
    check(entered.is_focused(), "the Scene returned by enter() was not focused")?;
    check(!parent.is_focused(), "the parent was still focused after enter()")?;
    check(
        visibility(&entered.inner.borrow().svg).as_deref() != Some("hidden"),
        "the child's own <svg> was still hidden after enter()",
    )?;
    check(
        visibility(&parent.inner.borrow().svg).as_deref() == Some("hidden"),
        "the parent's own <svg> was not hidden after enter()",
    )?;

    let exited = entered.exit().map_err(|e| e.to_string())?;
    let exited = exited.ok_or("exit() from a nested child returned None")?;
    check(exited.is_focused(), "the Scene returned by exit() was not focused")?;
    check(!entered.is_focused(), "the child was still focused after exit()")?;
    check(
        visibility(&exited.inner.borrow().svg).as_deref() != Some("hidden"),
        "the parent's own <svg> was still hidden after exit()",
    )?;
    check(
        visibility(&entered.inner.borrow().svg).as_deref() == Some("hidden"),
        "the child's own <svg> was not hidden again after exit()",
    )
}

/// `exit()` on a true root does nothing and reports `None` — there is no parent to exit to.
#[wasm_bindgen_test]
fn exit_on_a_root_scene_is_a_no_op() -> Result<(), String> {
    let root = Scene::new(make_svg("container-root-exit")).map_err(|e| e.to_string())?;
    let result = root.exit().map_err(|e| e.to_string())?;
    check(result.is_none(), "exit() on a root Scene returned Some")?;
    check(root.is_focused(), "a root Scene stopped being focused after a no-op exit()")
}

/// `enter` rejects a `NodeId` that names an ordinary label node, not a container node.
#[wasm_bindgen_test]
fn enter_rejects_a_node_that_is_not_a_container() -> Result<(), String> {
    let scene = Scene::new(make_svg("container-not-a-container")).map_err(|e| e.to_string())?;
    let plain = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "Plain")
        .map_err(|e| e.to_string())?;

    let result = scene.enter(plain);
    check(
        matches!(result, Err(Error::NotAContainerNode(id)) if id == plain),
        "enter() on a plain node did not fail with NotAContainerNode",
    )
}

/// Once a container node has been entered, the parent it was entered from is no longer the tree's focused Scene. So it
/// can no longer be navigated from until control returns via `exit`.
#[wasm_bindgen_test]
fn navigation_from_a_scene_that_is_not_focused_fails() -> Result<(), String> {
    let parent = Scene::new(make_svg("container-not-focused-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("container-not-focused-child")).map_err(|e| e.to_string())?;
    let other_child = Scene::new(make_svg("container-not-focused-other-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child)
        .map_err(|e| e.to_string())?;
    let other_node = parent
        .add_container_node(Point::new(80.0, 0.0), Size::new(60.0, 40.0), "B", other_child)
        .map_err(|e| e.to_string())?;

    parent.enter(node).map_err(|e| e.to_string())?;

    check(
        matches!(parent.enter(other_node), Err(Error::NotFocused)),
        "enter() on the no-longer-focused parent did not fail with NotFocused",
    )
}

/// A container node that was never entered is not the tree's focused Scene either — only the root (or whichever Scene
/// was last entered) is. So `exit()` on a freshly grafted, un-entered child also fails with `NotFocused`.
#[wasm_bindgen_test]
fn exit_on_a_grafted_but_never_entered_child_fails_with_not_focused() -> Result<(), String> {
    let parent = Scene::new(make_svg("container-ungrafted-exit-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("container-ungrafted-exit-child")).map_err(|e| e.to_string())?;
    parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child.clone())
        .map_err(|e| e.to_string())?;

    check(
        matches!(child.exit(), Err(Error::NotFocused)),
        "exit() on an un-entered child did not fail with NotFocused",
    )
}

/// `add_container_node` rejects a child that is not currently the focused Scene of its own tree — here, one of its own
/// descendants is focused instead. Grafting only ever happens by an inactive tree's own root.
#[wasm_bindgen_test]
fn add_container_node_rejects_a_child_with_a_focused_descendant() -> Result<(), String> {
    let parent = Scene::new(make_svg("container-child-not-focused-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("container-child-not-focused-child")).map_err(|e| e.to_string())?;
    let grandchild = Scene::new(make_svg("container-child-not-focused-grandchild")).map_err(|e| e.to_string())?;
    let gc_node = child
        .add_container_node(Point::origin(), Size::new(30.0, 30.0), "GC", grandchild)
        .map_err(|e| e.to_string())?;
    child.enter(gc_node).map_err(|e| e.to_string())?;

    let result = parent.add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child);
    check(
        matches!(result, Err(Error::ChildNotFocused)),
        "grafting a child with a focused descendant did not fail with ChildNotFocused",
    )
}

/// `add_container_node` rejects a child that already has a live parent — a nested `Scene` has exactly one owner at a
/// time.
#[wasm_bindgen_test]
fn add_container_node_rejects_an_already_nested_child() -> Result<(), String> {
    let parent_a = Scene::new(make_svg("container-already-nested-a")).map_err(|e| e.to_string())?;
    let parent_b = Scene::new(make_svg("container-already-nested-b")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("container-already-nested-child")).map_err(|e| e.to_string())?;
    parent_a
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child.clone())
        .map_err(|e| e.to_string())?;

    let result = parent_b.add_container_node(Point::origin(), Size::new(60.0, 40.0), "B", child);
    check(
        matches!(result, Err(Error::AlreadyNested)),
        "attaching an already-nested child a second time did not fail with AlreadyNested",
    )
}

/// `add_container_node` rejects `self` as its own child, and rejects any of `self`'s own ancestors as a child. Both
/// would close a cycle through the strong `Rc` chain nested `Scene` ownership is built from.
#[wasm_bindgen_test]
fn add_container_node_rejects_self_and_ancestor_nesting() -> Result<(), String> {
    let a = Scene::new(make_svg("container-self-nesting-a")).map_err(|e| e.to_string())?;
    let b = Scene::new(make_svg("container-self-nesting-b")).map_err(|e| e.to_string())?;

    let self_result = a.add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", a.clone());
    check(
        matches!(self_result, Err(Error::SelfNesting)),
        "a Scene nesting itself did not fail with SelfNesting",
    )?;

    a.add_container_node(Point::new(0.0, 60.0), Size::new(60.0, 40.0), "B", b.clone())
        .map_err(|e| e.to_string())?;
    let ancestor_result = b.add_container_node(Point::origin(), Size::new(60.0, 40.0), "A-again", a);
    check(
        matches!(ancestor_result, Err(Error::SelfNesting)),
        "nesting an ancestor as its own descendant's child did not fail with SelfNesting",
    )
}

/// The scenario external review's third round asked for explicitly: a detached subtree (its own former parent dropped)
/// becomes graftable again through ordinary navigation, with no special reset operation needed.
///
/// `A → B → C`, `C` focused; drop `A`; `B` becomes the effective root, `C` stays focused. Grafting `B` under `D` fails
/// (`C`, a live descendant, is still focused, not `B`). `C.exit()` focuses `B`. Grafting `B` under `D` now succeeds.
#[wasm_bindgen_test]
fn a_detached_subtree_can_be_regrafted_once_its_own_root_is_focused_again() -> Result<(), String> {
    let a = Scene::new(make_svg("container-regraft-a")).map_err(|e| e.to_string())?;
    let b = Scene::new(make_svg("container-regraft-b")).map_err(|e| e.to_string())?;
    let c = Scene::new(make_svg("container-regraft-c")).map_err(|e| e.to_string())?;
    let d = Scene::new(make_svg("container-regraft-d")).map_err(|e| e.to_string())?;

    let b_node = a
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "B", b.clone())
        .map_err(|e| e.to_string())?;
    let c_node = b
        .add_container_node(Point::origin(), Size::new(30.0, 30.0), "C", c.clone())
        .map_err(|e| e.to_string())?;
    a.enter(b_node).map_err(|e| e.to_string())?;
    b.enter(c_node).map_err(|e| e.to_string())?;
    check(c.is_focused(), "C was not focused after entering it")?;

    drop(a);

    check(b.parent().is_none(), "B still reported a parent after A was dropped")?;
    check(
        !b.is_focused(),
        "B was focused even though C, its own descendant, is the one actually entered",
    )?;
    check(c.is_focused(), "C stopped being focused merely because A was dropped")?;

    let rejected = d.add_container_node(Point::origin(), Size::new(60.0, 40.0), "B", b.clone());
    check(
        matches!(rejected, Err(Error::ChildNotFocused)),
        "grafting B while C (its own descendant) is focused did not fail with ChildNotFocused",
    )?;

    c.exit().map_err(|e| e.to_string())?;
    check(b.is_focused(), "B was not focused after C exited back to it")?;

    d.add_container_node(Point::origin(), Size::new(60.0, 40.0), "B", b.clone())
        .map_err(|e| e.to_string())?;
    check(b.is_nested(), "B did not report itself as nested after being regrafted under D")?;
    let (reported_parent, _) = b.parent().ok_or("B.parent() was None after being regrafted")?;
    check(
        d.is_focused() == reported_parent.is_focused(),
        "B's regrafted parent did not report the same focus state as D",
    )
}
