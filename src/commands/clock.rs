use anyhow::{Result, bail};
use jiff::Zoned;

use crate::config::{Location, Settings};
use crate::store::{self, Session};
use crate::{format, paths, timearg};

pub fn clock_in(note: Option<String>, at: Option<String>, switch: bool) -> Result<()> {
    let now = Zoned::now();
    let started_at = match at {
        Some(raw) => timearg::resolve(&raw, &now)?,
        None => now.clone(),
    };

    if let Some(running) = store::load_session()? {
        if !switch {
            bail!(
                "{} を {} から打刻中です\n\
                 先に dakoku out で締めるか、--switch を付けて切り替えてください",
                running.display_name(),
                format::clock(&running.started_at)
            );
        }
        if started_at < running.started_at {
            bail!(
                "{} には切り替えられません。打刻中のセッションはより後の {} に開始しています",
                format::clock(&started_at),
                format::clock(&running.started_at)
            );
        }
        let closed = running.close(started_at.clone());
        store::append_entry(&closed)?;
        println!(
            "■ {} → {}  {}  {}",
            format::clock(&closed.started_at),
            format::clock(&closed.ended_at),
            format::title(&closed.display_name(), closed.note.as_deref()),
            format::duration(closed.duration())
        );
    }

    let settings = Settings::load()?;
    let location = Location::here(&settings)?;
    let session = Session {
        path: paths::shorten(&location.root),
        label: location.label.clone(),
        note: note.filter(|note| !note.trim().is_empty()),
        started_at,
    };
    store::save_session(&session)?;

    println!(
        "▶ {}  {}",
        format::clock(&session.started_at),
        format::title(&location.display_name(), session.note.as_deref())
    );
    Ok(())
}

pub fn clock_out(at: Option<String>) -> Result<()> {
    let now = Zoned::now();
    let Some(session) = store::load_session()? else {
        bail!("打刻していません。dakoku in で開始してください");
    };
    let ended_at = match at {
        Some(raw) => timearg::resolve(&raw, &now)?,
        None => now,
    };
    if ended_at < session.started_at {
        bail!(
            "{} には終了できません。開始時刻はより後の {} です",
            format::clock(&ended_at),
            format::clock(&session.started_at)
        );
    }

    let entry = session.close(ended_at);
    store::append_entry(&entry)?;
    store::clear_session()?;

    println!(
        "■ {} → {}  {}  {}",
        format::clock(&entry.started_at),
        format::clock(&entry.ended_at),
        format::title(&entry.display_name(), entry.note.as_deref()),
        format::duration(entry.duration())
    );
    Ok(())
}

pub fn status() -> Result<()> {
    let Some(session) = store::load_session()? else {
        println!("· 打刻していません");
        return Ok(());
    };
    let now = Zoned::now();
    println!(
        "▶ {}  {}  {}",
        format::clock(&session.started_at),
        format::title(&session.display_name(), session.note.as_deref()),
        format::duration(session.elapsed(&now))
    );
    Ok(())
}
