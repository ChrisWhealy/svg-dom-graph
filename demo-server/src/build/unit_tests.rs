use super::*;

/// The workspace root — `demo-server`'s own parent directory.
fn workspace_root() -> Result<PathBuf, String> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "demo-server has a parent directory".to_owned())?
        .to_path_buf())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn prepare_stage_copies_the_real_index_html_verbatim() -> Result<(), String> {
    let root = workspace_root()?;
    let expected = fs::read_to_string(root.join("index.html")).map_err(|e| format!("read source index.html: {e:?}"))?;

    let stage_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let stage = StagePaths::new(stage_root.path());

    prepare_stage(&root, &stage).map_err(|e| format!("prepare_stage failed against the real project: {e}"))?;

    let staged =
        fs::read_to_string(stage.stage_dir.join("index.html")).map_err(|e| format!("read staged index.html: {e:?}"))?;
    if staged != expected {
        return Err("staged index.html does not match the source file verbatim".to_owned());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn prepare_stage_creates_a_stage_dir_that_does_not_already_exist() -> Result<(), String> {
    let root = workspace_root()?;
    let stage_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    // A fresh, not-yet-created subdirectory — proves `prepare_stage` creates it, rather than merely tolerating one
    // that already exists.
    let stage = StagePaths::new(&stage_root.path().join("nested").join("target"));

    prepare_stage(&root, &stage).map_err(|e| format!("prepare_stage failed: {e}"))?;

    if !stage.stage_dir.join("index.html").is_file() {
        return Err("expected index.html to exist under the newly created stage dir".to_owned());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn prepare_stage_reports_a_missing_source_index_html() -> Result<(), String> {
    let stage_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let stage = StagePaths::new(stage_root.path());
    // `stage_root` itself has no `index.html`, so using it as both "root" and stage target is a convenient way to
    // point `prepare_stage` at a source directory that definitely does not have one.
    let fake_root = stage_root.path().join("no-such-root");

    match prepare_stage(&fake_root, &stage) {
        Err(BuildError::CopyIndexHtml { .. }) => Ok(()),
        other => Err(format!("expected Err(BuildError::CopyIndexHtml), got {other:?}")),
    }
}
