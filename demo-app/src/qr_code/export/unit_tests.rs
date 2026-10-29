use super::{DARK, ExportError, FILE_NAME, LIGHT, MODULE_PIXELS, MODULES, QUIET_ZONE, svg, xml_escape};
use crate::qr_code::{
    grid::{BitGrid, CellState, SIZE},
    make_set,
    snapshots::COUNT,
};

/// The finished symbol of `text`, and the symbol as rows of text: `1` for dark and `0` for light.
fn finished(text: &str) -> (BitGrid, Vec<String>) {
    let set = make_set(text).expect("the text fits");
    let grid = set.snapshots[COUNT - 1].grid.clone();
    let rows = (0..SIZE)
        .map(|row| {
            (0..SIZE)
                .map(|col| if grid.get(row, col) == Some(CellState::Dark) { '1' } else { '0' })
                .collect()
        })
        .collect();
    (grid, rows)
}

/// The text of the attribute `name` of the first element called `tag`, which is how a file this module makes is read.
fn attribute(svg: &str, tag: &str, name: &str) -> String {
    let start = svg.find(&format!("<{tag}")).unwrap_or_else(|| panic!("no <{tag}> in {svg}"));
    let element = &svg[start..svg[start..].find('>').map_or(svg.len(), |end| start + end)];
    let key = format!("{name}=\"");
    let from = element.find(&key).unwrap_or_else(|| panic!("no {name} on <{tag}>: {element}")) + key.len();
    element[from..from + element[from..].find('"').expect("the attribute is closed")].to_owned()
}

/// The picture a file draws, as a square of `MODULES` rows of `0` and `1`, by reading its path as a scanner would see
/// it: the background is light, and each rectangle `Mx yhWv1h-Wz` of the path makes `W` modules dark.
fn picture(svg: &str) -> Vec<Vec<bool>> {
    let mut modules = vec![vec![false; MODULES]; MODULES];
    for rectangle in attribute(svg, "path", "d").split('z').filter(|part| !part.trim().is_empty()) {
        let rectangle = rectangle.trim();
        let (corner, rest) = rectangle.split_once('h').expect("a rectangle has a width");
        let (x, y) = corner
            .strip_prefix('M')
            .and_then(|c| c.split_once(' '))
            .expect("a rectangle starts with M x y");
        let (width, rest) = rest.split_once('v').expect("a rectangle has a height");
        assert_eq!(
            rest,
            format!("1h-{width}"),
            "the rectangle is one module high, and closes back to its start"
        );
        let (x, y, width): (usize, usize, usize) = (
            x.parse().expect("x is a whole number"),
            y.parse().expect("y is a whole number"),
            width.parse().expect("the width is a whole number"),
        );
        for module in &mut modules[y][x..x + width] {
            assert!(!*module, "two rectangles cover the same module");
            *module = true;
        }
    }
    modules
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The size
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_picture_is_the_symbol_and_four_light_modules_on_every_side() {
    assert_eq!((QUIET_ZONE, MODULES), (4, 29));
    let (grid, _) = finished("This is a QR code");
    let file = svg(&grid, "This is a QR code").expect("a finished symbol");
    assert_eq!(attribute(&file, "svg", "viewBox"), "0 0 29 29");
}

#[test]
fn every_module_is_a_whole_number_of_pixels() {
    let (grid, _) = finished("Hello");
    let file = svg(&grid, "Hello").expect("a finished symbol");
    let (width, height) = (attribute(&file, "svg", "width"), attribute(&file, "svg", "height"));
    assert_eq!((width.as_str(), height.as_str()), ("290", "290"));
    assert_eq!(290 % MODULES, 0);
    assert_eq!(290 / MODULES, MODULE_PIXELS);
    // The edges are sharp, not antialiased.
    assert_eq!(attribute(&file, "svg", "shape-rendering"), "crispEdges");
}

#[test]
fn the_file_is_pure_black_on_pure_white_and_nothing_else() {
    let (grid, _) = finished("Hello");
    let file = svg(&grid, "Hello").expect("a finished symbol");
    assert_eq!(attribute(&file, "rect", "fill"), LIGHT);
    assert_eq!(attribute(&file, "path", "fill"), DARK);
    assert_eq!(file.matches("fill=").count(), 2);
    assert!(!file.contains("stroke") && !file.contains("opacity") && !file.contains("gradient"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// What it draws
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_quiet_zone_is_light_and_the_symbol_inside_it_is_the_symbol() {
    for text in ["", "A", "This is a QR code", "abcdefghijklmnopq", "<&>\"'"] {
        let (grid, rows) = finished(text);
        let drawn = picture(&svg(&grid, text).expect("a finished symbol"));
        for (row, line) in drawn.iter().enumerate() {
            for (col, &dark) in line.iter().enumerate() {
                let inside =
                    (QUIET_ZONE..QUIET_ZONE + SIZE).contains(&row) && (QUIET_ZONE..QUIET_ZONE + SIZE).contains(&col);
                if inside {
                    let expected = rows[row - QUIET_ZONE].as_bytes()[col - QUIET_ZONE] == b'1';
                    assert_eq!(dark, expected, "{text:?} at ({}, {})", row - QUIET_ZONE, col - QUIET_ZONE);
                } else {
                    assert!(!dark, "{text:?}: the quiet zone is dark at ({row}, {col})");
                }
            }
        }
    }
}

#[test]
fn every_dark_module_is_drawn_once_and_no_other() {
    let (grid, _) = finished("Hello, World!");
    let drawn = picture(&svg(&grid, "Hello, World!").expect("a finished symbol"));
    let dark_in_symbol = grid.count_state(CellState::Dark);
    assert_eq!(drawn.iter().flatten().filter(|&&dark| dark).count(), dark_in_symbol);
}

#[test]
fn the_same_symbol_makes_the_same_file_and_another_makes_another() {
    let (a, _) = finished("Hello");
    let (b, _) = finished("Hellp");
    assert_eq!(svg(&a, "Hello"), svg(&a, "Hello"));
    assert_ne!(svg(&a, "Hello"), svg(&b, "Hellp"));
}

#[test]
fn a_dark_module_next_to_another_is_one_rectangle_with_no_gap_between() {
    let (grid, _) = finished("Hello");
    let file = svg(&grid, "Hello").expect("a finished symbol");
    // The first row of a finder pattern is seven dark modules, which is one rectangle seven wide.
    assert!(
        attribute(&file, "path", "d").starts_with("M4 4h7v1h-7z"),
        "{}",
        attribute(&file, "path", "d")
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The text in the file
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_four_characters_that_could_start_markup_are_escaped() {
    assert_eq!(xml_escape("a<b>&\"c'"), "a&lt;b&gt;&amp;&quot;c&apos;");
    assert_eq!(xml_escape("plain text"), "plain text");
    assert_eq!(xml_escape("&lt;"), "&amp;lt;", "an entity in the text is text, not an entity");
}

#[test]
fn text_that_looks_like_markup_is_character_data_and_never_markup() {
    let (grid, _) = finished("<b onclick=\"x\">");
    let file = svg(&grid, "<b onclick=\"x\">").expect("a finished symbol");
    assert_eq!(file.matches("<b").count(), 0, "markup from the text reached the file: {file}");
    assert_eq!(file.matches("<svg").count(), 1);
    assert_eq!(file.matches("</svg>").count(), 1);
    assert!(file.contains("&lt;b onclick=&quot;x&quot;&gt;"), "{file}");
}

#[test]
fn a_control_character_is_written_as_a_visible_escape_since_xml_cannot_hold_one() {
    let text = "a\u{1}\tb\u{7F}\n";
    let (grid, _) = finished(text);
    let file = svg(&grid, text).expect("a finished symbol");
    assert!(file.contains("a\\x01\\tb\\x7F\\n"), "{file}");
    assert!(
        file.chars().all(|c| !c.is_control() || c == '\n'),
        "a control character other than a line feed is in the file"
    );
}

#[test]
fn the_file_is_one_well_formed_document() {
    let (grid, _) = finished("This is a QR code");
    let file = svg(&grid, "This is a QR code").expect("a finished symbol");
    assert!(file.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(file.ends_with("</svg>\n"));
    assert_eq!(file.matches('<').count(), file.matches('>').count());
    assert_eq!(file.matches("<title").count(), 1);
    assert_eq!(file.matches("<desc").count(), 1);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// What cannot be exported
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_grid_that_is_not_a_finished_symbol_is_not_exported() {
    assert_eq!(
        svg(&BitGrid::new(5, 5), ""),
        Err(ExportError::NotASymbol { width: 5, height: 5 })
    );
    assert_eq!(
        svg(&BitGrid::new(SIZE, SIZE), ""),
        Err(ExportError::Unfinished { row: 0, col: 0 })
    );
    // A symbol with one module without a colour is not finished either, wherever it is.
    let set = make_set("Hello").expect("the text fits");
    let unfinished = &set.snapshots[26].grid;
    assert!(matches!(svg(unfinished, "Hello"), Err(ExportError::Unfinished { .. })));
}

#[test]
fn the_errors_say_what_is_wrong_in_words() {
    let wrong_size = ExportError::NotASymbol { width: 5, height: 4 }.to_string();
    assert!(wrong_size.contains("5 by 4") && wrong_size.contains("21 by 21"), "{wrong_size}");
    let unfinished = ExportError::Unfinished { row: 3, col: 8 }.to_string();
    assert!(
        unfinished.contains("(3, 8)") && unfinished.contains("no colour"),
        "{unfinished}"
    );
}

#[test]
fn the_file_is_named_so_that_a_browser_saves_it_as_svg() {
    assert_eq!(FILE_NAME, "qr-code.svg");
    assert!(FILE_NAME.ends_with(".svg"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Files for the decoder check
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// What the decoder check is given: the empty text, ordinary text, punctuation, the longest text and one byte under it,
/// text that looks like markup, and control characters.
const DECODER_INPUTS: [&str; 11] = [
    "",
    "A",
    "This is a QR code",
    "Hello, World!",
    "https://a.co/x",
    "!\"#$%&'()*+,-./:;",
    "abcdefghijklmnop",
    "abcdefghijklmnopq",
    "<b onclick=\"x\">",
    "tabs\tand\nlines",
    "\u{1}\u{7F}\0",
];

/// `text` as a JSON string.
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Writes an export of each of [`DECODER_INPUTS`] to the directory `QR_EXPORT_DIR`, with a `manifest.json` that says
/// what each file holds, for `decode_exports.py` to read. It is ignored, so that an ordinary test run writes nothing.
///
/// Run it with `QR_EXPORT_DIR=some/dir cargo test -p svg-dom-graph-demo write_exports -- --ignored`.
#[test]
#[ignore = "writes files, for decode_exports.py to read, when QR_EXPORT_DIR names a directory"]
fn write_exports_for_the_decoder_check() {
    let Some(dir) = std::env::var_os("QR_EXPORT_DIR") else {
        panic!("QR_EXPORT_DIR must name the directory to write the exports to");
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("the directory can be made");
    let mut entries = Vec::new();
    for (index, text) in DECODER_INPUTS.iter().enumerate() {
        let (grid, _) = finished(text);
        let name = format!("export-{index:02}.svg");
        std::fs::write(dir.join(&name), svg(&grid, text).expect("a finished symbol")).expect("the file can be written");
        entries.push(format!(
            "  {{\"file\": {}, \"text\": {}}}",
            json_string(&name),
            json_string(text)
        ));
    }
    std::fs::write(dir.join("manifest.json"), format!("[\n{}\n]\n", entries.join(",\n")))
        .expect("the manifest can be written");
}

#[test]
fn every_input_for_the_decoder_check_makes_a_finished_symbol() {
    for text in DECODER_INPUTS {
        let (grid, _) = finished(text);
        assert!(svg(&grid, text).is_ok(), "{text:?}");
    }
    assert!(DECODER_INPUTS.iter().any(|text| text.is_empty()));
    assert!(DECODER_INPUTS.iter().any(|text| text.len() == 17));
    assert!(DECODER_INPUTS.iter().any(|text| text.len() == 16));
}

#[test]
fn json_strings_are_escaped() {
    assert_eq!(json_string("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\u000a\\u0001\"");
}
