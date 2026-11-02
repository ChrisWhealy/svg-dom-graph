use std::{fmt, io, path::PathBuf};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that can go wrong assembling `index.html`. None of these checks need an HTML parser. Every one is a
/// targeted check against a specific convention this gallery already relies on. The conventions are one placeholder per
/// token, an `id` and a `data-label` on each fragment's opening tag, and the `NN-name` naming of folders and fragments.
/// None is a general claim about well-formed HTML.
#[derive(Debug)]
pub enum AssembleError {
    /// A file could not be read, or the assembled result could not be written.
    Io { path: PathBuf, source: io::Error },
    /// `template` does not contain `placeholder` at all.
    MissingPlaceholder { template_path: PathBuf, placeholder: &'static str },
    /// `template` contains `placeholder` more than once — `replacen(..., 1)` would silently leave every occurrence
    /// after the first sitting untouched in the output.
    DuplicatePlaceholder {
        template_path: PathBuf,
        placeholder: &'static str,
        count: usize,
    },
    /// A subfolder of `demo/panels/` is not named `NN-group-name`: two digits, a hyphen, and then lowercase letters,
    /// digits and hyphens.
    InvalidGroupFolder(String),
    /// A file in a group folder is not named `NN-demo-name.html` in the same way. The path is below `demo/panels/`.
    InvalidFragmentName(PathBuf),
    /// Two group folders, or two fragments in one group, start with the same number, so their order is not decided.
    DuplicateNumber { path: PathBuf, other: PathBuf },
    /// A fragment's `<section id>` does not match `panel-[a-z0-9-]+` — see [`is_valid_panel_id`] for why that pattern
    /// is enforced up front rather than escaped at each place an id is emitted.
    InvalidPanelId { id: String, fragment_path: PathBuf },
    /// Two fragments have the same `<section id>`.
    DuplicatePanelId { id: String, fragment_path: PathBuf, other: PathBuf },
    /// A fragment has no opening `<section>` tag, or the tag lacks the attribute `attribute`.
    MissingAttribute { fragment_path: PathBuf, attribute: &'static str },
    /// The fully assembled output still contains a `{{...}}` token after both placeholders were substituted — evidence
    /// of a typo'd or unexpected placeholder that no check above already caught.
    LeftoverPlaceholder(String),
}

impl fmt::Display for AssembleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{} ({source})", path.display()),
            Self::MissingPlaceholder { template_path, placeholder } => {
                write!(f, "{} is missing the {placeholder} placeholder", template_path.display())
            },
            Self::DuplicatePlaceholder {
                template_path,
                placeholder,
                count,
            } => {
                write!(
                    f,
                    "{} contains {placeholder} {count} times, expected exactly once",
                    template_path.display()
                )
            },
            Self::InvalidGroupFolder(name) => write!(
                f,
                "demo/panels/ contains the folder {name:?}, whose name does not match NN-group-name"
            ),
            Self::InvalidFragmentName(path) => {
                write!(f, "demo/panels/{} is not named NN-demo-name.html", path.display())
            },
            Self::DuplicateNumber { path, other } => write!(
                f,
                "demo/panels/{} and demo/panels/{} start with the same number",
                path.display(),
                other.display()
            ),
            Self::InvalidPanelId { id, fragment_path } => write!(
                f,
                "{} has the id {id:?}, which does not match panel-[a-z0-9-]+",
                fragment_path.display()
            ),
            Self::DuplicatePanelId { id, fragment_path, other } => {
                write!(f, "{} and {} both have the id {id:?}", fragment_path.display(), other.display())
            },
            Self::MissingAttribute { fragment_path, attribute } => write!(
                f,
                "{} has no <section> tag with the attribute {attribute}",
                fragment_path.display()
            ),
            Self::LeftoverPlaceholder(context) => {
                write!(
                    f,
                    "assembled index.html still contains an unresolved placeholder near: {context:?}"
                )
            },
        }
    }
}

impl std::error::Error for AssembleError {}
