//! Speaker notes: a TOML file with one `[[slide]]` table per slide.
//!
//! ```toml
//! title = "Объектив"   # optional, shown in the header
//! minutes = 8          # optional talk length
//!
//! [[slide]]
//! n = 1                # slide number in the deck, required
//! title = "Боль"       # optional caption
//! time = "0:30"        # optional limit: 30, "30s" or "0:30"
//! reserve = true       # optional "may skip" tag
//! text = '''
//! What is said on this slide.
//! '''
//! ```
//!
//! The schema is strict: unknown keys and wrong types are errors that point to
//! the exact line, so a typo never silently drops a field.

pub mod markup;

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use markup::Block;

/// Parsed notes, ready for the frontend.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notes {
    pub title: Option<String>,
    pub minutes: Option<f64>,
    /// Keyed by 1-based slide number.
    pub slides: BTreeMap<u32, SlideNote>,
    /// Non-fatal problems: duplicates, notes for slides the deck doesn't have.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideNote {
    pub title: String,
    pub blocks: Vec<Block>,
    /// Spoken words, used to split the talk time between slides.
    pub words: u32,
    /// Explicit time limit for the slide.
    pub seconds: Option<u32>,
    pub reserve: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NotesFile {
    title: Option<String>,
    minutes: Option<f64>,
    #[serde(default)]
    slide: Vec<SlideEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SlideEntry {
    n: u32,
    title: Option<String>,
    time: Option<SlideTime>,
    #[serde(default)]
    reserve: bool,
    text: String,
}

/// `45`, `"45s"`, `"45с"` (Cyrillic) or `"0:45"`.
#[derive(Deserialize)]
#[serde(untagged)]
enum SlideTime {
    Seconds(u32),
    Text(String),
}

impl SlideTime {
    fn seconds(&self) -> Option<u32> {
        match self {
            Self::Seconds(s) => Some(*s),
            Self::Text(s) => parse_time(s),
        }
    }
}

/// Parses `"0:45"`, `"45s"`, `"45с"` or `"45"` into seconds.
fn parse_time(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some((min, sec)) = s.split_once(':') {
        let (min, sec): (u32, u32) = (min.trim().parse().ok()?, sec.trim().parse().ok()?);
        return (sec < 60 && sec_digits(s) == 2).then(|| min * 60 + sec);
    }
    s.trim_end_matches(['s', 'с']).trim_end().parse().ok()
}

fn sec_digits(s: &str) -> usize {
    s.rsplit(':').next().map_or(0, |sec| sec.trim().len())
}

/// Reads and validates a notes file. `slide_count` is used for warnings only.
pub fn load(path: &Path, slide_count: u32) -> Result<Notes> {
    let source = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
    parse(&source, slide_count).map_err(|message| Error::Notes {
        file: path
            .file_name()
            .map_or_else(String::new, |f| f.to_string_lossy().into_owned()),
        message,
    })
}

fn parse(source: &str, slide_count: u32) -> std::result::Result<Notes, String> {
    let file: NotesFile = toml::from_str(source).map_err(|e| describe(&e, source))?;
    if file.minutes.is_some_and(|m| !m.is_finite() || m < 0.0) {
        return Err("minutes: длительность должна быть неотрицательным числом".into());
    }

    let mut notes = Notes {
        title: file.title,
        minutes: file.minutes,
        ..Notes::default()
    };
    for (i, entry) in file.slide.into_iter().enumerate() {
        let at = format!("[[slide]] №{}", i + 1);
        if entry.n == 0 {
            return Err(format!("{at}: слайды нумеруются с 1"));
        }
        let seconds = match &entry.time {
            None => None,
            Some(t) => Some(
                t.seconds()
                    .ok_or_else(|| format!("{at}: время пишется как 45, \"45s\" или \"0:45\""))?,
            ),
        };
        let note = SlideNote {
            title: entry.title.unwrap_or_else(|| format!("Слайд {}", entry.n)),
            blocks: markup::parse_blocks(&entry.text),
            words: markup::spoken_words(&entry.text),
            seconds,
            reserve: entry.reserve,
        };
        if notes.slides.insert(entry.n, note).is_some() {
            notes.warnings.push(format!(
                "Слайд {}: текст указан дважды, взят последний",
                entry.n
            ));
        }
    }

    let orphans: Vec<String> = notes
        .slides
        .keys()
        .filter(|&&n| n > slide_count)
        .map(u32::to_string)
        .collect();
    if !orphans.is_empty() {
        notes.warnings.push(format!(
            "В презентации {slide_count} слайдов, а текст есть для: {}",
            orphans.join(", ")
        ));
    }
    Ok(notes)
}

/// `строка 12: unknown field …` instead of toml's multi-line report, which
/// doesn't fit in a one-line banner.
fn describe(e: &toml::de::Error, source: &str) -> String {
    let message = e.message().trim();
    match e.span() {
        Some(span) => {
            let line = source[..span.start.min(source.len())].matches('\n').count() + 1;
            format!("строка {line}: {message}")
        }
        None => message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_example() {
        let src = r#"
            title = "Объектив"
            minutes = 8

            [[slide]]
            n = 1
            title = "Боль"
            time = "0:30"
            text = '''
            Происходит ли **сейчас** то, что должно?

            > пауза
            '''

            [[slide]]
            n = 3
            time = 20
            reserve = true
            text = "Пропускаем"
        "#;
        let notes = parse(src, 3).unwrap();
        assert_eq!(notes.title.as_deref(), Some("Объектив"));
        assert_eq!(notes.minutes, Some(8.0));
        assert_eq!(notes.warnings, Vec::<String>::new());

        let first = &notes.slides[&1];
        assert_eq!(
            (first.title.as_str(), first.seconds, first.words),
            ("Боль", Some(30), 6)
        );
        assert_eq!(first.blocks.len(), 2);

        let third = &notes.slides[&3];
        assert_eq!(
            (third.title.as_str(), third.seconds, third.reserve),
            ("Слайд 3", Some(20), true)
        );
    }

    #[test]
    fn time_formats() {
        for (s, want) in [
            ("0:45", Some(45)),
            ("1:05", Some(65)),
            ("45s", Some(45)),
            ("45с", Some(45)),
            ("45", Some(45)),
        ] {
            assert_eq!(parse_time(s), want, "{s}");
        }
        for s in ["1:5", "1:75", "abc", "-3", ""] {
            assert_eq!(parse_time(s), None, "{s}");
        }
    }

    #[test]
    fn integer_minutes_are_accepted() {
        assert_eq!(parse("minutes = 5", 1).unwrap().minutes, Some(5.0));
        assert_eq!(parse("minutes = 7.5", 1).unwrap().minutes, Some(7.5));
    }

    #[test]
    fn strict_schema() {
        assert!(parse("titel = \"x\"", 1).unwrap_err().contains("titel"));
        assert!(
            parse("[[slide]]\nn = 1\ntext = \"\"\ntiem = 3", 1)
                .unwrap_err()
                .contains("tiem")
        );
        assert!(
            parse("[[slide]]\ntext = \"x\"", 1)
                .unwrap_err()
                .contains('n')
        );
        assert!(parse("[[slide]]\nn = true\ntext = \"x\"", 1).is_err());
        assert!(
            parse("[[slide]]\nn = 0\ntext = \"x\"", 1)
                .unwrap_err()
                .contains("с 1")
        );
        assert!(
            parse("[[slide]]\nn = 1\ntime = \"soon\"\ntext = \"x\"", 1)
                .unwrap_err()
                .contains("0:45")
        );
        assert!(parse("minutes = -1", 1).is_err());
    }

    #[test]
    fn errors_name_the_line() {
        let err = parse(
            "title = \"x\"\n\n[[slide]]\nn = 1\ntiem = 3\ntext = \"\"",
            1,
        )
        .unwrap_err();
        assert!(err.starts_with("строка 5: unknown field `tiem`"), "{err}");
    }

    #[test]
    fn warnings() {
        let src = "[[slide]]\nn = 2\ntext = \"a\"\n[[slide]]\nn = 2\ntext = \"b\"\n[[slide]]\nn = 9\ntext = \"c\"";
        let notes = parse(src, 3).unwrap();
        assert_eq!(notes.warnings.len(), 2);
        assert!(notes.warnings[1].contains('9'));
    }
}
