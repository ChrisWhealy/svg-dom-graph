//! Assembles `index.html` from `demo/index.template.html`, a generated `<nav>` menu, and the panel fragments in
//! `demo/panels/`.
//!
//! Mirrors `svg-dom`'s own `demo-server/src/panels/mod.rs`. Its menu groups panels under headings like "Basic Nodes"
//! and "Operators", each of which expands to show its panels.
//!
//! The panels describe themselves. Each fragment's opening `<section>` tag carries the group the panel is filed under,
//! its menu label and its place within the group, as `data-group`, `data-label` and `data-order`. The fragments in
//! `demo/panels/` are therefore the single source of truth for which panels exist, and nothing else lists them.
//! [`GROUP_ORDER`] only decides the order of the headings.
//!
//! It does not know anything about the Rust demo functions that build each panel's content. That mapping lives in
//! `demo-app/src/lib.rs`'s `demo_gallery!` invocation instead. That invocation holds a separate list for a separate
//! reason (see that macro's own doc comment). The job of [`super::validate`] is to keep the two ids in step.
//!
//! # Adding a new demo
//!
//! Create `demo/panels/{id}.html`, whose root element is
//! `<section class="section" id="{id}" data-group="..." data-label="..." data-order="...">`. Orders within a group are
//! numbered in tens, so that a panel can be slotted between two others. Then add the matching `demo_gallery!` entry in
//! `demo-app/src/lib.rs`. A panel in a group that is new needs the group adding to [`GROUP_ORDER`].

mod error;

pub(crate) use error::AssembleError;
use std::{fs, path::Path};

const PANELS_PLACEHOLDER: &str = "{{PANELS}}";
const MENU_PLACEHOLDER: &str = "{{MENU}}";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The headings of the menu, in the order they are drawn. A group with no panels is not drawn.
const GROUP_ORDER: &[&str] = &[
    "Basic Nodes",
    "Data Nodes",
    "Operators",
    "Cell selection",
    "Text nodes",
    "Nested scenes",
    "Cryptographic Algorithms",
    "Encoding & Compression",
    "Error Detection & Correction",
    "Sorting",
    "Graph algorithms",
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One panel, as its own fragment describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    pub id: String,
    pub group: String,
    /// The text of the panel's menu item. A group that holds only this panel shows the group's heading instead.
    pub label: String,
    /// Where the panel comes in its group: lower first, and a tie goes to the lower id.
    pub order: u32,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every panel in `panels_dir`, in menu order: by the position of its group in [`GROUP_ORDER`], then by `data-order`,
/// then by id.
///
/// Reads and checks every fragment, so a fragment that is wrongly named, whose id does not match its name, or that
/// lacks its metadata is reported here, whichever caller asked.
pub fn catalogue(panels_dir: &Path) -> Result<Vec<Panel>, AssembleError> {
    let mut panels = Vec::new();
    for id in list_fragment_ids(panels_dir)? {
        if !is_valid_panel_id(&id) {
            return Err(AssembleError::InvalidPanelId(id));
        }
        let fragment_path = panels_dir.join(format!("{id}.html"));
        let fragment = read_to_string(&fragment_path)?;
        if !fragment.contains(&format!("id=\"{id}\"")) {
            return Err(AssembleError::FragmentIdMismatch { id, fragment_path });
        }
        let attribute = |name: &'static str| {
            section_attribute(&fragment, name).ok_or_else(|| AssembleError::MissingAttribute {
                fragment_path: fragment_path.clone(),
                attribute: name,
            })
        };
        let group = attribute("data-group")?;
        let label = attribute("data-label")?;
        let order_text = attribute("data-order")?;
        let order = order_text.parse().map_err(|_| AssembleError::InvalidOrder {
            fragment_path: fragment_path.clone(),
            value: order_text.clone(),
        })?;
        if !GROUP_ORDER.contains(&group.as_str()) {
            return Err(AssembleError::UnknownGroup { fragment_path, group });
        }
        panels.push(Panel { id, group, label, order });
    }
    sort_into_menu_order(&mut panels);
    Ok(panels)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts `panels` in menu order. See [`catalogue`].
fn sort_into_menu_order(panels: &mut [Panel]) {
    let group_position = |panel: &Panel| GROUP_ORDER.iter().position(|g| *g == panel.group);
    panels.sort_by(|a, b| {
        group_position(a)
            .cmp(&group_position(b))
            .then(a.order.cmp(&b.order))
            .then_with(|| a.id.cmp(&b.id))
    });
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel id each `demo/panels/*.html` fragment is filed under (its filename, minus `.html`), in alphabetical order.
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
    ids.sort();
    Ok(ids)
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
        body.push_str(&read_to_string(&panels_dir.join(format!("{}.html", panel.id)))?);
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
    for heading in GROUP_ORDER {
        let members: Vec<&Panel> = panels.iter().filter(|panel| panel.group == *heading).collect();
        match members.as_slice() {
            [] => {},
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

#[cfg(test)]
mod unit_tests;
