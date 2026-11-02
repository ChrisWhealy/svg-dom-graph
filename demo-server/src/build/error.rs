use crate::{panels, validate};
use std::{fmt, io, path::PathBuf, process::ExitStatus};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that can go wrong staging the demo, across every phase [`build_demo`] runs. `main` reports whichever
/// variant it gets via `Display` and exits — see this module's own doc comment for why staging itself never does that.
#[derive(Debug)]
pub enum BuildError {
    /// The staging directory could not be created.
    CreateStageDir { path: PathBuf, source: io::Error },
    /// The panel manifest and `demo_gallery!` have drifted apart — see [`validate::ValidationError`].
    Validate(validate::ValidationError),
    /// `index.html` could not be assembled from `demo/index.template.html` and its panel fragments — see
    /// [`panels::AssembleError`].
    Assemble(panels::AssembleError),
    /// A staged temporary file could not be renamed into place over the previously staged file at `dest`. See
    /// [`prepare_stage`]'s own doc comment for why there is a temporary file at all.
    PromoteStagedFile { src: PathBuf, dest: PathBuf, source: io::Error },
    /// A static asset (see `STATIC_ASSETS`) could not be copied into its own temporary file in the staging directory.
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
            Self::PromoteStagedFile { src, dest, source } => {
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
            | Self::PromoteStagedFile { source, .. }
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
