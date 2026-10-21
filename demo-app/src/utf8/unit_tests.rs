use super::encode::{
    SAMPLES, button_label, byte_template_bits, encode, explain, glyph, hex_of, length_of, payload_groups, payload_len,
    template, template_bits,
};
use super::layout::{centred_lefts, centred_over};

/// One character from every length, and from either side of each boundary between lengths.
const EDGES: [char; 11] = [
    'A', '\u{7F}', '\u{80}', '£', '\u{7FF}', '\u{800}', '€', '\u{FFFF}', '\u{10000}', '😀', '\u{10FFFF}',
];

#[test]
fn it_matches_the_standard_library_for_every_character() {
    for c in (0..=0x10FFFF).filter_map(char::from_u32) {
        let mut expected = [0_u8; 4];
        let expected = c.encode_utf8(&mut expected).as_bytes();
        assert_eq!(encode(c).bytes, expected, "U+{:04X}", u32::from(c));
    }
}

#[test]
fn a_character_takes_the_length_of_its_range() {
    let lengths: Vec<usize> = EDGES.iter().map(|&c| encode(c).bytes.len()).collect();
    assert_eq!(lengths, [1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4]);
    assert_eq!(length_of(0x7F), 1);
    assert_eq!(length_of(0x80), 2);
    assert_eq!(length_of(0x7FF), 2);
    assert_eq!(length_of(0x800), 3);
    assert_eq!(length_of(0xFFFF), 3);
    assert_eq!(length_of(0x10000), 4);
}

#[test]
fn the_euro_sign_is_e2_82_ac() {
    let e = encode('€');
    assert_eq!(e.bytes, [0xE2, 0x82, 0xAC]);
    assert_eq!(e.payload, [0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 0, 0]);
    assert_eq!(e.bits.len(), 24);
}

#[test]
fn the_payload_has_exactly_the_bits_the_template_leaves() {
    for c in EDGES {
        let e = encode(c);
        assert_eq!(e.payload.len(), payload_len(e.bytes.len()));
        assert_eq!(e.bits.len(), 8 * e.bytes.len());
        // Every `x` of the template, read off the encoded bits, is the payload in order.
        let x_bits: Vec<u8> = (0..e.bits.len())
            .filter(|i| !template_bits(e.bytes.len()).contains(i))
            .map(|i| e.bits[i])
            .collect();
        assert_eq!(x_bits, e.payload, "U+{:04X}", u32::from(c));
    }
}

#[test]
fn the_template_bits_are_the_marker_bits_of_each_byte() {
    assert_eq!(template_bits(1), [0]);
    assert_eq!(template_bits(2), [0, 1, 2, 8, 9]);
    assert_eq!(template_bits(3), [0, 1, 2, 3, 8, 9, 16, 17]);
    assert_eq!(template_bits(4), [0, 1, 2, 3, 4, 8, 9, 16, 17, 24, 25]);
    // The template's own 1s and 0s are what the highlighted bits of the encoding hold.
    for c in EDGES {
        let e = encode(c);
        let marked: String = template_bits(e.bytes.len())
            .iter()
            .map(|&i| char::from(b'0' + e.bits[i]))
            .collect();
        let expected: String = template(e.bytes.len()).chars().filter(|&t| t != 'x' && t != ' ').collect();
        assert_eq!(marked, expected, "U+{:04X}", u32::from(c));
    }
}

#[test]
fn the_note_names_the_code_point_the_length_and_the_range() {
    let note = explain('€', &encode('€'));
    assert!(note.starts_with("### U+20AC `€`\n"), "{note}");
    assert!(note.contains("U+0800 to U+FFFF") && note.contains("3 bytes"), "{note}");
    assert!(note.contains("`1110xxxx 10xxxxxx 10xxxxxx`"), "{note}");
    let ascii = explain('A', &encode('A'));
    assert!(ascii.contains("1 byte.") && ascii.contains("It is ASCII"), "{ascii}");
}

#[test]
fn the_glyph_is_left_out_when_it_would_show_as_nothing_or_break_the_markdown() {
    assert_eq!(glyph('€').as_deref(), Some("`€`"));
    for c in ['\t', ' ', '\u{0}', '`', '\\'] {
        assert_eq!(glyph(c), None, "U+{:04X}", u32::from(c));
    }
}

#[test]
fn there_is_one_sample_for_each_length_in_order() {
    let lengths: Vec<usize> = SAMPLES.iter().map(|&c| encode(c).bytes.len()).collect();
    assert_eq!(lengths, [1, 2, 3, 4]);
}

#[test]
fn the_result_is_the_bytes_of_the_character() {
    assert_eq!(hex_of('A'), "41");
    assert_eq!(hex_of('€'), "E2 82 AC");
    assert_eq!(hex_of('😀'), "F0 9F 98 80");
}

#[test]
fn a_button_names_the_character_its_code_point_and_its_length() {
    assert_eq!(button_label('A'), "A U+0041, 1 byte");
    assert_eq!(button_label('😀'), "😀 U+1F600, 4 bytes");
}

#[test]
fn the_groups_of_code_point_bits_are_what_each_byte_has_room_for() {
    assert_eq!(payload_groups(1), [7]);
    assert_eq!(payload_groups(2), [5, 6]);
    assert_eq!(payload_groups(3), [4, 6, 6]);
    assert_eq!(payload_groups(4), [3, 6, 6, 6]);
    for len in 1..=4 {
        assert_eq!(payload_groups(len).iter().sum::<usize>(), payload_len(len));
    }
}

#[test]
fn each_group_of_the_payload_is_the_non_template_bits_of_its_byte() {
    for c in EDGES {
        let e = encode(c);
        let len = e.bytes.len();
        let mut from = 0;
        for (byte, width) in payload_groups(len).into_iter().enumerate() {
            let template = byte_template_bits(len, byte);
            let room: Vec<u8> = (0..8)
                .filter(|bit| !template.contains(bit))
                .map(|bit| e.bits[8 * byte + bit])
                .collect();
            assert_eq!(room, e.payload[from..from + width], "U+{:04X} byte {byte}", u32::from(c));
            // The template is on the left of the byte, so the group sits on its right: that is what the demo lines up.
            assert_eq!(template, (0..8 - width).collect::<Vec<_>>());
            from += width;
        }
    }
}

/// The middle of a row as a person sees it: the middle of the node in the middle, or of the gap between the two in the
/// middle.
fn middle(lefts: &[f64], widths: &[f64]) -> f64 {
    let half = lefts.len() / 2;
    if lefts.len() % 2 == 1 {
        lefts[half] + widths[half] / 2.0
    } else {
        (lefts[half - 1] + widths[half - 1] + lefts[half]) / 2.0
    }
}

#[test]
fn a_row_of_any_number_of_unequal_nodes_is_centred_on_the_axis() {
    let rows: [&[f64]; 5] = [
        &[100.0],
        &[80.0, 120.0],
        &[60.0, 100.0, 100.0],
        &[60.0, 100.0, 100.0, 100.0],
        &[1.0; 7],
    ];
    for widths in rows {
        let lefts = centred_lefts(widths, 20.0, 500.0);
        assert!((middle(&lefts, widths) - 500.0).abs() < 1e-9, "{widths:?}");
        // The boxes are in order, and the gap between neighbours is the same.
        for i in 1..widths.len() {
            assert!(
                (lefts[i] - (lefts[i - 1] + widths[i - 1]) - 20.0).abs() < 1e-9,
                "{widths:?} at {i}"
            );
        }
    }
}

#[test]
fn one_node_is_centred_and_an_empty_row_is_empty() {
    assert_eq!(centred_lefts(&[100.0], 20.0, 500.0), [450.0]);
    assert!(centred_lefts(&[], 20.0, 500.0).is_empty());
}

#[test]
fn rows_of_equal_nodes_have_the_same_centres() {
    // The row of groups and the row of bytes of a three byte character: the nodes in the middle share a midline.
    let groups = centred_lefts(&[90.0, 130.0, 130.0], 20.0, 500.0);
    let bytes = centred_lefts(&[180.0, 180.0, 180.0], 20.0, 500.0);
    assert!((groups[1] + 65.0 - 500.0).abs() < 1e-9 && (bytes[1] + 90.0 - 500.0).abs() < 1e-9);
}

#[test]
fn each_group_is_centred_over_its_byte() {
    let bytes = centred_lefts(&[180.0; 4], 20.0, 500.0);
    let groups = centred_over(&bytes, &[180.0; 4], &[100.0, 140.0, 140.0, 140.0]);
    for (n, (&byte, &group)) in bytes.iter().zip(&groups).enumerate() {
        let widths = [100.0, 140.0, 140.0, 140.0];
        assert!((group + widths[n] / 2.0 - (byte + 90.0)).abs() < 1e-9, "group {n}");
    }
    // The two middle groups are as wide as each other, so the gap between them is on the axis, as the gap between the two
    // middle bytes is.
    assert!(((groups[1] + 140.0 + groups[2]) / 2.0 - 500.0).abs() < 1e-9);
}
