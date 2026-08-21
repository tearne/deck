use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn default_art_bright_idx() -> u8 { 1 }
fn default_panel_pct() -> u16 { 30 }

/// Filename shared by the canonical `.local` database and its workspace mirror,
/// so relocating one to the other is a plain file copy.
const TRACK_DATA_FILE: &str = "track-data.json";

fn track_data_path() -> PathBuf {
    crate::xdg::data_dir().join(TRACK_DATA_FILE)
}

fn session_path() -> PathBuf {
    crate::xdg::state_dir().join("session.json")
}

/// How long a store must sit untouched before the idle flush writes it out.
/// Keeps serialisation and file IO off key-repeat paths (BPM ramp, gain hold).
const SAVE_IDLE: std::time::Duration = std::time::Duration::from_secs(1);

/// Crash-safe write: serialise to a sibling temp file, then rename over the target.
fn write_json_atomic<T: Serialize>(path: &Path, value: &T) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("tmp");
    if let Ok(text) = serde_json::to_string_pretty(value) {
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

/// True once the store has sat unmutated for `SAVE_IDLE`.
fn idle_elapsed(dirty_at: Option<std::time::Instant>) -> bool {
    dirty_at.map_or(false, |t| t.elapsed() >= SAVE_IDLE)
}

// ---------------------------------------------------------------------------
// Track database — per-track memory keyed by audio hash, in the data dir
// ---------------------------------------------------------------------------

/// A confirmed beat grid: the BPM and phase offset together. Stored only once
/// the operator has established the tempo — a record with no grid is a track
/// that was played but never tapped, and must not reopen at the placeholder.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub(crate) struct Grid {
    pub(crate) bpm: f32,
    pub(crate) offset_ms: i64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(from = "RawCacheEntry")]
pub(crate) struct CacheEntry {
    pub(crate) grid: Option<Grid>,
    /// Filename at time of first detection — informational only, not used as key.
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) cue_sample: Option<usize>,
    #[serde(default)]
    pub(crate) gain_db: i8,
    /// The mode the track last ran in; applied at load.
    #[serde(default)]
    pub(crate) mode: Option<crate::deck::DeckMode>,
    /// The grid's phase datum in mono samples, when one has been pinned. Kept
    /// outside `grid`: an anchor can be pinned before the tempo is confirmed.
    #[serde(default)]
    pub(crate) anchor_sample: Option<usize>,
}

/// The on-disk record as read, accepting both shapes: the current nullable
/// `grid`, and the legacy flat `bpm` / `offset_ms`, which read as confirmed.
#[derive(Deserialize)]
struct RawCacheEntry {
    #[serde(default)]
    grid: Option<Grid>,
    #[serde(default)]
    bpm: Option<f32>,
    #[serde(default)]
    offset_ms: i64,
    name: String,
    #[serde(default)]
    cue_sample: Option<usize>,
    #[serde(default)]
    gain_db: i8,
    #[serde(default)]
    mode: Option<crate::deck::DeckMode>,
    #[serde(default)]
    anchor_sample: Option<usize>,
}

impl From<RawCacheEntry> for CacheEntry {
    fn from(raw: RawCacheEntry) -> Self {
        let legacy_grid = raw.bpm.map(|bpm| Grid { bpm, offset_ms: raw.offset_ms });
        CacheEntry {
            grid: raw.grid.or(legacy_grid),
            name: raw.name,
            cue_sample: raw.cue_sample,
            gain_db: raw.gain_db,
            mode: raw.mode,
            anchor_sample: raw.anchor_sample,
        }
    }
}

/// BTreeMap so both file copies serialise in one deterministic order — a
/// versioned library doesn't churn diffs on every save.
type TrackEntries = std::collections::BTreeMap<String, CacheEntry>;

/// The self-explaining header carried at the top of every `track-data.json`,
/// for the stranger who finds one in a library root.
const TRACK_DATA_ABOUT: &[&str] = &[
    "Deck's per-track memory: BPM, beat-grid offset, cue point, and gain trim,",
    "keyed by content identity — a hash of the audio data with tags excluded,",
    "so entries follow tracks across renames and retags.",
    "Canonical copy: ~/.local/share/deck/track-data.json; when a search",
    "workspace is set, a second copy travels in the library root.",
    "Safe to delete: analysis re-runs on demand, but cues and trims are lost.",
];

/// The on-disk form: header plus entries. Reading falls back to the legacy
/// flat map, so an old file upgrades silently at its next save.
#[derive(Deserialize)]
struct TrackDataFile {
    #[serde(rename = "_about")]
    #[allow(dead_code)] // present in the file; the app has no use for it
    about: Vec<String>,
    tracks: TrackEntries,
}

#[derive(Serialize)]
struct TrackDataFileRef<'a> {
    #[serde(rename = "_about")]
    about: &'a [&'a str],
    tracks: &'a TrackEntries,
}

pub(crate) struct TrackDatabase {
    path: PathBuf,
    /// The workspace copy (`<workspace>/track-data.json`), when a workspace is set.
    /// Written on every save so the database travels with the music.
    mirror_path: Option<PathBuf>,
    entries: TrackEntries,
    dirty_at: Option<std::time::Instant>,
}

impl TrackDatabase {
    pub(crate) fn load() -> Self {
        let path = track_data_path();
        let entries = read_entries(&path);
        Self { path, mirror_path: None, entries, dirty_at: None }
    }

    pub(crate) fn get(&self, hash: &str) -> Option<&CacheEntry> {
        self.entries.get(hash)
    }

    pub(crate) fn set(&mut self, hash: String, entry: CacheEntry) {
        self.entries.insert(hash, entry);
        self.mark_dirty();
    }

    /// Point the workspace mirror at `workspace`'s copy, or clear it.
    pub(crate) fn set_mirror(&mut self, workspace: Option<&Path>) {
        self.mirror_path = workspace
            .map(|w| w.join(TRACK_DATA_FILE))
            .filter(|m| m != &self.path);
    }

    /// Reconcile with the workspace copy at attach time and write both copies now,
    /// so they match immediately rather than at the next save. The carried library
    /// wins on any shared identity (its entries are adopted); the local-only entries
    /// are pushed out by the save. No-op without a mirror.
    pub(crate) fn sync_with_mirror(&mut self) {
        let Some(mirror) = self.mirror_path.clone() else { return };
        for (identity, entry) in read_entries(&mirror) {
            self.entries.insert(identity, entry);
        }
        self.save();
    }

    pub(crate) fn entries_snapshot(&self) -> TrackEntries {
        self.entries.clone()
    }

    fn mark_dirty(&mut self) {
        self.dirty_at = Some(std::time::Instant::now());
    }

    pub(crate) fn flush_if_idle(&mut self) {
        if idle_elapsed(self.dirty_at) {
            self.save();
            self.dirty_at = None;
        }
    }

    pub(crate) fn save(&self) {
        let file = TrackDataFileRef { about: TRACK_DATA_ABOUT, tracks: &self.entries };
        write_json_atomic(&self.path, &file);
        if let Some(mirror) = &self.mirror_path {
            write_json_atomic(mirror, &file);
        }
    }
}

fn read_entries(path: &Path) -> TrackEntries {
    let Ok(text) = std::fs::read_to_string(path) else { return TrackEntries::default() };
    if let Ok(file) = serde_json::from_str::<TrackDataFile>(&text) {
        return file.tracks;
    }
    // Legacy form: the bare entries map, headerless.
    serde_json::from_str(&text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(bpm: f32, name: &str) -> CacheEntry {
        CacheEntry { grid: Some(Grid { bpm, offset_ms: 0 }), name: name.to_string(), cue_sample: None, gain_db: 0, mode: None, anchor_sample: None }
    }

    fn bpm_of(e: &CacheEntry) -> f32 { e.grid.expect("confirmed grid").bpm }

    fn snapshot(path: &str, position_secs: f64) -> DeckSnapshot {
        DeckSnapshot { path: path.into(), position_secs, playlist_path: None, playlist_index: 0, bpm: 128.0, playback_speed: 1.0, volume: 0.8, pitch_semitones: -1, filter_offset: 3, filter_poles: 2, pfl_level: 0 }
    }

    #[test]
    fn deck_snapshots_round_trip_and_position_writes_are_paced() {
        let mut state = SessionState::from_file(PathBuf::from("/nonexistent/session.json"), SessionFile::default());
        state.record_deck(1, Some(snapshot("/music/a.flac", 10.0)), true);
        assert!(state.dirty_at.is_some(), "a new deck writes at once");
        state.dirty_at = None;
        state.record_deck(1, Some(snapshot("/music/a.flac", 12.0)), true);
        assert!(state.dirty_at.is_none(), "playing: small movement within the interval is not written");
        state.record_deck(1, Some(snapshot("/music/a.flac", 40.0)), true);
        assert!(state.dirty_at.is_some(), "playing: a seek-sized jump is written");
        state.dirty_at = None;
        state.record_deck(1, Some(snapshot("/music/a.flac", 40.5)), false);
        assert!(state.dirty_at.is_some(), "paused: any movement is written");
        state.record_selected_deck(1);

        let file = state.to_file();
        let text = serde_json::to_string(&file).unwrap();
        let back: SessionFile = serde_json::from_str(&text).unwrap();
        let restored = SessionState::from_file(PathBuf::from("/nonexistent/session.json"), back);
        assert_eq!(restored.deck_snapshots()[1], Some(snapshot("/music/a.flac", 40.5)));
        assert_eq!(restored.deck_snapshots()[0], None);
        assert_eq!(restored.saved_selected_deck(), 1);
    }

    #[test]
    fn legacy_flat_record_reads_as_confirmed_and_null_grid_round_trips() {
        let legacy = r#"{"A": {"bpm": 128.0, "offset_ms": 310, "name": "a", "offset_established": true, "gain_db": 2}}"#;
        let entries: TrackEntries = serde_json::from_str(legacy).unwrap();
        let a = entries.get("A").unwrap();
        assert_eq!(a.grid, Some(Grid { bpm: 128.0, offset_ms: 310 }), "flat fields become a confirmed grid");
        assert_eq!(a.gain_db, 2);

        let unconfirmed = CacheEntry { grid: None, name: "b".into(), cue_sample: Some(44100), gain_db: -1, mode: None, anchor_sample: None };
        let text = serde_json::to_string(&unconfirmed).unwrap();
        assert!(text.contains("\"grid\":null"), "absence is written explicitly: {text}");
        let back: CacheEntry = serde_json::from_str(&text).unwrap();
        assert_eq!(back.grid, None, "a never-confirmed record stays unconfirmed");
        assert_eq!(back.cue_sample, Some(44100));
    }

    #[test]
    fn workspace_copy_wins_on_import_and_mirror_writes_both() {
        let root = std::env::temp_dir().join(format!("deck-mirror-{}", std::process::id()));
        let workspace = root.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        // SAFETY: single-threaded test setup; no other test reads XDG_DATA_HOME.
        unsafe { std::env::set_var("XDG_DATA_HOME", root.join("data")); }

        // A library carried from elsewhere: its copy has a fresh X and a new Z.
        let mut incoming = TrackEntries::new();
        incoming.insert("X".into(), entry(140.0, "x-from-workspace"));
        incoming.insert("Z".into(), entry(90.0, "z"));
        std::fs::write(workspace.join(TRACK_DATA_FILE), serde_json::to_string(&incoming).unwrap()).unwrap();

        // Local database: a local-only Y and a stale X.
        let mut db = TrackDatabase::load();
        db.set("Y".into(), entry(120.0, "y"));
        db.set("X".into(), entry(100.0, "x-local-stale"));

        db.set_mirror(Some(&workspace));
        db.sync_with_mirror();

        assert_eq!(bpm_of(db.get("Y").unwrap()), 120.0, "local-only entry kept");
        assert_eq!(bpm_of(db.get("Z").unwrap()), 90.0, "workspace-only entry added");
        assert_eq!(db.get("X").unwrap().name, "x-from-workspace", "workspace wins on conflict");

        db.save();
        let local = read_entries(&track_data_path());
        let mirror = read_entries(&workspace.join(TRACK_DATA_FILE));
        assert_eq!(local.len(), 3, "canonical copy holds the union");
        assert_eq!(mirror.len(), 3, "mirror re-written with merged data");
        assert_eq!(mirror.get("X").unwrap().name, "x-from-workspace");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The saved file carries the `_about` header, stays valid JSON, reads
    /// back losslessly — and a legacy headerless file still reads (as the
    /// mirror test above also exercises).
    #[test]
    fn about_header_roundtrip_and_legacy_fallback() {
        let dir = std::env::temp_dir().join(format!("deck-about-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(TRACK_DATA_FILE);

        let mut entries = TrackEntries::new();
        entries.insert("A".into(), entry(128.0, "a"));
        write_json_atomic(&path, &TrackDataFileRef { about: TRACK_DATA_ABOUT, tracks: &entries });

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.trim_start().starts_with("{\n  \"_about\""), "header leads the file");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("stays plain valid JSON");
        assert!(parsed.get("_about").is_some());
        assert_eq!(bpm_of(read_entries(&path).get("A").unwrap()), 128.0, "roundtrip");

        std::fs::write(&path, serde_json::to_string(&entries).unwrap()).unwrap();
        assert_eq!(bpm_of(read_entries(&path).get("A").unwrap()), 128.0, "legacy fallback");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ---------------------------------------------------------------------------
// Session state — global player state, in the state dir
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct SessionFile {
    #[serde(default)]
    last_browser_path: Option<String>,
    #[serde(default)]
    browser_workspace: Option<String>,
    #[serde(default)]
    audio_latency_ms: i64,
    #[serde(default = "default_art_bright_idx")]
    art_bright_idx: u8,
    /// Browser panel width as a percentage of the browser area.
    #[serde(default = "default_panel_pct")]
    browser_panel_pct: u16,
    /// The decks as they last were, one entry per slot; restored on request.
    #[serde(default)]
    decks: Vec<Option<DeckSnapshot>>,
    #[serde(default)]
    selected_deck: usize,
    /// Ghost playheads (jump-key landing labels) shown.
    #[serde(default)]
    ghosts_on: bool,
}

impl Default for SessionFile {
    fn default() -> Self {
        Self {
            last_browser_path: None,
            browser_workspace: None,
            audio_latency_ms: 0,
            art_bright_idx: default_art_bright_idx(),
            browser_panel_pct: default_panel_pct(),
            decks: Vec::new(),
            selected_deck: 0,
            ghosts_on: false,
        }
    }
}

/// One deck as it was: enough to put the same track back at the same place
/// with the same settings. Mode is not here — it is per-track memory in the
/// track database and comes back with the load.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) struct DeckSnapshot {
    pub(crate) path: String,
    pub(crate) position_secs: f64,
    #[serde(default)]
    pub(crate) playlist_path: Option<String>,
    #[serde(default)]
    pub(crate) playlist_index: usize,
    pub(crate) bpm: f32,
    pub(crate) playback_speed: f32,
    pub(crate) volume: f32,
    pub(crate) pitch_semitones: i8,
    pub(crate) filter_offset: i32,
    pub(crate) filter_poles: u8,
    pub(crate) pfl_level: u8,
}

impl DeckSnapshot {
    fn same_apart_from_position(&self, other: &DeckSnapshot) -> bool {
        let mut a = self.clone();
        a.position_secs = other.position_secs;
        a == *other
    }
}

/// How often a playing deck's position is written to the snapshot. A restore
/// lands within this of where the set was; finer would churn the file forever.
const POSITION_WRITE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);
/// A position change larger than playback could produce between two writes
/// is a seek, recorded at once.
const SEEK_THRESHOLD_SECS: f64 = 5.0;

pub(crate) struct SessionState {
    path: PathBuf,
    last_browser_path: Option<PathBuf>,
    browser_workspace: Option<PathBuf>,
    audio_latency_ms: i64,
    art_bright_idx: u8,
    browser_panel_pct: u16,
    decks: [Option<DeckSnapshot>; 3],
    selected_deck: usize,
    ghosts_on: bool,
    position_written_at: [Option<std::time::Instant>; 3],
    dirty_at: Option<std::time::Instant>,
}

impl SessionState {
    pub(crate) fn load() -> Self {
        let path = session_path();
        let file: SessionFile = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self::from_file(path, file)
    }

    fn from_file(path: PathBuf, file: SessionFile) -> Self {
        Self {
            path,
            last_browser_path: file.last_browser_path.map(PathBuf::from),
            browser_workspace: file.browser_workspace
                .map(PathBuf::from)
                .filter(|p| p.is_dir()),
            audio_latency_ms: file.audio_latency_ms,
            art_bright_idx: file.art_bright_idx,
            browser_panel_pct: file.browser_panel_pct.clamp(15, 70),
            decks: std::array::from_fn(|i| file.decks.get(i).cloned().flatten()),
            selected_deck: file.selected_deck.min(2),
            ghosts_on: file.ghosts_on,
            position_written_at: [None; 3],
            dirty_at: None,
        }
    }

    pub(crate) fn deck_snapshots(&self) -> &[Option<DeckSnapshot>; 3] {
        &self.decks
    }

    pub(crate) fn has_deck_snapshot(&self) -> bool {
        self.decks.iter().any(Option::is_some)
    }

    pub(crate) fn saved_selected_deck(&self) -> usize {
        self.selected_deck
    }

    /// Offer the slot's current state. Anything but position is written at
    /// once; position is written on pause and seek, and otherwise every
    /// `POSITION_WRITE_INTERVAL` while playing.
    pub(crate) fn record_deck(&mut self, slot: usize, current: Option<DeckSnapshot>, playing: bool) {
        let now = std::time::Instant::now();
        let changed = match (&self.decks[slot], &current) {
            (None, None) => false,
            (Some(stored), Some(now_snap)) if stored.same_apart_from_position(now_snap) => {
                let moved = (stored.position_secs - now_snap.position_secs).abs();
                if moved == 0.0 { return; }
                let interval_due = self.position_written_at[slot]
                    .map_or(true, |t| now.duration_since(t) >= POSITION_WRITE_INTERVAL);
                !playing || moved > SEEK_THRESHOLD_SECS || interval_due
            }
            _ => true,
        };
        if changed {
            self.decks[slot] = current;
            self.position_written_at[slot] = Some(now);
            self.mark_dirty();
        }
    }

    pub(crate) fn ghosts_on(&self) -> bool {
        self.ghosts_on
    }

    pub(crate) fn set_ghosts_on(&mut self, on: bool) {
        self.ghosts_on = on;
        self.mark_dirty();
    }

    pub(crate) fn record_selected_deck(&mut self, slot: usize) {
        if self.selected_deck != slot {
            self.selected_deck = slot;
            self.mark_dirty();
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty_at = Some(std::time::Instant::now());
    }

    pub(crate) fn last_browser_path(&self) -> Option<&Path> {
        self.last_browser_path.as_deref()
    }

    pub(crate) fn set_last_browser_path(&mut self, p: &Path) {
        self.last_browser_path = Some(p.to_path_buf());
        self.mark_dirty();
    }

    pub(crate) fn workspace(&self) -> Option<&Path> {
        self.browser_workspace.as_deref()
    }

    pub(crate) fn set_workspace(&mut self, p: &Path) {
        self.browser_workspace = Some(p.to_path_buf());
        self.mark_dirty();
    }

    pub(crate) fn clear_workspace(&mut self) {
        self.browser_workspace = None;
        self.mark_dirty();
    }

    pub(crate) fn get_latency(&self) -> i64 {
        self.audio_latency_ms
    }

    pub(crate) fn set_latency(&mut self, ms: i64) {
        self.audio_latency_ms = ms;
        self.mark_dirty();
    }


    pub(crate) fn get_art_bright_idx(&self) -> u8 {
        self.art_bright_idx
    }

    pub(crate) fn set_art_bright_idx(&mut self, state: u8) {
        self.art_bright_idx = state;
        self.mark_dirty();
    }

    pub(crate) fn get_panel_pct(&self) -> u16 {
        self.browser_panel_pct
    }

    /// Step the panel width by `delta` percentage points, clamped 15–70.
    pub(crate) fn step_panel_pct(&mut self, delta: i16) {
        self.browser_panel_pct = (self.browser_panel_pct as i16 + delta).clamp(15, 70) as u16;
        self.mark_dirty();
    }

    pub(crate) fn flush_if_idle(&mut self) {
        if idle_elapsed(self.dirty_at) {
            self.save();
            self.dirty_at = None;
        }
    }

    fn to_file(&self) -> SessionFile {
        SessionFile {
            last_browser_path: self.last_browser_path
                .as_ref()
                .and_then(|p| p.to_str().map(str::to_string)),
            browser_workspace: self.browser_workspace
                .as_ref()
                .and_then(|p| p.to_str().map(str::to_string)),
            audio_latency_ms: self.audio_latency_ms,
            art_bright_idx: self.art_bright_idx,
            browser_panel_pct: self.browser_panel_pct,
            decks: self.decks.to_vec(),
            selected_deck: self.selected_deck,
            ghosts_on: self.ghosts_on,
        }
    }

    pub(crate) fn save(&self) {
        let file = self.to_file();
        write_json_atomic(&self.path, &file);
    }
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------

