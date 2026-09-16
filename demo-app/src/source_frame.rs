//! Each demo panel gets a collapsible frame below it showing the formatted Rust source of the function that built
//! it.
//!
//! [`crate::init_panel`] appends this itself, deliberately even when that function reported failure — the source
//! is still worth seeing when the demo itself broke.
//!
//! Every demo module embeds its own file's full source at compile time (`include_str!`, as a `pub(crate) const
//! SOURCE`), so the text shown here never drifts from what is actually running — there is no separate copy kept in
//! sync by hand. [`demo_fn_source`] slices a single top-level function's body out of whichever module's `SOURCE`
//! [`crate::DEMO_PANELS`] names for that panel. Mirrors `svg-dom`'s own demo gallery, which does the same thing
//! across its own many demo files.

use crate::DEMO_PANELS;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns the source text of the top-level `fn {name}` item in `source`, from its signature line through its
/// closing brace, or `None` if it cannot be located.
///
/// Relies on `rustfmt`'s guarantee that a top-level item's own closing brace always sits in column 0, while every
/// brace nested inside the body (including one inside a `format!` string) does not. Scanning for the first line
/// that is exactly `}` after the signature therefore finds the function's end without parsing anything. Mirrors
/// `svg-dom`'s own `demo_fn_source`.
pub(crate) fn demo_fn_source(source: &'static str, name: &str) -> Option<&'static str> {
    let needle = format!("fn {name}(");
    let hit = source.find(&needle)?;
    let start = source[..hit].rfind('\n').map_or(0, |i| i + 1);
    let tail = &source[hit..];
    let mut from = 0;

    loop {
        let rel = tail[from..].find("\n}")?;
        let close = from + rel + 1; // index of the '}' within `tail`
        let after = close + 1;
        // A genuine top-level close: the '}' stands alone on its line (next byte is a newline or end of file).
        if tail.as_bytes().get(after).is_none_or(|&b| b == b'\n') {
            return Some(&source[start..hit + after]);
        }
        from = after;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds `<details class="source" open><summary>...</summary><pre><code>...</code></pre></details>` and appends
/// it to `panel_id`'s own `<section>`, showing the exact source of the function that just built that panel. Mirrors
/// `svg-dom`'s own `append_source_frame`.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#{panel_id}`, if `panel_id` is not registered in
/// [`crate::DEMO_PANELS`]. Also returns `Err` if its function's source cannot be located (see [`demo_fn_source`]),
/// or if building any of the DOM nodes below fails.
pub(crate) fn append_demo_source(document: &web_sys::Document, panel_id: &str) -> Result<(), String> {
    let section = crate::util::required_element(document, panel_id)?;
    let panel = DEMO_PANELS
        .iter()
        .find(|panel| panel.id == panel_id)
        .ok_or_else(|| format!("{panel_id} is not registered in DEMO_PANELS"))?;
    let fn_name = panel.fn_name;
    let source_text =
        demo_fn_source(panel.source, fn_name).ok_or_else(|| format!("source not found for fn {fn_name}"))?;

    let create = |tag: &str| {
        document
            .create_element(tag)
            .map_err(|e| format!("create_element({tag:?}): {e:?}"))
    };

    let details = create("details")?;
    details.set_attribute("class", "source").map_err(|e| format!("{e:?}"))?;
    details.set_attribute("open", "").map_err(|e| format!("{e:?}"))?;

    let summary = create("summary")?;
    summary.set_text_content(Some(&format!("Rust source — fn {fn_name}")));
    details.append_child(&summary).map_err(|e| format!("{e:?}"))?;

    let pre = create("pre")?;
    let code = create("code")?;
    // `rust_to_html` returns `<span>`-wrapped, HTML-escaped tokens, so angle brackets and ampersands in the code
    // still render verbatim while keywords, strings, etc. are coloured by `demo/style.css`.
    code.set_inner_html(&crate::highlight::rust_to_html(source_text));
    pre.append_child(&code).map_err(|e| format!("{e:?}"))?;
    details.append_child(&pre).map_err(|e| format!("{e:?}"))?;

    section.append_child(&details).map_err(|e| format!("{e:?}"))?;
    Ok(())
}
