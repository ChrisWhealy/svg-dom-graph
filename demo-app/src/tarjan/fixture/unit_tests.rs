use super::*;

/// A SHA-256 of `message`, which only these tests use, so that the pinned hash of the design can be checked. It is
/// checked against two known digests first.
fn sha256(message: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
        0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
        0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
        0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut padded = message.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&((message.len() as u64) * 8).to_be_bytes());
    for block in padded.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

#[test]
fn the_test_hash_is_right_on_two_known_inputs() {
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(sha256(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn the_graph_has_64_vertices_114_unique_edges_and_17_bridges() {
    let edges = edges().expect("no edge is made twice");
    assert_eq!(edges.len(), EDGES);
    assert_eq!(SIZES.iter().sum::<usize>(), VERTICES);
    assert_eq!(DAG_PAIRS.len(), 17);
    let mut unique = edges.clone();
    unique.dedup();
    assert_eq!(unique.len(), EDGES);
    assert!(edges.windows(2).all(|w| w[0] < w[1]), "sorted by source and then by target");
    assert!(edges.iter().all(|&(a, b)| a < VERTICES && b < VERTICES));
    let bridges = edges.iter().filter(|&&(a, b)| region_of(a) != region_of(b)).count();
    assert_eq!(bridges, 17);
}

#[test]
fn the_edge_list_matches_the_hash_of_the_design() {
    let text = edges()
        .unwrap()
        .iter()
        .map(|(a, b)| format!("{a:02},{b:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        sha256(text.as_bytes()),
        "18ee2b468263021b07a3ed2659a023d6a7c5d3915f1772fb73c5291b776d47eb"
    );
}

#[test]
fn the_ring_and_chord_edges_are_63_and_34() {
    let edges = edges().unwrap();
    let inside: Vec<_> = edges.iter().filter(|&&(a, b)| region_of(a) == region_of(b)).collect();
    assert_eq!(inside.len(), 97);
    let ring = inside
        .iter()
        .filter(|&&&(a, b)| (b + SIZES[region_of(a)] - a) % SIZES[region_of(a)] == 1)
        .count();
    assert_eq!(ring, 63);
}

#[test]
fn the_bridges_follow_the_inter_region_dag_so_no_region_can_be_reached_back() {
    for &(a, b) in &DAG_PAIRS {
        assert!(LAYERS[a] < LAYERS[b], "{a} to {b}");
    }
    // Every region's layer is the longest path to it.
    for (g, &layer) in LAYERS.iter().enumerate() {
        let longest = DAG_PAIRS
            .iter()
            .filter(|&&(_, b)| b == g)
            .map(|&(a, _)| LAYERS[a] + 1)
            .max()
            .unwrap_or(0);
        assert_eq!(layer, longest, "region {g}");
    }
}

#[test]
fn regions_and_vertices_know_each_other() {
    assert_eq!(starts(), [0, 11, 21, 30, 38, 45, 51, 56, 60, 63]);
    assert_eq!(
        (region_of(0), region_of(10), region_of(11), region_of(62), region_of(63)),
        (0, 0, 1, 8, 9)
    );
    assert_eq!(members_of(9), 63..64);
    assert_eq!(name(7), "v07");
    assert_eq!(name(63), "v63");
}

#[test]
fn the_three_antiparallel_pairs_are_the_ones_of_the_design() {
    let edges = edges().unwrap();
    let pairs: Vec<(usize, usize)> = edges
        .iter()
        .copied()
        .filter(|&(a, b)| a < b && edges.contains(&(b, a)))
        .collect();
    assert_eq!(pairs, vec![(56, 58), (60, 62), (61, 62)]);
}

#[test]
fn vertex_62_has_the_six_incident_edges_of_the_design() {
    let edges = edges().unwrap();
    let at: Vec<_> = edges.iter().copied().filter(|&(a, b)| a == 62 || b == 62).collect();
    assert_eq!(at, vec![(55, 62), (60, 62), (61, 62), (62, 60), (62, 61), (62, 63)]);
}

#[test]
fn the_graph_is_built_with_sorted_neighbour_lists() {
    let g = graph();
    assert_eq!(g.len(), VERTICES);
    assert_eq!(g.edges(), edges().unwrap());
    for v in 0..VERTICES {
        assert!(g.targets(v).windows(2).all(|w| w[0] < w[1]));
    }
}
