use super::*;

fn bytes(build: impl FnOnce(&mut Encoder)) -> String {
    let mut encoder = Encoder::new();
    build(&mut encoder);
    encoder.as_slice().iter().map(|byte| format!("{byte:02x}")).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The expected bytes were produced by a separate encoder written in Python.
#[test]
fn each_type_has_its_tag_and_fixed_layout() -> Result<(), String> {
    let cases = [
        ("u64 max", bytes(|e| e.put_u64(u64::MAX)), "01ffffffffffffffff"),
        ("i64 min", bytes(|e| e.put_i64(i64::MIN)), "020000000000000080"),
        (
            "string with a multi-byte character",
            bytes(|e| e.put_str("é")),
            "0302000000c3a9",
        ),
        ("bool true", bytes(|e| e.put_bool(true)), "0501"),
    ];
    for (label, got, want) in cases {
        if got != want {
            return Err(format!("{label}: expected {want}, got {got}"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_float_is_its_bit_pattern_and_negative_zero_is_positive_zero() -> Result<(), String> {
    let one_and_a_half = bytes(|e| e.put_f64(1.5).unwrap());
    if one_and_a_half != "04000000000000f83f" {
        return Err(format!("1.5 encoded as {one_and_a_half}"));
    }
    let positive = bytes(|e| e.put_f64(0.0).unwrap());
    let negative = bytes(|e| e.put_f64(-0.0).unwrap());
    if positive != "040000000000000000" || negative != positive {
        return Err(format!("+0.0 gave {positive} and -0.0 gave {negative}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_float_that_is_not_finite_is_refused() -> Result<(), String> {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut encoder = Encoder::new();
        if encoder.put_f64(value) != Err(EncodeError::NotFinite) {
            return Err(format!("{value} was accepted"));
        }
        if !encoder.as_slice().is_empty() {
            return Err(format!("{value} left bytes behind"));
        }
    }
    Ok(())
}
