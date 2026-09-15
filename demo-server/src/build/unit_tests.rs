use super::*;

/// The workspace root — `demo-server`'s own parent directory.
fn workspace_root() -> Result<PathBuf, String> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "demo-server has a parent directory".to_owned())?
        .to_path_buf())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// prepare_stage — the end-to-end staging check: everything build_demo does except the wasm rebuild, run against
// the real project. This is what actually proves catalogue validation, template assembly, and asset copying stay
// wired together correctly as one pipeline, not just that each phase's own unit tests (in panels::unit_tests and
// validate::unit_tests) pass in isolation.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn prepare_stage_assembles_the_real_projects_index_html() -> Result<(), String> {
    let root = workspace_root()?;
    let stage_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let stage = StagePaths::new(stage_root.path());

    prepare_stage(&root, &stage).map_err(|e| format!("prepare_stage failed against the real project: {e}"))?;

    let html =
        fs::read_to_string(stage.stage_dir.join("index.html")).map_err(|e| format!("read staged index.html: {e:?}"))?;
    if !html.contains(r#"id="panel-tree""#) {
        return Err("staged index.html is missing a known real panel".to_owned());
    }
    if html.contains("{{") {
        return Err("staged index.html still contains an unresolved placeholder".to_owned());
    }
    if !stage.stage_dir.join("style.css").is_file() {
        return Err("expected style.css to be staged alongside index.html".to_owned());
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
fn prepare_stage_reports_a_missing_source_root() -> Result<(), String> {
    let stage_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let stage = StagePaths::new(stage_root.path());
    // `stage_root` itself has no `demo-app/src/lib.rs`, so using it as a fake root is a convenient way to point
    // `prepare_stage` at a source tree that definitely fails at the very first phase, validation.
    let fake_root = stage_root.path().join("no-such-root");

    match prepare_stage(&fake_root, &stage) {
        Err(BuildError::Validate(_)) => Ok(()),
        other => Err(format!("expected Err(BuildError::Validate(_)), got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A failed refresh must leave the previously staged `index.html` completely untouched — the guarantee `main`'s
/// per-request middleware documents. Proves `prepare_stage` assembles into a temporary file and `rename`s it into
/// place, rather than writing straight onto the live destination, where a failure partway could leave a
/// partially written file being served instead of the old, good one.
#[test]
fn prepare_stage_leaves_the_previously_staged_file_untouched_on_failure() -> Result<(), String> {
    let root = workspace_root()?;
    let stage_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let stage = StagePaths::new(stage_root.path());

    prepare_stage(&root, &stage).map_err(|e| format!("initial prepare_stage failed: {e}"))?;
    let before =
        fs::read_to_string(stage.stage_dir.join("index.html")).map_err(|e| format!("read staged index.html: {e:?}"))?;

    // A source root with no demo-app/src/lib.rs at all, so validation fails before assembly is ever attempted.
    let broken_root = stage_root.path().join("no-such-root");
    match prepare_stage(&broken_root, &stage) {
        Err(BuildError::Validate(_)) => {},
        other => {
            return Err(format!(
                "expected the second prepare_stage call to fail with Validate, got {other:?}"
            ));
        },
    }

    let after =
        fs::read_to_string(stage.stage_dir.join("index.html")).map_err(|e| format!("read staged index.html: {e:?}"))?;
    if after != before {
        return Err("a failed refresh changed the previously staged index.html".to_owned());
    }
    Ok(())
}
