use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn workspace_root() -> Result<PathBuf, String> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "demo-server has a parent directory".to_owned())?
        .to_path_buf())
}

fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn extract_gallery_panel_ids_finds_every_entry() -> Result<(), String> {
    let src = r#"
        demo_gallery! {
            "panel-a" => module::a,
            "panel-b" => module::b,
        }
    "#;
    let ids = extract_gallery_panel_ids(src);
    check(ids == vec!["panel-a".to_owned(), "panel-b".to_owned()], &format!("{ids:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn extract_gallery_panel_ids_ignores_a_mention_in_a_doc_comment() -> Result<(), String> {
    // A doc comment mentioning `demo_gallery!` by name, not immediately followed by `{`, must not be mistaken for
    // the real invocation.
    let src = r#"
        /// See demo_gallery! below for the real list, e.g. "panel-fake" => module::fake.
        demo_gallery! {
            "panel-real" => module::real,
        }
    "#;
    let ids = extract_gallery_panel_ids(src);
    check(ids == vec!["panel-real".to_owned()], &format!("{ids:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn find_duplicate_gallery_id_finds_a_repeated_id() -> Result<(), String> {
    let ids = vec!["panel-a".to_owned(), "panel-b".to_owned(), "panel-a".to_owned()];
    check(
        find_duplicate_gallery_id(&ids) == Some("panel-a"),
        "expected to find the duplicate",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn find_duplicate_gallery_id_returns_none_for_a_unique_list() -> Result<(), String> {
    let ids = vec!["panel-a".to_owned(), "panel-b".to_owned()];
    check(find_duplicate_gallery_id(&ids).is_none(), "expected no duplicate")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The end-to-end check: `validate` against the real project's own `demo-app/src/lib.rs` and
/// `demo-server/src/panels/mod.rs`'s real `MANIFEST` — this is what actually proves the two catalogues are wired
/// together correctly today, not just that each phase's own unit test above passes against synthetic input.
#[test]
fn validate_accepts_the_real_project() -> Result<(), String> {
    let root = workspace_root()?;
    validate(&root).map_err(|e| format!("validate failed against the real project: {e}"))
}
