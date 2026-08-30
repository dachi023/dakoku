use jiff::civil::{Date, Weekday};
use jiff::{SignedDuration, Zoned};
use unicode_width::UnicodeWidthStr;

/// Renders a duration as `2h35m`, or `45m` when it is under an hour.
pub fn duration(value: SignedDuration) -> String {
    let seconds = value.as_secs().max(0);
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h{minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

pub fn clock(at: &Zoned) -> String {
    format!("{:02}:{:02}", at.hour(), at.minute())
}

pub fn date(day: Date) -> String {
    format!("{day} ({})", weekday(day.weekday()))
}

fn weekday(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "月",
        Weekday::Tuesday => "火",
        Weekday::Wednesday => "水",
        Weekday::Thursday => "木",
        Weekday::Friday => "金",
        Weekday::Saturday => "土",
        Weekday::Sunday => "日",
    }
}

/// Pads to a display width, so CJK labels line up in the same column as ASCII ones.
pub fn pad(text: &str, width: usize) -> String {
    let padding = width.saturating_sub(text.width());
    format!("{text}{}", " ".repeat(padding))
}

pub fn width(text: &str) -> usize {
    text.width()
}

/// `label / note`, or just the label when no note was given.
pub fn title(label: &str, note: Option<&str>) -> String {
    match note {
        Some(note) if !note.is_empty() => format!("{label} / {note}"),
        _ => label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_as_hours_and_minutes() {
        assert_eq!(duration(SignedDuration::from_secs(0)), "0m");
        assert_eq!(duration(SignedDuration::from_secs(45 * 60)), "45m");
        assert_eq!(duration(SignedDuration::from_secs(3600)), "1h00m");
        assert_eq!(duration(SignedDuration::from_secs(9320)), "2h35m");
    }

    #[test]
    fn negative_durations_clamp_to_zero() {
        assert_eq!(duration(SignedDuration::from_secs(-60)), "0m");
    }

    #[test]
    fn padding_counts_display_width_not_bytes() {
        // Each CJK character occupies two columns.
        assert_eq!(pad("A社", 6), "A社   ");
        assert_eq!(pad("abc", 6), "abc   ");
    }

    #[test]
    fn titles_drop_an_absent_note() {
        assert_eq!(title("A社ZZ案件", Some("CLI実装")), "A社ZZ案件 / CLI実装");
        assert_eq!(title("A社ZZ案件", None), "A社ZZ案件");
        assert_eq!(title("A社ZZ案件", Some("")), "A社ZZ案件");
    }
}
