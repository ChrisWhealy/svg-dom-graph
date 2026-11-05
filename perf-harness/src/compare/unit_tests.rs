use super::*;
use crate::manifest::ViewPolicy;
use crate::model::Model;
use crate::ops::Op;

fn node(key: &str, x: i64, y: i64) -> Op {
    Op::AddNode {
        key: key.into(),
        x,
        y,
        w: 80,
        h: 36,
        label: key.to_uppercase(),
    }
}

fn checkpoint(ops: &[Op]) -> Result<Checkpoint, String> {
    let mut model = Model::new();
    model.apply(&Op::CreateScene)?;
    for op in ops {
        model.apply(op)?;
    }
    model.checkpoint(ViewPolicy::Unchanged)
}

fn pair() -> Result<Checkpoint, String> {
    checkpoint(&[
        node("a", 0, 0),
        node("b", 200, 100),
        Op::AddEdge {
            from: "a".into(),
            to: "b".into(),
        },
    ])
}

fn sections(found: &[Difference]) -> Vec<&'static str> {
    found.iter().map(|d| d.section).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn identical_checkpoints_agree() -> Result<(), String> {
    let found = compare(&pair()?, &pair()?);
    if found.is_empty() {
        Ok(())
    } else {
        Err(found.iter().map(Difference::summary).collect::<Vec<_>>().join("; "))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two values 0.0002 apart are within the 0.01 tolerance, although rounding them to two places would split them.
#[test]
fn geometry_is_compared_within_the_tolerance_and_not_by_rounding() -> Result<(), String> {
    let mut expected = pair()?;
    let mut actual = pair()?;
    for (checkpoint, value) in [(&mut expected, 10.0051), (&mut actual, 10.0049)] {
        let connector = checkpoint
            .geometry
            .elements
            .iter_mut()
            .find(|e| e.kind == "connector_path")
            .ok_or("no connector")?;
        connector.coordinates[1] = value;
    }
    if !compare(&expected, &actual).is_empty() {
        return Err("values 0.0002 apart were reported as different".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_geometry_difference_names_the_element_the_coordinate_both_values_and_the_distance() -> Result<(), String> {
    let expected = pair()?;
    let mut actual = pair()?;
    actual
        .geometry
        .elements
        .iter_mut()
        .find(|e| e.key == "b")
        .ok_or("no b")?
        .coordinates[1] += 3.0;
    let found = compare(&expected, &actual);
    let [difference] = found.as_slice() else {
        return Err(format!("{} differences", found.len()));
    };
    if difference.section != "geometry"
        || difference.element.as_deref() != Some("b")
        || !difference.detail.contains("node_box y")
        || difference.abs_diff != Some(3.0)
        || difference.basis != Basis::Modelled
    {
        return Err(format!("{difference:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_difference_is_modelled_and_an_unconfirmed_rule_would_say_so() -> Result<(), String> {
    let expected = pair()?;
    let mut actual = pair()?;
    for element in &mut actual.geometry.elements {
        element.coordinates[0] += 1.0;
    }
    let found = compare(&expected, &actual);
    // Two node boxes and one connector, every one of them modelled, since Phase 2 confirmed the rules for them.
    if found.len() != 3 || found.iter().any(|d| d.basis != Basis::Modelled) {
        return Err(format!("{:?}", found.iter().map(Difference::summary).collect::<Vec<_>>()));
    }
    // A provisional difference says so in its summary, so that nobody mistakes it for a settled fact.
    let mut provisional = found[0].clone();
    provisional.basis = Basis::Provisional;
    if !provisional.summary().contains("provisional rule") || found[0].summary().contains("provisional") {
        return Err("only a provisional difference may say so".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn missing_and_unexpected_nodes_edges_and_elements_are_each_reported() -> Result<(), String> {
    let expected = pair()?;
    let fewer = checkpoint(&[node("a", 0, 0)])?;
    let found = compare(&expected, &fewer);
    let kinds: Vec<String> = found.iter().map(|d| d.detail.clone()).collect();
    for want in [
        "a node is missing",
        "an edge is missing",
        "a node_box is missing",
        "a connector_path is missing",
    ] {
        if !kinds.iter().any(|k| k == want) {
            return Err(format!("no {want:?} in {kinds:?}"));
        }
    }
    let reversed = compare(&fewer, &expected);
    for want in [
        "an unexpected node",
        "an unexpected edge",
        "an unexpected node_box",
        "an unexpected connector_path",
    ] {
        if !reversed.iter().any(|d| d.detail == want) {
            return Err(format!("no {want:?} the other way round"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn edge_order_does_not_matter_and_node_order_matters_only_for_paint() -> Result<(), String> {
    let (e1, e2) = (
        Op::AddEdge {
            from: "a".into(),
            to: "b".into(),
        },
        Op::AddEdge {
            from: "b".into(),
            to: "a".into(),
        },
    );
    let one = checkpoint(&[node("a", 0, 0), node("b", 200, 0), e1.clone(), e2.clone()])?;
    let other = checkpoint(&[node("a", 0, 0), node("b", 200, 0), e2, e1])?;
    if !compare(&one, &other).is_empty() {
        return Err("a different order of edges was reported".to_string());
    }
    let swapped = checkpoint(&[node("b", 200, 0), node("a", 0, 0)])?;
    let plain = checkpoint(&[node("a", 0, 0), node("b", 200, 0)])?;
    let found = compare(&plain, &swapped);
    if sections(&found) != ["paint_order"] || found[0].basis != Basis::Modelled {
        return Err(format!("{:?}", found.iter().map(Difference::summary).collect::<Vec<_>>()));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn content_view_focus_selection_and_duplicate_ids_are_each_checked() -> Result<(), String> {
    let expected = pair()?;
    let mut actual = pair()?;
    actual.content.insert("a".into(), "wrong".into());
    actual.interaction.view = Some(crate::model::View { x: 5.0, y: 0.0, scale: 1.0 });
    actual.interaction.focused = Some("a".into());
    actual.interaction.selection = vec!["b".into()];
    actual.dom_identity.duplicates = vec!["marker".into()];
    let found = compare(&expected, &actual);
    let have = |section: &str, detail: &str| found.iter().any(|d| d.section == section && d.detail.contains(detail));
    for (section, detail) in [
        ("content", "text"),
        ("interaction", "x"),
        ("interaction", "focused"),
        ("interaction", "selection"),
        ("dom_identity", "ids used more than once"),
    ] {
        if !have(section, detail) {
            return Err(format!(
                "no {section} difference about {detail}: {:?}",
                found.iter().map(Difference::summary).collect::<Vec<_>>()
            ));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_coordinate_that_is_not_a_number_is_always_a_difference() -> Result<(), String> {
    let expected = pair()?;
    let mut actual = pair()?;
    actual.geometry.elements[0].coordinates[0] = f64::NAN;
    if compare(&expected, &actual).is_empty() {
        return Err("a NaN was within tolerance".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_accessibility_tree_and_the_ids_themselves_are_not_compared() -> Result<(), String> {
    let expected = pair()?;
    let mut actual = pair()?;
    actual.accessibility.elements.push(crate::model::AccessibleElement {
        key: "a".into(),
        role: "group".into(),
        name: "A".into(),
    });
    actual.dom_identity.ids = vec!["svg-dom-graph-arrow-7".into()];
    if compare(&expected, &actual).is_empty() {
        Ok(())
    } else {
        Err("the accessibility tree or the ids were compared".to_string())
    }
}
