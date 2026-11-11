//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// One test for each page, so that the CI shards can take them one at a time.

#[test]
fn the_sha1_step_driver_is_hidden_from_the_accessibility_tree_and_its_toolbar_is_not() -> Result<(), String> {
    driver_is_hidden_and_toolbar_is_not("sha1-window.html")
}

#[test]
fn the_sha2_256_step_driver_is_hidden_from_the_accessibility_tree_and_its_toolbar_is_not() -> Result<(), String> {
    driver_is_hidden_and_toolbar_is_not("sha2-256-window.html")
}

#[test]
fn the_sha3_sponge_step_driver_is_hidden_from_the_accessibility_tree_and_its_toolbar_is_not() -> Result<(), String> {
    driver_is_hidden_and_toolbar_is_not("sha3-sponge-window.html")
}

#[test]
fn the_chacha20_step_driver_is_hidden_from_the_accessibility_tree_and_its_toolbar_is_not() -> Result<(), String> {
    driver_is_hidden_and_toolbar_is_not("chacha20-window.html")
}

#[test]
fn the_huffman_step_driver_is_hidden_from_the_accessibility_tree_and_its_toolbar_is_not() -> Result<(), String> {
    driver_is_hidden_and_toolbar_is_not("huffman-window.html")
}
