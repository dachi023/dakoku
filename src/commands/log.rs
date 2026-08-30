use std::collections::BTreeMap;

use anyhow::{Context, Result};
use jiff::civil::{Date, Weekday};
use jiff::{SignedDuration, Zoned};

use crate::config::{Location, Settings};
use crate::format;
use crate::store::{self, Entry};

#[derive(Debug, Clone, Copy, Default)]
pub struct Range {
    pub week: bool,
    pub month: bool,
    pub all: bool,
}

pub fn run(range: Range, since: Option<String>, here: bool) -> Result<()> {
    let now = Zoned::now();
    let from = start_of_range(range, since, now.date())?;

    let scope = if here {
        let settings = Settings::load()?;
        Some(Location::here(&settings)?)
    } else {
        None
    };

    let entries: Vec<Entry> = store::load_entries()?
        .into_iter()
        .filter(|entry| from.is_none_or(|from| entry.started_at.date() >= from))
        .filter(|entry| {
            scope.as_ref().is_none_or(|location| {
                crate::paths::expand(&entry.path).is_ok_and(|path| path == location.root)
            })
        })
        .collect();

    if entries.is_empty() {
        println!("· 記録がありません");
        return report_running(&now);
    }

    let mut by_day: BTreeMap<Date, Vec<&Entry>> = BTreeMap::new();
    for entry in &entries {
        by_day
            .entry(entry.started_at.date())
            .or_default()
            .push(entry);
    }

    // One column width for the whole report keeps the days aligned with each other.
    let duration_width = entries
        .iter()
        .map(|entry| format::width(&format::duration(entry.duration())))
        .max()
        .unwrap_or(5)
        .max(5);

    let multiple_days = by_day.len() > 1;
    for (day, mut entries) in by_day {
        entries.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        println!("{}", format::date(day));
        for entry in &entries {
            println!(
                "  {} → {}  {}  {}",
                format::clock(&entry.started_at),
                format::clock(&entry.ended_at),
                format::pad(&format::duration(entry.duration()), duration_width),
                format::title(&entry.display_name(), entry.note.as_deref())
            );
        }
        if multiple_days {
            let total = total(entries.iter().copied());
            println!("  {:>13}  {}", "", format::duration(total));
        }
        println!();
    }

    print_summary(&entries);
    report_running(&now)
}

/// `None` means no lower bound, which is what `--all` asks for.
fn start_of_range(range: Range, since: Option<String>, today: Date) -> Result<Option<Date>> {
    if let Some(since) = since {
        let date: Date = since
            .parse()
            .with_context(|| format!("--since {since} は 2026-08-01 のような日付ではありません"))?;
        return Ok(Some(date));
    }
    if range.all {
        return Ok(None);
    }
    if range.month {
        let first =
            Date::new(today.year(), today.month(), 1).context("月初を計算できませんでした")?;
        return Ok(Some(first));
    }
    if range.week {
        let mut monday = today;
        while monday.weekday() != Weekday::Monday {
            monday = monday.yesterday().context("週初を計算できませんでした")?;
        }
        return Ok(Some(monday));
    }
    Ok(Some(today))
}

fn total<'a>(entries: impl Iterator<Item = &'a Entry>) -> SignedDuration {
    entries.fold(SignedDuration::ZERO, |sum, entry| sum + entry.duration())
}

fn print_summary(entries: &[Entry]) {
    let mut by_label: BTreeMap<String, SignedDuration> = BTreeMap::new();
    for entry in entries {
        *by_label
            .entry(entry.display_name())
            .or_insert(SignedDuration::ZERO) += entry.duration();
    }

    let label_width = by_label
        .keys()
        .map(|label| format::width(label))
        .max()
        .unwrap_or(0)
        .max(format::width("合計"));

    for (label, duration) in &by_label {
        println!(
            "{}  {}",
            format::pad(label, label_width),
            format::duration(*duration)
        );
    }
    let total = total(entries.iter());
    let rule = by_label
        .values()
        .map(|duration| format::width(&format::duration(*duration)))
        .chain(std::iter::once(format::width(&format::duration(total))))
        .max()
        .unwrap_or(0);
    println!("{}", "─".repeat(label_width + 2 + rule));
    println!(
        "{}  {}",
        format::pad("合計", label_width),
        format::duration(total)
    );
}

fn report_running(now: &Zoned) -> Result<()> {
    let Some(session) = store::load_session()? else {
        return Ok(());
    };
    println!();
    println!(
        "▶ {}  {}  {} (打刻中)",
        format::clock(&session.started_at),
        format::title(&session.display_name(), session.note.as_deref()),
        format::duration(session.elapsed(now))
    );
    Ok(())
}
