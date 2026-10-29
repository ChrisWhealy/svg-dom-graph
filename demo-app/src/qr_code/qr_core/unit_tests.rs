use super::{
    ERROR_CORRECTION_CODEWORDS, EncodeError, TOTAL_CODEWORDS,
    bits::{BitError, BitWriter},
    data::{DATA_CODEWORDS, PAD_CODEWORDS, encode_data},
    encode, encode_text,
    gf256::Gf256,
    golden::GOLDEN,
    input::{MAX_BYTES, check_length, escaped, text_to_bytes},
    reed_solomon::{divide, generator_polynomial, remainder},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// References that share nothing with the code under test. Multiplication is the shift-and-reduce loop, with no tables.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

fn slow_mul(mut a: u8, mut b: u8) -> u8 {
    let mut product = 0;
    while b != 0 {
        if b & 1 == 1 {
            product ^= a;
        }
        let carry = a & 0x80 != 0;
        a <<= 1;
        if carry {
            a ^= 0x1D;
        }
        b >>= 1;
    }
    product
}

/// 2 to the power `exponent`, by repeated slow multiplication.
fn slow_alpha(exponent: usize) -> u8 {
    (0..exponent).fold(1, |power, _| slow_mul(power, 2))
}

/// The polynomial `coefficients`, highest power first, at `x`, by Horner's rule.
fn evaluate(coefficients: &[u8], x: u8) -> u8 {
    coefficients.iter().fold(0, |sum, &c| slow_mul(sum, x) ^ c)
}

/// The generator of degree `degree`, highest power first, multiplied out one factor at a time.
fn slow_generator(degree: usize) -> Vec<u8> {
    let mut generator = vec![1];
    for exponent in 0..degree {
        let root = slow_alpha(exponent);
        let mut next = vec![0; generator.len() + 1];
        for (i, &c) in generator.iter().enumerate() {
            next[i] ^= c;
            next[i + 1] ^= slow_mul(c, root);
        }
        generator = next;
    }
    generator
}

/// The remainder of `data` times x^degree over the generator, by whole-polynomial long division.
fn slow_remainder(data: &[u8], degree: usize) -> Vec<u8> {
    let generator = slow_generator(degree);
    let mut work: Vec<u8> = data.iter().copied().chain(std::iter::repeat_n(0, degree)).collect();
    for i in 0..data.len() {
        let factor = work[i];
        for (j, &g) in generator.iter().enumerate() {
            work[i + j] ^= slow_mul(g, factor);
        }
    }
    work[data.len()..].to_vec()
}

/// Whether `codewords` is a valid Reed-Solomon codeword: it is zero at each root of the generator.
fn has_zero_syndrome(codewords: &[u8], degree: usize) -> bool {
    (0..degree).all(|i| evaluate(codewords, slow_alpha(i)) == 0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The bit writer
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn bits_are_written_highest_first() {
    let mut writer = BitWriter::new(32);
    writer.push(0b0100, 4).expect("it fits");
    writer.push(0b101, 3).expect("it fits");
    assert_eq!(writer.bits(), [0, 1, 0, 0, 1, 0, 1]);
    assert_eq!(writer.len(), 7);
}

#[test]
fn a_group_of_32_bits_is_allowed() {
    let mut writer = BitWriter::new(64);
    writer.push(u32::MAX, 32).expect("it fits");
    assert_eq!(writer.bits(), [1; 32]);
}

#[test]
fn a_bad_width_or_a_value_too_wide_is_refused_and_changes_nothing() {
    let mut writer = BitWriter::new(32);
    writer.push(1, 1).expect("it fits");
    let before = writer.clone();
    assert_eq!(writer.push(0, 0), Err(BitError::BadWidth { width: 0 }));
    assert_eq!(writer.push(0, 33), Err(BitError::BadWidth { width: 33 }));
    assert_eq!(writer.push(4, 2), Err(BitError::ValueTooWide { value: 4, width: 2 }));
    assert_eq!(writer, before);
}

#[test]
fn passing_the_limit_is_refused_and_changes_nothing() {
    let mut writer = BitWriter::new(10);
    writer.push(0xFF, 8).expect("it fits");
    let before = writer.clone();
    assert_eq!(writer.push(0, 3), Err(BitError::Overflow { len: 8, width: 3, limit: 10 }));
    assert_eq!(writer.push_zeros(3), Err(BitError::Overflow { len: 8, width: 3, limit: 10 }));
    assert_eq!(writer, before);
    writer.push(0b11, 2).expect("exactly the limit fits");
}

#[test]
fn a_width_too_large_to_add_is_refused_and_does_not_overflow() {
    let mut writer = BitWriter::new(10);
    writer.push(0b1, 1).expect("it fits");
    let before = writer.clone();
    let overflow = BitError::Overflow {
        len: 1,
        width: usize::MAX,
        limit: 10,
    };
    assert_eq!(writer.push_zeros(usize::MAX), Err(overflow));
    assert_eq!(
        writer.push_zeros(usize::MAX - 1),
        Err(BitError::Overflow {
            len: 1,
            width: usize::MAX - 1,
            limit: 10
        })
    );
    assert_eq!(writer, before);
    // A limit that is itself the largest there is cannot be passed by the same sum either.
    let mut huge = BitWriter::new(usize::MAX);
    huge.push(0b1, 1).expect("it fits");
    assert_eq!(
        huge.push_zeros(usize::MAX),
        Err(BitError::Overflow {
            len: 1,
            width: usize::MAX,
            limit: usize::MAX
        })
    );
}

#[test]
fn padding_to_a_byte_says_how_many_zeros_it_wrote() {
    let mut writer = BitWriter::new(32);
    writer.push(0b1, 1).expect("it fits");
    assert_eq!(writer.pad_to_byte(), Ok(7));
    assert_eq!(writer.pad_to_byte(), Ok(0));
    assert_eq!(writer.to_bytes(), Ok(vec![0b1000_0000]));
}

#[test]
fn bits_that_are_not_whole_bytes_are_refused() {
    let mut writer = BitWriter::new(32);
    writer.push(0b101, 3).expect("it fits");
    assert_eq!(writer.to_bytes(), Err(BitError::NotWholeBytes { len: 3 }));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The field
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn multiplying_by_tables_is_multiplying_by_shift_and_reduce_for_every_pair() {
    for a in 0..=255_u8 {
        for b in 0..=255_u8 {
            assert_eq!((Gf256::new(a) * Gf256::new(b)).value(), slow_mul(a, b), "{a} * {b}");
        }
    }
}

#[test]
fn the_powers_of_alpha_are_every_non_zero_element_once() {
    let mut seen = [false; 256];
    for exponent in 0..255 {
        let power = Gf256::alpha_pow(exponent).value();
        assert_eq!(power, slow_alpha(exponent), "alpha^{exponent}");
        assert_ne!(power, 0);
        assert!(!seen[usize::from(power)], "alpha^{exponent} repeats");
        seen[usize::from(power)] = true;
    }
    assert_eq!(Gf256::alpha_pow(255), Gf256::ONE);
    assert_eq!(Gf256::alpha_pow(256), Gf256::alpha_pow(1));
}

#[test]
fn adding_is_exclusive_or_and_an_element_is_its_own_negative() {
    assert_eq!((Gf256::new(0b1010) + Gf256::new(0b0110)).value(), 0b1100);
    for a in 0..=255_u8 {
        assert_eq!(Gf256::new(a) + Gf256::new(a), Gf256::ZERO);
        assert_eq!(Gf256::new(a) * Gf256::ONE, Gf256::new(a));
        assert_eq!(Gf256::new(a) * Gf256::ZERO, Gf256::ZERO);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Reed-Solomon
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The exponents of alpha that are the coefficients of the generator of degree 7, highest power first, as the
/// standard's own table of generator polynomials gives them.
const GENERATOR_7_EXPONENTS: [usize; 8] = [0, 87, 229, 146, 149, 238, 102, 21];

#[test]
fn the_generator_of_degree_7_is_the_one_in_the_standard() {
    let generator = generator_polynomial(7);
    let expected: Vec<Gf256> = GENERATOR_7_EXPONENTS.iter().map(|&e| Gf256::alpha_pow(e)).collect();
    assert_eq!(generator, expected);
}

#[test]
fn the_generator_has_degree_plus_one_coefficients_starts_with_one_and_is_zero_at_its_roots() {
    for degree in [1, 7, 10, 30] {
        let generator = generator_polynomial(degree);
        assert_eq!(generator.len(), degree + 1);
        assert_eq!(generator[0], Gf256::ONE);
        let bytes: Vec<u8> = generator.iter().map(|c| c.value()).collect();
        assert_eq!(bytes, slow_generator(degree));
        for root in 0..degree {
            assert_eq!(evaluate(&bytes, slow_alpha(root)), 0, "degree {degree}, root alpha^{root}");
        }
        assert_ne!(evaluate(&bytes, slow_alpha(degree)), 0, "alpha^{degree} is not a root");
    }
}

/// The worked example in the tutorial everyone learns this from: "HELLO WORLD" at 1-M, which has 10 codewords.
#[test]
fn the_remainder_matches_a_worked_example_of_another_degree() {
    let data = [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17];
    let expected = [196, 35, 39, 119, 235, 215, 231, 226, 93, 23];
    assert_eq!(remainder(&divide(&data, 10), 10), expected);
}

#[test]
fn the_remainder_is_the_one_whole_polynomial_division_gives() {
    let inputs: [&[u8]; 5] = [
        &[],
        &[1],
        &[0xFF; 19],
        &[0; 19],
        &[7, 0, 200, 3, 3, 3, 90, 91, 92, 93, 94, 95, 1, 2, 3, 4, 5, 6, 7],
    ];
    for data in inputs {
        for degree in [1, 7, 10] {
            assert_eq!(
                remainder(&divide(data, degree), degree),
                slow_remainder(data, degree),
                "{data:?}, {degree}"
            );
        }
    }
}

#[test]
fn there_is_one_step_for_each_data_byte_and_the_last_is_the_remainder() {
    let data = [1_u8, 2, 3, 4, 5];
    let steps = divide(&data, 7);
    assert_eq!(steps.len(), 5);
    for (step, &byte) in steps.iter().zip(&data) {
        assert_eq!(step.data_byte, byte);
        assert_eq!(step.register.len(), 7);
    }
    assert_eq!(steps[0].factor, 1);
    assert_eq!(remainder(&steps, 7), steps[4].register);
}

#[test]
fn no_data_has_a_remainder_of_zeros() {
    assert!(divide(&[], 7).is_empty());
    assert_eq!(remainder(&divide(&[], 7), 7), vec![0; 7]);
    assert_eq!(remainder(&divide(&[0; 19], 7), 7), vec![0; 7]);
}

#[test]
fn a_step_is_the_division_of_the_bytes_so_far() {
    let data = [9_u8, 8, 7, 6, 5, 4, 3, 2, 1, 0, 11, 12];
    let steps = divide(&data, 7);
    for (i, step) in steps.iter().enumerate() {
        // The register after i + 1 bytes is the remainder of those bytes alone, as if they were the whole message.
        assert_eq!(step.register, slow_remainder(&data[..=i], 7), "after byte {i}");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The input
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn seventeen_bytes_are_accepted_and_eighteen_are_not() {
    assert_eq!(check_length(0), Ok(()));
    assert_eq!(check_length(17), Ok(()));
    assert_eq!(check_length(18), Err(EncodeError::TooLong { bytes: 18, limit: 17 }));
    assert!(encode(&[b'a'; 17]).is_ok());
    assert_eq!(encode(&[b'a'; 18]), Err(EncodeError::TooLong { bytes: 18, limit: MAX_BYTES }));
    assert_eq!(encode_text("abcdefghijklmnopq").map(|e| e.input.len()), Ok(17));
    assert_eq!(
        encode_text("abcdefghijklmnopqr"),
        Err(EncodeError::TooLong { bytes: 18, limit: 17 })
    );
}

#[test]
fn the_length_is_in_bytes_not_in_characters() {
    // Five characters of three bytes each are 15 bytes, which fit, and one more is 18, which does not. Both are bytes
    // of UTF-8, which the byte interface takes without looking at.
    let sun = "日".as_bytes();
    assert_eq!(sun.len(), 3);
    assert!(encode(&sun.repeat(5)).is_ok());
    assert_eq!(encode(&sun.repeat(6)), Err(EncodeError::TooLong { bytes: 18, limit: 17 }));
}

#[test]
fn text_with_a_character_above_ascii_is_refused_and_not_turned_into_bytes() {
    assert_eq!(text_to_bytes("abc£"), Err(EncodeError::NotAscii { index: 3, character: '£' }));
    assert_eq!(text_to_bytes("日"), Err(EncodeError::NotAscii { index: 0, character: '日' }));
    assert_eq!(
        text_to_bytes("\u{80}"),
        Err(EncodeError::NotAscii { index: 0, character: '\u{80}' })
    );
    assert_eq!(text_to_bytes("\u{7F}"), Ok(vec![0x7F]));
    assert_eq!(text_to_bytes("\0"), Ok(vec![0]));
}

#[test]
fn a_bad_character_is_reported_before_a_bad_length() {
    let long_and_bad = format!("{}é", "a".repeat(30));
    assert_eq!(
        text_to_bytes(&long_and_bad),
        Err(EncodeError::NotAscii { index: 30, character: 'é' })
    );
}

#[test]
fn the_errors_say_what_is_wrong_in_words() {
    let too_long = EncodeError::TooLong { bytes: 18, limit: 17 }.to_string();
    assert!(
        too_long.contains("18") && too_long.contains("17") && too_long.contains("byte mode"),
        "{too_long}"
    );
    let not_ascii = EncodeError::NotAscii { index: 3, character: '£' }.to_string();
    assert!(
        not_ascii.contains("character 4") && not_ascii.contains("not ASCII"),
        "{not_ascii}"
    );
    let internal = EncodeError::Internal {
        stage: "padding",
        detail: "oops".to_owned(),
    }
    .to_string();
    assert!(internal.contains("padding") && internal.contains("oops"), "{internal}");
}

#[test]
fn bytes_are_escaped_so_that_they_can_be_read() {
    assert_eq!(escaped(b"Hi there"), "Hi there");
    assert_eq!(escaped(b"a\\b"), "a\\\\b");
    assert_eq!(escaped(b"\t\n\r"), "\\t\\n\\r");
    assert_eq!(escaped(&[0, 0x1F, 0x7F, 0x80, 0xFF]), "\\x00\\x1F\\x7F\\x80\\xFF");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The data codewords
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_bitstream_is_the_mode_the_count_and_the_bytes() {
    let data = encode_data(b"A").expect("one byte fits");
    // 0100 for byte mode, 00000001 for one byte, and 01000001 for the letter A.
    let expected: Vec<u8> = "0100 00000001 01000001"
        .chars()
        .filter(|c| *c != ' ')
        .map(|c| u8::from(c == '1'))
        .collect();
    assert_eq!(data.bitstream, expected);
}

#[test]
fn the_codewords_of_one_letter_are_the_ones_the_standard_works_out() {
    let data = encode_data(b"A").expect("one byte fits");
    // The header and the letter are 20 bits, and the terminator makes 24, so the third byte is 0001 then 0000, and the
    // pad codewords follow at once. No extra zero byte comes between, as one of the other encoders puts there.
    assert_eq!(data.codewords[..4], [0x40, 0x14, 0x10, 0xEC]);
    assert!(
        data.codewords[3..]
            .chunks(2)
            .all(|pair| pair[0] == 0xEC && pair.get(1).is_none_or(|&b| b == 0x11))
    );
}

#[test]
fn the_terminator_is_four_zeros_for_every_length_and_never_needs_a_boundary() {
    // The header is 12 bits and a byte is 8, so after four more zeros the length is always a multiple of 8, and the
    // fullest symbol, of 17 bytes, ends on the 152nd bit exactly. The step that fills a byte is there for the
    // standard's other versions, and does nothing here.
    for len in 0..=MAX_BYTES {
        let data = encode_data(&vec![0xAA; len]).expect("it fits");
        assert_eq!(data.bitstream.len(), 12 + 8 * len);
        assert_eq!(data.terminator_bits, 4, "{len} bytes");
        assert_eq!(data.boundary_bits, 0, "{len} bytes");
        assert_eq!(data.pad_codewords, DATA_CODEWORDS - (16 + 8 * len) / 8, "{len} bytes");
    }
}

#[test]
fn the_fullest_symbol_has_no_padding() {
    let data = encode_data(&[0x55; 17]).expect("it fits");
    assert_eq!((data.terminator_bits, data.pad_codewords), (4, 0));
    // The 148 bits of data and four zeros end the last codeword, which holds the last four bits of the last byte.
    assert_eq!(data.codewords[18], 0x50);
}

#[test]
fn padding_alternates_starting_with_0xec() {
    assert_eq!(PAD_CODEWORDS, [0xEC, 0x11]);
    let data = encode_data(&[]).expect("nothing fits");
    // The header is 12 bits and the terminator 4, so the two bytes are 0x40 and 0x00, and the padding starts at once.
    assert_eq!(data.codewords[..2], [0x40, 0x00]);
    for (i, &word) in data.codewords[2..].iter().enumerate() {
        assert_eq!(word, if i % 2 == 0 { 0xEC } else { 0x11 }, "pad {i}");
    }
}

#[test]
fn the_count_and_the_bytes_are_read_back_from_the_codewords() {
    for len in 0..=MAX_BYTES {
        let input: Vec<u8> = (0..len).map(|i| (i * 37 + 5) as u8).collect();
        let words = encode_data(&input).expect("it fits").codewords;
        let bits: String = words.iter().map(|w| format!("{w:08b}")).collect();
        assert_eq!(&bits[..4], "0100");
        assert_eq!(usize::from_str_radix(&bits[4..12], 2), Ok(len));
        let read: Vec<u8> = (0..len)
            .map(|i| u8::from_str_radix(&bits[12 + 8 * i..20 + 8 * i], 2).expect("bits"))
            .collect();
        assert_eq!(read, input);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The whole encoding
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_golden_vector_matches_what_segno_made() {
    assert!(GOLDEN.len() >= 10);
    for golden in GOLDEN {
        let encoded = encode(golden.input).expect("every golden input fits");
        assert_eq!(encoded.data.codewords, golden.data, "data of {:?}", golden.input);
        assert_eq!(
            encoded.error_correction, golden.error_correction,
            "error correction of {:?}",
            golden.input
        );
    }
}

#[test]
fn the_golden_vectors_cover_the_empty_input_the_limit_and_bytes_above_ascii() {
    let lengths: Vec<usize> = GOLDEN.iter().map(|g| g.input.len()).collect();
    assert!(lengths.contains(&0));
    assert!(lengths.contains(&17));
    assert!(lengths.contains(&16));
    assert!(GOLDEN.iter().any(|g| g.input.iter().any(|&b| b >= 0x80)));
    assert!(GOLDEN.iter().any(|g| g.input.iter().any(|&b| b < 0x20)));
}

#[test]
fn the_26_codewords_are_the_data_then_the_error_correction() {
    let encoded = encode(b"Hello").expect("it fits");
    let all = encoded.codewords();
    assert_eq!(all.len(), TOTAL_CODEWORDS);
    assert_eq!(all[..19], encoded.data.codewords);
    assert_eq!(all[19..], encoded.error_correction);
    assert_eq!(ERROR_CORRECTION_CODEWORDS, 7);
}

#[test]
fn every_length_of_every_kind_of_byte_makes_a_valid_reed_solomon_codeword() {
    let kinds: [fn(usize, usize) -> u8; 4] = [
        |_, _| 0x00,
        |_, _| 0xFF,
        |len, i| (len * 31 + i * 7) as u8,
        |len, i| ((len + i) * 0x9E) as u8,
    ];
    for len in 0..=MAX_BYTES {
        for kind in kinds {
            let input: Vec<u8> = (0..len).map(|i| kind(len, i)).collect();
            let encoded = encode(&input).expect("it fits");
            assert!(has_zero_syndrome(&encoded.codewords(), 7), "{input:?}");
            assert_eq!(encoded.division.len(), DATA_CODEWORDS);
        }
    }
}

#[test]
fn every_single_byte_is_encoded_and_makes_a_valid_codeword() {
    for byte in 0..=255_u8 {
        let encoded = encode(&[byte]).expect("one byte fits");
        assert!(has_zero_syndrome(&encoded.codewords(), 7), "{byte}");
        assert_eq!(encoded.data.codewords[0], 0x40);
        assert_eq!(encoded.data.codewords[1], 0x10 | byte >> 4);
        assert_eq!(encoded.data.codewords[2], byte << 4);
    }
}

#[test]
fn a_changed_data_byte_changes_the_error_correction() {
    let a = encode(b"abcdefg").expect("it fits");
    let b = encode(b"abcdefh").expect("it fits");
    assert_ne!(a.error_correction, b.error_correction);
}

#[test]
fn text_and_bytes_encode_alike() {
    assert_eq!(encode_text("Hello, World!"), encode(b"Hello, World!"));
}

#[test]
fn a_division_step_for_each_data_codeword_ends_on_the_error_correction() {
    let encoded = encode(b"Hello").expect("it fits");
    assert_eq!(encoded.division.len(), 19);
    assert_eq!(encoded.division[18].register, encoded.error_correction);
    for (step, &word) in encoded.division.iter().zip(&encoded.data.codewords) {
        assert_eq!(step.data_byte, word);
    }
}
