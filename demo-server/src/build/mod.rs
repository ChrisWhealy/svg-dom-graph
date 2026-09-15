//! The demo's build pipeline — resolving staging paths, validating the catalogue, assembling `index.html`, copying
//! static assets, and rebuilding the wasm package — factored out of `main` so it can run, and be tested, without
//! ever touching Actix or the network.
//!
//! Mirrors `svg-dom`'s own `demo-server/src/build/mod.rs`, including its two-layer split:
//!  - [`prepare_stage`] runs every phase except the wasm build: validate the catalogue, assemble `index.html`,
//!    copy `style.css`. This is the part a plain `wasm-pack build demo-app ...` (the invocation CI's `wasm` job
//!    runs to build the wasm package) does not exercise at all.
//!  - [`build_demo`] runs [`prepare_stage`] and then rebuilds the wasm package, which is what `cargo demo`
//!    actually needs to serve the page.
//!
//! Separating them means CI (via `demo-server --prepare-only`, see `main`) or a test can exercise catalogue
//! validation, fragment validation, and template assembly together — the actual pipeline, not just each phase's
//! own unit tests in isolation — without paying for a full wasm build every time. Deciding what a failure means
//! for the process — report to stderr, `exit(1)` — stays `main`'s job alone.
use crate::{panels, validate};
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the demo's staged `index.html`, `style.css`, and wasm package live (see `main`'s own doc comment for the
/// full directory layout). Bundled into one struct since every phase below needs some subset of these same paths,
/// derived together from a single `target_dir`.
#[derive(Clone)]
pub struct StagePaths {
    pub stage_dir: PathBuf,
    pub pkg_dir: PathBuf,
}

impl StagePaths {
    /// `target_dir` is the already-resolved Cargo target directory (respecting a `CARGO_TARGET_DIR` override —
    /// see `main`), not necessarily `root.join("target")`.
    pub fn new(target_dir: &Path) -> Self {
        let stage_dir = target_dir.join("demo-stage");
        let pkg_dir = stage_dir.join("pkg");
        Self { stage_dir, pkg_dir }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that can go wrong staging the demo, across every phase [`build_demo`] runs. `main` reports whichever
/// variant it gets via `Display` and exits — see this module's own doc comment for why staging itself never does
/// that.
#[derive(Debug)]
pub enum BuildError {
    /// The staging directory could not be created.
    CreateStageDir { path: PathBuf, source: io::Error },
    /// The panel manifest and `demo_gallery!` have drifted apart — see [`validate::ValidationError`].
    Validate(validate::ValidationError),
    /// `index.html` could not be assembled from `demo/index.template.html` and its panel fragments — see
    /// [`panels::AssembleError`].
    Assemble(panels::AssembleError),
    /// The assembled temporary file could not be renamed into place over the previously staged `index.html` —
    /// see [`prepare_stage`]'s own doc comment for why there is a temporary file at all.
    RenameIndexHtml { src: PathBuf, dest: PathBuf, source: io::Error },
    /// A static asset (`style.css`) could not be copied into the staging directory.
    CopyAsset { src: PathBuf, dest: PathBuf, source: io::Error },
    /// `wasm-pack` could not even be started (e.g. not on `PATH`).
    WasmSpawn(io::Error),
    /// `wasm-pack` ran but exited with a non-success status.
    WasmBuildFailed(ExitStatus),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateStageDir { path, source } => write!(f, "could not create {} ({source})", path.display()),
            Self::Validate(err) => write!(f, "{err}"),
            Self::Assemble(err) => write!(f, "{err}"),
            Self::RenameIndexHtml { src, dest, source } => {
                write!(f, "could not rename {} to {} ({source})", src.display(), dest.display())
            },
            Self::CopyAsset { src, dest, source } => {
                write!(f, "could not copy {} to {} ({source})", src.display(), dest.display())
            },
            Self::WasmSpawn(err) => write!(f, "could not run wasm-pack ({err})"),
            Self::WasmBuildFailed(status) => write!(f, "wasm-pack exited with {status}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateStageDir { source, .. }
            | Self::RenameIndexHtml { source, .. }
            | Self::CopyAsset { source, .. } => Some(source),
            Self::Validate(err) => Some(err),
            Self::Assemble(err) => Some(err),
            Self::WasmSpawn(source) => Some(source),
            Self::WasmBuildFailed(_) => None,
        }
    }
}

impl From<validate::ValidationError> for BuildError {
    fn from(err: validate::ValidationError) -> Self {
        Self::Validate(err)
    }
}

impl From<panels::AssembleError> for BuildError {
    fn from(err: panels::AssembleError) -> Self {
        Self::Assemble(err)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs every phase except the wasm rebuild: validates the catalogue, assembles `index.html` (substituting
/// `{{PANELS}}`), and copies `style.css` — everything needed to stage a servable demo except `pkg/`.
///
/// Returns as soon as any phase fails, via `?`: a stale catalogue is caught before `index.html` is ever
/// assembled, and a broken assembly is caught before it is ever written into place.
///
/// `index.html` is assembled into a temporary file in `stage.stage_dir` first, then `fs::rename`-d into place
/// only once that assembly has fully succeeded — the same atomic-replace reasoning this function's own
/// predecessor used for a plain file copy: `rename` within one directory means every request either sees the old
/// `index.html` or the new one, never a partially written one, and a failure partway leaves the previous
/// `index.html` completely untouched.
pub fn prepare_stage(root: &Path, stage: &StagePaths) -> Result<(), BuildError> {
    fs::create_dir_all(&stage.stage_dir).map_err(|source| BuildError::CreateStageDir {
        path: stage.stage_dir.clone(),
        source,
    })?;

    // Check whether the panel manifest is out of sync with demo-app's demo_gallery! before it produces a broken
    // or incomplete gallery, rather than after.
    validate::validate(root)?;

    let source_demo_dir = root.join("demo");
    let dest_index = stage.stage_dir.join("index.html");
    // Same directory as `dest_index`, so the rename below is guaranteed to be a same-filesystem, atomic replace.
    let tmp_index = stage.stage_dir.join("index.html.tmp");
    panels::assemble(&source_demo_dir, &tmp_index)?;
    fs::rename(&tmp_index, &dest_index).map_err(|source| BuildError::RenameIndexHtml {
        src: tmp_index,
        dest: dest_index,
        source,
    })?;

    // style.css is not generated — it is a static asset index.html references by a plain relative path, so it
    // needs to sit alongside the assembled file in the staging directory too.
    copy_asset(&source_demo_dir.join("style.css"), &stage.stage_dir.join("style.css"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs [`prepare_stage`] and then rebuilds the wasm package, leaving only starting the Actix server itself to
/// `main`. This is the full pipeline `cargo demo` needs; `main --prepare-only` runs [`prepare_stage`] alone
/// instead (see this module's own doc comment for why that split exists).
pub fn build_demo(root: &Path, stage: &StagePaths) -> Result<(), BuildError> {
    prepare_stage(root, stage)?;
    build_wasm(root, &stage.pkg_dir)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the `svg-dom-graph-demo` crate's wasm package into `out_dir` so the served `pkg/` is up to date.
fn build_wasm(root: &Path, out_dir: &Path) -> Result<(), BuildError> {
    println!(
        "Building wasm package: wasm-pack build demo-app --target web --out-dir {}",
        out_dir.display()
    );

    // demo-app is a separate workspace crate (svg-dom-graph-demo) consuming svg-dom-graph only through its public
    // API — see that crate's own doc comment for why. `--out-dir` is given as an absolute path so it lands exactly
    // at `out_dir` regardless of demo-app's own location, rather than relying on relative-path arithmetic from it.
    let status = Command::new("wasm-pack")
        .current_dir(root)
        .arg("build")
        .arg("demo-app")
        .args(["--target", "web", "--out-dir"])
        .arg(out_dir)
        .status()
        .map_err(BuildError::WasmSpawn)?;

    if status.success() { Ok(()) } else { Err(BuildError::WasmBuildFailed(status)) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn copy_asset(src: &Path, dest: &Path) -> Result<(), BuildError> {
    fs::copy(src, dest).map(|_| ()).map_err(|source| BuildError::CopyAsset {
        src: src.to_path_buf(),
        dest: dest.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod unit_tests;
