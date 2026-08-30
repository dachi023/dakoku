use std::path::Path;

use anyhow::{Context, Result};
use jiff::{SignedDuration, Zoned};
use serde::{Deserialize, Serialize};

use crate::paths;

/// A session that has been clocked in but not yet clocked out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Repository root, stored with `~` abbreviated.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub started_at: Zoned,
}

/// A closed session, appended as one line of `entries.jsonl`.
///
/// The label is stored alongside the path rather than resolved at display time,
/// so renaming a project in the settings does not rewrite past records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub started_at: Zoned,
    pub ended_at: Zoned,
}

impl Session {
    pub fn elapsed(&self, now: &Zoned) -> SignedDuration {
        self.started_at.duration_until(now)
    }

    pub fn close(self, ended_at: Zoned) -> Entry {
        Entry {
            path: self.path,
            label: self.label,
            note: self.note,
            started_at: self.started_at,
            ended_at,
        }
    }

    pub fn display_name(&self) -> String {
        display_name(&self.label, &self.path)
    }
}

impl Entry {
    pub fn duration(&self) -> SignedDuration {
        self.started_at.duration_until(&self.ended_at)
    }

    pub fn display_name(&self) -> String {
        display_name(&self.label, &self.path)
    }
}

fn display_name(label: &Option<String>, path: &str) -> String {
    label.clone().unwrap_or_else(|| {
        Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string())
    })
}

pub fn load_session() -> Result<Option<Session>> {
    let file = paths::current_file()?;
    let raw = match std::fs::read_to_string(&file) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| format!("{} を読み込めませんでした", file.display()));
        }
    };
    let session = serde_json::from_str(&raw)
        .with_context(|| format!("{} は正しいセッションではありません", file.display()))?;
    Ok(Some(session))
}

pub fn save_session(session: &Session) -> Result<()> {
    let file = paths::current_file()?;
    let mut json = serde_json::to_string_pretty(session)?;
    json.push('\n');
    paths::write_atomically(&file, &json)
}

pub fn clear_session() -> Result<()> {
    let file = paths::current_file()?;
    match std::fs::remove_file(&file) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("{} を削除できませんでした", file.display())),
    }
}

pub fn load_entries() -> Result<Vec<Entry>> {
    let file = paths::entries_file()?;
    let raw = match std::fs::read_to_string(&file) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(err).with_context(|| format!("{} を読み込めませんでした", file.display()));
        }
    };

    let mut entries = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let entry = serde_json::from_str(line).with_context(|| {
            format!(
                "{}:{} 行目が正しい記録ではありません",
                file.display(),
                index + 1
            )
        })?;
        entries.push(entry);
    }
    Ok(entries)
}

pub fn append_entry(entry: &Entry) -> Result<()> {
    use std::io::Write;

    let file = paths::entries_file()?;
    let dir = paths::data_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("{} を作成できませんでした", dir.display()))?;

    let mut handle = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .with_context(|| format!("{} を開けませんでした", file.display()))?;
    writeln!(handle, "{}", serde_json::to_string(entry)?)
        .with_context(|| format!("{} に追記できませんでした", file.display()))?;
    Ok(())
}

pub fn save_entries(entries: &[Entry]) -> Result<()> {
    let file = paths::entries_file()?;
    let mut out = String::new();
    for entry in entries {
        out.push_str(&serde_json::to_string(entry)?);
        out.push('\n');
    }
    paths::write_atomically(&file, &out)
}
