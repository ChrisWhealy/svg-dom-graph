//! The words of the diagram: what each box is, and how its values are worked out. Every bit in them is a real one.

use super::code::{CODEWORD_LEN, DATA_POSITIONS, PARITY_POSITIONS, Transmission, covered, xor_over};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The bits written with a space between each, such as `1 0 1 1`.
fn spaced(bits: &[u8]) -> String {
    bits.iter().map(u8::to_string).collect::<Vec<_>>().join(" ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The positions written as a list, such as `1, 3, 5, 7`.
fn listed(positions: &[usize]) -> String {
    positions.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The bits at `positions` of `word`, joined with `XOR` between, then ` = ` and what they come to. The names say which
/// bits they are: `names` gives the name of the bit at position 1, 2 and so on.
fn sum(word: &[u8; CODEWORD_LEN], positions: &[usize], names: &[&str; CODEWORD_LEN]) -> String {
    let named = positions.iter().map(|p| names[p - 1]).collect::<Vec<_>>().join(" XOR ");
    let values = positions
        .iter()
        .map(|p| word[p - 1].to_string())
        .collect::<Vec<_>>()
        .join(" XOR ");
    format!("{named} = {values} = `{}`", xor_over(word, positions))
}

/// The name of each position of a received codeword.
const RECEIVED_NAMES: [&str; CODEWORD_LEN] = ["b1", "b2", "b3", "b4", "b5", "b6", "b7"];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The note under each box of the path, in the order of the path.
pub(super) fn notes(t: &Transmission) -> [String; 9] {
    let parity_lines: Vec<String> = (0..3)
        .map(|check| {
            let positions = covered(check);
            // Which data bits are in the group, named d1..d4 by their place in the data.
            let data_positions: Vec<usize> = positions.iter().copied().filter(|p| DATA_POSITIONS.contains(p)).collect();
            let named = data_positions
                .iter()
                .map(|p| format!("d{}", DATA_POSITIONS.iter().position(|d| d == p).map_or(0, |i| i + 1)))
                .collect::<Vec<_>>()
                .join(" XOR ");
            let values = data_positions
                .iter()
                .map(|p| t.codeword[p - 1].to_string())
                .collect::<Vec<_>>()
                .join(" XOR ");
            format!(
                "- p{} covers {}: {named} = {values} = `{}`",
                PARITY_POSITIONS[check],
                listed(&positions),
                t.parity[check]
            )
        })
        .collect();

    let check_lines: Vec<String> = (0..3)
        .map(|check| {
            format!(
                "- c{} over {}: {}",
                PARITY_POSITIONS[check],
                listed(&covered(check)),
                sum(&t.received, &covered(check), &RECEIVED_NAMES)
            )
        })
        .collect();

    let arrival = match t.error {
        None => "No bit was flipped, so the codeword arrived as it was sent.".to_owned(),
        Some(position) => format!(
            "The bit at position {position} was flipped: it was `{}` and arrived as `{}`. The receiver is not told which bit it was.",
            t.codeword[position - 1],
            t.received[position - 1]
        ),
    };
    let syndrome = if t.syndrome == 0 {
        format!(
            "c4 c2 c1 = `{}` = **0**. Every group still holds an even number of 1s, so nothing needs correcting.",
            spaced(&[t.checks[2], t.checks[1], t.checks[0]])
        )
    } else {
        format!(
            "c4 c2 c1 = `{}` = **{}**. If one bit flipped, this number is its position, because each position is covered \
             by exactly the checks that are the 1s in its number.",
            spaced(&[t.checks[2], t.checks[1], t.checks[0]]),
            t.syndrome
        )
    };
    let position = match t.syndrome {
        0 => "Zero means that no bit is in error.".to_owned(),
        n => format!(
            "Position **{n}**. The bit there is wrong. It may be a parity bit or a data bit, and the receiver does not \
             need to know which."
        ),
    };
    let corrected = match t.syndrome {
        0 => "There was nothing to correct, so this is the codeword that arrived.".to_owned(),
        n => format!(
            "Flipping the bit at position {n} back gives `{}`. {}",
            spaced(&t.corrected),
            if t.corrected == t.codeword {
                "That is the codeword that was sent."
            } else {
                ""
            }
        ),
    };
    [
        format!(
            "### Data bits\nThe four bits to protect, d1 to d4: `{}`. Toggle one with the buttons above. Hamming(7,4) turns \
             them into a codeword of seven bits by adding three parity bits, so that **one** flipped bit can be found and \
             put right.",
            spaced(&t.data)
        ),
        format!(
            "### Parity bits\nEach parity bit is chosen so that a group of bits holds an **even** number of 1s, which is \
             the same as XORing the group to 0. A parity bit covers the positions whose number has the parity bit's own \
             number among its binary 1s.\n\n{}",
            parity_lines.join("\n")
        ),
        format!(
            "### Codeword\nThe seven bits are numbered 1 to 7 from the left. The parity bits sit at the positions that are \
             powers of two: 1, 2 and 4. The data bits fill the rest: 3, 5, 6 and 7. The codeword p1 p2 d1 p4 d2 d3 d4 is \
             `{}`. This is what the sender sends.",
            spaced(&t.codeword)
        ),
        format!("### Received codeword\nThe channel may flip one bit. {arrival}"),
        format!(
            "### Check bits\nThe receiver does not know whether a bit changed, so it checks each group again over the bits \
             that **arrived**, this time including the parity bit. A check is `0` if its group still holds an even number \
             of 1s.\n\n{}",
            check_lines.join("\n")
        ),
        format!("### Syndrome\nThe checks, read as a binary number: {syndrome}"),
        format!("### Error position\n{position}"),
        format!("### Corrected codeword\n{corrected}"),
        format!(
            "### Recovered data\nThe data bits are at positions 3, 5, 6 and 7: `{}`.{}\n\nHamming(7,4) corrects only one \
             flipped bit. If two bits flip, the syndrome points at a third position, and the receiver corrects the wrong \
             bit without knowing.",
            spaced(&t.recovered),
            if t.recovered == t.data { " They are the data that was sent." } else { "" }
        ),
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;