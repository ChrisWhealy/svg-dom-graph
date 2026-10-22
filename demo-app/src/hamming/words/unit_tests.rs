use super::notes;
use crate::hamming::code::Transmission;

#[test]
fn the_notes_use_the_real_bits() {
    let notes = notes(&Transmission::new([1, 0, 1, 1], Some(5)));
    assert!(notes[0].contains("`1 0 1 1`"));
    assert!(notes[1].contains("p1 covers 1, 3, 5, 7: d1 XOR d2 XOR d4 = 1 XOR 0 XOR 1 = `0`"));
    assert!(notes[2].contains("`0 1 1 0 0 1 1`"));
    assert!(notes[3].contains("position 5 was flipped: it was `0` and arrived as `1`"));
    assert!(notes[5].contains("**5**"));
    assert!(notes[7].contains("That is the codeword that was sent."));
    assert!(notes[8].contains("They are the data that was sent."));
}

#[test]
fn with_no_error_the_notes_say_there_is_nothing_to_correct() {
    let notes = notes(&Transmission::new([0, 1, 1, 0], None));
    assert!(notes[3].contains("No bit was flipped"));
    assert!(notes[5].contains("**0**"));
    assert!(notes[6].contains("no bit is in error"));
    assert!(notes[7].contains("nothing to correct"));
}
