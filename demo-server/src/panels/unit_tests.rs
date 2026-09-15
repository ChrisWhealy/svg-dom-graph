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
fn check_unique_manifest_ids_rejects_a_duplicate() -> Result<(), String> {
    let entries = [("panel-a", "A"), ("panel-b", "B"), ("panel-a", "A again")];
    match check_unique_manifest_ids(&entries) {
        Err(AssembleError::DuplicateManifestId("panel-a")) => Ok(()),
        other => Err(format!("expected Err(DuplicateManifestId(\"panel-a\")), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn check_catalogue_consistency_rejects_a_fragment_missing_from_disk() -> Result<(), String> {
    let manifest = [("panel-a", "A"), ("panel-b", "B")];
    let fragment_ids = vec!["panel-a".to_owned()];
    match check_catalogue_consistency(&manifest, &fragment_ids) {
        Err(AssembleError::CatalogueMismatch(detail)) => check(detail.contains("panel-b"), &detail),
        other => Err(format!("expected Err(CatalogueMismatch(_)), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn check_catalogue_consistency_rejects_an_orphan_fragment() -> Result<(), String> {
    let manifest = [("panel-a", "A")];
    let fragment_ids = vec!["panel-a".to_owned(), "panel-stale".to_owned()];
    match check_catalogue_consistency(&manifest, &fragment_ids) {
        Err(AssembleError::CatalogueMismatch(detail)) => check(detail.contains("panel-stale"), &detail),
        other => Err(format!("expected Err(CatalogueMismatch(_)), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn check_catalogue_consistency_accepts_a_matching_set() -> Result<(), String> {
    let manifest = [("panel-a", "A"), ("panel-b", "B")];
    let fragment_ids = vec!["panel-b".to_owned(), "panel-a".to_owned()];
    match check_catalogue_consistency(&manifest, &fragment_ids) {
        Ok(()) => Ok(()),
        other => Err(format!("expected Ok(()), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The end-to-end check: `assemble` against the real project's own `demo/` directory, not a synthetic fixture —
/// this is what actually proves `index.template.html` and every real `demo/panels/*.html` fragment stay wired
/// together correctly, not just that each phase's own unit test above passes in isolation.
#[test]
fn assemble_produces_the_real_projects_index_html() -> Result<(), String> {
    let root = workspace_root()?;
    let source_demo_dir = root.join("demo");

    let out_dir = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let out_path = out_dir.path().join("index.html");

    assemble(&source_demo_dir, &out_path).map_err(|e| format!("assemble failed against the real project: {e}"))?;

    let html = fs::read_to_string(&out_path).map_err(|e| format!("read assembled index.html: {e:?}"))?;
    for &(id, _) in MANIFEST {
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
