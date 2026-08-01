use color_eyre::Result as EyreResult;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use stratum_dsp::{analyze_audio, AnalysisConfig};

fn default_art_bright_idx() -> u8 { 1 }

pub(crate) fn hash_mono(samples: &[f32]) -> String {
    let bytes = unsafe {
        std::slice::from_raw_parts(samples.as_ptr() as *const u8, samples.len() * 4)
    };
    blake3::Hasher::new().update(bytes).finalize().to_hex().to_string()
}

fn track_data_path() -> PathBuf {
    crate::xdg::data_dir().join("track-data.json")
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
}

type TrackEntries = std::collections::HashMap<String, CacheEntry>;

pub(crate) struct TrackDatabase {
    path: PathBuf,
    entries: TrackEntries,
    dirty_at: Option<std::time::Instant>,
}

impl TrackDatabase {
    pub(crate) fn load() -> Self {
        let path = track_data_path();
        let entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { path, entries, dirty_at: None }
    }

    pub(crate) fn get(&self, hash: &str) -> Option<&CacheEntry> {
        self.entries.get(hash)
    }

    pub(crate) fn set(&mut self, hash: String, entry: CacheEntry) {
        self.entries.insert(hash, entry);
        self.dirty_at = Some(std::time::Instant::now());
    }

    pub(crate) fn entries_snapshot(&self) -> TrackEntries {
        self.entries.clone()
    }

    pub(crate) fn flush_if_idle(&mut self) {
        if idle_elapsed(self.dirty_at) {
            self.save();
            self.dirty_at = None;
        }
    }

    pub(crate) fn save(&self) {
        write_json_atomic(&self.path, &self.entries);
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
    #[serde(default)]
    vinyl_mode: bool,
    #[serde(default = "default_art_bright_idx")]
    art_bright_idx: u8,
}

impl Default for SessionFile {
    fn default() -> Self {
        Self {
            last_browser_path: None,
            browser_workspace: None,
            audio_latency_ms: 0,
            vinyl_mode: false,
            art_bright_idx: default_art_bright_idx(),
        }
    }
}

pub(crate) struct SessionState {
    path: PathBuf,
    last_browser_path: Option<PathBuf>,
    browser_workspace: Option<PathBuf>,
    audio_latency_ms: i64,
    vinyl_mode: bool,
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
            vinyl_mode: file.vinyl_mode,
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

    pub(crate) fn get_vinyl_mode(&self) -> bool {
        self.vinyl_mode
    }

    pub(crate) fn set_vinyl_mode(&mut self, mode: bool) {
        self.vinyl_mode = mode;
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
            vinyl_mode: self.vinyl_mode,
            art_bright_idx: self.art_bright_idx,
        };
        write_json_atomic(&self.path, &file);
    }
}

// ---------------------------------------------------------------------------
// Legacy migration — one bundled cache.json under ~/.config into the two stores
// ---------------------------------------------------------------------------

/// The pre-split `~/.config/deck/cache.json`: per-track entries and session
/// state in one file. Read-only; retired after migration.
#[derive(Deserialize)]
struct LegacyCacheFile {
    #[serde(default)]
    last_browser_path: Option<String>,
    #[serde(default)]
    browser_workspace: Option<String>,
    #[serde(default)]
    audio_latency_ms: i64,
    #[serde(default)]
    vinyl_mode: bool,
    #[serde(default = "default_art_bright_idx")]
    art_bright_idx: u8,
    #[serde(default)]
    entries: TrackEntries,
}

/// Splits an existing bundled `cache.json` into the new data/state files, then
/// deletes it. Skips writing either target if it already exists, so a partial
/// prior migration is never clobbered. No-op once the old file is gone.
pub(crate) fn migrate_legacy_cache() {
    let legacy_path = crate::xdg::config_dir().join("cache.json");
    let Some(legacy) = read_legacy(&legacy_path) else { return };

    if !track_data_path().exists() {
        write_json_atomic(&track_data_path(), &legacy.entries);
    }
    if !session_path().exists() {
        let session = SessionFile {
            last_browser_path: legacy.last_browser_path,
            browser_workspace: legacy.browser_workspace,
            audio_latency_ms: legacy.audio_latency_ms,
            vinyl_mode: legacy.vinyl_mode,
            art_bright_idx: legacy.art_bright_idx,
        };
        write_json_atomic(&session_path(), &session);
    }
    let _ = std::fs::remove_file(&legacy_path);
}

/// Reads the bundled file, tolerating both the wrapped object and the oldest
/// flat `{hash: entry}` map that predated it.
fn read_legacy(path: &Path) -> Option<LegacyCacheFile> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<LegacyCacheFile>(&text)
        .ok()
        .or_else(|| serde_json::from_str::<TrackEntries>(&text)
            .ok()
            .map(|entries| LegacyCacheFile {
                last_browser_path: None,
                browser_workspace: None,
                audio_latency_ms: 0,
                vinyl_mode: false,
                art_bright_idx: default_art_bright_idx(),
                entries,
            }))
}

// ---------------------------------------------------------------------------
// BPM detection
// ---------------------------------------------------------------------------

pub(crate) fn detect_bpm(samples: &[f32], sample_rate: u32) -> EyreResult<f32> {
    let result = analyze_audio(samples, sample_rate, AnalysisConfig::default())
        .map_err(|e| color_eyre::eyre::eyre!("stratum-dsp: {e:?}"))?;
    Ok(result.bpm)
}
