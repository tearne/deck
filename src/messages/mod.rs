//! The application's messages: events and hints. Every **event** — routine or
//! interrupting — passes through one sink and appends to an in-memory log and
//! the log file, so an event leaving the screen (or never shown) is not lost.
//! **Hints** are transient guidance, displayed and forgotten. The global bar
//! shows the latest displayed entry; the history view shows the whole log.

use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Severity { Info, Success, Warning, Error }

impl Severity {
    fn token(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Success => "ok",
            Severity::Warning => "warn",
            Severity::Error => "error",
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        match token {
            "info" => Some(Severity::Info),
            "ok" => Some(Severity::Success),
            "warn" => Some(Severity::Warning),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
}

/// What a message concerns. In a single display area, position no longer
/// carries this — the message itself must.
#[derive(Clone, Copy)]
pub(crate) enum Source {
    Deck(usize),
    Playlist,
    Tags,
    Files,
    App,
}

impl Source {
    fn token(self) -> &'static str {
        match self {
            Source::Deck(0) => "deck1",
            Source::Deck(1) => "deck2",
            Source::Deck(_) => "deck3",
            Source::Playlist => "playlist",
            Source::Tags => "tags",
            Source::Files => "files",
            Source::App => "app",
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        match token {
            "deck1" => Some(Source::Deck(0)),
            "deck2" => Some(Source::Deck(1)),
            "deck3" => Some(Source::Deck(2)),
            "playlist" => Some(Source::Playlist),
            "tags" => Some(Source::Tags),
            "files" => Some(Source::Files),
            "app" => Some(Source::App),
            _ => None,
        }
    }
}

pub(crate) struct Event {
    pub(crate) at: SystemTime,
    pub(crate) severity: Severity,
    pub(crate) source: Source,
    pub(crate) text: String,
}

impl Event {
    pub(crate) fn new(source: Source, severity: Severity, text: impl Into<String>) -> Self {
        // Single-line by construction, so the log file stays line-per-message.
        let text = text.into().replace(['\n', '\r'], " ");
        Self { at: SystemTime::now(), severity, source, text }
    }

    /// The file form: `YYYY-MM-DD HH:MM:SS <severity> <source>  <text>`, local time.
    pub(crate) fn log_line(&self, utc_offset_secs: i64) -> String {
        let unix = self.at.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        let local = unix + utc_offset_secs;
        let (year, month, day) = crate::error_reports::civil_from_days(local.div_euclid(86_400));
        let t = local.rem_euclid(86_400);
        format!(
            "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} {:<5} {:<8}  {}",
            t / 3600, (t % 3600) / 60, t % 60,
            self.severity.token(), self.source.token(), self.text,
        )
    }

    /// Parse a [`log_line`](Self::log_line) back; `None` for lines that don't conform.
    pub(crate) fn from_log_line(line: &str, utc_offset_secs: i64) -> Option<Self> {
        let at = parse_local_timestamp(line.get(0..19)?, utc_offset_secs)?;
        let rest = line.get(19..)?.trim_start();
        let (severity_token, rest) = rest.split_once(' ')?;
        let rest = rest.trim_start();
        let (source_token, text) = rest.split_once(' ').unwrap_or((rest, ""));
        Some(Self {
            at,
            severity: Severity::from_token(severity_token)?,
            source: Source::from_token(source_token)?,
            text: text.trim_start().to_string(),
        })
    }

    /// The text as displayed: deck messages name their deck, since the bar
    /// no longer sits next to it.
    pub(crate) fn display_text(&self) -> String {
        match self.source {
            Source::Deck(slot) => format!("Deck {}: {}", slot + 1, self.text),
            _ => self.text.clone(),
        }
    }

    /// `HH:MM:SS` local clock time, given the zone offset from [`local_utc_offset_secs`].
    pub(crate) fn clock_time(&self, utc_offset_secs: i64) -> String {
        let unix = self.at.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        let local = (unix + utc_offset_secs).rem_euclid(86_400);
        format!("{:02}:{:02}:{:02}", local / 3600, (local % 3600) / 60, local % 60)
    }
}

/// `YYYY-MM-DD HH:MM:SS` in local time, back to a [`SystemTime`].
fn parse_local_timestamp(s: &str, utc_offset_secs: i64) -> Option<SystemTime> {
    let bytes = s.as_bytes();
    if bytes.len() != 19 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b' ' || bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let num = |range: std::ops::Range<usize>| s.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hour, minute, second) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let days = days_from_civil(year, month as u32, day as u32);
    let local = days * 86_400 + hour * 3600 + minute * 60 + second;
    let unix = local - utc_offset_secs;
    if unix < 0 { return None; }
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(unix as u64))
}

/// Unix day number from a calendar date. Howard Hinnant's `days_from_civil` —
/// the inverse of [`crate::error_reports::civil_from_days`].
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_part = if month > 2 { month - 3 } else { month + 9 } as i64;
    let day_of_year = (153 * month_part + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The local zone's offset from UTC, asked of the system once at startup
/// (std exposes only UTC, and a timezone database dependency isn't warranted
/// for clock display). Falls back to UTC if the query fails.
pub(crate) fn local_utc_offset_secs() -> i64 {
    let output = std::process::Command::new("date").arg("+%z").output();
    let Ok(output) = output else { return 0 };
    let s = String::from_utf8_lossy(&output.stdout);
    let s = s.trim();
    if s.len() != 5 { return 0 }
    let sign = if s.starts_with('-') { -1 } else { 1 };
    let (Ok(hours), Ok(minutes)) = (s[1..3].parse::<i64>(), s[3..5].parse::<i64>()) else { return 0 };
    sign * (hours * 3600 + minutes * 60)
}

/// How long a message stays on screen unless a later one replaces it.
const DISPLAY_TIME: Duration = Duration::from_secs(5);

pub(crate) struct EventStream {
    log: Vec<Event>,
    showing_until: Option<Instant>,
    /// Transient guidance shown on the bar but never remembered — not history,
    /// not the log file. Guidance isn't an event.
    hint: Option<(String, Instant)>,
    log_file: Option<std::fs::File>,
    utc_offset_secs: i64,
}

impl EventStream {
    pub(crate) fn new() -> Self {
        Self { log: Vec::new(), showing_until: None, hint: None, log_file: None, utc_offset_secs: 0 }
    }

    /// Adopt previous sessions' messages (oldest first) without displaying or
    /// re-writing them — the history view scrolls into them, the bar ignores them.
    pub(crate) fn seed(&mut self, messages: Vec<Event>) {
        self.log = messages;
    }

    /// Attach the append-only log file; every emit from here on writes a line
    /// and flushes, so a crash loses nothing.
    pub(crate) fn attach_log_file(&mut self, path: &std::path::Path, utc_offset_secs: i64) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        self.log_file = std::fs::OpenOptions::new().append(true).create(true).open(path).ok();
        self.utc_offset_secs = utc_offset_secs;
    }

    pub(crate) fn emit(&mut self, event: Event) {
        self.emit_showing_for(event, DISPLAY_TIME);
    }

    /// Emit with a non-standard display time, for alerts that must outlive the
    /// usual few seconds (or hints that may sit until something else happens).
    pub(crate) fn emit_showing_for(&mut self, event: Event, display: Duration) {
        self.append(event);
        self.showing_until = Some(Instant::now() + display);
    }

    /// Record a routine event without displaying it — history and file, never
    /// the bar. For events that narrate the session rather than interrupt it.
    pub(crate) fn record(&mut self, event: Event) {
        self.append(event);
    }

    fn append(&mut self, event: Event) {
        if let Some(file) = self.log_file.as_mut() {
            use std::io::Write;
            let _ = writeln!(file, "{}", event.log_line(self.utc_offset_secs));
            let _ = file.flush();
        }
        self.log.push(event);
    }

    /// The message currently on screen and when it leaves, if any.
    pub(crate) fn showing(&self) -> Option<(&Event, Instant)> {
        let until = self.showing_until.filter(|&u| Instant::now() < u)?;
        self.log.last().map(|m| (m, until))
    }

    /// Show transient guidance on the bar without recording it anywhere.
    pub(crate) fn show_hint(&mut self, text: impl Into<String>, display: Duration) {
        self.hint = Some((text.into(), Instant::now() + display));
    }

    /// The hint currently on screen and when it leaves, if any.
    pub(crate) fn hint_showing(&self) -> Option<(&str, Instant)> {
        let (text, until) = self.hint.as_ref()?;
        (Instant::now() < *until).then(|| (text.as_str(), *until))
    }

    /// Take what the bar shows off screen — the message if one is showing,
    /// else the hint. Precedence means a hint behind a message is revealed,
    /// not lost. Messages stay in the log.
    pub(crate) fn dismiss(&mut self) {
        if self.showing().is_some() {
            self.showing_until = None;
        } else {
            self.hint = None;
        }
    }

    /// Take the hint off screen, whatever else the bar shows.
    pub(crate) fn dismiss_hint(&mut self) {
        self.hint = None;
    }

    /// The whole session's messages, oldest first.
    pub(crate) fn entries(&self) -> &[Event] {
        &self.log
    }
}

/// Read the log file, drop lines older than the retention cutoff (and any that
/// don't parse), rewrite it pruned, and return what survives for seeding.
pub(crate) fn load_and_prune(path: &std::path::Path, retention_days: u64, utc_offset_secs: i64) -> Vec<Event> {
    let Ok(content) = std::fs::read_to_string(path) else { return Vec::new() };
    let cutoff = SystemTime::now() - Duration::from_secs(retention_days.saturating_mul(86_400));
    let kept: Vec<Event> = content.lines()
        .filter_map(|line| Event::from_log_line(line, utc_offset_secs))
        .filter(|m| m.at >= cutoff)
        .collect();
    let body: String = kept.iter().map(|m| m.log_line(utc_offset_secs) + "\n").collect();
    let staging = path.with_extension("log.tmp");
    if std::fs::write(&staging, &body).is_ok() {
        let _ = std::fs::rename(&staging, path);
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_line_roundtrip() {
        let m = Event::new(Source::Deck(1), Severity::Warning, "3 tracks unavailable — open the playlist to see which");
        for offset in [-11 * 3600, 0, 3600, 5 * 3600 + 1800] {
            let parsed = Event::from_log_line(&m.log_line(offset), offset).expect("line should parse back");
            assert_eq!(parsed.text, m.text);
            assert!(parsed.severity == m.severity);
            assert_eq!(parsed.source.token(), m.source.token());
            // SystemTime survives to second precision.
            let dt = m.at.duration_since(parsed.at).unwrap_or_default();
            assert!(dt < Duration::from_secs(1), "timestamp drifted by {dt:?}");
        }
    }

    #[test]
    fn newlines_are_sanitised_at_construction() {
        let m = Event::new(Source::App, Severity::Info, "two\nlines\r\nhere");
        assert!(!m.text.contains('\n') && !m.text.contains('\r'));
    }

    #[test]
    fn prune_drops_old_and_malformed_lines_and_rewrites() {
        let dir = std::env::temp_dir().join(format!("deck-msg-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("messages.log");
        let recent = Event::new(Source::App, Severity::Info, "recent").log_line(0);
        let content = format!("2001-01-01 00:00:00 info  app       ancient\nnot a log line\n{recent}\n");
        std::fs::write(&path, content).unwrap();

        let kept = load_and_prune(&path, 90, 0);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].text, "recent");
        let rewritten = std::fs::read_to_string(&path).unwrap();
        assert_eq!(rewritten.lines().count(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_lines_do_not_parse() {
        for line in ["", "not a log line", "2026-08-11 10:00:00 nope app  text", "2026-08-11 10:00 warn app  short stamp"] {
            assert!(Event::from_log_line(line, 0).is_none(), "parsed unexpectedly: {line}");
        }
    }
}
