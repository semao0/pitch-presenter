//! Lightweight markup used inside speaker notes.
//!
//! The notes text is parsed into a small structured tree instead of HTML, so the
//! frontend renders it with regular components and never touches `innerHTML`.
//!
//! Supported syntax:
//!
//! | Markup            | Meaning                                         |
//! |-------------------|-------------------------------------------------|
//! | blank line        | paragraph break                                 |
//! | `- item`          | bullet list (every line of the block)           |
//! | `> remark`        | cue: a note to self, not spoken, not timed      |
//! | `**bold**`        | strong emphasis                                 |
//! | `*italic*`, `_x_` | emphasis                                        |
//! | `==highlight==`   | highlighted key phrase                          |
//! | `` `code` ``      | verbatim                                        |

use serde::Serialize;

/// A paragraph-level element of a note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "content", rename_all = "camelCase")]
pub enum Block {
    Paragraph(Vec<Inline>),
    List(Vec<Vec<Inline>>),
    Cue(Vec<Inline>),
}

/// A span of text inside a block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Inline {
    Text { text: String },
    Code { text: String },
    Strong { children: Vec<Inline> },
    Em { children: Vec<Inline> },
    Mark { children: Vec<Inline> },
}

/// Splits a note into blocks separated by blank lines.
///
/// Lines inside a paragraph are joined with spaces: line breaks in the source
/// are for the author's convenience, the presenter view reflows the text.
#[must_use]
pub fn parse_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines().chain(std::iter::once("")) {
        if line.trim().is_empty() {
            if !lines.is_empty() {
                blocks.push(block(&lines));
                lines.clear();
            }
        } else {
            lines.push(line.trim());
        }
    }
    blocks
}

fn block(lines: &[&str]) -> Block {
    if lines.iter().all(|l| bullet_item(l).is_some()) {
        let items = lines
            .iter()
            .filter_map(|l| bullet_item(l))
            .map(parse_inline);
        return Block::List(items.collect());
    }
    if lines.iter().all(|l| l.starts_with('>')) {
        let text = lines
            .iter()
            .map(|l| l.trim_start_matches(['>', ' ']))
            .collect::<Vec<_>>()
            .join(" ");
        return Block::Cue(parse_inline(&text));
    }
    Block::Paragraph(parse_inline(&lines.join(" ")))
}

/// `- item`, `* item` or `+ item` → `item`.
fn bullet_item(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(['-', '*', '+'])?;
    rest.starts_with(char::is_whitespace)
        .then(|| rest.trim_start())
}

/// Counts the words that are actually spoken: cue lines (`> …`) and markup
/// characters are ignored. Hyphenated words count as one.
#[must_use]
pub fn spoken_words(text: &str) -> u32 {
    let mut count = 0;
    for line in text.lines().filter(|l| !l.trim_start().starts_with('>')) {
        let mut in_word = false;
        for c in line.chars().filter(|c| !matches!(c, '*' | '_' | '=' | '`')) {
            if c.is_alphanumeric() {
                if !in_word {
                    count += 1;
                }
                in_word = true;
            } else if !(in_word && matches!(c, '-' | '–')) {
                in_word = false;
            }
        }
    }
    count
}

/// Parses inline markup. Unmatched delimiters are kept as literal text.
#[must_use]
pub fn parse_inline(text: &str) -> Vec<Inline> {
    let chars: Vec<char> = text.chars().collect();
    InlineParser { chars: &chars }.parse(0, chars.len())
}

struct InlineParser<'a> {
    chars: &'a [char],
}

impl InlineParser<'_> {
    fn parse(&self, start: usize, end: usize) -> Vec<Inline> {
        let mut out = Vec::new();
        let mut text = String::new();
        let mut i = start;
        while i < end {
            if let Some((node, next)) = self.span_at(i, end) {
                if !text.is_empty() {
                    out.push(Inline::Text {
                        text: std::mem::take(&mut text),
                    });
                }
                out.push(node);
                i = next;
            } else {
                text.push(self.chars[i]);
                i += 1;
            }
        }
        if !text.is_empty() {
            out.push(Inline::Text { text });
        }
        out
    }

    /// Tries to parse a span opening at `i`; returns it with the index after it.
    fn span_at(&self, i: usize, end: usize) -> Option<(Inline, usize)> {
        let c = self.chars[i];
        match c {
            '`' => {
                let close = (i + 2..end).find(|&j| self.chars[j] == '`')?;
                let text = self.chars[i + 1..close].iter().collect();
                Some((Inline::Code { text }, close + 1))
            }
            '*' | '=' if self.pair_at(i, end, c) => {
                let mut close =
                    (i + 3..end.saturating_sub(1)).find(|&j| self.pair_at(j, end, c))?;
                // In `**a *b***` the closing pair is the last two of the run.
                while close + 2 < end && self.chars[close + 2] == c {
                    close += 1;
                }
                let children = self.parse(i + 2, close);
                let node = if c == '*' {
                    Inline::Strong { children }
                } else {
                    Inline::Mark { children }
                };
                Some((node, close + 2))
            }
            '*' | '_' if self.can_open(i, end, c) => {
                let close = self.emphasis_close(i, end, c)?;
                Some((
                    Inline::Em {
                        children: self.parse(i + 1, close),
                    },
                    close + 1,
                ))
            }
            _ => None,
        }
    }

    fn pair_at(&self, i: usize, end: usize, c: char) -> bool {
        i + 1 < end && self.chars[i] == c && self.chars[i + 1] == c
    }

    /// An emphasis delimiter opens when it is not glued to a preceding word
    /// (`snake_case` stays literal) and is followed by a non-space.
    fn can_open(&self, i: usize, end: usize, c: char) -> bool {
        let before = i.checked_sub(1).map(|j| self.chars[j]);
        let after = (i + 1 < end).then(|| self.chars[i + 1]);
        !before.is_some_and(|b| is_word(b) || b == c) && after.is_some_and(|a| !a.is_whitespace())
    }

    fn emphasis_close(&self, open: usize, end: usize, c: char) -> Option<usize> {
        let mut j = open + 2;
        while j < end {
            if c == '*' && self.pair_at(j, end, '*') {
                j += 2; // a nested **strong** span, not our closing star
                continue;
            }
            let after = (j + 1 < end).then(|| self.chars[j + 1]);
            if self.chars[j] == c
                && !self.chars[j - 1].is_whitespace()
                && !after.is_some_and(|a| is_word(a) || a == c)
            {
                return Some(j);
            }
            j += 1;
        }
        None
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Inline {
        Inline::Text { text: s.into() }
    }

    #[test]
    fn plain_text_is_one_node() {
        assert_eq!(parse_inline("просто текст"), vec![text("просто текст")]);
    }

    #[test]
    fn strong_mark_code_em() {
        assert_eq!(
            parse_inline("a **b** ==c== `d*` *e* _f_"),
            vec![
                text("a "),
                Inline::Strong {
                    children: vec![text("b")]
                },
                text(" "),
                Inline::Mark {
                    children: vec![text("c")]
                },
                text(" "),
                Inline::Code { text: "d*".into() },
                text(" "),
                Inline::Em {
                    children: vec![text("e")]
                },
                text(" "),
                Inline::Em {
                    children: vec![text("f")]
                },
            ]
        );
    }

    #[test]
    fn nesting() {
        assert_eq!(
            parse_inline("*a **b** c*"),
            vec![Inline::Em {
                children: vec![
                    text("a "),
                    Inline::Strong {
                        children: vec![text("b")]
                    },
                    text(" c")
                ]
            }]
        );
        assert_eq!(
            parse_inline("**жирный *курсив***"),
            vec![Inline::Strong {
                children: vec![
                    text("жирный "),
                    Inline::Em {
                        children: vec![text("курсив")]
                    }
                ]
            }]
        );
    }

    #[test]
    fn unmatched_and_intraword_delimiters_stay_literal() {
        assert_eq!(
            parse_inline("snake_case_name"),
            vec![text("snake_case_name")]
        );
        assert_eq!(parse_inline("2 * 3 * 4"), vec![text("2 * 3 * 4")]);
        assert_eq!(parse_inline("**open"), vec![text("**open")]);
        assert_eq!(parse_inline("a == b"), vec![text("a == b")]);
        assert_eq!(parse_inline("``"), vec![text("``")]);
    }

    #[test]
    fn blocks() {
        let src = "Первая строка\nвторая строка\n\n- раз\n- *два*\n\n> пауза\n> и вдох\n\n  \n";
        assert_eq!(
            parse_blocks(src),
            vec![
                Block::Paragraph(vec![text("Первая строка вторая строка")]),
                Block::List(vec![
                    vec![text("раз")],
                    vec![Inline::Em {
                        children: vec![text("два")]
                    }]
                ]),
                Block::Cue(vec![text("пауза и вдох")]),
            ]
        );
    }

    #[test]
    fn mixed_block_is_a_paragraph() {
        assert_eq!(
            parse_blocks("- a\nb"),
            vec![Block::Paragraph(vec![text("- a b")])]
        );
        assert_eq!(parse_blocks("-a"), vec![Block::Paragraph(vec![text("-a")])]);
    }

    #[test]
    fn words_skip_cues_and_markup() {
        assert_eq!(
            spoken_words("Раз **два** три-четыре\n> не считается\n- пять"),
            4
        );
        assert_eq!(spoken_words("snake_case и ==UX=="), 3);
        assert_eq!(spoken_words(""), 0);
    }
}
