//! The marks over a hex dump's bytes are painted in a fixed order, whichever order the selection calls were made in.
//!
//! From the bottom: the dump's own characters, the blocks that dim unreached bytes, the secondary marks, and last the
//! selected byte's marks. SVG paints later siblings over earlier ones, so this reads where each mark is among its
//! siblings. The fixture has three dumps with byte 3 unreached and secondary, set in each order, and the third also
//! selected.

use crate::common::new_tab;

/// For the dump named `name`: the position among its group's children of the last block that dims a byte, of the hex
/// digits' secondary mark, and of the hex digits' primary mark, or `-1` for one that is not there.
fn positions(name: &str) -> Result<(i64, i64, i64), String> {
    let tab = new_tab()?;
    let script = format!(
        "JSON.stringify((() => {{ \
           const g = document.querySelector('#diagram g[aria-label^=\"{name}\"]'); \
           const kids = [...g.children]; \
           const shown = kids.filter(k => k.tagName === 'rect' && k.getAttribute('visibility') === 'visible'); \
           const at = el => el ? kids.indexOf(el) : -1; \
           const dim = shown.filter(r => r.getAttribute('opacity') === '0.65').pop(); \
           const mark = fill => shown.find(r => r.getAttribute('fill') === fill && r.nextElementSibling.textContent.length === 2); \
           return [at(dim), at(mark('#7fd8be')), at(mark('#ff6b4a'))]; }})())"
    );
    let json = tab
        .evaluate(&script, false)
        .map_err(|e| format!("could not read {name}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or("no positions returned")?;
    let [dim, secondary, primary]: [i64; 3] = serde_json::from_str(&json).map_err(|e| format!("bad positions: {e}"))?;
    Ok((dim, secondary, primary))
}

fn check_order(name: &str, with_primary: bool) -> Result<(), String> {
    let (dim, secondary, primary) = positions(name)?;
    if dim < 0 || secondary < 0 {
        return Err(format!(
            "{name}: the dimmed and secondary marks must both be shown, got {dim} and {secondary}"
        ));
    }
    if dim >= secondary {
        return Err(format!(
            "{name}: the dimming (at {dim}) is painted over the secondary mark (at {secondary})"
        ));
    }
    if with_primary && secondary >= primary {
        return Err(format!(
            "{name}: the secondary mark (at {secondary}) is painted over the primary one (at {primary})"
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Secondary first and then unreached: the secondary mark is still over the dimming.
#[test]
fn secondary_set_first_is_still_painted_over_the_dimming() -> Result<(), String> {
    check_order("dump_a", false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Unreached first and then secondary: the same order.
#[test]
fn secondary_set_last_is_painted_over_the_dimming() -> Result<(), String> {
    check_order("dump_b", false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With the byte also selected, the order is dimming, then secondary, then primary.
#[test]
fn the_selected_byte_is_over_the_secondary_mark_which_is_over_the_dimming() -> Result<(), String> {
    check_order("dump_c", true)
}
