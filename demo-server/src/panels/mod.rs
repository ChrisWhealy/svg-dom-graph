//! Assembles `index.html` from `demo/index.template.html`, a generated `<nav>` menu, and the panel fragments in
//! `demo/panels/`.
//!
//! Mirrors `svg-dom`'s own `demo-server/src/panels/mod.rs`. Its menu groups panels under headings like "Basic Nodes"
//! and "Operators", each of which expands to show its panels.
//!
//! The folder structure of `demo/panels/` is the menu. Each subfolder is a group, named `NN-group-name`, where `NN` is
//! two digits that put the group in order, and the hyphens of the name become the spaces of the heading. Each fragment
//! in a group folder is named `NN-demo-name.html` in the same way, and `NN` puts the panel in order within its group.
//! The fragment's own opening `<section>` tag still carries the panel's `id` and its menu label as `data-label`,
//! since neither can be worked out from a file name.
//!
//! It does not know anything about the Rust demo functions that build each panel's content. That mapping lives in
//! `demo-app/src/lib.rs`'s `demo_gallery!` invocation instead. That invocation holds a separate list for a separate
//! reason (see that macro's own doc comment). The job of [`super::validate`] is to keep the two ids in step.
//!
//! # Adding a new demo
//!
//! Create `demo/panels/NN-group-name/NN-demo-name.html`, whose root element is
//! `<section class="section" id="panel-..." data-label="...">`. Number the files so that a panel can be slotted between
//! two others by renumbering. Then add the matching `demo_gallery!` entry in `demo-app/src/lib.rs`. A panel in a group
//! that is new needs a new `NN-group-name` folder.

mod error;

pub(crate) use error::AssembleError;
use std::{
    fs,
    path::{Path, PathBuf},
};

const PANELS_PLACEHOLDER: &str = "{{PANELS}}";
const MENU_PLACEHOLDER: &str = "{{MENU}}";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One panel, as its place in `demo/panels/` and its own fragment describe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    pub id: String,
    /// The heading of the panel's group, made from the name of the group's folder.
    pub group: String,
    /// The text of the panel's menu item. A group that holds only this panel shows the group's heading instead.
    pub label: String,
    /// The fragment's path below the panels folder: its group's folder, and then its own file name.
    pub path: PathBuf,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every panel in `panels_dir`, in menu order: by group folder name, and then by file name. The two-digit numbers that
/// start those names put them in order.
///
/// Reads and checks every fragment, so a folder or fragment that is wrongly named, a number that is used twice, an id
/// that is not valid or is used twice, or a fragment that lacks its label is reported here, whichever caller asked.
pub fn catalogue(panels_dir: &Path) -> Result<Vec<Panel>, AssembleError> {
    let mut panels: Vec<Panel> = Vec::new();
    let mut group_numbers: Vec<(String, PathBuf)> = Vec::new();
    for group_name in list_names(panels_dir, true)? {
        let (number, heading) =
            split_name(&group_name).ok_or_else(|| AssembleError::InvalidGroupFolder(group_name.clone()))?;
        if let Some((_, other)) = group_numbers.iter().find(|(n, _)| *n == number) {
            return Err(AssembleError::DuplicateNumber {
                path: PathBuf::from(&group_name),
                other: other.clone(),
            });
        }
        group_numbers.push((number.to_owned(), PathBuf::from(&group_name)));
        let heading = heading_of(heading);

        let group_dir = panels_dir.join(&group_name);
        let mut fragment_numbers: Vec<(String, PathBuf)> = Vec::new();
        for file_name in list_names(&group_dir, false)? {
            let path = Path::new(&group_name).join(&file_name);
            let stem = file_name.strip_suffix(".html").unwrap_or(&file_name);
            let (number, _) = split_name(stem).ok_or_else(|| AssembleError::InvalidFragmentName(path.clone()))?;
            if let Some((_, other)) = fragment_numbers.iter().find(|(n, _)| *n == number) {
                return Err(AssembleError::DuplicateNumber { path, other: other.clone() });
            }
            fragment_numbers.push((number.to_owned(), path.clone()));

            let fragment_path = panels_dir.join(&path);
            let fragment = read_to_string(&fragment_path)?;
            let attribute = |name: &'static str| {
                section_attribute(&fragment, name).ok_or_else(|| AssembleError::MissingAttribute {
                    fragment_path: fragment_path.clone(),
                    attribute: name,
                })
            };
            let id = attribute("id")?;
            if !is_valid_panel_id(&id) {
                return Err(AssembleError::InvalidPanelId { id, fragment_path });
            }
            if let Some(other) = panels.iter().find(|panel| panel.id == id) {
                return Err(AssembleError::DuplicatePanelId {
                    id,
                    fragment_path,
                    other: panels_dir.join(&other.path),
                });
            }
            let label = attribute("data-label")?;
            panels.push(Panel {
                id,
                group: heading.clone(),
                label,
                path,
            });
        }
    }
    Ok(panels)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Splits a folder name or a fragment's name without its extension, `NN-some-name`, into `NN` and `some-name`. `None` if
/// it is not two digits, a hyphen, and then one or more lowercase letters, digits and hyphens that do not start or end
/// with a hyphen.
fn split_name(name: &str) -> Option<(&str, &str)> {
    let (number, rest) = (name.get(..2)?, name.get(3..)?);
    let valid = number.bytes().all(|b| b.is_ascii_digit())
        && name.as_bytes()[2] == b'-'
        && !rest.is_empty()
        && !rest.starts_with('-')
        && !rest.ends_with('-')
        && rest.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    valid.then_some((number, rest))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The heading for a group folder's name without its number: the hyphens become spaces, and each word starts with a
/// capital, except for the small words that join others, which only do at the start.
fn heading_of(name: &str) -> String {
    const SMALL: &[&str] = &["a", "an", "and", "of", "the", "or"];
    name.split('-')
        .enumerate()
        .map(|(position, word)| {
            let mut letters = word.chars();
            match letters.next() {
                Some(first) if position == 0 || !SMALL.contains(&word) => first.to_uppercase().chain(letters).collect(),
                _ => word.to_owned(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The value of the attribute `name` on the first `<section>` tag of `fragment`, with the five HTML escapes undone, or
/// `None` if there is no such tag or attribute. The tag ends at its first `>`, so a value must not contain one. Write
/// it as `&gt;`.
fn section_attribute(fragment: &str, name: &str) -> Option<String> {
    let start = fragment.find("<section")?;
    let tag = &fragment[start..start + fragment[start..].find('>')?];
    let needle = format!(" {name}=\"");
    let value_start = tag.find(&needle)? + needle.len();
    let value = &tag[value_start..value_start + tag[value_start..].find('"')?];
    Some(
        value
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`catalogue`]'s panel ids, in menu order. [`super::validate`] cross-checks them against `demo-app`'s `demo_gallery!`
/// list, so the two cannot silently drift apart.
pub fn panel_ids(panels_dir: &Path) -> Result<Vec<String>, AssembleError> {
    Ok(catalogue(panels_dir)?.into_iter().map(|panel| panel.id).collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds `index.html` from `source_demo_dir`'s `index.template.html`, a `<nav>` menu generated from [`catalogue`], and
/// `panels/*.html` fragments, and writes the result to `out_path`.
///
/// Every check runs, and the complete assembled output is built in memory, before anything is written — a call that
/// returns `Err` never touches `out_path` on disk.
pub fn assemble(source_demo_dir: &Path, out_path: &Path) -> Result<(), AssembleError> {
    let panels_dir = source_demo_dir.join("panels");
    let template_path = source_demo_dir.join("index.template.html");

    let template = read_to_string(&template_path)?;
    check_placeholder_count(&template, &template_path, PANELS_PLACEHOLDER)?;
    check_placeholder_count(&template, &template_path, MENU_PLACEHOLDER)?;

    let panels = catalogue(&panels_dir)?;
    let panels_body = render_panels(&panels_dir, &panels)?;
    let menu_body = render_menu(&panels);
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn is_valid_panel_id(id: &str) -> bool {
    id.strip_prefix("panel-").is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `template` must contain `placeholder` exactly once. Zero means nothing will ever be substituted in. More than one
/// means `replacen(..., 1)` would leave every occurrence after the first sitting in the output untouched.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The names in `dir`, in alphabetical order, which is menu order: the subfolders if `folders` is true, otherwise the
/// `.html` files. Anything else is ignored, such as `.DS_Store`.
fn list_names(dir: &Path, folders: bool) -> Result<Vec<String>, AssembleError> {
    let io_error = |source| AssembleError::Io {
        path: dir.to_path_buf(),
        source,
    };
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        let wanted = if folders {
            path.is_dir()
        } else {
            path.is_file() && path.extension().is_some_and(|ext| ext == "html")
        };
        if wanted && let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            names.push(name.to_owned());
        }
    }
    names.sort();
    Ok(names)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The assembled output must not contain any `{{...}}` token once both placeholders have been substituted. A leftover
/// one means a typo'd or unexpected placeholder that none of the checks above already caught. Mirrors `svg-dom`'s own
/// `check_no_leftover_placeholders`.
fn check_no_leftover_placeholders(assembled: &str) -> Result<(), AssembleError> {
    if let Some(start) = assembled.find("{{") {
        let end = (start + 40).min(assembled.len());
        return Err(AssembleError::LeftoverPlaceholder(assembled[start..end].to_owned()));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads each of `panels`' fragments, in order, and concatenates them into the finished panels body.
fn render_panels(panels_dir: &Path, panels: &[Panel]) -> Result<String, AssembleError> {
    let mut body = String::new();
    for panel in panels {
        body.push_str(&read_to_string(&panels_dir.join(&panel.path))?);
        body.push('\n');
    }
    Ok(body)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the left-hand `<nav>` this gallery's `demo/index.template.html` script uses to switch between panels (see
/// that file's own `selectDemo`) from `panels`, which [`catalogue`] has already put in menu order.
///
/// A group of several panels is a `<details class="menu-group">`, a native disclosure widget, with one
/// `<button class="menu-item" data-target="...">` per panel. A group of one panel is that button alone, with the
/// `menu-item-top` class and the group's own label. The script finds every panel by `.menu-item`, so it does not need
/// to know the two shapes apart.
fn render_menu(panels: &[Panel]) -> String {
    let button = |class: &str, id: &str, label: &str| {
        format!(
            "<button type=\"button\" class=\"{class}\" data-target=\"{id}\">{}</button>\n",
            escape_text(label)
        )
    };
    let mut menu = String::new();
    // Panels are in menu order, so the panels of a group are next to each other.
    for members in panels.chunk_by(|a, b| a.group == b.group) {
        let heading = members[0].group.as_str();
        match members {
            [only] => {
                menu.push_str("            ");
                menu.push_str(&button("menu-item menu-item-top", &only.id, heading));
            },
            members => {
                menu.push_str("            <details class=\"menu-group\">\n");
                menu.push_str(&format!("                <summary>{}</summary>\n", escape_text(heading)));
                for panel in members {
                    menu.push_str("                ");
                    menu.push_str(&button("menu-item", &panel.id, &panel.label));
                }
                menu.push_str("            </details>\n");
            },
        }
    }
    menu
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Escapes `&`, `<` and `>` for safe insertion into an HTML text node — the full set that matters there. Labels are
/// trusted source code, not user input, so this is not a security boundary. It exists so a future label (e.g. one
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
