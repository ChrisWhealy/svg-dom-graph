use super::encode::{Alphabet, encode, encoded_len, sextets, utf8_prefix};

fn text(input: &[u8]) -> String {
    String::from_utf8(encode(input, Alphabet::Standard).text).unwrap_or_default()
}

#[test]
fn the_rfc_4648_test_vectors_encode_as_published() {
    for (input, expected) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(text(input.as_bytes()), expected, "encoding {input:?}");
    }
}

#[test]
fn three_bytes_make_four_values_of_six_bits() {
    // "Man" is 01001101 01100001 01101110, which cuts into 010011 010110 000101 101110.
    assert_eq!(sextets(*b"Man"), [19, 22, 5, 46]);
}

#[test]
fn a_short_group_is_padded_with_zero_bits() {
    assert_eq!(encode(b"f", Alphabet::Standard).sextets, vec![25, 32, 0, 0]);
}

#[test]
fn the_encoded_length_counts_the_padding() {
    for len in 0..20 {
        let input = vec![0xAB; len];
        assert_eq!(encode(&input, Alphabet::Standard).text.len(), encoded_len(len));
    }
}

fn url_text(input: &[u8]) -> String {
    String::from_utf8(encode(input, Alphabet::UrlSafe).text).unwrap_or_default()
}

#[test]
fn the_url_safe_alphabet_replaces_plus_and_slash_with_minus_and_underscore() {
    // 0xFB 0xEF 0xBE is four sextets of 62, and 0xFF 0xFF 0xFF is four of 63.
    assert_eq!(text(&[0xFB, 0xEF, 0xBE]), "++++");
    assert_eq!(url_text(&[0xFB, 0xEF, 0xBE]), "----");
    assert_eq!(text(&[0xFF, 0xFF, 0xFF]), "////");
    assert_eq!(url_text(&[0xFF, 0xFF, 0xFF]), "____");
}

#[test]
fn the_two_alphabets_differ_only_at_values_62_and_63() {
    let (standard, url) = (Alphabet::Standard.characters(), Alphabet::UrlSafe.characters());
    assert_eq!(standard[..62], url[..62]);
    assert_eq!([standard[62], standard[63]], *b"+/");
    assert_eq!([url[62], url[63]], *b"-_");
}

#[test]
fn the_url_safe_alphabet_still_pads_with_equals() {
    assert_eq!(url_text(b"f"), "Zg==");
}

#[test]
fn text_is_cut_between_characters_never_inside_one() {
    // 255 ASCII bytes and then a euro sign of three bytes: only the first byte of it would fit in 256.
    let text = format!("{}\u{20AC}", "a".repeat(255));
    let prefix = utf8_prefix(&text, 256);
    assert_eq!(prefix.len(), 255);
    assert!(std::str::from_utf8(prefix).is_ok());
    // Text that fits is left whole, and so is text that ends exactly at the limit.
    assert_eq!(utf8_prefix("h\u{E9}llo", 256), "h\u{E9}llo".as_bytes());
    assert_eq!(utf8_prefix("abcd", 4), b"abcd");
    // A limit inside the first character leaves nothing.
    assert!(utf8_prefix("\u{20AC}", 2).is_empty());
}
