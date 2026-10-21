use super::{CODEWORD_LEN, Transmission, covered};

/// Every value the four data bits can have.
fn every_data() -> impl Iterator<Item = [u8; 4]> {
    (0..16u8).map(|n| [n >> 3 & 1, n >> 2 & 1, n >> 1 & 1, n & 1])
}

#[test]
fn each_parity_bit_covers_the_positions_with_its_bit_set() {
    assert_eq!(covered(0), vec![1, 3, 5, 7]);
    assert_eq!(covered(1), vec![2, 3, 6, 7]);
    assert_eq!(covered(2), vec![4, 5, 6, 7]);
}

#[test]
fn the_textbook_example_1011_is_the_codeword_0110011() {
    // d1 d2 d3 d4 = 1 0 1 1 gives p1 = 1^0^1 = 0, p2 = 1^1^1 = 1, p4 = 0^1^1 = 0.
    let sent = Transmission::new([1, 0, 1, 1], None);
    assert_eq!(sent.parity, [0, 1, 0]);
    assert_eq!(sent.codeword, [0, 1, 1, 0, 0, 1, 1]);
}

#[test]
fn every_codeword_has_an_even_number_of_ones_in_each_group() {
    for data in every_data() {
        let sent = Transmission::new(data, None);
        for check in 0..3 {
            assert_eq!(super::xor_over(&sent.codeword, &covered(check)), 0, "{data:?}");
        }
    }
}

#[test]
fn with_no_error_the_syndrome_is_zero_and_nothing_changes() {
    for data in every_data() {
        let t = Transmission::new(data, None);
        assert_eq!((t.syndrome, t.checks), (0, [0, 0, 0]));
        assert_eq!(t.received, t.codeword);
        assert_eq!(t.corrected, t.codeword);
        assert_eq!(t.recovered, data);
    }
}

#[test]
fn the_syndrome_of_a_flipped_bit_is_its_position_and_the_data_is_recovered() {
    for data in every_data() {
        for position in 1..=CODEWORD_LEN {
            let t = Transmission::new(data, Some(position));
            assert_eq!(usize::from(t.syndrome), position, "{data:?} flipped at {position}");
            assert_ne!(t.received, t.codeword);
            assert_eq!(t.corrected, t.codeword);
            assert_eq!(t.recovered, data);
        }
    }
}

#[test]
fn a_position_that_does_not_exist_is_ignored() {
    for position in [0, 8, 100] {
        assert_eq!(Transmission::new([1, 0, 1, 1], Some(position)).error, None);
    }
}
