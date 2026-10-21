//! Row labels drawn from `LabellingStyle::Text` stay inside their node, whatever script they are in.
//!
//! A label's width is not its character count: a CJK glyph is wider than a Latin one, and a combining mark has no
//! advance at all. The node's box is sized by measuring every label as rendered, so no label should ever spill out of
//! it. This asserts that observable contract in a real browser, against the real rendered geometry, instead of repeating
//! the width-selection algorithm. Which label is widest depends on the fonts the machine has, and nothing here assumes
//! it.

use crate::common::new_tab;

/// How far a label may sit outside the node's box, in pixels, before it counts as spilling out. A rendered bounding box
/// is rounded to the browser's own layout units, so an exact comparison would fail on rounding alone.
const TOLERANCE: f64 = 0.5;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The fixture's `labelled` node (`#diagram > g.svg-dom-graph-content > g:nth-of-type(9)`) has four rows labelled in
/// Latin, CJK, combining marks and Latin again. Every visible row label's own bounding box must lie within the node's
/// outer box, which is the node's own first `<rect>`.
#[test]
fn every_row_label_stays_inside_the_node_whatever_its_script() -> Result<(), String> {
    let tab = new_tab()?;
    let script = r#"
        (() => {
            const group = document.querySelector('#diagram > g.svg-dom-graph-content > g:nth-of-type(9)');
            const outer = group.querySelector('rect').getBoundingClientRect();
            const labels = [...group.querySelectorAll('text[aria-hidden]')];
            return JSON.stringify({
                outer: [outer.left, outer.top, outer.right, outer.bottom],
                labels: labels.map(label => {
                    const box = label.getBoundingClientRect();
                    return [label.textContent, box.left, box.top, box.right, box.bottom];
                }),
            });
        })()
    "#;
    let result = tab
        .evaluate(script, false)
        .map_err(|e| format!("could not read the node's geometry: {e}"))?;
    let json = result
        .value
        .as_ref()
        .and_then(|v| v.as_str())
        .ok_or("the geometry script returned no string")?;
    let parsed: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("bad geometry JSON: {e}"))?;

    let number = |value: &serde_json::Value| value.as_f64().ok_or_else(|| format!("{value} is not a number"));
    let outer: Vec<f64> = parsed["outer"]
        .as_array()
        .ok_or("no outer box")?
        .iter()
        .map(number)
        .collect::<Result<_, _>>()?;
    let labels = parsed["labels"].as_array().ok_or("no labels")?;
    if labels.len() != 4 {
        return Err(format!("expected 4 visible row labels, found {}", labels.len()));
    }

    for label in labels {
        let text = label[0].as_str().unwrap_or("?");
        let (left, top, right, bottom) =
            (number(&label[1])?, number(&label[2])?, number(&label[3])?, number(&label[4])?);
        if left < outer[0] - TOLERANCE
            || top < outer[1] - TOLERANCE
            || right > outer[2] + TOLERANCE
            || bottom > outer[3] + TOLERANCE
        {
            return Err(format!(
                "label {text:?} spans ({left}, {top}) to ({right}, {bottom}), outside the node's box {outer:?}"
            ));
        }
    }
    Ok(())
}
