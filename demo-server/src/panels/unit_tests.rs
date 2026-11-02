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
    let fragment = "<section class=\"section\" id=\"panel-a\" data-label=\"X &amp; Y\">\n<p x=\"no\">";
    check(
        section_attribute(fragment, "data-label").as_deref() == Some("X & Y"),
        &format!("{:?}", section_attribute(fragment, "data-label")),
    )?;
    check(section_attribute(fragment, "id").as_deref() == Some("panel-a"), "id")?;
    check(
        section_attribute(fragment, "x").is_none(),
        "an attribute of another tag is not read",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn section_attribute_reads_a_tag_that_runs_over_several_lines() -> Result<(), String> {
    let fragment = "<section\n    class=\"section\"\n    id=\"panel-a\"\n    data-label=\"A\"\n>";
    check(section_attribute(fragment, "id").as_deref() == Some("panel-a"), "id")?;
    check(section_attribute(fragment, "data-label").as_deref() == Some("A"), "data-label")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn section_attribute_does_not_mistake_a_longer_name_for_a_shorter_one() -> Result<(), String> {
    let fragment = "<section id=\"panel-a\" data-label-extra=\"wrong\" data-label=\"Right\">";
    check(
        section_attribute(fragment, "data-label").as_deref() == Some("Right"),
        &format!("{:?}", section_attribute(fragment, "data-label")),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn split_name_wants_two_digits_a_hyphen_and_a_lowercase_name() -> Result<(), String> {
    check(split_name("01-basic-nodes") == Some(("01", "basic-nodes")), "plain")?;
    check(split_name("10-sha2-256") == Some(("10", "sha2-256")), "digits in the name")?;
    check(split_name("1-basic").is_none(), "one digit")?;
    check(split_name("01basic").is_none(), "no hyphen")?;
    check(split_name("01-").is_none(), "no name")?;
    check(split_name("01--x").is_none(), "name starts with a hyphen")?;
    check(split_name("01-Basic").is_none(), "uppercase")?;
    check(split_name("é1-basic").is_none(), "not ASCII")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn heading_of_removes_hyphens_and_capitalises_all_but_the_small_words() -> Result<(), String> {
    check(heading_of("basic-nodes") == "Basic Nodes", &heading_of("basic-nodes"))?;
    check(
        heading_of("error-detection-and-correction") == "Error Detection and Correction",
        &heading_of("error-detection-and-correction"),
    )?;
    check(heading_of("sorting") == "Sorting", "one word")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn panel(id: &str, group: &str) -> Panel {
    Panel {
        id: id.to_owned(),
        group: group.to_owned(),
        label: id.to_owned(),
        path: PathBuf::new(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn render_menu_makes_a_disclosure_for_a_group_and_a_top_level_item_for_a_group_of_one() -> Result<(), String> {
    let panels = vec![
        panel("panel-a", "Basic Nodes"),
        panel("panel-b", "Basic Nodes"),
        panel("panel-c", "Nested Scenes"),
    ];
    let menu = render_menu(&panels);
    check(menu.matches("<details").count() == 1, &menu)?;
    check(menu.contains("<summary>Basic Nodes</summary>"), &menu)?;
    check(
        menu.contains("class=\"menu-item menu-item-top\" data-target=\"panel-c\">Nested Scenes<"),
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
    let panels = vec![panel("panel-a", "R & D"), panel("panel-b", "R & D")];
    let menu = render_menu(&panels);
    check(menu.contains("<summary>R &amp; D</summary>"), &menu)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes a throwaway `demo/panels/` from `files`, each a path below it and its contents, and returns what [`catalogue`]
/// makes of it.
fn catalogue_of(files: &[(&str, &str)]) -> Result<Result<Vec<Panel>, AssembleError>, String> {
    let dir = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    for (path, contents) in files {
        let path = dir.path().join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create folder: {e:?}"))?;
        }
        fs::write(path, contents).map_err(|e| format!("write fragment: {e:?}"))?;
    }
    Ok(catalogue(dir.path()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn section(id: &str) -> String {
    format!("<section class=\"section\" id=\"{id}\" data-label=\"{id} &amp; label\">")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_reads_a_well_formed_fragment() -> Result<(), String> {
    let panels = catalogue_of(&[("01-sorting/01-x.html", &section("panel-x"))])?.map_err(|e| e.to_string())?;
    check(
        panels
            == [Panel {
                id: "panel-x".to_owned(),
                group: "Sorting".to_owned(),
                label: "panel-x & label".to_owned(),
                path: PathBuf::from("01-sorting/01-x.html"),
            }],
        &format!("{panels:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_orders_by_folder_number_then_file_number_whatever_the_names() -> Result<(), String> {
    let panels = catalogue_of(&[
        ("02-alpha/01-m.html", &section("panel-m")),
        ("01-zulu/02-a.html", &section("panel-a")),
        ("01-zulu/01-z.html", &section("panel-z")),
        ("01-zulu/.DS_Store", "ignored"),
    ])?
    .map_err(|e| e.to_string())?;
    let ids: Vec<&str> = panels.iter().map(|p| p.id.as_str()).collect();
    check(ids == ["panel-z", "panel-a", "panel-m"], &format!("{ids:?}"))?;
    check(panels[2].group == "Alpha", &panels[2].group)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_a_fragment_without_its_label() -> Result<(), String> {
    match catalogue_of(&[("01-g/01-x.html", "<section class=\"section\" id=\"panel-x\">")])? {
        Err(AssembleError::MissingAttribute { attribute: "data-label", .. }) => Ok(()),
        other => Err(format!("expected MissingAttribute(data-label), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_a_group_folder_without_a_number() -> Result<(), String> {
    match catalogue_of(&[("sorting/01-x.html", &section("panel-x"))])? {
        Err(AssembleError::InvalidGroupFolder(name)) => check(name == "sorting", &name),
        other => Err(format!("expected InvalidGroupFolder, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_a_fragment_without_a_number() -> Result<(), String> {
    match catalogue_of(&[("01-g/panel-x.html", &section("panel-x"))])? {
        Err(AssembleError::InvalidFragmentName(path)) => check(path.ends_with("panel-x.html"), &format!("{path:?}")),
        other => Err(format!("expected InvalidFragmentName, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_two_fragments_with_one_number() -> Result<(), String> {
    match catalogue_of(&[("01-g/01-x.html", &section("panel-x")), ("01-g/01-y.html", &section("panel-y"))])? {
        Err(AssembleError::DuplicateNumber { .. }) => Ok(()),
        other => Err(format!("expected DuplicateNumber, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_two_group_folders_with_one_number() -> Result<(), String> {
    match catalogue_of(&[("01-g/01-x.html", &section("panel-x")), ("01-h/01-y.html", &section("panel-y"))])? {
        Err(AssembleError::DuplicateNumber { .. }) => Ok(()),
        other => Err(format!("expected DuplicateNumber, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_two_fragments_with_one_id() -> Result<(), String> {
    match catalogue_of(&[("01-g/01-x.html", &section("panel-x")), ("02-h/01-y.html", &section("panel-x"))])? {
        Err(AssembleError::DuplicatePanelId { id, .. }) => check(id == "panel-x", &id),
        other => Err(format!("expected DuplicatePanelId, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn catalogue_rejects_an_id_without_the_panel_prefix() -> Result<(), String> {
    match catalogue_of(&[("01-g/01-x.html", &section("x"))])? {
        Err(AssembleError::InvalidPanelId { id, .. }) => check(id == "x", &id),
        other => Err(format!("expected InvalidPanelId, got {other:?}")),
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
/// what actually proves `index.template.html` and every real `demo/panels/*/*.html` fragment stay wired together
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
