use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn default_art_bright_idx() -> u8 { 1 }

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

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct CacheEntry {
    pub(crate) bpm: f32,
    pub(crate) offset_ms: i64,
    /// Filename at time of first detection — informational only, not used as key.
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) cue_sample: Option<usize>,
    #[serde(default)]
    pub(crate) offset_established: bool,
    #[serde(default)]
    pub(crate) gain_db: i8,
    /// The mode the track last ran in; applied at load.
    #[serde(default)]
    pub(crate) mode: Option<crate::deck::DeckMode>,
    /// The grid's phase datum in mono samples, when one has been pinned.
    #[serde(default)]
    pub(crate) anchor_sample: Option<usize>,
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
        CacheEntry { bpm, offset_ms: 0, name: name.to_string(), cue_sample: None, offset_established: false, gain_db: 0, mode: None, anchor_sample: None }
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

        assert_eq!(db.get("Y").unwrap().bpm, 120.0, "local-only entry kept");
        assert_eq!(db.get("Z").unwrap().bpm, 90.0, "workspace-only entry added");
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
        assert_eq!(read_entries(&path).get("A").unwrap().bpm, 128.0, "roundtrip");

        std::fs::write(&path, serde_json::to_string(&entries).unwrap()).unwrap();
        assert_eq!(read_entries(&path).get("A").unwrap().bpm, 128.0, "legacy fallback");

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
}

impl Default for SessionFile {
    fn default() -> Self {
        Self {
            last_browser_path: None,
            browser_workspace: None,
            audio_latency_ms: 0,
            art_bright_idx: default_art_bright_idx(),
        }
    }
}

pub(crate) struct SessionState {
    path: PathBuf,
    last_browser_path: Option<PathBuf>,
    browser_workspace: Option<PathBuf>,
    audio_latency_ms: i64,
    art_bright_idx: u8,
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
            dirty_at: None,
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

    pub(crate) fn flush_if_idle(&mut self) {
        if idle_elapsed(self.dirty_at) {
            self.save();
            self.dirty_at = None;
        }
    }

    pub(crate) fn save(&self) {
        let file = SessionFile {
            last_browser_path: self.last_browser_path
                .as_ref()
                .and_then(|p| p.to_str().map(str::to_string)),
            browser_workspace: self.browser_workspace
                .as_ref()
                .and_then(|p| p.to_str().map(str::to_string)),
            audio_latency_ms: self.audio_latency_ms,
            art_bright_idx: self.art_bright_idx,
        };
        write_json_atomic(&self.path, &file);
    }
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------

