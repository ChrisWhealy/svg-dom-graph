//! `Scene::replace_container_child`

use super::support::{FailingWrites, make_svg, visibility};
use crate::{error::Error, scene::Scene, test_support::check};
use svg_dom::root::utils::{Point, Size};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ordinary case: `new_child` takes over `node`'s own slot, and the old child comes back visible, focused, and no
/// longer nested — indistinguishable from a freshly constructed, standalone `Scene`.
#[wasm_bindgen_test]
fn replace_container_child_swaps_the_child_and_returns_the_old_one_visible_and_focused() -> Result<(), String> {
    let parent = Scene::new(make_svg("replace-swap-parent")).map_err(|e| e.to_string())?;
    let old_child = Scene::new(make_svg("replace-swap-old")).map_err(|e| e.to_string())?;
    let new_child = Scene::new(make_svg("replace-swap-new")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", old_child.clone())
        .map_err(|e| e.to_string())?;

    let returned = parent
        .replace_container_child(node, new_child.clone())
        .map_err(|e| e.to_string())?;

    check(returned.is_focused(), "the returned old child was not focused")?;
    check(!returned.is_nested(), "the returned old child still reported itself as nested")?;
    check(
        visibility(&returned.inner.borrow().svg).as_deref() != Some("hidden"),
        "the returned old child's own <svg> was still hidden",
    )?;
    // The returned handle and `old_child` share the same underlying Scene: mutating through one is visible through the
    // other.
    check(
        old_child.is_focused(),
        "old_child itself did not report the same focus state as the returned handle",
    )?;

    check(
        new_child.is_nested(),
        "new_child did not report itself as nested after replacing the old one",
    )?;
    check(!new_child.is_focused(), "new_child was still focused after being grafted in")?;
    check(
        visibility(&new_child.inner.borrow().svg).as_deref() == Some("hidden"),
        "new_child's own <svg> was not hidden after being grafted in",
    )?;

    let (reported_parent, reported_node) = new_child.parent().ok_or("new_child.parent() was None after replacing")?;
    check(
        reported_parent.is_focused(),
        "new_child's own reported parent was not the focused Scene",
    )?;
    check(reported_node == node, "new_child.parent() reported the wrong container NodeId")?;

    // The container node's own slot now genuinely holds new_child, not old_child — entering it shows new_child.
    let entered = parent.enter(node).map_err(|e| e.to_string())?;
    check(
        visibility(&entered.inner.borrow().svg).as_deref() != Some("hidden"),
        "entering the container after replacement did not show new_child",
    )?;
    check(
        visibility(&old_child.inner.borrow().svg).as_deref() != Some("hidden"),
        "the detached old child was hidden again merely because the container was entered",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checked first: replacing a child while `self` is not the tree's currently focused Scene fails, and touches nothing.
#[wasm_bindgen_test]
fn replace_container_child_rejects_when_self_is_not_focused() -> Result<(), String> {
    let parent = Scene::new(make_svg("replace-not-focused-parent")).map_err(|e| e.to_string())?;
    let old_child = Scene::new(make_svg("replace-not-focused-old")).map_err(|e| e.to_string())?;
    let new_child = Scene::new(make_svg("replace-not-focused-new")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", old_child.clone())
        .map_err(|e| e.to_string())?;
    parent.enter(node).map_err(|e| e.to_string())?;

    let result = parent.replace_container_child(node, new_child.clone());
    check(
        matches!(result, Err(Error::NotFocused)),
        "replacing a child while self is not focused did not fail with NotFocused",
    )?;
    check(!new_child.is_nested(), "the rejected call grafted new_child in anyway")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `node` must actually name a node in this scene.
#[wasm_bindgen_test]
fn replace_container_child_rejects_an_unknown_node() -> Result<(), String> {
    let scene_a = Scene::new(make_svg("replace-unknown-a")).map_err(|e| e.to_string())?;
    let scene_b = Scene::new(make_svg("replace-unknown-b")).map_err(|e| e.to_string())?;
    let new_child = Scene::new(make_svg("replace-unknown-new")).map_err(|e| e.to_string())?;
    let foreign_node = scene_b
        .add_node(Point::origin(), Size::new(40.0, 20.0), "Foreign")
        .map_err(|e| e.to_string())?;

    let result = scene_a.replace_container_child(foreign_node, new_child);
    check(
        matches!(result, Err(Error::UnknownNode(id)) if id == foreign_node),
        "replacing a child of a foreign NodeId did not fail with UnknownNode",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `node` must name a container node — there is no existing child for `new_child` to replace otherwise.
#[wasm_bindgen_test]
fn replace_container_child_rejects_a_non_container_node() -> Result<(), String> {
    let scene = Scene::new(make_svg("replace-non-container")).map_err(|e| e.to_string())?;
    let plain = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "Plain")
        .map_err(|e| e.to_string())?;
    let new_child = Scene::new(make_svg("replace-non-container-new")).map_err(|e| e.to_string())?;

    let result = scene.replace_container_child(plain, new_child);
    check(
        matches!(result, Err(Error::NotAContainerNode(id)) if id == plain),
        "replacing a plain node's own (nonexistent) child did not fail with NotAContainerNode",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `new_child` cannot be `self`, or any of `self`'s own ancestors — both would close a cycle, the same reasoning
/// `add_container_node`'s own `SelfNesting` rejection already documents.
#[wasm_bindgen_test]
fn replace_container_child_rejects_self_and_ancestor_nesting() -> Result<(), String> {
    let a = Scene::new(make_svg("replace-self-nesting-a")).map_err(|e| e.to_string())?;
    let b = Scene::new(make_svg("replace-self-nesting-b")).map_err(|e| e.to_string())?;
    let old_under_a = Scene::new(make_svg("replace-self-nesting-old-a")).map_err(|e| e.to_string())?;
    let old_under_b = Scene::new(make_svg("replace-self-nesting-old-b")).map_err(|e| e.to_string())?;

    let node_a = a
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", old_under_a)
        .map_err(|e| e.to_string())?;
    let self_result = a.replace_container_child(node_a, a.clone());
    check(
        matches!(self_result, Err(Error::SelfNesting)),
        "replacing a child with self did not fail with SelfNesting",
    )?;

    let b_node = a
        .add_container_node(Point::new(0.0, 60.0), Size::new(60.0, 40.0), "B", b.clone())
        .map_err(|e| e.to_string())?;
    let node_b = b
        .add_container_node(Point::origin(), Size::new(30.0, 30.0), "C", old_under_b)
        .map_err(|e| e.to_string())?;
    // `b` must be focused for `replace_container_child` to reach the SelfNesting check at all — enter it first.
    a.enter(b_node).map_err(|e| e.to_string())?;
    let ancestor_result = b.replace_container_child(node_b, a);
    check(
        matches!(ancestor_result, Err(Error::SelfNesting)),
        "replacing a child with an ancestor did not fail with SelfNesting",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `new_child` must not already have a live parent elsewhere — a nested `Scene` has exactly one owner at a time.
#[wasm_bindgen_test]
fn replace_container_child_rejects_an_already_nested_new_child() -> Result<(), String> {
    let parent_a = Scene::new(make_svg("replace-already-nested-a")).map_err(|e| e.to_string())?;
    let parent_b = Scene::new(make_svg("replace-already-nested-b")).map_err(|e| e.to_string())?;
    let old_under_b = Scene::new(make_svg("replace-already-nested-old-b")).map_err(|e| e.to_string())?;
    let already_nested = Scene::new(make_svg("replace-already-nested-child")).map_err(|e| e.to_string())?;

    parent_a
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", already_nested.clone())
        .map_err(|e| e.to_string())?;
    let node_b = parent_b
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "B", old_under_b)
        .map_err(|e| e.to_string())?;

    let result = parent_b.replace_container_child(node_b, already_nested);
    check(
        matches!(result, Err(Error::AlreadyNested)),
        "replacing a child with one already nested elsewhere did not fail with AlreadyNested",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Replacing a container node's child with itself is rejected, deliberately via `AlreadyNested` and not a dedicated
/// check. The old child already counts as its own live parent (`self`) at the point this is checked, since it has not
/// been detached yet.
#[wasm_bindgen_test]
fn replace_container_child_rejects_replacing_a_child_with_itself() -> Result<(), String> {
    let parent = Scene::new(make_svg("replace-with-itself-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("replace-with-itself-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child.clone())
        .map_err(|e| e.to_string())?;

    let result = parent.replace_container_child(node, child);
    check(
        matches!(result, Err(Error::AlreadyNested)),
        "replacing a child with itself did not fail with AlreadyNested",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `new_child` must be the currently focused Scene of its own tree — here, one of its own descendants is focused
/// instead.
#[wasm_bindgen_test]
fn replace_container_child_rejects_a_new_child_with_a_focused_descendant() -> Result<(), String> {
    let parent = Scene::new(make_svg("replace-child-not-focused-parent")).map_err(|e| e.to_string())?;
    let old_child = Scene::new(make_svg("replace-child-not-focused-old")).map_err(|e| e.to_string())?;
    let new_child = Scene::new(make_svg("replace-child-not-focused-new")).map_err(|e| e.to_string())?;
    let grandchild = Scene::new(make_svg("replace-child-not-focused-grandchild")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", old_child)
        .map_err(|e| e.to_string())?;
    let gc_node = new_child
        .add_container_node(Point::origin(), Size::new(30.0, 30.0), "GC", grandchild)
        .map_err(|e| e.to_string())?;
    new_child.enter(gc_node).map_err(|e| e.to_string())?;

    let result = parent.replace_container_child(node, new_child);
    check(
        matches!(result, Err(Error::ChildNotFocused)),
        "replacing with a child whose own descendant is focused did not fail with ChildNotFocused",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The detached old child gets a genuinely fresh, independent `NavigationState` — not merely unlinked from its former
/// parent. Regrafting it elsewhere would not distinguish the two. `add_container_node`'s own `repoint_subtree` call
/// repoints *any* child it is handed, fresh state or not. So that alone cannot prove `replace_container_child` itself
/// already gave it one. The real test is before any regraft. If the detached child still secretly shared its former
/// parent's `NavigationState`, moving that parent's own focus elsewhere would move the detached child's own reported
/// focus too. They would still be reading the same shared `focused` pointer.
#[wasm_bindgen_test]
fn replace_container_child_gives_the_detached_child_a_fresh_independent_navigation_state() -> Result<(), String> {
    let parent = Scene::new(make_svg("replace-fresh-nav-parent")).map_err(|e| e.to_string())?;
    let old_child = Scene::new(make_svg("replace-fresh-nav-old")).map_err(|e| e.to_string())?;
    let new_child = Scene::new(make_svg("replace-fresh-nav-new")).map_err(|e| e.to_string())?;
    let sibling_child = Scene::new(make_svg("replace-fresh-nav-sibling")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", old_child.clone())
        .map_err(|e| e.to_string())?;
    let sibling_node = parent
        .add_container_node(Point::new(80.0, 0.0), Size::new(60.0, 40.0), "B", sibling_child)
        .map_err(|e| e.to_string())?;

    let detached = parent.replace_container_child(node, new_child).map_err(|e| e.to_string())?;
    check(
        detached.is_focused(),
        "the detached child was not focused right after being replaced",
    )?;

    parent.enter(sibling_node).map_err(|e| e.to_string())?;
    check(
        detached.is_focused(),
        "entering an unrelated sibling under parent stole focus from the already-detached child — it is still \
         sharing parent's own NavigationState",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The recursive half of the previous test. `detach_subtree` calls `repoint_subtree`, which walks the *whole* subtree,
/// not just its own root. So a descendant of the detached child, not only the detached child itself, must end up
/// sharing its fresh `NavigationState` too. `parent → old_child → grandchild`, with `old_child` (not `grandchild`)
/// focused within its own tree before the replacement. After detaching `old_child`, it is focused and `grandchild` is
/// not. Entering `grandchild` from `old_child` succeeds and focuses it, without disturbing `parent`'s own, now entirely
/// separate, tree. Exiting `grandchild` returns focus to `old_child`.
#[wasm_bindgen_test]
fn replace_container_child_migrates_every_descendant_onto_the_detached_childs_own_navigation_state()
-> Result<(), String> {
    let parent = Scene::new(make_svg("replace-descendants-parent")).map_err(|e| e.to_string())?;
    let old_child = Scene::new(make_svg("replace-descendants-old")).map_err(|e| e.to_string())?;
    let grandchild = Scene::new(make_svg("replace-descendants-grandchild")).map_err(|e| e.to_string())?;
    let new_child = Scene::new(make_svg("replace-descendants-new")).map_err(|e| e.to_string())?;

    let grandchild_node = old_child
        .add_container_node(Point::origin(), Size::new(30.0, 30.0), "GC", grandchild.clone())
        .map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", old_child.clone())
        .map_err(|e| e.to_string())?;

    let detached = parent.replace_container_child(node, new_child).map_err(|e| e.to_string())?;
    check(detached.is_focused(), "old_child was not focused right after being detached")?;
    check(
        !grandchild.is_focused(),
        "grandchild was already focused right after old_child was detached",
    )?;

    let entered = detached.enter(grandchild_node).map_err(|e| e.to_string())?;
    check(entered.is_focused(), "the Scene returned by enter() was not focused")?;
    check(
        grandchild.is_focused(),
        "grandchild did not report itself as focused after being entered",
    )?;
    check(
        !detached.is_focused(),
        "old_child was still focused after entering its own grandchild",
    )?;
    check(
        parent.is_focused(),
        "entering grandchild within the detached tree disturbed parent's own, unrelated tree",
    )?;

    let exited = entered.exit().map_err(|e| e.to_string())?;
    let exited = exited.ok_or("exit() from grandchild returned None")?;
    check(exited.is_focused(), "the Scene returned by exit() was not focused")?;
    check(
        detached.is_focused(),
        "old_child was not focused again after grandchild exited back to it",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `replace_container_child`'s own documented transactional guarantee, forced open. Every precondition has already
/// passed by the time `hide_root(&new_child...)` runs, so that is the one DOM write left that can still fail. At that
/// point neither scene's own model has been touched yet. A failure there has nothing to roll back, only something to
/// not do. Forces exactly that write to fail, then proves both scenes are left exactly as they were. The original child
/// stays attached, focused-through-`parent`, and hidden. `new_child` stays independent, focused, and visible.
/// `parent.enter(node)` still shows the original child.
#[wasm_bindgen_test]
fn a_failed_hide_of_new_childs_root_leaves_both_scenes_exactly_as_they_were() -> Result<(), String> {
    let parent = Scene::new(make_svg("replace-failed-hide-parent")).map_err(|e| e.to_string())?;
    let old_child = Scene::new(make_svg("replace-failed-hide-old")).map_err(|e| e.to_string())?;
    let new_child = Scene::new(make_svg("replace-failed-hide-new")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", old_child.clone())
        .map_err(|e| e.to_string())?;

    let result = {
        let _failing = FailingWrites::start(&["visibility"])?;
        parent.replace_container_child(node, new_child.clone())
    };
    check(
        matches!(result, Err(Error::Svg(_))),
        "replace_container_child with a failing visibility write did not fail with Error::Svg",
    )?;

    // The original child is still exactly where it was: attached, focused through parent, hidden.
    check(
        old_child.is_nested(),
        "the original child was detached despite the rejected call",
    )?;
    let (reported_parent, reported_node) = old_child
        .parent()
        .ok_or("old_child.parent() was None after the rejected call")?;
    check(
        reported_parent.is_focused(),
        "old_child's own reported parent was not the focused Scene after the rejected call",
    )?;
    check(
        reported_node == node,
        "old_child.parent() reported the wrong container NodeId after the rejected call",
    )?;
    check(
        visibility(&old_child.inner.borrow().svg).as_deref() == Some("hidden"),
        "the original child was made visible despite the rejected call",
    )?;

    // `new_child` is exactly as it was too: independent, focused, visible.
    check(!new_child.is_nested(), "new_child was grafted in despite the rejected call")?;
    check(
        new_child.is_focused(),
        "new_child stopped being focused despite the rejected call",
    )?;
    check(
        visibility(&new_child.inner.borrow().svg).as_deref() != Some("hidden"),
        "new_child was hidden despite the rejected call",
    )?;

    // Entering the container still reaches the original child, not new_child.
    let entered = parent.enter(node).map_err(|e| e.to_string())?;
    check(
        visibility(&entered.inner.borrow().svg).as_deref() != Some("hidden"),
        "entering the container after the rejected call did not show the original child",
    )
}
