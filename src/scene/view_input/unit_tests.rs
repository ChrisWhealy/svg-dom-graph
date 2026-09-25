use super::keyboard::{label, percent, write_label};
use crate::test_support::check;

#[test]
fn the_label_reports_the_zoom_as_a_rounded_percentage() -> Result<(), String> {
    check(label(1.0) == "Graph view, zoom 100%", &label(1.0))?;
    check(label(1.25) == "Graph view, zoom 125%", &label(1.25))?;
    check(label(1.5625) == "Graph view, zoom 156%", &label(1.5625))?;
    check(label(0.25) == "Graph view, zoom 25%", &label(0.25))?;
    check(label(4.0) == "Graph view, zoom 400%", &label(4.0))
}

/// The label is rebuilt on every flushed frame of a pan or zoom. Writing into a reused buffer must therefore replace its
/// content and never reallocate once it is big enough.
#[test]
fn writing_the_label_replaces_the_buffer_and_never_reallocates_it() -> Result<(), String> {
    let mut buffer = String::with_capacity(64);
    let (pointer, capacity) = (buffer.as_ptr(), buffer.capacity());

    for scale in [1.0, 1.25, 1.5625, 4.0, 0.25, 1.0, 2.5, 0.75] {
        write_label(scale, &mut buffer);
        check(buffer == label(scale), &format!("stale content: {buffer:?}"))?;
    }
    check(buffer.as_ptr() == pointer, "the buffer was moved to a new allocation")?;
    check(buffer.capacity() == capacity, "the buffer's capacity changed")
}

/// A pan never changes the zoom, so it never changes the percentage, and the label is then left alone. The percentage is
/// what is compared, so it must agree with what the label says, and must change exactly when the label's text would.
#[test]
fn the_percentage_changes_exactly_when_the_labels_text_does() -> Result<(), String> {
    let scales = [0.25, 0.8, 1.0, 1.004, 1.0049, 1.25, 1.5625, 1.9531, 2.4414, 4.0];
    for pair in scales.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        check(
            (percent(a) == percent(b)) == (label(a) == label(b)),
            &format!(
                "{a} and {b}: percent {} vs {}, label {:?} vs {:?}",
                percent(a),
                percent(b),
                label(a),
                label(b)
            ),
        )?;
    }
    check(
        percent(1.0) == 100 && percent(1.25) == 125 && percent(0.25) == 25,
        "wrong percentages",
    )?;
    check(label(1.004) == label(1.0), "a change too small to show altered the label")
}
