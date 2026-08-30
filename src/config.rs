use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::paths;

/// The contents of `~/.dakoku/settings.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub projects: BTreeMap<String, Project>,
    /// Keys dakoku does not know about, kept so hand-written settings survive a write.
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Project {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let file = paths::settings_file()?;
        Self::load_from(&file)
    }

    pub fn load_from(file: &Path) -> Result<Self> {
        let raw = match std::fs::read_to_string(file) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("{} を読み込めませんでした", file.display()));
            }
        };
        serde_json::from_str(&raw)
            .with_context(|| format!("{} は正しい JSON ではありません", file.display()))
    }

    pub fn save(&self) -> Result<()> {
        let file = paths::settings_file()?;
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        paths::write_atomically(&file, &json)
    }

    /// Finds the configured project that most closely encloses `path`.
    ///
    /// The longest match wins, so clocking in from a subdirectory still picks up
    /// the label configured on the repository root.
    pub fn resolve(&self, path: &Path) -> Option<(&str, &Project)> {
        let mut best: Option<(usize, &str, &Project)> = None;
        for (key, project) in &self.projects {
            let Ok(configured) = paths::expand(key) else {
                continue;
            };
            if !path.starts_with(&configured) {
                continue;
            }
            let depth = configured.components().count();
            if best.is_none_or(|(current, _, _)| depth > current) {
                best = Some((depth, key.as_str(), project));
            }
        }
        best.map(|(_, key, project)| (key, project))
    }

    pub fn label_for(&self, path: &Path) -> Option<String> {
        self.resolve(path)
            .and_then(|(_, project)| project.label.clone())
    }

    /// Sets a label, reusing the existing key when the path is already configured.
    pub fn set_label(&mut self, path: &Path, label: String) -> String {
        let key = self
            .projects
            .keys()
            .find(|key| paths::expand(key).is_ok_and(|configured| configured == path))
            .cloned()
            .unwrap_or_else(|| paths::shorten(path));

        self.projects.entry(key.clone()).or_default().label = Some(label);
        key
    }

    pub fn remove(&mut self, path: &Path) -> Option<String> {
        let key = self
            .projects
            .keys()
            .find(|key| paths::expand(key).is_ok_and(|configured| configured == path))
            .cloned()?;
        self.projects.remove(&key);
        Some(key)
    }
}

/// The repository (or plain directory) a session belongs to.
#[derive(Debug, Clone)]
pub struct Location {
    pub root: PathBuf,
    pub label: Option<String>,
}

impl Location {
    /// Walks up from `start` looking for a repository root, falling back to `start`.
    ///
    /// `.git` is a file rather than a directory inside a worktree, so both count.
    pub fn detect(start: &Path, settings: &Settings) -> Self {
        let root = start
            .ancestors()
            .find(|dir| dir.join(".git").exists())
            .unwrap_or(start)
            .to_path_buf();
        let label = settings.label_for(&root);
        Location { root, label }
    }

    pub fn here(settings: &Settings) -> Result<Self> {
        let cwd = std::env::current_dir().context("カレントディレクトリを取得できませんでした")?;
        Ok(Self::detect(&cwd, settings))
    }

    /// The label to print: the configured one, else the directory name.
    pub fn display_name(&self) -> String {
        self.label.clone().unwrap_or_else(|| {
            self.root
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| paths::shorten(&self.root))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(json: &str) -> Settings {
        serde_json::from_str(json).expect("valid settings")
    }

    #[test]
    fn the_closest_configured_ancestor_wins() {
        let settings = settings(
            r#"{"projects": {
                "/work": {"label": "House"},
                "/work/acme": {"label": "Acme"}
            }}"#,
        );
        assert_eq!(
            settings
                .label_for(Path::new("/work/acme/api/src"))
                .as_deref(),
            Some("Acme")
        );
        assert_eq!(
            settings.label_for(Path::new("/work/other")).as_deref(),
            Some("House")
        );
        assert_eq!(settings.label_for(Path::new("/elsewhere")), None);
    }

    #[test]
    fn sibling_directories_do_not_match_on_a_shared_prefix() {
        let settings = settings(r#"{"projects": {"/work/acme": {"label": "Acme"}}}"#);
        assert_eq!(settings.label_for(Path::new("/work/acme-labs")), None);
    }

    #[test]
    fn unknown_keys_survive_a_round_trip() {
        let settings =
            settings(r#"{"version": 2, "projects": {"/work": {"label": "House", "rate": 100}}}"#);
        let json = serde_json::to_string(&settings).expect("serializable");
        assert!(json.contains("\"version\":2"));
        assert!(json.contains("\"rate\":100"));
    }

    #[test]
    fn setting_a_label_reuses_an_existing_key() {
        let mut settings = settings(r#"{"projects": {"/work/acme": {"label": "Acme"}}}"#);
        let key = settings.set_label(Path::new("/work/acme"), "Acme Corp".to_string());
        assert_eq!(key, "/work/acme");
        assert_eq!(settings.projects.len(), 1);
        assert_eq!(
            settings.label_for(Path::new("/work/acme")).as_deref(),
            Some("Acme Corp")
        );
    }

    #[test]
    fn a_directory_name_stands_in_for_a_missing_label() {
        let settings = Settings::default();
        let location = Location::detect(Path::new("/work/acme-api"), &settings);
        assert_eq!(location.display_name(), "acme-api");
    }
}
