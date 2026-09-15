//! Assembles `index.html` from `demo/index.template.html`, a generated `<nav>` menu, and the panel fragments in
//! `demo/panels/`.
//!
//! Mirrors `svg-dom`'s own `demo-server/src/panels/mod.rs`, minus its category dividers: `svg-dom`'s gallery groups
//! roughly eighty panels under menu headings like "Basic Shapes" and "Filters", which this gallery's much smaller
//! panel count has no need for yet. Add category support here the same way `svg-dom` does, if this list ever grows
//! large enough to want it.
//!
//! [`MANIFEST`] is this gallery's single source of truth for both panel order and menu labelling: it drives the
//! generated `<nav>` menu and the generated panel body, so the two can never disagree about which panels exist or
//! what order they come in.
//!
//! It does not know anything about the Rust demo functions that build each panel's content — that mapping lives in
//! `demo-app/src/lib.rs`'s `demo_gallery!` invocation instead, which holds a separate list for a separate reason
//! (see that macro's own doc comment). The job of [`super::validate`] is to keep the two ids in step.
//!
//! # Adding a new demo
//!
//! Add its id and menu label to [`MANIFEST`], create the matching `demo/panels/{id}.html` fragment (containing an
//! element with `id="{id}"`, since [`assemble`] checks the two match), and add the matching `demo_gallery!` entry in
//! `demo-app/src/lib.rs`.

use std::{
    collections::HashSet,
    fmt, fs, io,
    path::{Path, PathBuf},
};

const PANELS_PLACEHOLDER: &str = "{{PANELS}}";
const MENU_PLACEHOLDER: &str = "{{MENU}}";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `(panel id, menu label)`, in the order the menu and the assembled page both present them. A panel's own body
/// markup — including its `<h2>` heading — lives entirely in `demo/panels/{id}.html`; this list only decides which
/// fragments exist, what order they appear in, and what their `<nav>` link reads.
const MANIFEST: &[(&str, &str)] = &[
    ("panel-tree", "Directed tree"),
    ("panel-elbow", "Connector routing"),
    ("panel-edge-anchors", "Fixing points"),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`MANIFEST`]'s own panel ids — used by [`super::validate`] to cross-check against `demo-app`'s `demo_gallery!`
/// list, so the two cannot silently drift apart.
pub fn panel_ids() -> Vec<&'static str> {
    MANIFEST.iter().map(|&(id, _)| id).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that can go wrong assembling `index.html`. None of these checks need an HTML parser — every one is
/// a targeted check against a specific convention this gallery already relies on (one placeholder per token, one
/// `id="..."` per fragment, a fragments directory that matches [`MANIFEST`] exactly), not a general claim about
/// well-formed HTML.
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
    /// The same panel id appears in [`MANIFEST`] more than once.
    DuplicateManifestId(&'static str),
    /// A panel id does not match `panel-[a-z0-9-]+` — see [`check_panel_id_format`] for why that pattern is
    /// enforced up front rather than escaped at each place an id is emitted.
    InvalidPanelId(&'static str),
    /// A fragment's own content does not contain `id="{id}"` for the id it is filed under — it may have been
    /// copy-pasted from another panel's fragment and never updated.
    FragmentIdMismatch { id: &'static str, fragment_path: PathBuf },
    /// [`MANIFEST`]'s panel ids and `demo/panels/*.html`'s own filenames are not the same set — exactly how an
    /// orphaned fragment (removed from `MANIFEST` but left on disk) or a missing one (added to `MANIFEST` but
    /// never created) gets caught.
    CatalogueMismatch(String),
    /// The fully assembled output still contains a `{{...}}` token after both placeholders were substituted —
    /// evidence of a typo'd or unexpected placeholder that no check above already caught.
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
            Self::DuplicateManifestId(id) => write!(f, "MANIFEST contains the panel id {id:?} more than once"),
            Self::InvalidPanelId(id) => write!(
                f,
                "MANIFEST contains the panel id {id:?}, which does not match panel-[a-z0-9-]+"
            ),
            Self::FragmentIdMismatch { id, fragment_path } => {
                write!(
                    f,
                    "{} does not contain id=\"{id}\", the id it is filed under",
                    fragment_path.display()
                )
            },
            Self::CatalogueMismatch(detail) => {
                write!(f, "MANIFEST and demo/panels/ disagree about which panels exist:\n{detail}")
            },
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds `index.html` from `source_demo_dir`'s `index.template.html`, a `<nav>` menu generated from [`MANIFEST`],
/// and `panels/*.html` fragments, and writes the result to `out_path`.
///
/// Every check runs, and the complete assembled output is built in memory, before anything is written — a call
/// that returns `Err` never touches `out_path` on disk.
pub fn assemble(source_demo_dir: &Path, out_path: &Path) -> Result<(), AssembleError> {
    let panels_dir = source_demo_dir.join("panels");
    let template_path = source_demo_dir.join("index.template.html");

    check_unique_manifest_ids(MANIFEST)?;
    check_panel_id_format(MANIFEST)?;

    let template = read_to_string(&template_path)?;
    check_placeholder_count(&template, &template_path, PANELS_PLACEHOLDER)?;
    check_placeholder_count(&template, &template_path, MENU_PLACEHOLDER)?;

    let fragment_ids = list_fragment_ids(&panels_dir)?;
    check_catalogue_consistency(MANIFEST, &fragment_ids)?;

    let panels_body = render_panels(&panels_dir)?;
    let menu_body = render_menu();
    let assembled = template.replacen(PANELS_PLACEHOLDER, &panels_body, 1);
    let assembled = assembled.replacen(MENU_PLACEHOLDER, &menu_body, 1);
    check_no_leftover_placeholders(&assembled)?;

    fs::write(out_path, &assembled).map_err(|source| AssembleError::Io {
        path: out_path.to_path_buf(),
        source,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Checks
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// Every panel id among `entries` must be unique. A duplicate would silently render the same fragment twice.
///
/// Takes the entry list as a parameter, rather than reading [`MANIFEST`] directly, purely so the tests below can
/// exercise the duplicate-detection logic against a small synthetic list.
fn check_unique_manifest_ids(entries: &[(&'static str, &'static str)]) -> Result<(), AssembleError> {
    let mut seen = HashSet::new();
    for &(id, _) in entries {
        if !seen.insert(id) {
            return Err(AssembleError::DuplicateManifestId(id));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every panel id among `entries` must match `panel-[a-z0-9-]+`.
///
/// Ids are emitted verbatim into both HTML attribute values and Rust match arms: `id="{id}"` in each fragment,
/// `data-target="{id}"` in the generated menu, and `"panel-..." => name` in `demo-app/src/lib.rs`'s
/// `demo_gallery!`. This restricts them to a known-safe ASCII pattern up front, meaning that downstream —
/// [`render_menu`], a fragment file, `demo_gallery!` — never has to escape or re-validate an id itself.
///
/// Unlike labels (see [`escape_text`]), ids are not free text, so a fixed character set is the natural fit rather
/// than an escaper. Mirrors `svg-dom`'s own `check_panel_id_format`.
fn check_panel_id_format(entries: &[(&'static str, &'static str)]) -> Result<(), AssembleError> {
    for &(id, _) in entries {
        if !is_valid_panel_id(id) {
            return Err(AssembleError::InvalidPanelId(id));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn is_valid_panel_id(id: &str) -> bool {
    id.strip_prefix("panel-").is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `template` must contain `placeholder` exactly once: zero means nothing will ever be substituted in, and more
/// than one means `replacen(..., 1)` would leave every occurrence after the first sitting in the output untouched.
fn check_placeholder_count(
    template: &str,
    template_path: &Path,
    placeholder: &'static str,
) -> Result<(), AssembleError> {
    match template.matches(placeholder).count() {
        0 => Err(AssembleError::MissingPlaceholder {
            template_path: template_path.to_path_buf(),
            placeholder,
        }),
        1 => Ok(()),
        count => Err(AssembleError::DuplicatePlaceholder {
            template_path: template_path.to_path_buf(),
            placeholder,
            count,
        }),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel id each `demo/panels/*.html` fragment is filed under (its filename, minus `.html`), in whatever order
/// `fs::read_dir` happens to return.
fn list_fragment_ids(panels_dir: &Path) -> Result<Vec<String>, AssembleError> {
    let entries = fs::read_dir(panels_dir).map_err(|source| AssembleError::Io {
        path: panels_dir.to_path_buf(),
        source,
    })?;

    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| AssembleError::Io {
            path: panels_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "html")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            ids.push(stem.to_owned());
        }
    }
    Ok(ids)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`MANIFEST`] and `fragment_ids` (the fragments directory's own filenames) must name exactly the same set of
/// panels — see [`AssembleError::CatalogueMismatch`] for what drifting apart would mean.
fn check_catalogue_consistency(
    manifest: &[(&'static str, &'static str)],
    fragment_ids: &[String],
) -> Result<(), AssembleError> {
    let missing_fragment: Vec<_> = manifest
        .iter()
        .map(|&(id, _)| id)
        .filter(|id| !fragment_ids.iter().any(|f| f == id))
        .collect();
    let orphan_fragment: Vec<_> = fragment_ids
        .iter()
        .filter(|id| !manifest.iter().any(|&(m, _)| m == id.as_str()))
        .collect();

    if missing_fragment.is_empty() && orphan_fragment.is_empty() {
        return Ok(());
    }

    let mut detail = Vec::new();
    if !missing_fragment.is_empty() {
        detail.push(format!("  in MANIFEST but missing from demo/panels/: {missing_fragment:?}"));
    }
    if !orphan_fragment.is_empty() {
        detail.push(format!("  in demo/panels/ but missing from MANIFEST: {orphan_fragment:?}"));
    }
    Err(AssembleError::CatalogueMismatch(detail.join("\n")))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The assembled output must not contain any `{{...}}` token once both placeholders have been substituted — a
/// leftover one means a typo'd or unexpected placeholder that none of the checks above already caught. Mirrors
/// `svg-dom`'s own `check_no_leftover_placeholders`.
fn check_no_leftover_placeholders(assembled: &str) -> Result<(), AssembleError> {
    if let Some(start) = assembled.find("{{") {
        let end = (start + 40).min(assembled.len());
        return Err(AssembleError::LeftoverPlaceholder(assembled[start..end].to_owned()));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads each of [`MANIFEST`]'s fragments, in order, checking each one's own content contains `id="{id}"` for the
/// id it is filed under, and concatenates them into the finished panels body.
fn render_panels(panels_dir: &Path) -> Result<String, AssembleError> {
    let mut body = String::new();
    for &(id, _) in MANIFEST {
        let fragment_path = panels_dir.join(format!("{id}.html"));
        let fragment = read_to_string(&fragment_path)?;
        if !fragment.contains(&format!("id=\"{id}\"")) {
            return Err(AssembleError::FragmentIdMismatch { id, fragment_path });
        }
        body.push_str(&fragment);
        body.push('\n');
    }
    Ok(body)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds one `<button class="menu-item" data-target="...">` per [`MANIFEST`] entry — the left-hand `<nav>` this
/// gallery's `demo/index.template.html` script uses to switch between panels (see that file's own `selectDemo`).
fn render_menu() -> String {
    MANIFEST
        .iter()
        .map(|(id, label)| {
            format!(
                "            <button type=\"button\" class=\"menu-item\" data-target=\"{id}\">{}</button>\n",
                escape_text(label)
            )
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Escapes `&`, `<` and `>` for safe insertion into an HTML text node — the full set that matters there. Labels are
/// trusted source code, not user input, so this is not a security boundary; it exists so a future label (e.g. one
/// containing `<`) is rendered as text rather than parsed as markup.
fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn read_to_string(path: &Path) -> Result<String, AssembleError> {
    fs::read_to_string(path).map_err(|source| AssembleError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod unit_tests;
