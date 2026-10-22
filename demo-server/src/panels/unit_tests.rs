use super::*;
use std::path::PathBuf;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn workspace_root() -> Result<PathBuf, String> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "demo-server has a parent directory".to_owned())?
        .to_path_buf())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn section_attribute_reads_a_value_and_undoes_the_escapes() -> Result<(), String> {
    let fragment = "<section class=\"section\" id=\"panel-a\" data-group=\"Encoding &amp; Compression\" \
                    data-order=\"20\">\n<p x=\"no\">";
    check(
        section_attribute(fragment, "data-group").as_deref() == Some("Encoding & Compression"),
        &format!("{:?}", section_attribute(fragment, "data-group")),
    )?;
    check(section_attribute(fragment, "data-order").as_deref() == Some("20"), "data-order")?;
    check(
        section_attribute(fragment, "data-label").is_none(),
        "an absent attribute is None",
    )?;
    check(
        section_attribute(fragment, "x").is_none(),
        "an attribute of another tag is not read",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn section_attribute_does_not_mistake_a_longer_name_for_a_shorter_one() -> Result<(), String> {
    let fragment = "<section id=\"panel-a\" data-group-extra=\"wrong\" data-group=\"Right\">";
    check(
        section_attribute(fragment, "data-group").as_deref() == Some("Right"),
        &format!("{:?}", section_attribute(fragment, "data-group")),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn panel(id: &str, group: &str, order: u32) -> Panel {
    Panel {
        id: id.to_owned(),
        group: group.to_owned(),
        label: id.to_owned(),
        order,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn sort_into_menu_order_goes_by_group_then_order_then_id() -> Result<(), String> {
    let mut panels = vec![
        panel("panel-z", "Sorting", 10),
        panel("panel-b", "Basic Nodes", 20),
        panel("panel-d", "Sorting", 5),
        panel("panel-a", "Basic Nodes", 20),
        panel("panel-c", "Basic Nodes", 10),
    ];
    sort_into_menu_order(&mut panels);
    let ids: Vec<&str> = panels.iter().map(|p| p.id.as_str()).collect();
    check(
        ids == ["panel-c", "panel-a", "panel-b", "panel-d", "panel-z"],
        &format!("{ids:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn render_menu_makes_a_disclosure_for_a_group_and_a_top_level_item_for_a_group_of_one() -> Result<(), String> {
    let panels = vec![
        panel("panel-a", "Basic Nodes", 10),
        panel("panel-b", "Basic Nodes", 20),
        panel("panel-c", "Nested scenes", 10),
    ];
    let menu = render_menu(&panels);
    check(menu.matches("<details").count() == 1, &menu)?;
    check(menu.contains("<summary>Basic Nodes</summary>"), &menu)?;
    check(
        menu.contains("class=\"menu-item menu-item-top\" data-target=\"panel-c\">Nested scenes<"),
        &menu,
    )?;
    check(
        !menu.contains("panel-c</button>"),
        "a group of one shows the group's label, not the panel's",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn render_menu_escapes_the_ampersand_in_a_group_heading() -> Result<(), String> {
    let panels = vec![
        panel("panel-a", "Encoding & Compression", 10),
        panel("panel-b", "Encoding & Compression", 20),
    ];
    let menu = render_menu(&panels);
    check(menu.contains("<summary>Encoding &amp; Compression</summary>"), &menu)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `contents` as `panel-x.html` in a throwaway directory and returns what [`catalogue`] makes of the directory.
fn catalogue_of(contents: &str) -> Result<Result<Vec<Panel>, AssembleError>, String> {
    let dir = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    fs::write(dir.path().join("panel-x.html"), contents).map_err(|e| format!("write fragment: {e:?}"))?;
    Ok(catalogue(dir.path()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_reads_a_well_formed_fragment() -> Result<(), String> {
    let panels = catalogue_of(
        "<section class=\"section\" id=\"panel-x\" data-group=\"Sorting\" data-label=\"X &amp; Y\" data-order=\"30\">",
    )?
    .map_err(|e| e.to_string())?;
    check(
        panels
            == [Panel {
                id: "panel-x".to_owned(),
                group: "Sorting".to_owned(),
                label: "X & Y".to_owned(),
                order: 30,
            }],
        &format!("{panels:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_a_fragment_without_its_metadata() -> Result<(), String> {
    match catalogue_of("<section class=\"section\" id=\"panel-x\">")? {
        Err(AssembleError::MissingAttribute { attribute: "data-group", .. }) => Ok(()),
        other => Err(format!("expected MissingAttribute(data-group), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_an_order_that_is_not_a_number() -> Result<(), String> {
    match catalogue_of("<section id=\"panel-x\" data-group=\"Sorting\" data-label=\"X\" data-order=\"first\">")? {
        Err(AssembleError::InvalidOrder { value, .. }) => check(value == "first", &value),
        other => Err(format!("expected InvalidOrder, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_a_group_that_is_not_in_the_menu() -> Result<(), String> {
    match catalogue_of("<section id=\"panel-x\" data-group=\"Nonesuch\" data-label=\"X\" data-order=\"10\">")? {
        Err(AssembleError::UnknownGroup { group, .. }) => check(group == "Nonesuch", &group),
        other => Err(format!("expected UnknownGroup, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_a_fragment_whose_id_is_not_its_filename() -> Result<(), String> {
    match catalogue_of("<section id=\"panel-other\" data-group=\"Sorting\" data-label=\"X\" data-order=\"10\">")? {
        Err(AssembleError::FragmentIdMismatch { id, .. }) => check(id == "panel-x", &id),
        other => Err(format!("expected FragmentIdMismatch, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn is_valid_panel_id_wants_the_prefix_and_lowercase() -> Result<(), String> {
    check(is_valid_panel_id("panel-tree-2"), "panel-tree-2")?;
    check(!is_valid_panel_id("not-a-panel"), "no prefix")?;
    check(!is_valid_panel_id("panel-Foo"), "uppercase")?;
    check(!is_valid_panel_id("panel-"), "empty rest")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn check_no_leftover_placeholders_rejects_an_unresolved_token() -> Result<(), String> {
    match check_no_leftover_placeholders("<body>{{OOPS}}</body>") {
        Err(AssembleError::LeftoverPlaceholder(context)) => check(context.contains("{{OOPS}}"), &context),
        other => Err(format!("expected Err(LeftoverPlaceholder(_)), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn check_no_leftover_placeholders_accepts_clean_output() -> Result<(), String> {
    match check_no_leftover_placeholders("<body>all resolved</body>") {
        Ok(()) => Ok(()),
        other => Err(format!("expected Ok(()), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The end-to-end check: `assemble` against the real project's own `demo/` directory, not a synthetic fixture. This is
/// what actually proves `index.template.html` and every real `demo/panels/*.html` fragment stay wired together
/// correctly. It does not merely prove that each phase's own unit test above passes in isolation.
#[test]
fn assemble_produces_the_real_projects_index_html() -> Result<(), String> {
    let root = workspace_root()?;
    let source_demo_dir = root.join("demo");

    let out_dir = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let out_path = out_dir.path().join("index.html");

    assemble(&source_demo_dir, &out_path).map_err(|e| format!("assemble failed against the real project: {e}"))?;

    let html = fs::read_to_string(&out_path).map_err(|e| format!("read assembled index.html: {e:?}"))?;
    for id in panel_ids(&source_demo_dir.join("panels")).map_err(|e| e.to_string())? {
        check(
            html.contains(&format!("id=\"{id}\"")),
            &format!("assembled index.html is missing {id:?}"),
        )?;
    }
    check(
        !html.contains("{{"),
        "assembled index.html still contains an unresolved placeholder",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}
