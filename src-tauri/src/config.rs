//! Persistent settings: talk length and recently opened decks.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

const RECENT_LIMIT: usize = 8;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Talk length from the start screen; 0 means "no limit".
    pub minutes: f64,
    /// Most recent first.
    pub recent: Vec<RecentDeck>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDeck {
    pub deck: PathBuf,
    pub notes: Option<PathBuf>,
    /// Unix time, seconds.
    pub opened_at: u64,
}

impl Config {
    /// Loads the config, falling back to defaults if it is missing or corrupt.
    pub fn load(path: &Path) -> Self {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                log::warn!("ignoring corrupt config {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Writes atomically: a crash mid-write never leaves a truncated file.
    pub fn save(&self, path: &Path) {
        let write = || -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let tmp = path.with_extension("json.tmp");
            fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
            fs::rename(tmp, path)
        };
        if let Err(e) = write() {
            log::warn!("cannot save config {}: {e}", path.display());
        }
    }

    /// Moves `deck` to the top of the recent list.
    pub fn remember(&mut self, deck: &Path, notes: Option<&Path>, minutes: f64) {
        let opened_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        self.minutes = minutes;
        self.recent.retain(|r| r.deck != deck);
        self.recent.insert(
            0,
            RecentDeck {
                deck: deck.to_path_buf(),
                notes: notes.map(Path::to_path_buf),
                opened_at,
            },
        );
        self.recent.truncate(RECENT_LIMIT);
    }

    /// Recent decks whose files still exist; vanished notes are dropped.
    pub fn existing_recent(&self) -> Vec<RecentDeck> {
        self.recent
            .iter()
            .filter(|r| r.deck.is_file())
            .map(|r| RecentDeck {
                notes: r.notes.clone().filter(|n| n.is_file()),
                ..r.clone()
            })
            .collect()
    }
}

/// `talk.pdf` → `talk.toml`, if it exists.
pub fn guess_notes(deck: &Path) -> Option<PathBuf> {
    Some(deck.with_extension("toml")).filter(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remember_dedupes_and_limits() {
        let mut config = Config::default();
        for i in 0..10 {
            config.remember(Path::new(&format!("/d{i}.pdf")), None, 5.0);
        }
        config.remember(Path::new("/d3.pdf"), Some(Path::new("/d3.toml")), 7.0);
        assert_eq!(config.recent.len(), RECENT_LIMIT);
        assert_eq!(config.recent[0].deck, Path::new("/d3.pdf"));
        assert_eq!(
            config
                .recent
                .iter()
                .filter(|r| r.deck == Path::new("/d3.pdf"))
                .count(),
            1
        );
        assert!((config.minutes - 7.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tolerates_partial_config() {
        let config: Config = serde_json::from_str(r#"{"minutes": 3}"#).unwrap();
        assert_eq!(config.recent, []);
    }
}
