use std::{fmt, io, path::PathBuf};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that can go wrong assembling `index.html`. None of these checks need an HTML parser. Every one is a
/// targeted check against a specific convention this gallery already relies on. The conventions are one placeholder per
/// token, one `id="..."` per fragment that matches its filename, and the three `data-` attributes on its opening tag.
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
    /// A fragment's filename does not match `panel-[a-z0-9-]+` — see [`is_valid_panel_id`] for why that pattern is
    /// enforced up front rather than escaped at each place an id is emitted.
    InvalidPanelId(String),
    /// A fragment's own content does not contain `id="{id}"` for the id it is filed under — it may have been
    /// copy-pasted from another panel's fragment and never updated.
    FragmentIdMismatch { id: String, fragment_path: PathBuf },
    /// A fragment has no opening `<section>` tag, or the tag lacks the `data-` attribute `attribute`.
    MissingAttribute { fragment_path: PathBuf, attribute: &'static str },
    /// A fragment's `data-order` is not a whole number.
    InvalidOrder { fragment_path: PathBuf, value: String },
    /// A fragment's `data-group` is not in [`GROUP_ORDER`], which would leave the panel out of the menu.
    UnknownGroup { fragment_path: PathBuf, group: String },
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
            Self::InvalidPanelId(id) => write!(
                f,
                "demo/panels/ contains the fragment {id:?}, whose name does not match panel-[a-z0-9-]+"
            ),
            Self::FragmentIdMismatch { id, fragment_path } => {
                write!(
                    f,
                    "{} does not contain id=\"{id}\", the id it is filed under",
                    fragment_path.display()
                )
            },
            Self::MissingAttribute { fragment_path, attribute } => write!(
                f,
                "{} has no <section> tag with the attribute {attribute}",
                fragment_path.display()
            ),
            Self::InvalidOrder { fragment_path, value } => write!(
                f,
                "{} has data-order={value:?}, which is not a whole number",
                fragment_path.display()
            ),
            Self::UnknownGroup { fragment_path, group } => write!(
                f,
                "{} is filed under data-group={group:?}, which is not in GROUP_ORDER",
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
