use super::file_requested;

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
