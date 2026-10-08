//! Best results per challenge, saved as JSON in the OS data directory
//! (spec FR-012). A missing or corrupt file just means "no progress yet".

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub best_moves: u32,
    pub stars: u8,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    records: BTreeMap<u32, Record>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Progress {
    fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "rush-hour").map(|d| d.data_dir().join("progress.json"))
    }

    pub fn load() -> Progress {
        let path = Self::default_path();
        let mut p: Progress = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        p.path = path;
        p
    }

    /// In-memory progress that is never written to disk (dev screenshots).
    pub fn ephemeral() -> Progress {
        Progress::default()
    }

    pub fn get(&self, number: u32) -> Option<Record> {
        self.records.get(&number).copied()
    }

    pub fn total_stars(&self) -> u32 {
        self.records.values().map(|r| r.stars as u32).sum()
    }

    pub fn solved_count(&self, numbers: impl Iterator<Item = u32>) -> usize {
        numbers.filter(|n| self.records.contains_key(n)).count()
    }

    /// Records a solve; returns true if it beat the previous best.
    pub fn record(&mut self, number: u32, moves: u32, stars: u8) -> bool {
        let better = self.get(number).is_none_or(|r| moves < r.best_moves);
        if better {
            self.records.insert(
                number,
                Record {
                    best_moves: moves,
                    stars,
                },
            );
            self.save();
        }
        better
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let write = || -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
            std::fs::rename(&tmp, path)
        };
        if let Err(e) = write() {
            eprintln!("rush-hour: could not save progress to {}: {e}", path.display());
        }
    }
}
