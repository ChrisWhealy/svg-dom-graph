//! The demo's build pipeline — resolving staging paths, staging `index.html`, and rebuilding the wasm package —
//! factored out of `main` so it can run, and be tested, without ever touching Actix or the network.
//!
//! Unlike `svg-dom`'s own `demo-server` (which assembles `index.html` from a template plus many per-feature panel
//! fragments, for a gallery of dozens of demos), this crate has exactly three demo scenes on one hand-written
//! `index.html` — there is no template, no panel manifest, and nothing to validate for drift. Staging here is just
//! "copy `index.html` as-is, then build the wasm package alongside it".
//!
//! The pipeline is still split into two layers, for the same reason `svg-dom`'s does:
//!  - [`prepare_stage`] copies `index.html` into the staging directory. This is the part a plain
//!    `wasm-pack build demo-app ...` (the invocation CI's `wasm` job runs to build the wasm package) does not
//!    exercise at all.
//!  - [`build_demo`] runs [`prepare_stage`] and then rebuilds the wasm package, which is what `cargo demo` actually
//!    needs to serve the page.
//!
//! Separating them means CI (via `demo-server --prepare-only`, see `main`) or a test can exercise the staging copy
//! without paying for a full wasm build every time. Deciding what a failure means for the process — report to
//! stderr, `exit(1)` — stays `main`'s job alone.
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the demo's staged `index.html` and wasm package live (see `main`'s own doc comment for the full directory
/// layout). Bundled into one struct since every phase below needs some subset of these same paths, derived
/// together from a single `target_dir`.
#[derive(Clone)]
pub struct StagePaths {
    pub stage_dir: PathBuf,
    pub pkg_dir: PathBuf,
}

impl StagePaths {
    /// `target_dir` is the already-resolved Cargo target directory (respecting a `CARGO_TARGET_DIR` override — see
    /// `main`), not necessarily `root.join("target")`.
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
    /// `index.html` could not be copied into the staging directory's temporary file — see [`prepare_stage`]'s own
    /// doc comment for why there is one.
    CopyIndexHtml { src: PathBuf, dest: PathBuf, source: io::Error },
    /// The staged temporary file could not be renamed into place over the previously staged `index.html`.
    RenameIndexHtml { src: PathBuf, dest: PathBuf, source: io::Error },
    /// `wasm-pack` could not even be started (e.g. not on `PATH`).
    WasmSpawn(io::Error),
    /// `wasm-pack` ran but exited with a non-success status.
    WasmBuildFailed(ExitStatus),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateStageDir { path, source } => write!(f, "could not create {} ({source})", path.display()),
            Self::CopyIndexHtml { src, dest, source } => {
                write!(f, "could not copy {} to {} ({source})", src.display(), dest.display())
            },
            Self::RenameIndexHtml { src, dest, source } => {
                write!(f, "could not rename {} to {} ({source})", src.display(), dest.display())
            },
            Self::WasmSpawn(err) => write!(f, "could not run wasm-pack ({err})"),
            Self::WasmBuildFailed(status) => write!(f, "wasm-pack exited with {status}"),
        }
    }
}

impl std::error::Error for BuildError {
    /// Exposes the wrapped `io::Error` for every variant that has one, so error-reporting tools and callers
    /// walking the standard error chain can discover it — `Display` already forwards its message inline, but that
    /// alone does not help code that specifically walks using `source()`.
    ///
    /// `WasmBuildFailed` wraps an `ExitStatus`, not an `io::Error`, so `None` is correct for it — there is no
    /// underlying error to expose, just a non-zero exit code `Display` already reports in full.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateStageDir { source, .. }
            | Self::CopyIndexHtml { source, .. }
            | Self::RenameIndexHtml { source, .. }
            | Self::WasmSpawn(source) => Some(source),
            Self::WasmBuildFailed(_) => None,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stages `index.html` for serving: creates `stage.stage_dir` if it does not already exist, then atomically
/// replaces its `index.html` with a fresh copy of `root/index.html`. Everything [`build_demo`] does except the
/// wasm rebuild.
///
/// `main`'s per-request middleware calls this before every request, and documents that a failed refresh leaves
/// the previously staged file serving unchanged. A plain `fs::copy` straight onto the live `index.html` would not
/// actually guarantee that: it writes into the destination in place, so a copy that fails partway (disk full,
/// process killed) — or, just as likely, an editor's autosave catching `root/index.html` mid-write — leaves a
/// truncated or incomplete file being served, not the previous good one. Copying to a temporary file in the same
/// directory first, then `fs::rename`-ing it over `index.html` only once that copy has fully succeeded, makes the
/// documented guarantee actually true: `rename` within one directory is atomic, so every request either sees the
/// old `index.html` or the new one, never a partially written one, and a failure at either step leaves the
/// previous `index.html` completely untouched.
pub fn prepare_stage(root: &Path, stage: &StagePaths) -> Result<(), BuildError> {
    fs::create_dir_all(&stage.stage_dir).map_err(|source| BuildError::CreateStageDir {
        path: stage.stage_dir.clone(),
        source,
    })?;

    let src = root.join("index.html");
    let dest = stage.stage_dir.join("index.html");
    // Same directory as `dest`, so the rename below is guaranteed to be a same-filesystem, atomic replace rather
    // than a cross-filesystem move (which the OS would otherwise have to fall back to copying anyway).
    let tmp = stage.stage_dir.join("index.html.tmp");

    fs::copy(&src, &tmp).map_err(|source| BuildError::CopyIndexHtml {
        src: src.clone(),
        dest: tmp.clone(),
        source,
    })?;

    fs::rename(&tmp, &dest).map_err(|source| BuildError::RenameIndexHtml { src: tmp, dest, source })
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

#[cfg(test)]
mod unit_tests;
