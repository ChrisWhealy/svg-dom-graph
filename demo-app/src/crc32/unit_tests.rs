use super::algorithm::{FINAL_MASK, INITIAL, TABLE, bitwise, compute};

#[test]
fn the_check_value_of_crc_32_is_cbf43926() {
    assert_eq!(compute(b"123456789").crc, 0xCBF4_3926);
}

#[test]
fn known_checksums_are_as_published() {
    assert_eq!(compute(b"").crc, 0);
    assert_eq!(compute(b"a").crc, 0xE8B7_BE43);
    assert_eq!(compute(b"abc").crc, 0x3524_41C2);
    assert_eq!(compute(b"The quick brown fox jumps over the lazy dog").crc, 0x414F_A339);
}

#[test]
fn the_table_has_the_well_known_first_and_last_entries() {
    assert_eq!(TABLE[0], 0);
    assert_eq!(TABLE[1], 0x7707_3096);
    assert_eq!(TABLE[2], 0xEE0E_612C);
    assert_eq!(TABLE[255], 0x2D02_EF8D);
}

#[test]
fn the_table_agrees_with_the_bitwise_calculation_for_every_length_up_to_a_few_bytes() {
    let bytes: Vec<u8> = (0..=255).collect();
    for len in 0..=bytes.len() {
        assert_eq!(compute(&bytes[..len]).crc, bitwise(&bytes[..len]), "{len} bytes");
    }
}

#[test]
fn each_step_shifts_the_register_a_byte_and_xors_the_entry() {
    let computed = compute(b"ab");
    let first = computed.steps[0];
    // The register starts as all ones, so the index is the byte flipped.
    assert_eq!(first.index, !b'a');
    assert_eq!(first.entry, TABLE[usize::from(first.index)]);
    assert_eq!(first.register, (INITIAL >> 8) ^ first.entry);
    let second = computed.steps[1];
    assert_eq!(second.index, ((first.register ^ u32::from(b'b')) & 0xFF) as u8);
    assert_eq!(computed.register, second.register);
    assert_eq!(computed.crc, computed.register ^ FINAL_MASK);
}

#[test]
fn no_bytes_leave_the_register_as_it_started() {
    let computed = compute(b"");
    assert!(computed.steps.is_empty());
    assert_eq!(computed.register, INITIAL);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Correcting a single flipped bit
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

use super::algorithm::{Finding, locate, syndrome_of_bit};

#[test]
fn every_single_bit_of_data_up_to_the_limit_has_a_syndrome_of_its_own() {
    for len in 1..=64 {
        let mut seen = std::collections::HashSet::new();
        for byte in 0..len {
            for bit in 0..8 {
                let syndrome = syndrome_of_bit(len, byte, bit);
                assert_ne!(syndrome, 0, "{len} bytes, byte {byte}, bit {bit}");
                // A flipped bit of the CRC has a syndrome of a single one, which a flipped bit of the data must not have.
                assert!(syndrome.count_ones() > 1, "{len} bytes, byte {byte}, bit {bit}");
                assert!(seen.insert(syndrome), "{len} bytes, byte {byte}, bit {bit} repeats a syndrome");
            }
        }
    }
}

#[test]
fn the_syndrome_of_a_bit_does_not_depend_on_the_data() {
    let data: Vec<u8> = (0..40).map(|n| (n * 37 + 11) as u8).collect();
    for (byte, bit) in [(0, 0), (7, 3), (39, 7), (20, 5)] {
        let mut flipped = data.clone();
        flipped[byte] ^= 1 << bit;
        assert_eq!(
            compute(&data).crc ^ compute(&flipped).crc,
            syndrome_of_bit(data.len(), byte, bit)
        );
    }
}

#[test]
fn a_single_flipped_bit_is_found_and_flipping_it_back_restores_the_data() {
    let data = b"The quick brown fox jumps over the lazy dog".to_vec();
    let sent = compute(&data).crc;
    for byte in 0..data.len() {
        for bit in 0..8 {
            let mut received = data.clone();
            received[byte] ^= 1 << bit;
            let found = locate(received.len(), sent ^ compute(&received).crc);
            assert_eq!(found, Finding::Data { byte, bit });
            received[byte] ^= 1 << bit;
            assert_eq!(received, data);
            assert_eq!(compute(&received).crc, sent);
        }
    }
}

#[test]
fn no_error_a_flipped_crc_bit_and_two_flipped_bits_are_told_apart() {
    let data = b"123456789".to_vec();
    let sent = compute(&data).crc;
    assert_eq!(locate(data.len(), sent ^ compute(&data).crc), Finding::NoError);
    // The CRC arrived with bit 12 changed, and the data is as it was.
    assert_eq!(locate(data.len(), sent ^ (sent ^ (1 << 12))), Finding::Crc { bit: 12 });
    // Two bits of the data changed, which no single flipped bit explains.
    let mut received = data.clone();
    received[1] ^= 1;
    received[6] ^= 0x40;
    assert_eq!(locate(received.len(), sent ^ compute(&received).crc), Finding::Uncorrectable);
}

#[test]
fn two_flipped_bits_are_never_mistaken_for_one_in_data_up_to_the_limit() {
    for len in [1, 9, 31, 64] {
        let syndromes: Vec<u32> = (0..len)
            .flat_map(|byte| (0..8).map(move |bit| syndrome_of_bit(len, byte, bit)))
            .collect();
        let singles: std::collections::HashSet<u32> = syndromes.iter().copied().collect();
        for (i, a) in syndromes.iter().enumerate() {
            for b in &syndromes[i + 1..] {
                let both = a ^ b;
                assert_ne!(both, 0, "{len} bytes");
                assert!(
                    both.count_ones() > 1,
                    "{len} bytes: two bits look like a flipped bit of the CRC"
                );
                assert!(!singles.contains(&both), "{len} bytes: two bits look like one bit of the data");
            }
        }
    }
}
