use super::*;

/// The workspace root — `demo-server`'s own parent directory.
fn workspace_root() -> Result<PathBuf, String> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "demo-server has a parent directory".to_owned())?
        .to_path_buf())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Copies just the files `prepare_stage` actually reads — `demo-app/src/lib.rs` (for [`validate::validate`]) and
/// `demo/` (for [`panels::assemble`]) — from `src_root` into `dst_root`, so a test can freely edit a copied
/// fragment afterwards without ever touching the real project's own source tree.
fn copy_minimal_source_root(src_root: &Path, dst_root: &Path) -> Result<(), String> {
    let copy = |rel: &str| -> Result<(), String> {
        let src = src_root.join(rel);
        let dst = dst_root.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create_dir_all({}): {e:?}", parent.display()))?;
        }
        fs::copy(&src, &dst).map_err(|e| format!("copy {rel}: {e:?}")).map(|_| ())
    };

    copy("demo-app/src/lib.rs")?;
    copy("demo/index.template.html")?;
    copy("demo/style.css")?;
    for id in panels::panel_ids() {
        copy(&format!("demo/panels/{id}.html"))?;
    }
    Ok(())
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The successful counterpart to
/// [`prepare_stage_leaves_the_previously_staged_file_untouched_on_failure`]: two successful calls against the
/// same stage directory, with a source fragment edited in between, must actually replace the previously staged
/// `index.html` with content reflecting that edit — not just leave the old one in place, and not just fail to
/// error. Runs against a copied source root (see [`copy_minimal_source_root`]) rather than the real project's own
/// `demo/`, so this test can freely edit a fragment without ever touching a real source file.
#[test]
fn prepare_stage_replaces_an_existing_index_html_on_a_successful_rerun() -> Result<(), String> {
    let real_root = workspace_root()?;
    let temp_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    copy_minimal_source_root(&real_root, temp_root.path())?;

    let stage_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let stage = StagePaths::new(stage_root.path());

    prepare_stage(temp_root.path(), &stage).map_err(|e| format!("first prepare_stage failed: {e}"))?;
    let before =
        fs::read_to_string(stage.stage_dir.join("index.html")).map_err(|e| format!("read staged index.html: {e:?}"))?;

    const MARKER: &str = "<!-- prepare-stage-regression-test-marker -->";
    let fragment_path = temp_root.path().join("demo").join("panels").join("panel-tree.html");
    let mut fragment = fs::read_to_string(&fragment_path).map_err(|e| format!("read copied panel-tree.html: {e:?}"))?;
    fragment.push_str(MARKER);
    fs::write(&fragment_path, &fragment).map_err(|e| format!("write edited panel-tree.html: {e:?}"))?;

    prepare_stage(temp_root.path(), &stage).map_err(|e| format!("second prepare_stage failed: {e}"))?;
    let after =
        fs::read_to_string(stage.stage_dir.join("index.html")).map_err(|e| format!("read staged index.html: {e:?}"))?;

    if before.contains(MARKER) {
        return Err("marker unexpectedly present before the fragment was ever edited".to_owned());
    }
    if !after.contains(MARKER) {
        return Err("staged index.html was not replaced with the edited fragment's content".to_owned());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The exact gap an external review flagged: a failure copying `style.css` must leave the previously staged
/// `index.html` untouched too, not just `style.css`. Proves `prepare_stage` stages both files into temporary
/// files before promoting either, rather than promoting `index.html` first and only then attempting `style.css` —
/// which would otherwise let this exact scenario silently replace `index.html` even though the overall call
/// fails.
#[test]
fn prepare_stage_leaves_index_html_untouched_when_style_css_copy_fails() -> Result<(), String> {
    let real_root = workspace_root()?;
    let temp_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    copy_minimal_source_root(&real_root, temp_root.path())?;

    let stage_root = tempfile::tempdir().map_err(|e| format!("create temp dir: {e:?}"))?;
    let stage = StagePaths::new(stage_root.path());

    prepare_stage(temp_root.path(), &stage).map_err(|e| format!("first prepare_stage failed: {e}"))?;
    let before_index =
        fs::read_to_string(stage.stage_dir.join("index.html")).map_err(|e| format!("read staged index.html: {e:?}"))?;
    let before_style =
        fs::read_to_string(stage.stage_dir.join("style.css")).map_err(|e| format!("read staged style.css: {e:?}"))?;

    // Edit the fragment too, so a bug that promotes index.html before checking style.css would actually be
    // visible below: the staged index.html would change even though the whole call is expected to fail.
    const MARKER: &str = "<!-- prepare-stage-style-failure-marker -->";
    let fragment_path = temp_root.path().join("demo").join("panels").join("panel-tree.html");
    let mut fragment = fs::read_to_string(&fragment_path).map_err(|e| format!("read copied panel-tree.html: {e:?}"))?;
    fragment.push_str(MARKER);
    fs::write(&fragment_path, &fragment).map_err(|e| format!("write edited panel-tree.html: {e:?}"))?;

    // Remove the copied source style.css so the asset-copy phase fails, after assembly has already succeeded.
    fs::remove_file(temp_root.path().join("demo").join("style.css"))
        .map_err(|e| format!("remove copied style.css: {e:?}"))?;

    match prepare_stage(temp_root.path(), &stage) {
        Err(BuildError::CopyAsset { .. }) => {},
        other => return Err(format!("expected Err(BuildError::CopyAsset(_)), got {other:?}")),
    }

    let after_index =
        fs::read_to_string(stage.stage_dir.join("index.html")).map_err(|e| format!("read staged index.html: {e:?}"))?;
    let after_style =
        fs::read_to_string(stage.stage_dir.join("style.css")).map_err(|e| format!("read staged style.css: {e:?}"))?;

    if after_index != before_index {
        return Err("a style.css copy failure changed the previously staged index.html".to_owned());
    }
    if after_style != before_style {
        return Err("a style.css copy failure changed the previously staged style.css".to_owned());
    }
    Ok(())
}
