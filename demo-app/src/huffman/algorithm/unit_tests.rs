//! Huffman encoding against hand-worked cases: the tree and canonical codes for known text, bit packing across byte
//! boundaries, the edge cases, and a round trip through a decoder that uses the code lengths alone.

use super::*;

fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn trace_of(input: &[u8]) -> Result<Trace, String> {
    trace(input).ok_or_else(|| "the input is encodable".to_owned())
}

/// The code lengths as `(symbol, len)` in canonical order.
fn lengths_of(t: &Trace) -> Vec<(u8, u8)> {
    t.codebook.entries.iter().map(|e| (e.symbol, e.code.len)).collect()
}

/// Rebuilds canonical codes from `lengths` alone, then reads `bit_len` bits of `bytes` back into symbols.
fn decode(lengths: &[(u8, u8)], bytes: &[u8], bit_len: usize) -> Result<Vec<u8>, String> {
    let mut sorted = lengths.to_vec();
    sorted.sort_unstable_by_key(|&(symbol, len)| (len, symbol));

    let mut table = std::collections::HashMap::new();
    let (mut next, mut prev_len) = (0u32, sorted[0].1);
    for &(symbol, len) in &sorted {
        next <<= len - prev_len;
        prev_len = len;
        table.insert((len, next), symbol);
        next += 1;
    }

    let (mut out, mut bits, mut len) = (Vec::new(), 0u32, 0u8);
    for i in 0..bit_len {
        bits = bits << 1 | u32::from(bytes[i / 8] >> (7 - i % 8) & 1);
        len += 1;
        if let Some(&symbol) = table.get(&(len, bits)) {
            out.push(symbol);
            (bits, len) = (0, 0);
        }
    }
    check_eq(len, 0)?;
    Ok(out)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Pass one
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn frequencies_are_counted_in_byte_order() -> Result<(), String> {
    let got: Vec<(u8, u32)> = frequencies(b"abracadabra").iter().map(|f| (f.symbol, f.count)).collect();
    check_eq(got, vec![(b'a', 5), (b'b', 2), (b'c', 1), (b'd', 1), (b'r', 2)])
}

#[test]
fn abracadabra_has_the_hand_worked_lengths() -> Result<(), String> {
    // Canonical order is by length, then byte value, so b, c, d, r.
    let t = trace_of(b"abracadabra")?;
    check_eq(lengths_of(&t), vec![(b'a', 1), (b'b', 3), (b'c', 3), (b'd', 3), (b'r', 3)])?;
    check_eq(t.tree.merges.len(), 4)?;
    check_eq(t.tree.nodes[t.tree.root()].weight(), 11)
}

#[test]
fn the_first_merge_takes_the_two_lightest_and_ties_go_to_the_older() -> Result<(), String> {
    let t = trace_of(b"abracadabra")?;
    // Leaves are a=0 b=1 c=2 d=3 r=4. The 1s (c, d) merge first, into node 5.
    check_eq(t.tree.merges[0], Merge { left: 2, right: 3, parent: 5 })?;
    // The next lightest are b and r (2 each), older than the new node 5 (also 2).
    check_eq(t.tree.merges[1], Merge { left: 1, right: 4, parent: 6 })
}

#[test]
fn canonical_codes_follow_length_then_byte_order() -> Result<(), String> {
    let t = trace_of(b"abracadabra")?;
    let codes: Vec<(u8, u32, u8)> = t.codebook.entries.iter().map(|e| (e.symbol, e.code.bits, e.code.len)).collect();
    check_eq(
        codes,
        vec![
            (b'a', 0b0, 1),
            (b'b', 0b100, 3),
            (b'c', 0b101, 3),
            (b'd', 0b110, 3),
            (b'r', 0b111, 3),
        ],
    )
}

#[test]
fn the_codes_are_prefix_free_and_fill_the_code_space() -> Result<(), String> {
    for input in [
        &b"abracadabra"[..],
        b"hello, huffman world",
        b"aaaabbbccd",
        b"the quick brown fox",
    ] {
        let t = trace_of(input)?;
        let kraft: f64 = t.codebook.entries.iter().map(|e| 0.5f64.powi(i32::from(e.code.len))).sum();
        check_eq(kraft, 1.0)?;

        for (i, a) in t.codebook.entries.iter().enumerate() {
            for b in t.codebook.entries.iter().skip(i + 1) {
                let (short, long) = if a.code.len <= b.code.len { (a.code, b.code) } else { (b.code, a.code) };
                check(
                    long.bits >> (long.len - short.len) != short.bits,
                    "one code is a prefix of another",
                )?;
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Pass two
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn abracadabra_packs_to_the_hand_worked_bytes() -> Result<(), String> {
    // a=0 b=100 r=111 c=101 d=110, so a b r a c a d a b r a is
    // 0 100 111 0 101 0 110 0 100 111 0 = 0100 1110 1010 1100 1001 110(0 padding)
    let t = trace_of(b"abracadabra")?;
    check_eq(t.packed.bytes.clone(), vec![0b0100_1110, 0b1010_1100, 0b1001_1100])?;
    check_eq(t.packed.bit_len, 23)?;
    check_eq(t.packed.padding(), 1)
}

#[test]
fn a_code_that_straddles_a_byte_boundary_is_split_across_two_bytes() -> Result<(), String> {
    // Every code in "abracadabra" lands on a byte boundary, so this text is used instead.
    let t = trace_of(b"hello, huffman world")?;
    let straddlers: Vec<&PackStep> = t
        .packed
        .steps
        .iter()
        .filter(|s| s.nbits_before > 0 && !s.emitted.is_empty() && s.nbits_after > 0)
        .collect();
    check(!straddlers.is_empty(), "some code should straddle a byte boundary")?;

    // The first emitted byte is the pending bits followed by the top bits of the code.
    for s in straddlers {
        let room = 8 - s.nbits_before;
        let expected = (s.acc_before << room) as u8 | (u64::from(s.code.bits) >> (s.code.len - room)) as u8;
        check_eq(s.emitted[0], expected)?;
    }
    Ok(())
}

#[test]
fn codes_much_longer_than_a_byte_complete_several_bytes_in_one_step() -> Result<(), String> {
    // No input within the length limit makes codes this long, so the codebook is built by hand.
    let long = Code { bits: 0xdead_beef, len: 32 };
    let mid = Code { bits: 0b1_0101_0101, len: 9 };
    let one = Code { bits: 1, len: 1 };
    let codebook = Codebook::from_entries(vec![
        Entry { symbol: b'a', code: long },
        Entry { symbol: b'b', code: mid },
        Entry { symbol: b'c', code: one },
    ]);

    let input = b"cabacab";
    let packed = pack(input, &codebook).ok_or("every byte has a code")?;

    // Reference: write the same bits out as text, then cut that into bytes.
    let text: String = input
        .iter()
        .filter_map(|&b| codebook.code_for(b))
        .map(|c| format!("{:0width$b}", c.bits, width = usize::from(c.len)))
        .collect();
    let expected: Vec<u8> = text
        .as_bytes()
        .chunks(8)
        .map(|chunk| {
            let padded = format!("{:0<8}", std::str::from_utf8(chunk).unwrap_or_default());
            u8::from_str_radix(&padded, 2).unwrap_or_default()
        })
        .collect();
    check_eq(packed.bytes.clone(), expected)?;
    check_eq(packed.bit_len, text.len())?;

    // 'c' leaves 1 bit pending, so the 32-bit 'a' that follows completes 33 bits: four whole bytes and 1 bit left.
    check_eq(packed.steps[1].emitted.len(), 4)?;
    check_eq(packed.steps[1].nbits_after, 1)
}

#[test]
fn every_step_carries_the_accumulator_forward() -> Result<(), String> {
    let t = trace_of(b"the quick brown fox jumps over the lazy dog")?;
    for pair in t.packed.steps.windows(2) {
        check_eq(
            (pair[0].acc_after, pair[0].nbits_after),
            (pair[1].acc_before, pair[1].nbits_before),
        )?;
        check(pair[0].nbits_after < 8, "fewer than 8 bits stay pending")?;
    }
    let emitted: Vec<u8> = t.packed.steps.iter().flat_map(|s| s.emitted.clone()).collect();
    check_eq(emitted.len(), t.packed.bytes.len() - usize::from(t.packed.padding() > 0))
}

#[test]
fn the_final_flush_can_be_derived_from_the_last_step() -> Result<(), String> {
    // The padded byte has no step of its own. The walk rebuilds it from the last step and the padding count.
    for input in [
        &b"abracadabra"[..],
        b"hello, huffman world",
        b"aaaaaaaaa",
        b"the quick brown fox",
    ] {
        let t = trace_of(input)?;
        let last = t.packed.steps.last().ok_or("at least one step")?;
        let padding = t.packed.padding();

        check_eq(padding, if last.nbits_after > 0 { 8 - last.nbits_after } else { 0 })?;
        if last.nbits_after > 0 {
            check_eq(t.packed.bytes.last().copied(), Some((last.acc_after << padding) as u8))?;
        } else {
            let emitted: usize = t.packed.steps.iter().map(|s| s.emitted.len()).sum();
            check_eq(emitted, t.packed.bytes.len())?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Edge cases
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn one_distinct_byte_gets_a_one_bit_code() -> Result<(), String> {
    let t = trace_of(b"aaaaaaaaa")?;
    check_eq(t.tree.merges.len(), 0)?;
    check_eq(lengths_of(&t), vec![(b'a', 1)])?;
    // 9 one-bit zero codes: one full byte and one padded byte.
    check_eq(t.packed.bytes.clone(), vec![0, 0])?;
    check_eq(t.packed.bit_len, 9)
}

#[test]
fn empty_and_oversized_input_are_refused() -> Result<(), String> {
    check(trace(b"").is_none(), "empty input has nothing to encode")?;
    check(trace(&[b'a'; MAX_INPUT_LEN + 1]).is_none(), "oversized input is refused")?;
    check(trace(&[b'a'; MAX_INPUT_LEN]).is_some(), "input at the limit is accepted")
}

#[test]
fn the_worst_case_input_stays_inside_the_code_length_limit() -> Result<(), String> {
    // Fibonacci counts make the deepest possible tree. 1,1,2,3,5,8,13,21 totals 54 bytes.
    let counts = [1usize, 1, 2, 3, 5, 8, 13, 21];
    let input: Vec<u8> = counts.iter().enumerate().flat_map(|(i, &n)| vec![b'a' + i as u8; n]).collect();
    let t = trace_of(&input)?;
    check_eq(t.codebook.entries.iter().map(|e| e.code.len).max(), Some(7))
}

#[test]
fn non_ascii_text_is_encoded_as_its_utf8_bytes() -> Result<(), String> {
    let text = "héllo wörld ✓";
    let t = trace_of(text.as_bytes())?;
    let back = decode(&lengths_of(&t), &t.packed.bytes, t.packed.bit_len)?;
    check_eq(String::from_utf8(back).map_err(|e| e.to_string())?, text.to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Round trip
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn decoding_from_the_lengths_alone_recovers_the_input() -> Result<(), String> {
    // A small deterministic generator, so that the test needs no random number crate.
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..200 {
        let len = 1 + (next() % MAX_INPUT_LEN as u64) as usize;
        let alphabet = 1 + next() % 40;
        let input: Vec<u8> = (0..len).map(|_| b' ' + (next() % alphabet) as u8).collect();

        let t = trace_of(&input)?;
        let back = decode(&lengths_of(&t), &t.packed.bytes, t.packed.bit_len)?;
        check_eq(back, input)?;
    }
    Ok(())
}
