//! Reads numbers and relationships out of the text that the library writes into the document.
//!
//! These are plain functions of strings, so they are tested natively. What they are given comes from `observe`.

/// The numbers in a string such as `translate(20, 20)` or `M 100 40 L 200 40`, in order.
pub fn numbers_in(text: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, numbers: &mut Vec<f64>| {
        if let Ok(value) = current.parse::<f64>() {
            numbers.push(value);
        }
        current.clear();
    };
    for character in text.chars() {
        if character.is_ascii_digit()
            || character == '.'
            || (character == '-' && current.is_empty())
            || (matches!(character, 'e' | 'E') && !current.is_empty())
        {
            current.push(character);
        } else {
            flush(&mut current, &mut numbers);
            if character == '-' {
                current.push('-');
            }
        }
    }
    flush(&mut current, &mut numbers);
    numbers
}

/// The nodes that a node's accessible name says it outputs to.
///
/// The name reads `label. Output to X. Input from Y.`, so it is split on its full stops and the clauses that begin with the
/// words "Output to" are the targets.
pub fn outputs_in(accessible_name: &str) -> Vec<String> {
    accessible_name
        .split(". ")
        .map(|clause| clause.trim().trim_end_matches('.'))
        .filter_map(|clause| clause.strip_prefix("Output to "))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod unit_tests;
