use super::{Span, Style};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Parses the inline Markdown in `text` into runs of styled text. See [`super`] for what is understood.
pub(crate) fn parse_inline(text: &str) -> Vec<Span> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    parse_into(&chars, Style::default(), &mut spans);
    merge(spans)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Joins neighbouring runs that are drawn the same way, and drops empty ones.
fn merge(spans: Vec<Span>) -> Vec<Span> {
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans.into_iter().filter(|s| !s.text.is_empty()) {
        match merged.last_mut() {
            Some(last) if last.style == span.style => last.text.push_str(&span.text),
            _ => merged.push(span),
        }
    }
    merged
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many times `c` repeats in `chars` from `at`.
fn run_length(chars: &[char], at: usize, c: char) -> usize {
    chars[at..].iter().take_while(|&&x| x == c).count()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the closing run of `n` copies of `marker` for an opening run that ends just before `from`. The closer
/// must be exactly `n` long, must not follow whitespace, and for an underscore must end a word. A longer run is
/// skipped whole, since it is a different kind of emphasis. `None` if there is no closer.
fn find_closer(chars: &[char], from: usize, marker: char, n: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == marker {
            let run = run_length(chars, i, marker);
            let after = chars.get(i + run).copied();
            let follows_space = i > from && chars[i - 1].is_whitespace();
            let word_end = marker != '_' || !after.is_some_and(is_word_char);
            if run == n && i > from && !follows_space && word_end {
                return Some(i);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the closing run of exactly `n` backticks, for an opening run of `n` that ends just before `from`.
fn find_code_end(chars: &[char], from: usize, n: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '`' {
            let run = run_length(chars, i, '`');
            if run == n {
                return Some(i);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Parses `chars` under `style`, appending to `out`.
fn parse_into(chars: &[char], style: Style, out: &mut Vec<Span>) {
    let mut literal = String::new();
    let flush = |literal: &mut String, out: &mut Vec<Span>| {
        if !literal.is_empty() {
            out.push(Span {
                text: std::mem::take(literal),
                style,
            });
        }
    };

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if chars.get(i + 1).is_some_and(char::is_ascii_punctuation) => {
                literal.push(chars[i + 1]);
                i += 2;
            },
            '`' => {
                let n = run_length(chars, i, '`');
                if let Some(end) = find_code_end(chars, i + n, n) {
                    flush(&mut literal, out);
                    let mut code: String = chars[i + n..end].iter().collect();
                    // One space of padding on both sides lets code start or end with a backtick.
                    if code.len() > 1 && code.starts_with(' ') && code.ends_with(' ') && code.trim() != "" {
                        code = code[1..code.len() - 1].to_owned();
                    }
                    out.push(Span {
                        text: code,
                        style: Style { code: true, ..style },
                    });
                    i = end + n;
                } else {
                    literal.extend(std::iter::repeat_n('`', n));
                    i += n;
                }
            },
            '*' | '_' => {
                let run = run_length(chars, i, c);
                let n = run.min(3);
                let opens = chars.get(i + run).is_some_and(|x| !x.is_whitespace())
                    && (c == '*' || i == 0 || !is_word_char(chars[i - 1]));
                let closer = (run == n && opens).then(|| find_closer(chars, i + n, c, n)).flatten();
                match closer {
                    Some(end) => {
                        flush(&mut literal, out);
                        let inner = Style {
                            bold: style.bold || n >= 2,
                            italic: style.italic || n == 1 || n == 3,
                            ..style
                        };
                        parse_into(&chars[i + n..end], inner, out);
                        i = end + n;
                    },
                    None => {
                        // No closer, so the whole run is text.
                        literal.extend(std::iter::repeat_n(c, run));
                        i += run;
                    },
                }
            },
            _ => {
                literal.push(c);
                i += 1;
            },
        }
    }
    flush(&mut literal, out);
}
