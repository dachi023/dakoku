use anyhow::{Result, bail};
use jiff::Zoned;

use crate::store;
use crate::{format, timearg};

/// Amends the running session, or the most recent one once it has been closed.
pub fn run(note: Option<String>, start: Option<String>, end: Option<String>) -> Result<()> {
    if note.is_none() && start.is_none() && end.is_none() {
        bail!("変更内容がありません。--note、--in、--out のいずれかを指定してください");
    }
    let now = Zoned::now();

    if let Some(mut session) = store::load_session()? {
        if end.is_some() {
            bail!("打刻中のセッションです。締めるには dakoku out --at <時刻> を使ってください");
        }
        if let Some(note) = note {
            session.note = Some(note).filter(|note| !note.trim().is_empty());
        }
        if let Some(raw) = start {
            let started_at = timearg::resolve(&raw, &now)?;
            if started_at > now {
                bail!("{} は未来の時刻です", format::clock(&started_at));
            }
            session.started_at = started_at;
        }
        store::save_session(&session)?;
        println!(
            "▶ {}  {}  {}",
            format::clock(&session.started_at),
            format::title(&session.display_name(), session.note.as_deref()),
            format::duration(session.elapsed(&now))
        );
        return Ok(());
    }

    let mut entries = store::load_entries()?;
    let Some(entry) = entries.last_mut() else {
        bail!("記録がまだありません");
    };

    if let Some(note) = note {
        entry.note = Some(note).filter(|note| !note.trim().is_empty());
    }
    if let Some(raw) = start {
        entry.started_at = timearg::resolve(&raw, &now)?;
    }
    if let Some(raw) = end {
        entry.ended_at = timearg::resolve(&raw, &now)?;
    }
    if entry.ended_at < entry.started_at {
        bail!(
            "終了 {} が開始 {} より前になっています",
            format::clock(&entry.ended_at),
            format::clock(&entry.started_at)
        );
    }

    let updated = entry.clone();
    store::save_entries(&entries)?;
    println!(
        "■ {} → {}  {}  {}",
        format::clock(&updated.started_at),
        format::clock(&updated.ended_at),
        format::title(&updated.display_name(), updated.note.as_deref()),
        format::duration(updated.duration())
    );
    Ok(())
}
