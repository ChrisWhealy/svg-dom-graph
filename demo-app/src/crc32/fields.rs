//! Checking what the user typed in the two fields that choose the bit to flip. Nothing here touches the DOM or the
//! library.

/// What is wrong with a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Problem {
    /// It is empty, or has anything in it but the digits 0 to 9: a letter, a sign, a point, a space inside it.
    NotANumber,
    /// It is a whole number, but past the end of what the field can hold.
    OutOfRange,
}

/// A field that is wrong, and what to tell the user about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Complaint {
    pub(super) problem: Problem,
    pub(super) message: String,
}

/// Reads `text` as a whole number from 0 to `max` for the field called `name`. Spaces at either end are ignored.
///
/// The message says which field, what it held, and what it may hold, so it is enough by itself for someone who cannot
/// see which field is coloured.
pub(super) fn whole_number(name: &str, text: &str, max: usize) -> Result<usize, Complaint> {
    let typed = text.trim();
    if typed.is_empty() {
        return Err(Complaint {
            problem: Problem::NotANumber,
            message: format!("{name} is empty. Enter a whole number from 0 to {max}."),
        });
    }
    if !typed.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Complaint {
            problem: Problem::NotANumber,
            message: format!("{name}: {typed:?} is not a whole number. Enter a whole number from 0 to {max}."),
        });
    }
    // Only digits, so the only way to fail is to be too big for a `usize`.
    match typed.parse::<usize>() {
        Ok(value) if value <= max => Ok(value),
        _ => Err(Complaint {
            problem: Problem::OutOfRange,
            message: format!("{name}: {typed} is more than {max}. Enter a whole number from 0 to {max}."),
        }),
    }
}

/// The bit and the byte that the two fields choose, or what is wrong with each field that is wrong. `length` is the
/// number of bytes the text has, which must be at least 1.
pub(super) fn bit_and_byte(
    bit_text: &str,
    byte_text: &str,
    length: usize,
) -> Result<(usize, u8), [Option<Complaint>; 2]> {
    let bit = whole_number("Bit", bit_text, 7);
    let byte = whole_number("Byte", byte_text, length.saturating_sub(1));
    match (bit, byte) {
        (Ok(bit), Ok(byte)) => Ok((byte, bit as u8)),
        (bit, byte) => Err([bit.err(), byte.err()]),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::{Problem, bit_and_byte, whole_number};

    #[test]
    fn whole_numbers_in_range_are_accepted_with_spaces_at_either_end_ignored() {
        assert_eq!(whole_number("Bit", "0", 7).ok(), Some(0));
        assert_eq!(whole_number("Bit", " 7 ", 7).ok(), Some(7));
        assert_eq!(whole_number("Byte", "007", 48).ok(), Some(7));
    }

    #[test]
    fn anything_but_digits_is_not_a_number() {
        for text in ["", "  ", "abc", "a1", "1a", "-1", "+1", "1.5", "1 2", "\u{0663}"] {
            let complaint = whole_number("Bit", text, 7).err();
            assert_eq!(complaint.map(|c| c.problem), Some(Problem::NotANumber), "{text:?}");
        }
    }

    #[test]
    fn a_whole_number_past_the_end_is_out_of_range_however_long_it_is() {
        for text in ["8", "9", "100", "99999999999999999999999999"] {
            let complaint = whole_number("Bit", text, 7).err();
            assert_eq!(complaint.map(|c| c.problem), Some(Problem::OutOfRange), "{text:?}");
        }
    }

    #[test]
    fn a_message_names_the_field_what_it_held_and_what_it_may_hold() {
        let message = whole_number("Bit", "abc", 7).err().map(|c| c.message).unwrap_or_default();
        assert_eq!(message, "Bit: \"abc\" is not a whole number. Enter a whole number from 0 to 7.");
        let message = whole_number("Byte", "70", 48).err().map(|c| c.message).unwrap_or_default();
        assert_eq!(message, "Byte: 70 is more than 48. Enter a whole number from 0 to 48.");
    }

    #[test]
    fn both_fields_are_checked_and_each_wrong_one_is_reported() {
        assert_eq!(bit_and_byte("5", "2", 10).ok(), Some((2, 5)));
        let [bit, byte] = bit_and_byte("x", "10", 10).err().unwrap_or_default();
        assert_eq!(bit.map(|c| c.problem), Some(Problem::NotANumber));
        assert_eq!(byte.map(|c| c.problem), Some(Problem::OutOfRange));
        // A field that is right is not reported while the other is wrong.
        let [bit, byte] = bit_and_byte("3", "x", 10).err().unwrap_or_default();
        assert!(bit.is_none() && byte.is_some());
    }
}
