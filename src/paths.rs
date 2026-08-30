use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Environment variable that overrides where dakoku keeps its data.
const HOME_ENV: &str = "DAKOKU_HOME";

pub fn home() -> Result<PathBuf> {
    std::env::home_dir().context("ホームディレクトリを特定できませんでした")
}

/// Expands a leading `~` so that settings written by hand keep working.
pub fn expand(raw: &str) -> Result<PathBuf> {
    if raw == "~" {
        return home();
    }
    match raw.strip_prefix("~/") {
        Some(rest) => Ok(home()?.join(rest)),
        None => Ok(PathBuf::from(raw)),
    }
}

/// Renders a path with the home directory abbreviated back to `~`.
pub fn shorten(path: &Path) -> String {
    let Ok(home) = home() else {
        return path.display().to_string();
    };
    match path.strip_prefix(&home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

pub fn data_dir() -> Result<PathBuf> {
    match std::env::var_os(HOME_ENV) {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Ok(home()?.join(".dakoku")),
    }
}

pub fn settings_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("settings.json"))
}

pub fn current_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("current.json"))
}

pub fn entries_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("entries.jsonl"))
}

/// Writes through a temporary file so an interrupted run cannot truncate data.
pub fn write_atomically(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} に親ディレクトリがありません", path.display()))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("{} を作成できませんでした", parent.display()))?;

    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)
        .with_context(|| format!("{} に書き込めませんでした", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("{} を置き換えられませんでした", path.display()))?;
    Ok(())
}
