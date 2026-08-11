//! The application's message stream. Every passive message — notice, warning,
//! error, success — passes through one sink and appends to an in-memory log,
//! so a message leaving the screen is never lost. Display surfaces render
//! views of the log; today that is the global bar showing the latest entry.

use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Severity { Info, Success, Warning, Error }

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

pub(crate) struct Message {
    pub(crate) at: SystemTime,
    pub(crate) severity: Severity,
    pub(crate) source: Source,
    pub(crate) text: String,
}

impl Message {
    pub(crate) fn new(source: Source, severity: Severity, text: impl Into<String>) -> Self {
        Self { at: SystemTime::now(), severity, source, text: text.into() }
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

pub(crate) struct MessageStream {
    log: Vec<Message>,
    showing_until: Option<Instant>,
}

impl MessageStream {
    pub(crate) fn new() -> Self {
        Self { log: Vec::new(), showing_until: None }
    }

    pub(crate) fn emit(&mut self, message: Message) {
        self.emit_showing_for(message, DISPLAY_TIME);
    }

    /// Emit with a non-standard display time, for alerts that must outlive the
    /// usual few seconds (or hints that may sit until something else happens).
    pub(crate) fn emit_showing_for(&mut self, message: Message, display: Duration) {
        self.log.push(message);
        self.showing_until = Some(Instant::now() + display);
    }

    /// The message currently on screen and when it leaves, if any.
    pub(crate) fn showing(&self) -> Option<(&Message, Instant)> {
        let until = self.showing_until.filter(|&u| Instant::now() < u)?;
        self.log.last().map(|m| (m, until))
    }

    /// Take the current message off screen; it stays in the log.
    pub(crate) fn dismiss(&mut self) {
        self.showing_until = None;
    }

    /// The whole session's messages, oldest first.
    pub(crate) fn entries(&self) -> &[Message] {
        &self.log
    }
}
