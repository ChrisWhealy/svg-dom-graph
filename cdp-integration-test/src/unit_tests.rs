use super::{file_requested, require_prebuilt};
use std::path::PathBuf;

#[test]
fn a_path_is_the_file_of_that_name() {
    assert_eq!(file_requested("/crc32-window.html"), "crc32-window.html");
    assert_eq!(file_requested("/pkg/demo_app_bg.wasm"), "pkg/demo_app_bg.wasm");
}

#[test]
fn the_query_string_is_not_part_of_the_file_name() {
    assert_eq!(file_requested("/hamming-window.html?error=5"), "hamming-window.html");
    assert_eq!(file_requested("/hamming-window.html?error=5&x=1"), "hamming-window.html");
}

#[test]
fn the_fragment_is_not_part_of_the_file_name() {
    assert_eq!(file_requested("/index.html#top"), "index.html");
    assert_eq!(file_requested("/index.html?a=b#top"), "index.html");
}

#[test]
fn an_empty_path_is_the_index_even_with_a_query() {
    assert_eq!(file_requested("/"), "index.html");
    assert_eq!(file_requested(""), "index.html");
    assert_eq!(file_requested("/?error=5"), "index.html");
}

/// A directory of its own under the system's temporary one, empty, for a test to put files in.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cdp-prebuilt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("could not create the scratch directory");
    dir
}

#[test]
fn a_prebuilt_package_with_its_marker_file_is_accepted() {
    let dir = scratch("present");
    std::fs::create_dir_all(dir.join("pkg")).expect("could not create pkg");
    std::fs::write(dir.join("pkg/fixture.js"), "").expect("could not write the marker");
    assert_eq!(require_prebuilt(&dir, "pkg/fixture.js"), Ok(()));
}

#[test]
fn a_prebuilt_package_without_its_marker_file_says_what_is_missing() {
    let dir = scratch("absent");
    let message = require_prebuilt(&dir, "pkg/fixture.js").expect_err("the marker is not there");
    assert!(message.contains("CDP_PREBUILT"), "{message}");
    assert!(message.contains("fixture.js"), "{message}");
}
