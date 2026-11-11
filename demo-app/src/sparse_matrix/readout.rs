//! The words and numbers of the readout, and the story of a step.

use super::live::Live;
use demo_common::util::document;
use spmv_core::{UpdateOrder, explain, stats::Latency};
use spmv_scene::scene::Selection;

pub(super) fn text(id: &str, value: &str) {
    if let Some(e) = document().ok().and_then(|d| d.get_element_by_id(id)) {
        e.set_text_content(Some(value));
    }
}

/// A count with thousands separators.
pub(super) fn n(v: u64) -> String {
    let digits = v.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub(super) fn ms(v: f64) -> String {
    if v >= 100.0 {
        format!("{v:.0} ms")
    } else if v >= 1.0 {
        format!("{v:.1} ms")
    } else {
        format!("{:.0} µs", v * 1000.0)
    }
}

/// A time as the page's clock can tell it: one that is less than a step of the clock is not told as zero.
pub(super) fn timed(v: f64, step: f64) -> String {
    if v < step { format!("under {}", ms(step)) } else { ms(v) }
}

pub(super) fn megabytes(v: Option<f64>) -> String {
    v.map_or_else(
        || "not reported by this browser".to_owned(),
        |b| format!("{:.1} MB", b / 1_048_576.0),
    )
}

pub(super) fn latency(samples: &[f64], step: f64) -> String {
    Latency::from_samples(samples).map_or_else(
        || "—".to_owned(),
        |l| {
            format!(
                "p50 {}, p95 {}, p99 {}, worst {} over {} samples",
                timed(l.p50, step),
                timed(l.p95, step),
                timed(l.p99, step),
                timed(l.max, step),
                l.n
            )
        },
    )
}

/// What the selected node computes, written out from the matrix and the vector as they are now.
pub(super) fn describe(live: &Live) -> String {
    let (matrix, x) = (live.prepared.matrix(), &live.run.x);
    match live.selection {
        Some(Selection::Row(i)) if i < matrix.rows => format!(
            "y{i} is made from row {i} of A: each number in the row is multiplied by the x of its column, and the products are \
             added.\n{}",
            explain::row(matrix, x, i)
        ),
        Some(Selection::Column(j)) if j < matrix.cols => explain::column_story(matrix, x, j),
        _ => "Click a node, or choose one above, to see what it computes.".to_owned(),
    }
}

/// The task, in plain words, for the graph that was built.
pub(super) fn task(rows: u64, cols: u64, nnz: usize, worked: bool) -> String {
    let what = "What this shows: a computer often has to multiply a table of numbers (the matrix A) by a list of numbers (x) to make a new \
                list (y). Search, graphics and machine learning all do it.";
    let step = "What a step does: A never changes. A step changes one number of x, and only the y that use that x are worked out again, \
                and they are outlined in green. Run keeps stepping, at the speed you choose.";
    if worked {
        format!(
            "{what}\nHow to read it: each y is made from one row of A. Multiply every number in the row by the x in its column, then add \
             the products. The boxes at the right do exactly that for the selected row, and the bars draw y. Click another row to see it.\n\
             {step} Steps go in order, x0 then x1 and so on, so you can tell what comes next. The Step order list can make it random.\n\
             Why it matters: most big tables are mostly zeros, so a computer keeps only the numbers that are not zero and skips the rest. \
             Choose a larger size to see how much a page can draw before it slows."
        )
    } else {
        format!(
            "{what}\nHow to read it: this matrix has {rows} rows, {cols} columns and {} numbers that are not zero. Each is a line from an \
             x on the left to a y on the right, and a y is the sum of each line's number times the x it comes from. Click a node to see \
             its sum.\n{step} The Step order list chooses in order or at random.\nWhy it matters: this size tests how big a drawing the \
             page can hold. The larger the matrix, the more lines the page must draw, and the readout under Measurements says what that \
             cost.",
            n(nnz as u64)
        )
    }
}

/// What the story line says before a step: how the steps will go.
pub(super) fn story_intro(order: UpdateOrder) -> &'static str {
    if order == UpdateOrder::InOrder {
        "Press Step to change the next entry of x, starting with x0, and see which y change. Steps go in order: x0, x1, x2 and so on round."
    } else {
        "Press Step to change an entry of x chosen at random, and see which y change. The choice comes from a seeded random stream, so it has no pattern."
    }
}

/// Says in plain words what a step did: which inputs changed, and so which outputs.
pub(super) fn story(step: u64, x0: &[i64], y0: &[i64], x1: &[i64], y1: &[i64]) -> String {
    let list = |before: &[i64], after: &[i64], name: &str, limit: usize| -> String {
        let changed: Vec<String> = (0..after.len().min(before.len()))
            .filter(|n| before[*n] != after[*n])
            .map(|n| format!("{name}{n} {} → {}", before[n], after[n]))
            .collect();
        let more = changed.len().saturating_sub(limit);
        let shown = changed.into_iter().take(limit).collect::<Vec<_>>().join(", ");
        if more > 0 { format!("{shown} and {more} more") } else { shown }
    };
    let (xs, ys) = (list(x0, x1, "x", 3), list(y0, y1, "y", 4));
    if xs.is_empty() {
        return format!("Step {step}: no input changed, so no output did.");
    }
    let outputs = if ys.is_empty() {
        "None of the outputs changed value.".to_owned()
    } else {
        format!("Outputs that use it changed: {ys}.")
    };
    format!("Step {step}: the input changed ({xs}). {outputs} A never changes, only x does.")
}
