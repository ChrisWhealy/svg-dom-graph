use super::{from_theta_grid, keccak_f, round_traces, theta, to_theta_grid};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Absorbs one SHA3-256 block of `message` (shorter than the 136-byte rate) into a zero state, permutes it, and
/// reads the first 32 bytes back — the whole of SHA3-256 for a short message.
fn sha3_256(message: &[u8]) -> String {
    let mut block = [0u8; 136];
    block[..message.len()].copy_from_slice(message);
    block[message.len()] ^= 0x06;
    block[135] ^= 0x80;
    let mut state = [0u64; 25];
    for (lane, bytes) in block.chunks(8).enumerate() {
        state[lane] ^= u64::from_le_bytes(bytes.try_into().unwrap());
    }
    keccak_f(state)
        .iter()
        .flat_map(|lane| lane.to_le_bytes())
        .take(32)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn keccak_f_gives_the_published_sha3_256_digests() {
    assert_eq!(
        sha3_256(b""),
        "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
    );
    assert_eq!(
        sha3_256(b"abc"),
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn keccak_f_of_the_zero_state_starts_with_the_published_lane() {
    assert_eq!(keccak_f([0; 25])[0], 0xF125_8F79_40E1_DDE7);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_traces_chain_and_end_where_keccak_f_ends() {
    let seed: [u64; 25] = std::array::from_fn(|i| 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul(i as u64 + 1));
    let traces = round_traces(seed);
    assert_eq!(traces[0].input, seed);
    for pair in traces.windows(2) {
        assert_eq!(
            pair[1].input, pair[0].output,
            "a round starts from the previous round's own output"
        );
    }
    assert_eq!(traces[23].output, keccak_f(seed));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn theta_agrees_with_the_theta_scenes_own_calculation() {
    let state: [u64; 25] = std::array::from_fn(|i| 0x0123_4567_89AB_CDEF_u64.rotate_left(i as u32) ^ i as u64);
    assert_eq!(from_theta_grid(to_theta_grid(state)), state);
    let from_scene = from_theta_grid(super::super::theta::output(to_theta_grid(state)));
    assert_eq!(theta(state), from_scene);
}
