use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::config::{Location, Settings};
use crate::{format, paths};

pub fn set(label: String, path: Option<String>) -> Result<()> {
    let mut settings = Settings::load()?;
    let target = target(&settings, path)?;
    let key = settings.set_label(&target, label.clone());
    settings.save()?;
    println!("{key}  {label}");
    Ok(())
}

pub fn unset(path: Option<String>) -> Result<()> {
    let mut settings = Settings::load()?;
    let target = target(&settings, path)?;
    match settings.remove(&target) {
        Some(key) => {
            settings.save()?;
            println!("{key} の設定を削除しました");
        }
        None => println!("· {} には固有の設定がありません", paths::shorten(&target)),
    }
    Ok(())
}

pub fn show(path: Option<String>) -> Result<()> {
    let settings = Settings::load()?;
    let target = target(&settings, path)?;
    println!("{}", paths::shorten(&target));

    match settings.resolve(&target) {
        Some((key, project)) => {
            let label = project.label.as_deref().unwrap_or("(未設定)");
            println!("  ラベル  {label}");
            // Point at the ancestor the label actually came from.
            if paths::expand(key).is_ok_and(|configured| configured != target) {
                println!("  継承元  {key}");
            }
        }
        None => {
            let name = Location::detect(&target, &settings).display_name();
            println!("  ラベル  (未設定、{name} を使用)");
        }
    }
    Ok(())
}

pub fn list() -> Result<()> {
    let settings = Settings::load()?;
    if settings.projects.is_empty() {
        println!("· 設定されたプロジェクトがありません");
        return Ok(());
    }

    let width = settings
        .projects
        .keys()
        .map(|key| format::width(key))
        .max()
        .unwrap_or(0);
    for (key, project) in &settings.projects {
        println!(
            "{}  {}",
            format::pad(key, width),
            project.label.as_deref().unwrap_or("(未設定)")
        );
    }
    Ok(())
}

/// The path a label command applies to: `--path` if given, else the repository here.
fn target(settings: &Settings, path: Option<String>) -> Result<PathBuf> {
    match path {
        Some(raw) => {
            let expanded = paths::expand(&raw)?;
            if expanded.is_absolute() {
                Ok(expanded)
            } else {
                let cwd = std::env::current_dir()
                    .context("カレントディレクトリを取得できませんでした")?;
                Ok(cwd.join(expanded))
            }
        }
        None => Ok(Location::here(settings)?.root),
    }
}
