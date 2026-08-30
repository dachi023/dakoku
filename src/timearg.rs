use anyhow::{Context, Result, bail};
use jiff::civil;
use jiff::{SignedDuration, Zoned};

/// Resolves the value of `--at` against `now`.
///
/// Accepted forms:
///   `18:00`              a clock time today, or yesterday when that is still ahead of now
///   `2026-08-30 18:00`   an explicit date and time (a `T` separator also works)
///   `-1h30m`             an offset back from now
pub fn resolve(raw: &str, now: &Zoned) -> Result<Zoned> {
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("--at には 18:00、2026-08-30 18:00、-90m のような値を指定してください");
    }

    if let Some(rest) = raw.strip_prefix('-') {
        let back = parse_offset(rest)?;
        return now
            .checked_sub(back)
            .with_context(|| format!("{raw} は扱える範囲を超えています"));
    }

    if let Some(datetime) = parse_datetime(raw)? {
        return now
            .time_zone()
            .to_zoned(datetime)
            .with_context(|| format!("{raw} はこのタイムゾーンに存在しない時刻です"));
    }

    let time = parse_clock(raw)?;
    let today = now
        .time_zone()
        .to_zoned(now.date().to_datetime(time))
        .with_context(|| format!("{raw} はこのタイムゾーンに存在しない時刻です"))?;

    // A time later than now almost always means the shift that just ended overnight.
    if &today > now {
        let yesterday = now
            .date()
            .yesterday()
            .with_context(|| format!("{raw} は扱える範囲を超えています"))?;
        return now
            .time_zone()
            .to_zoned(yesterday.to_datetime(time))
            .with_context(|| format!("{raw} はこのタイムゾーンに存在しない時刻です"));
    }
    Ok(today)
}

fn parse_datetime(raw: &str) -> Result<Option<civil::DateTime>> {
    let Some((date, time)) = raw.split_once(['T', ' ']) else {
        return Ok(None);
    };
    let date: civil::Date = date
        .trim()
        .parse()
        .with_context(|| format!("{raw} は 2026-08-30 18:00 のような日時ではありません"))?;
    Ok(Some(date.to_datetime(parse_clock(time.trim())?)))
}

fn parse_clock(raw: &str) -> Result<civil::Time> {
    let (hour, minute) = raw
        .split_once(':')
        .with_context(|| format!("{raw} は 18:00 のような時刻ではありません"))?;
    let hour: i8 = hour
        .parse()
        .with_context(|| format!("{raw} は 18:00 のような時刻ではありません"))?;
    let minute: i8 = minute
        .parse()
        .with_context(|| format!("{raw} は 18:00 のような時刻ではありません"))?;
    civil::Time::new(hour, minute, 0, 0).with_context(|| format!("{raw} は存在しない時刻です"))
}

/// Parses `90m`, `1h30m` or `2h` into a duration.
fn parse_offset(raw: &str) -> Result<SignedDuration> {
    let invalid = || anyhow::anyhow!("{raw} は 90m、1h30m、2h のような差分ではありません");

    let mut seconds: i64 = 0;
    let mut digits = String::new();
    let mut matched = false;
    for ch in raw.chars() {
        match ch {
            '0'..='9' => digits.push(ch),
            'h' | 'm' if !digits.is_empty() => {
                let value: i64 = digits.parse().map_err(|_| invalid())?;
                seconds += value * if ch == 'h' { 3600 } else { 60 };
                digits.clear();
                matched = true;
            }
            _ => return Err(invalid()),
        }
    }
    // A bare number is read as minutes, which is what `-30` most likely means.
    if !digits.is_empty() {
        seconds += digits.parse::<i64>().map_err(|_| invalid())? * 60;
        matched = true;
    }
    if !matched {
        return Err(invalid());
    }
    Ok(SignedDuration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Zoned {
        "2026-08-31T10:00:00+09:00[Asia/Tokyo]"
            .parse()
            .expect("a valid fixture timestamp")
    }

    fn resolved(raw: &str) -> String {
        resolve(raw, &now()).expect("a resolvable time").to_string()
    }

    #[test]
    fn a_clock_time_earlier_today_stays_today() {
        assert!(resolved("09:30").starts_with("2026-08-31T09:30:00"));
    }

    #[test]
    fn a_clock_time_still_ahead_falls_back_to_yesterday() {
        assert!(resolved("22:00").starts_with("2026-08-30T22:00:00"));
    }

    #[test]
    fn explicit_dates_are_taken_as_given() {
        assert!(resolved("2026-08-29 13:00").starts_with("2026-08-29T13:00:00"));
        assert!(resolved("2026-08-29T13:00").starts_with("2026-08-29T13:00:00"));
    }

    #[test]
    fn offsets_count_back_from_now() {
        assert!(resolved("-90m").starts_with("2026-08-31T08:30:00"));
        assert!(resolved("-1h30m").starts_with("2026-08-31T08:30:00"));
        assert!(resolved("-2h").starts_with("2026-08-31T08:00:00"));
        // A bare number is read as minutes.
        assert!(resolved("-45").starts_with("2026-08-31T09:15:00"));
    }

    #[test]
    fn malformed_values_are_rejected() {
        for raw in ["", "18", "25:00", "10:99", "-", "-5x", "tomorrow"] {
            assert!(resolve(raw, &now()).is_err(), "{raw} should not parse");
        }
    }
}
