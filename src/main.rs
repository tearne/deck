use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;

use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    DisableMouseCapture, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement,
    EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use rodio::stream::DeviceSinkBuilder;
use rodio::Player;

mod audio;
mod browser;
mod cache;
mod config;
mod deck;
mod error_reports;
mod frame_stats;
mod library;
mod playlist;
mod render;
mod tags;
mod xdg;

use audio::{decode_audio, scrub_audio, play_click_tone, FilterSource, PitchSource, PreviewOutput, TrackingSource, WaveformData, SeekHandle, FADE_SAMPLES};
use browser::{BrowserMode, BrowserResult, BrowserState, EntryKind, handle_browser_key, render_browser};
use cache::{SessionState, TrackDatabase, detect_bpm};
use config::{load_config, snap_to_fps_level, Action, FPS_LEVELS, KeyBinding};
use deck::do_time_jump;
use deck::{
    anchor_beat_grid_to_cue, apply_offset_step, cache_entry_for_deck, compute_spectrum,
    compute_tap_bpm_offset, ActivePlaylist, Deck, DeckAudio, NudgeMode, Notification, NotificationStyle,
    NOTIFICATION_TIMEOUT, PALETTE_SCHEMES, TagEditorState, TAG_FIELD_LABELS,
};
use library::WorkspaceLibrary;
use render::{
    extract_tick_viewport, halfblock_art, info_line_empty, DEFAULT_ZOOM_IDX,
    info_line_for_deck, notification_line_empty, notification_line_for_deck,
    overview_empty, refresh_overview_for_deck, render_detail_empty, render_detail_waveform, render_loop_panels,
    render_keyboard_help, render_shared_tick_row,
    render_tag_editor, SharedDetailRenderer, ZOOM_LEVELS,
};
use tags::{propose_rename_stem, read_cover_art, read_tags_for_editor, read_track_name};

fn cleanup_terminal() {
    let _ = disable_raw_mode();
    let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
}

fn panic_log_path() -> std::path::PathBuf {
    xdg::state_dir().join("panic.log")
}

/// A minimal terminal DJ player.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// File or directory to open
    path: Option<PathBuf>,

    /// Resolve config from the current directory instead of ~/.config/deck
    #[arg(long)]
    local_config: bool,

    /// Record per-frame timing statistics to frame-stats.csv in the current directory
    #[arg(long)]
    frame_stats: bool,
}

fn main() {
    color_eyre::install().expect("color_eyre initialisation should succeed at startup");

    // Chain a file-writing hook around color_eyre's hook so panics are preserved
    // even when the terminal is in raw mode and stderr is invisible.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let log_path = panic_log_path();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");
        let msg = format!(
            "[{timestamp}] thread '{thread_name}' {info}\n",
        );
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&log_path, &msg);
        cleanup_terminal();
        prev_hook(info);
    }));

    let cli = Cli::parse();

    let use_local_config = cli.local_config;

    let arg = cli.path;
    let start = arg.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

    // Set up terminal once — shared by browser and player.
    let metering = cli.frame_stats;
    let setup = (|| -> io::Result<Terminal<CrosstermBackend<frame_stats::MeteredStdout>>> {
        enable_raw_mode()?;
        io::stdout()
            .execute(EnterAlternateScreen)?
            .execute(EnableMouseCapture)?
            // DISAMBIGUATE_ESCAPE_CODES is load-bearing, not cosmetic. Without it Esc keeps
            // its legacy bare-`\x1b` encoding, which cannot express an event type — so the
            // release REPORT_EVENT_TYPES asks for arrives as a second, identical Press and
            // every Esc tap acts twice.
            .execute(PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
            ))?;
        Terminal::new(CrosstermBackend::new(frame_stats::MeteredStdout::new(metering)))
    })();
    let mut terminal = match setup {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Terminal error: {e}");
            std::process::exit(1);
        }
    };

    // Warp nudge ends on key release, which only terminals speaking the kitty
    // keyboard protocol can report — without it a warp would latch on forever.
    let release_events_supported = supports_keyboard_enhancement().unwrap_or(false);

    // Load persisted state early so we can read last_browser_path before the browser opens.
    let mut track_data = TrackDatabase::load();
    let mut session = SessionState::load();

    // If a workspace is already attached, adopt the database copy that travels with it.
    if let Some(workspace) = session.workspace() {
        track_data.set_mirror(Some(workspace));
        track_data.sync_with_mirror();
    }

    // Compute the initial browser directory:
    //   CLI dir arg  → that directory (overrides last-visited for this first open only)
    //   CLI file arg → the file's parent directory
    //   no arg       → last visited path from cache (if it still exists), else CWD
    let mut browser_dir: std::path::PathBuf = if arg.as_deref().map(|p| p.is_dir()).unwrap_or(false) {
        start.clone()
    } else if start.is_file() {
        start.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| start.clone())
    } else {
        session.last_browser_path()
            .filter(|p| p.exists())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
    };

    let handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(h) => h,
        Err(e) => {
            cleanup_terminal();
            eprintln!("Audio output error: {e}");
            std::process::exit(1);
        }
    };
    let mixer = handle.mixer();

    let initial_load: Option<PendingLoad> = if start.is_file() {
        Some(start_load(&start))
    } else {
        None
    };
    let mut recorder = if cli.frame_stats {
        match frame_stats::Recorder::create() {
            Ok(r) => Some(r),
            Err(e) => {
                cleanup_terminal();
                eprintln!("Could not create {}: {e}", frame_stats::CAPTURE_FILENAME);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    if let Err(e) = tui_loop(&mut terminal, initial_load, &mut track_data, &mut session, &mut browser_dir, &mixer, use_local_config, &mut recorder, release_events_supported) {
        cleanup_terminal();
        eprintln!("TUI error: {e}");
        std::process::exit(1);
    }

    cleanup_terminal();
}

struct PendingLoad {
    filename: String,
    path:     PathBuf,
    rx:       mpsc::Receiver<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>,
    decoded:  Arc<AtomicUsize>,
    total:    Arc<AtomicUsize>,
    /// When the load came from a playlist, the playlist state to attach to the
    /// deck once it finishes building.
    attach_playlist: Option<ActivePlaylist>,
}

fn start_load(path: &Path) -> PendingLoad {
    let path_str = path.to_string_lossy().to_string();
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path_str)
        .to_string();
    let decoded = Arc::new(AtomicUsize::new(0));
    let total   = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel::<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>();
    {
        let decoded_for_thread = Arc::clone(&decoded);
        let total_for_thread   = Arc::clone(&total);
        thread::spawn(move || {
            let _ = tx.send(decode_audio(&path_str, decoded_for_thread, total_for_thread).map_err(|e| e.to_string()));
        });
    }
    PendingLoad { filename, path: path.to_path_buf(), rx, decoded, total, attach_playlist: None }
}

/// A browser selection awaiting load onto a deck — a standalone track or a playlist.
enum BrowserLoad {
    Track(std::path::PathBuf),
    Playlist(std::path::PathBuf),
}

/// A playlist shown in the context panel: its entries, a cursor, and cached
/// availability (recomputed on structural change, not every frame). Used for the
/// read-only preview/browse views and as the working buffer in edit mode.
/// Per-entry resolution status, from `resolve`. `NeedsConfirmation` entries can be
/// repaired via the candidate picker; only `Found` entries are playable.
#[derive(Clone, Copy, PartialEq)]
enum EntryStatus { Found, NeedsConfirmation, Unavailable }

#[derive(Clone)]
struct PlaylistPanel {
    path: PathBuf,
    playlist: playlist::Playlist,
    cursor: usize,
    status: Vec<EntryStatus>,
}

impl PlaylistPanel {
    fn open(path: PathBuf, workspace: Option<&Path>) -> Self {
        let playlist = playlist::read_playlist(&path).map(|(p, _)| p).unwrap_or_else(|_| playlist::Playlist::empty());
        let mut p = Self { path, playlist, cursor: 0, status: Vec::new() };
        // Opening for view heals in place: relocated hints and refreshed tags persist.
        p.recompute_status(workspace, true);
        p
    }

    /// Recompute per-entry status. Applies `resolve`'s healing (relocated hints, tag
    /// refresh) to the in-memory entries; when `persist` and anything changed, rewrites
    /// the `.rpl`. `persist` is false during transactional edit so the buffer isn't written.
    fn recompute_status(&mut self, workspace: Option<&Path>, persist: bool) {
        let dir = self.path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let library = WorkspaceLibrary::new(workspace);
        let mut changed = false;
        self.status = self.playlist.entries.iter_mut()
            .map(|e| match playlist::resolve(e, &dir, &library) {
                playlist::Resolution::Found { updated_entry, .. } => {
                    if let Some(updated) = updated_entry { *e = updated; changed = true; }
                    EntryStatus::Found
                }
                playlist::Resolution::NeedsConfirmation { .. } => EntryStatus::NeedsConfirmation,
                playlist::Resolution::Unavailable => EntryStatus::Unavailable,
            })
            .collect();
        if changed && persist {
            let _ = playlist::write_playlist(&self.path, &self.playlist);
        }
    }

    fn status_at(&self, i: usize) -> EntryStatus {
        self.status.get(i).copied().unwrap_or(EntryStatus::Unavailable)
    }

    fn cursor_up(&mut self) { self.cursor = self.cursor.saturating_sub(1); }
    fn cursor_down(&mut self) {
        if self.cursor + 1 < self.playlist.entries.len() { self.cursor += 1; }
    }

    fn remove_at_cursor(&mut self) {
        if self.cursor < self.playlist.entries.len() {
            self.playlist.entries.remove(self.cursor);
            self.status.remove(self.cursor);
            self.cursor = self.cursor.min(self.playlist.entries.len().saturating_sub(1));
        }
    }

    fn move_up(&mut self) {
        if self.cursor > 0 {
            self.playlist.entries.swap(self.cursor, self.cursor - 1);
            self.status.swap(self.cursor, self.cursor - 1);
            self.cursor -= 1;
        }
    }
    fn move_down(&mut self) {
        if self.cursor + 1 < self.playlist.entries.len() {
            self.playlist.entries.swap(self.cursor, self.cursor + 1);
            self.status.swap(self.cursor, self.cursor + 1);
            self.cursor += 1;
        }
    }

    /// Insert `entries` at position `at` (edit mode), leaving the cursor on the first.
    /// Inserted entries are marked unavailable until the next `recompute_status`.
    fn insert_at(&mut self, at: usize, entries: Vec<playlist::Entry>) {
        let at = at.min(self.playlist.entries.len());
        for (offset, entry) in entries.into_iter().enumerate() {
            self.playlist.entries.insert(at + offset, entry);
            self.status.insert(at + offset, EntryStatus::Unavailable);
        }
        self.cursor = at;
    }

    /// The entry auto-advance would pick next: first `Found` strictly after `current`
    /// (the playing index), or the first `Found` when nothing plays yet.
    fn next_up(&self, current: Option<usize>) -> Option<usize> {
        let start = current.map(|c| c + 1).unwrap_or(0);
        (start..self.playlist.entries.len()).find(|&i| self.status_at(i) == EntryStatus::Found)
    }
}

/// The permanent context panel. `Preview` mirrors the browser highlight (read-only);
/// `Browse` locks onto a playlist read-only with the panel focused (Enter plays an
/// entry); `Edit` is a transactional buffer, written only on commit.
enum Panel {
    Preview(Preview),
    Browse(PlaylistPanel),
    /// Transactional edit — a working buffer written only on commit, so aborting
    /// just drops it. `focus` toggles between picking tracks in the browser and
    /// reordering in the playlist.
    Edit { panel: PlaylistPanel, focus: EditFocus },
    /// Descriptive-fallback candidate picker for entry `entry` of `panel`. `cursor` is a
    /// line-scroll offset; the active candidate is the one at the top of the view. `layout`
    /// is filled by the renderer (variable-height cards) for the input's line-scroll to read.
    Confirm { panel: PlaylistPanel, entry: usize, candidates: Vec<playlist::Candidate>, cursor: usize, layout: Rc<RefCell<ConfirmLayout>> },
}

/// The picker's variable-height card layout, published by the renderer for the input's
/// line-scroll to read: each card's start line, plus the total line count.
#[derive(Default)]
struct ConfirmLayout {
    card_starts: Vec<usize>,
    total_lines: usize,
}

/// The active candidate at line-scroll `offset`: the topmost card whose header line is still
/// visible — the first whose start is at or below the top, so once a card's header scrolls off
/// the top the next takes over.
fn confirm_active_card(offset: usize, card_starts: &[usize]) -> usize {
    card_starts.partition_point(|&start| start < offset).min(card_starts.len().saturating_sub(1))
}

#[derive(Clone, Copy, PartialEq)]
enum EditFocus { Browser, Playlist }

enum Preview {
    Empty,
    Track { fields: [String; 7] },
    Playlist(PlaylistPanel),
}

impl Panel {
    /// True when the browser should be dimmed — the playlist list is the active target
    /// (Browse, or Edit with the playlist focused). In Edit-Browser you're picking tracks,
    /// so the browser stays bright.
    fn dim_browser(&self) -> bool {
        matches!(self, Panel::Browse(_) | Panel::Confirm { .. } | Panel::Edit { focus: EditFocus::Playlist, .. })
    }

    /// The playlist the panel is showing/editing, if any.
    fn playlist_mut(&mut self) -> Option<&mut PlaylistPanel> {
        match self {
            Panel::Preview(Preview::Playlist(pp)) | Panel::Browse(pp)
            | Panel::Edit { panel: pp, .. } | Panel::Confirm { panel: pp, .. } => Some(pp),
            _ => None,
        }
    }
}

/// Write `panel`'s playlist and adopt it on any deck that has it loaded, keeping
/// the playing entry pointed at the same track by identity so audio isn't interrupted.
fn commit_playlist(panel: &PlaylistPanel, decks: &mut [Option<Deck>; 3]) {
    let _ = playlist::write_playlist(&panel.path, &panel.playlist);
    for deck in decks.iter_mut().flatten() {
        let Some(active) = deck.playlist.as_mut() else { continue };
        if active.path != panel.path { continue; }
        let playing_hash = active.playlist.entries.get(active.index).map(|e| e.identity.content_hash.clone());
        active.playlist = panel.playlist.clone();
        active.index = playing_hash
            .and_then(|h| active.playlist.entries.iter().position(|e| e.identity.content_hash == h))
            .unwrap_or_else(|| active.index.min(active.playlist.entries.len().saturating_sub(1)));
    }
}

/// Clear the target deck and start loading the selection. Returns a notification
/// on failure (e.g. an unreadable or empty playlist).
fn apply_browser_load(
    load: BrowserLoad,
    deck: usize,
    decks: &mut [Option<Deck>; 3],
    pending_loads: &mut [Option<PendingLoad>; 3],
    workspace: Option<&Path>,
) -> Option<Notification> {
    if let Some(ref d) = decks[deck] { d.audio.player.stop(); }
    decks[deck] = None;
    match load {
        BrowserLoad::Track(path) => { pending_loads[deck] = Some(start_load(&path)); None }
        BrowserLoad::Playlist(path) => open_playlist_on_deck(&path, deck, workspace, pending_loads),
    }
}

/// Open a `.rpl` on `deck`: read it, load the first resolvable entry, and attach
/// the playlist so auto-advance and the position indicator follow.
fn open_playlist_on_deck(
    rpl_path: &Path,
    deck: usize,
    workspace: Option<&Path>,
    pending_loads: &mut [Option<PendingLoad>; 3],
) -> Option<Notification> {
    let mut playlist = match playlist::read_playlist(rpl_path) {
        Ok((playlist, _migrated)) => playlist,
        Err(e) => return Some(notification(format!("Playlist read failed: {e}"), NotificationStyle::Error)),
    };
    let Some((index, track_path)) = resolve_and_heal(&mut playlist, rpl_path, workspace, 0) else {
        return Some(notification("No playable tracks in playlist", NotificationStyle::Warning));
    };
    let nudge = (workspace.is_none() && playlist_has_missing(&playlist, rpl_path))
        .then(|| notification("Some tracks are missing — set a workspace (@) to relocate moved files", NotificationStyle::Warning));
    let mut load = start_load(&track_path);
    load.attach_playlist = Some(ActivePlaylist { playlist, path: rpl_path.to_path_buf(), index, advance_requested: false });
    pending_loads[deck] = Some(load);
    nudge
}

/// Create an empty `.rpl` named `name` in `dir`. Errs if a file already exists.
fn create_playlist_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let stem = tags::sanitise_for_filename(name);
    let path = dir.join(format!("{stem}.rpl"));
    if path.exists() {
        return Err(format!("{stem}.rpl already exists"));
    }
    playlist::write_playlist(&path, &playlist::Playlist::empty()).map_err(|e| e.to_string())?;
    Ok(path)
}

/// The content/tag facts the engine needs to build an entry for a track.
fn track_facts(path: &Path) -> Option<playlist::TrackFacts> {
    let bytes = std::fs::read(path).ok()?;
    let file_size_bytes = bytes.len() as u64;
    let duration_secs = audio::probe_duration_secs(path)?;
    let [artist, title, album, year, ..] = tags::read_tags_for_editor(path);
    Some(playlist::TrackFacts {
        bytes, duration_secs, file_size_bytes,
        description: playlist::Description { artist, title, album, year },
    })
}

/// Build a playlist entry for `track`, relative to `playlist_dir`.
fn build_entry_for(track: &Path, playlist_dir: &Path) -> Result<playlist::Entry, String> {
    let facts = track_facts(track).ok_or_else(|| "couldn't read track".to_string())?;
    playlist::entry_from_track(track, playlist_dir, &facts).map_err(|e| format!("{e:?}"))
}

/// Entries to splice in at the edit cursor: one for an audio file, or all of another
/// playlist's entries (identity-based, so they resolve even if the hint is stale).
fn gather_insert_entries(path: &Path, kind: EntryKind, playlist_dir: &Path) -> Result<Vec<playlist::Entry>, String> {
    match kind {
        EntryKind::Audio => Ok(vec![build_entry_for(path, playlist_dir)?]),
        EntryKind::Playlist => playlist::read_playlist(path).map(|(pl, _)| pl.entries).map_err(|e| e.to_string()),
        _ => Ok(Vec::new()),
    }
}

/// Load entry `index` of the panel's playlist onto `deck`, attaching the playlist at
/// that index. Returns a notification when the entry can't be resolved.
fn play_panel_entry(
    pp: &PlaylistPanel,
    index: usize,
    deck: usize,
    decks: &mut [Option<Deck>; 3],
    pending_loads: &mut [Option<PendingLoad>; 3],
    workspace: Option<&Path>,
) -> Option<Notification> {
    let Some(entry) = pp.playlist.entries.get(index) else { return None };
    let dir = pp.path.parent().unwrap_or_else(|| Path::new("."));
    let library = WorkspaceLibrary::new(workspace);
    let playlist::Resolution::Found { path, .. } = playlist::resolve(entry, dir, &library) else {
        return Some(notification("That entry is unavailable", NotificationStyle::Warning));
    };
    if let Some(ref d) = decks[deck] { d.audio.player.stop(); }
    decks[deck] = None;
    let mut load = start_load(&path);
    load.attach_playlist = Some(ActivePlaylist { playlist: pp.playlist.clone(), path: pp.path.clone(), index, advance_requested: false });
    pending_loads[deck] = Some(load);
    None
}

/// Re-resolve every entry against the (now-available) library, adopting relocated
/// hints. Rewrites the `.rpl` and returns true if anything changed.
fn heal_playlist(playlist: &mut playlist::Playlist, rpl_path: &Path, workspace: Option<&Path>) -> bool {
    let dir = rpl_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let library = WorkspaceLibrary::new(workspace);
    let mut changed = false;
    for entry in &mut playlist.entries {
        if let playlist::Resolution::Found { updated_entry: Some(updated), .. } =
            playlist::resolve(entry, &dir, &library)
        {
            *entry = updated;
            changed = true;
        }
    }
    if changed {
        let _ = playlist::write_playlist(rpl_path, playlist);
    }
    changed
}

/// Skip the selected deck's playlist to the next (`forward`) or previous resolvable
/// entry and load it. No-op without an active playlist or a resolvable entry that way.
fn play_playlist_step(
    slot: usize,
    forward: bool,
    decks: &mut [Option<Deck>; 3],
    pending_loads: &mut [Option<PendingLoad>; 3],
    workspace: Option<&Path>,
) {
    let Some(active) = decks[slot].as_ref().and_then(|d| d.playlist.as_ref()) else { return };
    let dir = active.path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let library = WorkspaceLibrary::new(workspace);
    let current = active.index;
    let order: Vec<usize> = if forward {
        ((current + 1)..active.playlist.entries.len()).collect()
    } else {
        (0..current).rev().collect()
    };
    let found = order.into_iter().find_map(|i| match playlist::resolve(&active.playlist.entries[i], &dir, &library) {
        playlist::Resolution::Found { path, .. } => Some((i, path)),
        _ => None,
    });
    let Some((index, track_path)) = found else { return };
    let playlist = active.playlist.clone();
    let rpl_path = active.path.clone();
    if let Some(ref d) = decks[slot] { d.audio.player.stop(); }
    decks[slot] = None;
    let mut load = start_load(&track_path);
    load.attach_playlist = Some(ActivePlaylist { playlist, path: rpl_path, index, advance_requested: false });
    pending_loads[slot] = Some(load);
}

/// True when the playlist has an entry whose hinted file is missing — used to nudge
/// the operator to set a workspace so moved tracks can be relocated.
fn playlist_has_missing(playlist: &playlist::Playlist, rpl_path: &Path) -> bool {
    let dir = rpl_path.parent().unwrap_or_else(|| Path::new("."));
    playlist.entries.iter().any(|e| !dir.join(&e.hints.relative_path).exists())
}

/// Resolve entries from `start`, returning the first that locates a file (and its
/// index), persisting any relocated hints back to the `.rpl`. `None` if none resolve.
fn resolve_and_heal(
    playlist: &mut playlist::Playlist,
    rpl_path: &Path,
    workspace: Option<&Path>,
    start: usize,
) -> Option<(usize, PathBuf)> {
    let dir = rpl_path.parent().unwrap_or_else(|| Path::new("."));
    let library = WorkspaceLibrary::new(workspace);
    for index in start..playlist.entries.len() {
        if let playlist::Resolution::Found { path, updated_entry } =
            playlist::resolve(&playlist.entries[index], dir, &library)
        {
            if let Some(entry) = updated_entry {
                playlist.entries[index] = entry;
                let _ = playlist::write_playlist(rpl_path, playlist);
            }
            return Some((index, path));
        }
    }
    None
}

fn build_deck(
    path:            &Path,
    filename:        String,
    mono:            Vec<f32>,
    stereo:          Vec<f32>,
    sample_rate:     u32,
    channels:        u16,
    mixer:           &rodio::mixer::Mixer,
    track_data:      &TrackDatabase,
    pfl_active_deck: Arc<AtomicUsize>,
    deck_slot:       usize,
) -> Deck {
    use std::sync::atomic::{AtomicBool, AtomicI8, AtomicI32, AtomicI64, AtomicU8, AtomicU32};
    let track_name  = read_track_name(&path.to_string_lossy());
    let rename_hint = {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if stem.is_empty() {
            None
        } else {
            let proposed = propose_rename_stem(path);
            if proposed == stem { None } else { Some(proposed) }
        }
    };
    let total_duration = mono.len() as f64 / sample_rate as f64;
    let mono           = Arc::new(mono);
    let waveform       = Arc::new(WaveformData::compute(Arc::clone(&mono), sample_rate));

    let samples         = Arc::new(stereo);
    let position        = Arc::new(AtomicUsize::new(0));
    let output_position = Arc::new(AtomicUsize::new(0));
    let fade_remaining  = Arc::new(AtomicI64::new(0));
    let fade_len        = Arc::new(AtomicI64::new(FADE_SAMPLES));
    let pending_target  = Arc::new(AtomicUsize::new(usize::MAX));
    let flush_pitch     = Arc::new(AtomicBool::new(false));
    let seek_handle = SeekHandle {
        samples: Arc::clone(&samples),
        position: Arc::clone(&position),
        output_position: Arc::clone(&output_position),
        fade_remaining: Arc::clone(&fade_remaining),
        fade_len: Arc::clone(&fade_len),
        pending_target: Arc::clone(&pending_target),
        sample_rate,
        channels,
        flush_pitch: Arc::clone(&flush_pitch),
    };

    let filter_offset_shared = Arc::new(AtomicI32::new(0));
    let filter_state_reset   = Arc::new(AtomicBool::new(false));
    let pfl_level            = Arc::new(AtomicU8::new(0));
    let deck_volume_atomic   = Arc::new(AtomicU32::new(1.0f32.to_bits()));
    let gain_linear          = Arc::new(AtomicU32::new(1.0f32.to_bits()));
    let filter_poles         = Arc::new(AtomicU8::new(2));
    let pitch_semitones      = Arc::new(AtomicI8::new(0));
    let loop_active          = Arc::new(AtomicBool::new(false));
    let loop_start           = Arc::new(AtomicUsize::new(0));
    let loop_end             = Arc::new(AtomicUsize::new(0));
    let player = Player::connect_new(mixer);
    player.append(PitchSource::new(
        FilterSource::new(
            TrackingSource::new(
                samples, position, fade_remaining, fade_len, pending_target, sample_rate, channels,
                Arc::clone(&loop_active), Arc::clone(&loop_start), Arc::clone(&loop_end),
                Arc::clone(&flush_pitch), Arc::clone(&output_position),
            ),
            Arc::clone(&filter_offset_shared),
            Arc::clone(&filter_state_reset),
            Arc::clone(&pfl_level),
            pfl_active_deck,
            deck_slot,
            Arc::clone(&deck_volume_atomic),
            Arc::clone(&gain_linear),
            Arc::clone(&filter_poles),
        ),
        Arc::clone(&pitch_semitones),
        Arc::clone(&flush_pitch),
        Arc::clone(&output_position),
    ));
    player.pause();

    let (bpm_tx, bpm_rx) = mpsc::channel::<(String, f32, i64, bool)>();
    {
        let entries = track_data.entries_snapshot();
        let identity_path = path.to_path_buf();
        thread::spawn(move || {
            // Key on content identity (audio payload, tags excluded) — the same identity
            // playlists and the tag editor use. It is mandatory: a track that can't be
            // hashed is unsupported app-wide, so record the fault and signal it with an
            // empty hash (the sentinel the receiver treats as unhashable) rather than
            // inventing a masquerading key.
            let hash = match content_identity(&identity_path) {
                Ok(hash) => hash,
                Err(error) => {
                    record_identity_failure(&identity_path, &error);
                    let _ = bpm_tx.send((String::new(), 120.0, 0, true));
                    return;
                }
            };
            // is_fresh=false → applied immediately and marks bpm_established=true (confirmed).
            // is_fresh=true  → applied immediately only when bpm_established is false, leaves it false (unconfirmed).
            let (bpm, offset_ms, is_fresh) = if let Some(entry) = entries.get(&hash) {
                let snapped = (entry.offset_ms as f64 / 10.0).round() as i64 * 10;
                let period  = (60_000.0 / entry.bpm as f64 / 10.0).round() as i64 * 10;
                let snapped = snapped.rem_euclid(period);
                (entry.bpm, snapped, false)
            } else {
                // No cache entry: use 120 as a placeholder; leave bpm_established false so the UI
                // signals that the BPM has not been confirmed.
                (120.0f32, 0i64, true)
            };
            let _ = bpm_tx.send((hash, bpm, offset_ms, is_fresh));
        });
    }

    let mut deck = Deck::new(
        filename,
        path.to_path_buf(),
        track_name,
        total_duration,
        rename_hint,
        DeckAudio {
            player,
            seek_handle,
            mono,
            waveform,
            sample_rate,
            filter_offset_shared,
            filter_state_reset,
            filter_poles,
            pfl_level,
            deck_volume_atomic,
            gain_linear,
            pitch_semitones,
            loop_active,
            loop_start,
            loop_end,
        },
        bpm_rx,
    );
    deck.cover_art = read_cover_art(path);
    deck
}

/// Move `source` into `dest_dir`. Audio already decoded in memory is unaffected.
/// Returns the outcome notification and, on success, the file's new path.
fn move_file_to_directory(source: &Path, dest_dir: &Path) -> (Notification, Option<PathBuf>) {
    let Some(filename) = source.file_name() else {
        return (notification("File has no name", NotificationStyle::Error), None);
    };
    let destination = dest_dir.join(filename);
    if destination == source {
        return (notification("Already in that directory", NotificationStyle::Warning), None);
    }
    if destination.exists() {
        return (
            notification(format!("A file named {} already exists there", filename.to_string_lossy()), NotificationStyle::Error),
            None,
        );
    }
    match std::fs::rename(source, &destination) {
        Ok(()) => (notification(format!("Moved to {}", dest_dir.display()), NotificationStyle::Success), Some(destination)),
        // EXDEV: rename can't cross filesystems, and a copy-then-delete fallback
        // would need progress and interruption handling out of proportion here.
        Err(e) if e.raw_os_error() == Some(18) => (notification("Can't move across filesystems", NotificationStyle::Error), None),
        Err(e) => (notification(format!("Move failed: {e}"), NotificationStyle::Error), None),
    }
}

/// After a file is renamed or moved, update any deck loaded from the old path so
/// it follows the file. `new_track_name` refreshes the display name on a retag.
fn sync_deck_path(decks: &mut [Option<Deck>; 3], old_path: &Path, new_path: &Path, new_track_name: Option<&str>) {
    for slot in 0..3 {
        if let Some(ref mut d) = decks[slot] {
            if d.path == old_path {
                d.path = new_path.to_path_buf();
                d.filename = new_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                if let Some(name) = new_track_name { d.track_name = name.to_string(); }
                d.rename_hint = None;
                d.rename_offer_started = None;
            }
        }
    }
}

fn notification(message: impl Into<String>, style: NotificationStyle) -> Notification {
    Notification { message: message.into(), style, expires: Instant::now() + NOTIFICATION_TIMEOUT }
}

/// The file's content-identity hash, or the reason it can't be computed.
fn content_identity(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    resilient_playlists::content_hash(&bytes).map_err(|e| format!("hash failed: {e:?}"))
}

/// The file's content-identity hash, or `None` if it can't be read or hashed
/// (an unverifiable identity — a different concern from a changed one).
fn identity_of(path: &Path) -> Option<String> {
    content_identity(path).ok()
}

/// Record a load-time identity failure as a harmonised error report. The track
/// is unsupported app-wide (no playlist can reference it), so this is surfaced,
/// not swallowed. Called off the main thread.
fn record_identity_failure(path: &Path, error: &str) {
    let base = error_reports::dir();
    let _ = std::fs::create_dir_all(&base);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("track");
    let report = base.join(format!("{}.txt", error_reports::stamped_name("identity-unhashable", stem)));
    let body = format!(
        "Content identity could not be computed for this track.\n\n\
         file:  {}\n\
         error: {error}\n\n\
         The track loads and plays, but its analysis and edits are not saved, and it\n\
         cannot be referenced by playlists (which key on content identity).\n",
        path.display(),
    );
    let _ = std::fs::write(report, body);
}

/// Write tags to `path`, verifying the content identity is unchanged — tag edits
/// must never alter the audio payload. Returns `Ok(None)` when the identity holds
/// (or can't be verified), `Ok(Some(dir))` when it changed (a fault: the original
/// and edited files plus details are preserved in the returned incident folder),
/// or `Err` if the write itself failed.
fn write_tags_verified(path: &Path, fields: &[(String, usize)]) -> Result<Option<PathBuf>, String> {
    let base = error_reports::dir();
    let _ = std::fs::create_dir_all(&base);
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("track");
    let staged = base.join(format!(".staging-{filename}"));
    let staged_ok = std::fs::copy(path, &staged).is_ok();

    let before = identity_of(path);
    crate::tags::write_tags(path, fields)?; // propagates a write failure unchanged
    let mut after = identity_of(path);
    // Fault injection for exercising the safety net without touching the file: when
    // this env var is set, force the identity comparison to fail so the alert and
    // incident-folder path runs on a real (uncorrupted) edit.
    if std::env::var_os("DECK_SIMULATE_IDENTITY_FAULT").is_some() {
        after = Some("simulated-identity-fault".to_string());
    }

    let mismatch = matches!((&before, &after), (Some(b), Some(a)) if b != a);
    if !mismatch || !staged_ok {
        let _ = std::fs::remove_file(&staged);
        return Ok(None);
    }

    // Assemble a self-contained incident folder.
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("track");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("bin");
    let incident = base.join(error_reports::stamped_name("identity-mismatch", stem));
    std::fs::create_dir_all(&incident).map_err(|e| e.to_string())?;
    let original = incident.join(format!("original.{ext}"));
    let edited = incident.join(format!("edited.{ext}"));
    let _ = std::fs::rename(&staged, &original);
    let _ = std::fs::copy(path, &edited);
    let _ = std::fs::write(incident.join("details.txt"), identity_incident_details(path, ts, &before, &after, &original, &edited));
    Ok(Some(incident))
}

/// Human-readable details for an identity-mismatch incident, including the audio
/// payload byte ranges each file reports (the two files sit alongside for diffing).
fn identity_incident_details(source: &Path, ts: u64, before: &Option<String>, after: &Option<String>, original: &Path, edited: &Path) -> String {
    let ranges = |p: &Path| -> String {
        match std::fs::read(p) {
            Ok(b) => match resilient_playlists::detect_format(&b).map(|f| resilient_playlists::payload_ranges(&b, f)) {
                Some(Ok(r)) => format!("{r:?}"),
                _ => "unknown".to_string(),
            },
            Err(_) => "unreadable".to_string(),
        }
    };
    format!(
        "Identity mismatch on tag edit — the audio payload changed, which must never happen.\n\
         This most likely indicates a byte-range extraction bug. The original and edited\n\
         files are preserved here for analysis (diff them to find the differing bytes).\n\n\
         time (unix seconds): {ts}\n\
         source path:         {}\n\
         hash before:         {}\n\
         hash after:          {}\n\
         payload range (original): {}\n\
         payload range (edited):   {}\n",
        source.display(),
        before.as_deref().unwrap_or("<unverifiable>"),
        after.as_deref().unwrap_or("<unverifiable>"),
        ranges(original),
        ranges(edited),
    )
}

/// A background tag-compliance scan of one directory, streaming `(path, non_compliant)`
/// results and cancellable when the operator navigates away.
struct ComplianceScan {
    dir: PathBuf,
    cancel: Arc<AtomicBool>,
    rx: mpsc::Receiver<(PathBuf, bool)>,
}

/// A file is non-compliant when its stem differs from its tag-derived name — the
/// same check the load-time rename offer uses. Opens and probes the file for tags.
fn is_non_compliant(path: &Path) -> bool {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    !stem.is_empty() && propose_rename_stem(path) != stem
}

fn spawn_compliance_scan(dir: PathBuf, paths: Vec<PathBuf>) -> ComplianceScan {
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let cancel_bg = Arc::clone(&cancel);
    thread::spawn(move || {
        for path in paths {
            if cancel_bg.load(Ordering::Relaxed) { break; }
            let flagged = is_non_compliant(&path);
            if tx.send((path, flagged)).is_err() { break; }
        }
    });
    ComplianceScan { dir, cancel, rx }
}

/// Each frame while the browser is open: drain scan results into the cache, mark
/// entries from it, and keep a scan running for the current directory's unscanned
/// files (cancelling and restarting on navigation). No-op work when off.
fn drive_compliance_scan(
    bs: &mut BrowserState,
    cache: &mut HashMap<PathBuf, bool>,
    scan: &mut Option<ComplianceScan>,
) {
    if !bs.compliance_on {
        if let Some(s) = scan.take() { s.cancel.store(true, Ordering::Relaxed); }
        for e in &mut bs.entries { e.compliance = None; }
        return;
    }
    if let Some(s) = scan.as_ref() {
        while let Ok((path, flagged)) = s.rx.try_recv() { cache.insert(path, flagged); }
    }
    for e in &mut bs.entries {
        if e.kind == EntryKind::Audio {
            e.compliance = cache.get(&e.path).copied();
        }
    }
    let uncached: Vec<PathBuf> = bs.entries.iter()
        .filter(|e| e.kind == EntryKind::Audio && !cache.contains_key(&e.path))
        .map(|e| e.path.clone())
        .collect();
    let scan_covers_cwd = scan.as_ref().map_or(false, |s| s.dir == bs.cwd);
    if !uncached.is_empty() && !scan_covers_cwd {
        if let Some(s) = scan.take() { s.cancel.store(true, Ordering::Relaxed); }
        *scan = Some(spawn_compliance_scan(bs.cwd.clone(), uncached));
    } else if uncached.is_empty() && scan_covers_cwd {
        *scan = None; // directory fully scanned
    }
}

/// The least-disruptive deck to load into: an empty deck first, then a
/// loaded-but-not-playing one, falling back to `selected` when all are playing.
fn default_target_deck(decks: &[Option<Deck>; 3], selected: usize) -> usize {
    (0..3).find(|&i| decks[i].is_none())
        .or_else(|| (0..3).find(|&i| decks[i].as_ref().is_some_and(|d| d.audio.player.is_paused())))
        .unwrap_or(selected)
}

/// One key for the standalone tag editor. Saving writes tags (and renames on a
/// stem change) to the editor's own file, syncs any deck loaded from it, refreshes
/// the browser if open, and reports through the global notification.
fn handle_tag_editor_key(
    tag_editor: &mut Option<TagEditorState>,
    decks: &mut [Option<Deck>; 3],
    browser_state: &mut Option<BrowserState>,
    global_notification: &mut Option<Notification>,
    compliance_cache: &mut HashMap<PathBuf, bool>,
    key: crossterm::event::KeyEvent,
) -> bool {
    match key.code {
        KeyCode::Esc => { *tag_editor = None; false }
        KeyCode::Enter => {
            enum Job { Skip, Save { old: PathBuf, target: PathBuf, needs_rename: bool, fields: Vec<(String, usize)>, name: String, stem: String } }
            let job = {
                let editor = tag_editor.as_mut().unwrap();
                if editor.fields[0].0.trim().is_empty() || editor.fields[1].0.trim().is_empty() {
                    Job::Skip
                } else {
                    for (val, cursor) in &mut editor.fields {
                        let trimmed = val.trim().to_string();
                        *cursor = (*cursor).min(trimmed.chars().count());
                        *val = trimmed;
                    }
                    let stem = editor.preview();
                    let needs_rename = stem != editor.current_stem;
                    let old = editor.current_path();
                    let target = editor.target_path();
                    if needs_rename && target.exists() {
                        let fname = target.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                        editor.collision_error = Some(format!("already exists: {fname}"));
                        Job::Skip
                    } else {
                        editor.collision_error = None;
                        let fields = editor.fields.clone();
                        let name = format!("{} \u{2013} {}", fields[1].0, fields[0].0);
                        Job::Save { old, target, needs_rename, fields, name, stem }
                    }
                }
            };
            if let Job::Save { old, target, needs_rename, fields, name, stem } = job {
                *tag_editor = None;
                // The edit changes tags (and maybe the name), so its compliance must
                // be recomputed — drop both possible paths from the cache.
                compliance_cache.remove(&old);
                compliance_cache.remove(&target);
                match write_tags_verified(&old, &fields) {
                    Err(e) => {
                        *global_notification = Some(notification(format!("tag write failed: {e}"), NotificationStyle::Error));
                        false
                    }
                    Ok(incident) => {
                        let saved = if needs_rename {
                            match std::fs::rename(&old, &target) {
                                Err(e) => {
                                    *global_notification = Some(notification(format!("rename failed: {e}"), NotificationStyle::Error));
                                    false
                                }
                                Ok(()) => {
                                    sync_deck_path(decks, &old, &target, Some(&name));
                                    if let Some(bs) = browser_state.as_mut() { let _ = bs.refresh(); }
                                    *global_notification = Some(notification(format!("\u{2192} {stem}"), NotificationStyle::Success));
                                    true
                                }
                            }
                        } else {
                            sync_deck_path(decks, &old, &old, Some(&name));
                            *global_notification = Some(notification("tags saved", NotificationStyle::Info));
                            true
                        };
                        // A changed identity is a critical alert. Show it in the
                        // browser header (near the eyes) with a long timeout when the
                        // browser is open, else fall back to the global notification.
                        if let Some(dir) = incident {
                            let msg = format!("⚠ IDENTITY CHANGED by tag edit — files preserved in {}", dir.display());
                            let expiry = Instant::now() + Duration::from_secs(30);
                            if let Some(bs) = browser_state.as_mut() {
                                bs.alert = Some((msg, expiry));
                            } else {
                                *global_notification = Some(Notification { message: msg, style: NotificationStyle::Error, expires: expiry });
                            }
                        }
                        saved
                    }
                }
            } else {
                false
            }
        }
        _ => {
            let editor = tag_editor.as_mut().unwrap();
            match key.code {
                KeyCode::Tab | KeyCode::Down => { editor.active_field = (editor.active_field + 1) % TAG_FIELD_LABELS.len(); }
                KeyCode::BackTab | KeyCode::Up => { editor.active_field = (editor.active_field + TAG_FIELD_LABELS.len() - 1) % TAG_FIELD_LABELS.len(); }
                KeyCode::Left => { let (_, cursor) = editor.active_field_mut(); if *cursor > 0 { *cursor -= 1; } }
                KeyCode::Right => { let (text, cursor) = editor.active_field_mut(); let len = text.chars().count(); if *cursor < len { *cursor += 1; } }
                KeyCode::Home => { let (_, cursor) = editor.active_field_mut(); *cursor = 0; }
                KeyCode::End => { let (text, cursor) = editor.active_field_mut(); *cursor = text.chars().count(); }
                KeyCode::Backspace => {
                    let (text, cursor) = editor.active_field_mut();
                    if *cursor > 0 {
                        let mut chars: Vec<char> = text.chars().collect();
                        chars.remove(*cursor - 1);
                        *text = chars.into_iter().collect();
                        *cursor -= 1;
                    }
                }
                KeyCode::Delete => {
                    let (text, cursor) = editor.active_field_mut();
                    let mut chars: Vec<char> = text.chars().collect();
                    if *cursor < chars.len() {
                        chars.remove(*cursor);
                        *text = chars.into_iter().collect();
                    }
                }
                KeyCode::Char(c) => {
                    let (text, cursor) = editor.active_field_mut();
                    let mut chars: Vec<char> = text.chars().collect();
                    chars.insert(*cursor, c);
                    *text = chars.into_iter().collect();
                    *cursor += 1;
                }
                _ => {}
            }
            false
        }
    }
}

fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<frame_stats::MeteredStdout>>,
    initial_load: Option<PendingLoad>,
    track_data: &mut TrackDatabase,
    session: &mut SessionState,
    browser_dir: &mut std::path::PathBuf,
    mixer: &rodio::mixer::Mixer,
    use_local_config: bool,
    recorder: &mut Option<frame_stats::Recorder>,
    release_events_supported: bool,
) -> io::Result<()> {
    // Per-deck display values computed each frame from current deck state.
    struct DeckRenderState {
        display_samp:     f64,
        display_pos_samp: usize,
        analysing:        bool,
        spinner_active:   bool,
        beat_on:          bool,
        warning_active:   bool,
        warn_beat_on:     bool,
    }
    let (keymap, display_cfg, config_notice) = load_config(use_local_config);
    let mut target_fps: u32 = display_cfg.target_fps;
    let mut global_notification: Option<Notification> = None;
    if let Some(msg) = config_notice {
        global_notification = Some(Notification {
            message: msg,
            style: NotificationStyle::Success,
            expires: Instant::now() + NOTIFICATION_TIMEOUT,
        });
    }
    let mut decks: [Option<Deck>; 3] = [None, None, None];
    let mut pending_loads: [Option<PendingLoad>; 3] = [initial_load, None, None];
    if pending_loads[0].is_none() && global_notification.is_none() {
        global_notification = Some(Notification {
            message: "No track loaded — press z to open the file browser".to_string(),
            style: NotificationStyle::Info,
            expires: Instant::now() + Duration::from_secs(60),
        });
    }
    const DET_MIN: u16 = 3;
    let mut audio_latency_ms: i64 = ((session.get_latency() as f64 / 10.0).round() as i64 * 10).clamp(0, 250);
    let mut scheme_idx: usize = 0;
    let mut art_bright_idx: u8 = session.get_art_bright_idx();
    let mut zoom_idx: usize = DEFAULT_ZOOM_IDX;
    let mut vinyl_mode: bool = session.get_vinyl_mode();

    let shared_renderer = SharedDetailRenderer::new(zoom_idx);
    let mut detail_height: usize = display_cfg.detail_height.max(DET_MIN as usize);
    let mut frame_count: usize = 0;
    let mut last_render = Instant::now();
    let mut fps_sample_start = Instant::now();
    let mut fps_sample_frames: u32 = 0;
    let mut fps_display: (u32, u32, u32) = (0, 0, target_fps); // (current, budget, cap)
    let mut help_open = false;
    // The tag editor is a standalone overlay (may sit over the browser or the
    // player), not attached to a deck.
    let mut tag_editor: Option<TagEditorState> = None;
    let mut browser_state: Option<BrowserState> = None;
    // The permanent context panel and the browser path its preview last reflected.
    let mut panel: Panel = Panel::Preview(Preview::Empty);
    let mut panel_source: Option<PathBuf> = None;
    // Rotation index for the `` ` `` jump through loaded-track locations (0 = the
    // opening directory); reset when the browser opens.
    let mut location_cycle: usize = 0;
    // A pending load awaiting confirmation because its target deck is playing.
    let mut browser_load_confirm: Option<(BrowserLoad, usize)> = None;
    // Tag-compliance scan: a per-session cache keyed by path, and the current
    // background scan (if any).
    let mut compliance_cache: HashMap<PathBuf, bool> = HashMap::new();
    let mut compliance_scan: Option<ComplianceScan> = None;
    // Cleanup-mode auto-advance. `edit_resume_anchor` is the entry just below the
    // one being edited (captured at edit-open, stable across a rename); on a save
    // it becomes `cleanup_advance_to`, applied a frame later once the listing and
    // markers are current, moving the cursor to the next flagged entry from there.
    let mut edit_resume_anchor: Option<PathBuf> = None;
    let mut cleanup_advance_to: Option<PathBuf> = None;
    // The browser's primary mode persists across open/close within a session, so
    // reopening restores Command or Search as last used.
    let mut last_browser_mode = BrowserMode::Command;
    let mut preview_output: Option<PreviewOutput> = None;
    let mut max_det_h: usize = usize::MAX;
    let pfl_active_deck = Arc::new(AtomicUsize::new(usize::MAX));
    let mut selected_deck: usize = 0;
    let mut space_held = false;
    // After a chord fires, suppress further Space-Press events until at least one frame
    // passes with no Space activity. Crossterm decodes Kitty key-repeats as Press events,
    // so without this guard the repeat stream re-arms space_held immediately after the
    // post-chord reset, leaving the modifier stuck until the repeats stop — which never
    // happens via a Release event (those also don't arrive in crossterm 0.29 + Kitty).
    let mut space_repeat_suppressed = false;
    let mut space_saw_event_this_frame = false;
    let mut pending_quit: Option<Instant> = None;
    let mut bpm_ramp_started: Option<Instant> = None;
    let mut bpm_ramp_last: Option<Instant> = None;

    'tui: loop {
        frame_count += 1;

        // Clear the repeat-suppression latch once a full frame passes with no Space events,
        // indicating the key has been physically released.
        if space_repeat_suppressed && !space_saw_event_this_frame {
            space_repeat_suppressed = false;
        }
        space_saw_event_this_frame = false;

        // Frame timing — computed once and shared by both decks.
        let dc = shared_renderer.cols.load(Ordering::Relaxed);
        let zoom_secs = ZOOM_LEVELS[zoom_idx];
        let col_secs = if dc > 0 { zoom_secs as f64 / dc as f64 } else { 0.033 };

        // Frame budget: one half-column of scroll time, clamped to a sane range.
        // Sleep is deferred to the END of the loop so variable draw/write time is absorbed
        // automatically — the sleep shrinks to compensate for a slow terminal flush.
        // When the tag editor is open, bypass the waveform-derived budget entirely so text
        // navigation and input are never throttled by zoom level.
        let tag_editor_open = tag_editor.is_some();
        let frame_dur = if tag_editor_open {
            Duration::from_millis(16)
        } else {
            let floor = Duration::from_secs_f64(1.0 / target_fps as f64);
            Duration::from_secs_f64(col_secs / 2.0)
                .max(floor)
                .min(Duration::from_millis(50))
        };

        let frame_start = Instant::now();
        fps_sample_frames += 1;
        let window_secs = frame_start.duration_since(fps_sample_start).as_secs_f64();
        if window_secs >= 1.0 {
            fps_display = (
                (fps_sample_frames as f64 / window_secs).round() as u32,
                (1.0 / frame_dur.as_secs_f64()).round() as u32,
                target_fps,
            );
            fps_sample_start = frame_start;
            fps_sample_frames = 0;
        }
        let elapsed_uncapped = frame_start.duration_since(last_render).as_secs_f64();
        // The cap (4 columns per frame) bounds the scrub jump after a stall on the paused-warp
        // path. The playing path integrates the uncapped interval — the cap can sit below
        // frame_dur at low target_fps and narrow zoom, which would systematically lose time.
        let elapsed = elapsed_uncapped.min(col_secs * 4.0);
        last_render = frame_start;

        // Expire global notification.
        if global_notification.as_ref().map_or(false, |n| frame_start >= n.expires) {
            global_notification = None;
        }
        track_data.flush_if_idle();
        session.flush_if_idle();
        // Complete any pending loads.
        for slot in 0..3 {
            if pending_loads[slot].is_none() { continue; }
            let recv = pending_loads[slot].as_ref().unwrap().rx.try_recv();
            match recv {
                Ok(Ok((mono, stereo, sample_rate, channels))) => {
                    let pending = pending_loads[slot].take().unwrap();
                    let mut new_deck = build_deck(&pending.path, pending.filename, mono, stereo, sample_rate, channels, mixer, &track_data, Arc::clone(&pfl_active_deck), slot);
                    new_deck.playlist = pending.attach_playlist;
                    shared_renderer.set_deck(slot, Arc::clone(&new_deck.audio.waveform), new_deck.audio.seek_handle.channels, new_deck.audio.sample_rate);
                    decks[slot] = Some(new_deck);
                    if let Some(ref mut d) = decks[slot] {
                        d.display.palette = if slot == 0 { PALETTE_SCHEMES[scheme_idx].1 } else { PALETTE_SCHEMES[scheme_idx].2 };
                    }
                }
                Ok(Err(e)) => {
                    global_notification = Some(Notification {
                        message: format!("Load failed: {e}"),
                        style: NotificationStyle::Error,
                        expires: Instant::now() + NOTIFICATION_TIMEOUT,
                    });
                    pending_loads[slot] = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => { pending_loads[slot] = None; }
            }
        }

        // Service all three decks: BPM results, position, metronome, tap timeout, spectrum.
        let service_start = Instant::now();
        for slot in 0..3 {
            service_deck_frame(slot, &mut decks, col_secs, elapsed, elapsed_uncapped, mixer, &shared_renderer, track_data, audio_latency_ms, vinyl_mode);
        }
        let service_dur = service_start.elapsed();

        // Auto-advance: a playlist deck signalled end-of-track. Load its next
        // resolvable entry (lazily resolved and healed just before it plays).
        for slot in 0..3 {
            let requested = decks[slot].as_ref().and_then(|d| d.playlist.as_ref())
                .map_or(false, |pl| pl.advance_requested);
            if !requested { continue; }
            let Some(mut active) = decks[slot].as_mut().and_then(|d| d.playlist.take()) else { continue };
            let workspace = session.workspace().map(|p| p.to_path_buf());
            if let Some((index, track_path)) =
                resolve_and_heal(&mut active.playlist, &active.path, workspace.as_deref(), active.index + 1)
            {
                let mut load = start_load(&track_path);
                load.attach_playlist = Some(ActivePlaylist { playlist: active.playlist, path: active.path, index, advance_requested: false });
                if let Some(ref d) = decks[slot] { d.audio.player.stop(); }
                decks[slot] = None;
                pending_loads[slot] = Some(load);
            }
        }

        // Compute render state for all three decks.
        let render: [Option<DeckRenderState>; 3] = std::array::from_fn(|slot| {
            let d = decks[slot].as_ref()?;
            // Latency correction only applies during playback — when paused there is
            // no buffer fill ahead, so the raw position is the heard position.
            let latency_correction = if d.audio.player.is_paused() { 0.0 } else { audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0 };
            let display_samp = (d.display.smooth_display_samp - latency_correction).max(0.0);
            let display_pos_samp = display_samp as usize;
            let pos_interleaved  = display_pos_samp * d.audio.seek_handle.channels as usize;
            match slot {
                0 => shared_renderer.display_pos_a.store(pos_interleaved, Ordering::Relaxed),
                1 => shared_renderer.display_pos_b.store(pos_interleaved, Ordering::Relaxed),
                _ => shared_renderer.display_pos_c.store(pos_interleaved, Ordering::Relaxed),
            }
            let spinner_active = !d.tempo.analysis_settled || d.tempo.redetecting;
            let analysing      = vinyl_mode || spinner_active || !d.tempo.bpm_established;
            let beat_period    = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
            let flash_window   = beat_period.mul_f64(0.15);
            let smooth_pos_ns  = (display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
                - d.tempo.offset_ms as i128 * 1_000_000;
            let phase          = smooth_pos_ns.rem_euclid(beat_period.as_nanos() as i128);
            let beat_on        = phase < flash_window.as_nanos() as i128;
            let audio_pos_samp = d.audio.seek_handle.position.load(Ordering::Relaxed)
                / d.audio.seek_handle.channels as usize;
            let pos_dur        = Duration::from_secs_f64(audio_pos_samp as f64 / d.audio.sample_rate as f64);
            let remaining_secs = d.total_duration - pos_dur.as_secs_f64();
            let warning_active = !d.audio.player.is_paused()
                && remaining_secs < display_cfg.warning_threshold_secs as f64;
            let beat_index     = smooth_pos_ns.div_euclid(beat_period.as_nanos() as i128);
            let warn_beat_on   = warning_active && (beat_index % 2 == 0);
            Some(DeckRenderState { display_samp, display_pos_samp, analysing, spinner_active, beat_on, warning_active, warn_beat_on })
        });

        shared_renderer.zoom_at.store(zoom_idx, Ordering::Relaxed);
        let buf_a = Arc::clone(&*shared_renderer.shared_a.lock().unwrap());
        let buf_b = Arc::clone(&*shared_renderer.shared_b.lock().unwrap());
        let buf_c = Arc::clone(&*shared_renderer.shared_c.lock().unwrap());
        let scrub_spc_a = buf_a.samples_per_col;
        let scrub_spc_b = buf_b.samples_per_col;
        let scrub_spc_c = buf_c.samples_per_col;

        // Take all three decks out so the draw closure can mutate them.
        let mut d0 = decks[0].take();
        let mut d1 = decks[1].take();
        let mut d2 = decks[2].take();

        // Compute loading labels for slots that have a pending load but no deck.
        let loading_label: [Option<String>; 3] = std::array::from_fn(|slot| {
            let p = pending_loads[slot].as_ref()?;
            let done  = p.decoded.load(Ordering::Relaxed);
            let total = p.total.load(Ordering::Relaxed);
            let pct   = if total > 0 { format!(" {}%", (done * 100 / total).min(100)) } else { String::new() };
            Some(format!("Loading {}…{}", p.filename, pct))
        });

        // While the browser is open, player commands are intercepted and the load
        // target (not the selected deck) is what matters, so the selected-deck
        // highlight is suspended to avoid reading as the target.
        let browser_open = browser_state.is_some();
        // Drive the tag-compliance scan (drains results, marks entries, keeps a
        // scan running for the current directory) before rendering.
        if let Some(bs) = browser_state.as_mut() {
            drive_compliance_scan(bs, &mut compliance_cache, &mut compliance_scan);
            // After an edit in cleanup mode, resume from the neighbour that was below
            // the fixed file (anchored by path, so a re-sort doesn't offset it): land
            // on it if it's flagged, otherwise the next flagged below it.
            if let Some(anchor) = cleanup_advance_to.take() {
                if let Some(i) = bs.entries.iter().position(|e| e.path == anchor) {
                    bs.cursor = i;
                    if bs.entries[i].compliance != Some(true) {
                        bs.jump_flagged(true);
                    }
                } else {
                    bs.jump_flagged(true);
                }
            }
        }
        // Context-panel preview: while the panel isn't focused (Browse/Edit), it mirrors
        // the browser highlight — a track's metadata or a playlist's contents — recomputed
        // only when the highlighted path changes so resolution stays off the hot path.
        if matches!(panel, Panel::Preview(_)) {
            let highlighted = browser_state.as_ref().and_then(|bs| bs.highlighted_entry());
            let source = highlighted.as_ref().map(|(p, _)| p.clone());
            if source != panel_source {
                panel_source = source;
                let ws = session.workspace().map(|p| p.to_path_buf());
                panel = Panel::Preview(match highlighted {
                    Some((path, EntryKind::Audio)) => Preview::Track { fields: read_tags_for_editor(&path) },
                    Some((path, EntryKind::Playlist)) => Preview::Playlist(PlaylistPanel::open(path, ws.as_deref())),
                    _ => Preview::Empty,
                });
            }
        }
        if browser_state.is_none() {
            panel = Panel::Preview(Preview::Empty);
            panel_source = None;
        }

        let draw_start = Instant::now();
        terminal.draw(|frame| {
            let area = frame.area();
            // Reserve a one-column gutter on the left for the active-deck accent bar;
            // everything else lays out in the content area to its right.
            let gutter_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(area);
            let gutter = gutter_split[0];
            let inner = gutter_split[1];

            // Compression order as the terminal shrinks:
            //   1. Detail waveforms compress evenly: detail_height → DET_MIN
            //   2. Overview waveforms compress evenly: OV_MAX → OV_MIN
            //   3. No further compression — elements fall off the bottom
            //
            // Row heights are pre-computed and sum exactly to inner.height so the
            // cassowary solver never receives an infeasible system and proportionally
            // shrinks things it shouldn't.
            const OV_MAX:  u16 = 3;
            const OV_MIN:  u16 = 2;
            let det_max = detail_height as u16;
            let ih = inner.height;
            let fixed = 10_u16; // global + detail-info + shared-tick×2 + notif×3 + info×3

            // Cap detail_height to what the current terminal can actually display,
            // so HeightIncrease never outruns the screen.
            max_det_h = (ih.saturating_sub(fixed + OV_MIN * 3) / 3) as usize;

            // Compute a unified pool for each waveform type so all three decks always
            // get the same height (no sequential-allocation asymmetry).
            // Phase 1: detail compresses; overviews stay at OV_MAX.
            // Phase 2: overviews compress; detail stays at DET_MIN.
            // Phase 3: items fall off bottom (heights stay at minimums).
            let total_variable = ih.saturating_sub(fixed);
            let det_full = det_max * 3;
            let ov_full  = OV_MAX * 3;

            let (all_det, all_ov) = if total_variable >= det_full + ov_full {
                (det_full, ov_full)
            } else if total_variable >= DET_MIN * 3 + ov_full {
                (total_variable - ov_full, ov_full)
            } else if total_variable >= DET_MIN * 3 + OV_MIN * 3 {
                (DET_MIN * 3, total_variable - DET_MIN * 3)
            } else {
                let d = total_variable.min(DET_MIN * 3);
                (d, total_variable.saturating_sub(d))
            };

            // Clamp to minimums: the pool calculation drives compression through
            // the normal phase range; below minimum, take_h handles falloff.
            let effective_det_h = (all_det / 3).max(DET_MIN).min(det_max);
            let effective_ov_h  = (all_ov  / 3).clamp(OV_MIN, OV_MAX);

            // Allocate rows top-to-bottom using take_exact for all waveform rows:
            // each waveform shows at its computed height or disappears entirely.
            // This prevents partial heights below the minimum (e.g. a 3-row
            // detail area where the tick rows leave only 1 waveform row).
            let mut rem = ih;
            // take: allocate up to n rows (partial ok — used for 1-row fixed items).
            // take_consume: show at full height or not at all, but always consume
            //   up to n rows so freed space cannot cause lower items to reappear.
            let take         = |rem: &mut u16, n: u16| -> u16 { let h = (*rem).min(n); *rem -= h; h };
            let take_consume = |rem: &mut u16, n: u16| -> u16 {
                let actual = if *rem >= n { n } else { 0 };
                *rem = rem.saturating_sub(n);
                actual
            };
            let hh = [
                take(&mut rem, 1),                       // 0:  global bar
                take(&mut rem, 1),                       // 1:  detail info bar
                take_consume(&mut rem, effective_det_h),  // 2:  detail A
                take(&mut rem, 1),                       // 3:  shared tick row A/B
                take_consume(&mut rem, effective_det_h),  // 4:  detail B
                take(&mut rem, 1),                       // 5:  shared tick row B/C
                take_consume(&mut rem, effective_det_h),  // 6:  detail C
                take(&mut rem, 1),                       // 7:  notif A
                take(&mut rem, 1),                       // 8:  info A
                take_consume(&mut rem, effective_ov_h),   // 9:  overview A
                take(&mut rem, 1),                       // 10: notif B
                take(&mut rem, 1),                       // 11: info B
                take_consume(&mut rem, effective_ov_h),   // 12: overview B
                take(&mut rem, 1),                       // 13: notif C
                take(&mut rem, 1),                       // 14: info C
                take_consume(&mut rem, effective_ov_h),   // 15: overview C
                rem,                                     // 16: spacer (leftover)
            ];

            let c = Layout::default()
                .direction(Direction::Vertical)
                .constraints(hh.map(Constraint::Length))
                .split(inner);
            let (area_detail_info, area_detail_a, area_tick_ab,
                 area_detail_b, area_tick_bc, area_detail_c,
                 area_notif_a, area_info_a, area_overview_a,
                 area_notif_b, area_info_b, area_overview_b,
                 area_notif_c, area_info_c, area_overview_c,
                 area_global) = (c[1], c[2], c[3], c[4], c[5], c[6],
                                 c[7], c[8], c[9], c[10], c[11], c[12],
                                 c[13], c[14], c[15], c[0]);

            // Update renderer dimensions from layout.
            {
                let w = area_detail_a.width as usize;
                let h = area_detail_a.height as usize;
                if w > 0 && h > 0 {
                    shared_renderer.cols.store(w, Ordering::Relaxed);
                    shared_renderer.rows.store(h, Ordering::Relaxed);
                }
            }

            // Active-deck accent bar in the reserved gutter: one segment beside the
            // deck's detail waveform, one beside its header/info/overview strip.
            // Suspended while the browser is open.
            if !browser_open {
                let detail_area   = [area_detail_a, area_detail_b, area_detail_c][selected_deck];
                let notif_area    = [area_notif_a, area_notif_b, area_notif_c][selected_deck];
                let info_area     = [area_info_a, area_info_b, area_info_c][selected_deck];
                let overview_area = [area_overview_a, area_overview_b, area_overview_c][selected_deck];
                let strip_top    = notif_area.y;
                let strip_bottom = (notif_area.y + notif_area.height)
                    .max(info_area.y + info_area.height)
                    .max(overview_area.y + overview_area.height);
                let bar_style = Style::default().fg(Color::Yellow);
                let mut draw_bar = |y: u16, h: u16| {
                    if h == 0 { return; }
                    let rect = ratatui::layout::Rect { x: gutter.x, y, width: gutter.width, height: h };
                    let lines: Vec<Line> = (0..h).map(|_| Line::from(Span::styled("┃", bar_style))).collect();
                    frame.render_widget(Paragraph::new(lines), rect);
                };
                draw_bar(detail_area.y, detail_area.height);
                draw_bar(strip_top, strip_bottom.saturating_sub(strip_top));
            }

            // Update tempo and cue state for background buffer rendering.
            // In vinyl mode: suppress ticks (analysing=true); the cue column stays visible.
            for (slot, deck) in [(0usize, d0.as_ref()), (1, d1.as_ref()), (2, d2.as_ref())] {
                let (base_bpm, offset_ms, analysing, cue_sample) = deck.map(|d| {
                    let analysing = vinyl_mode || !d.tempo.analysis_settled || d.tempo.redetecting || !d.tempo.bpm_established;
                    (d.tempo.base_bpm, d.tempo.offset_ms, analysing, d.cue_sample)
                }).unwrap_or((0.0, 0, true, None));
                shared_renderer.store_tempo(slot, base_bpm, offset_ms, analysing);
                shared_renderer.store_cue(slot, cue_sample);
            }

            // Detail info bar
            {
                let nudge_label = match d0.as_ref().or(d1.as_ref()).or(d2.as_ref()).map(|d| d.nudge_mode) {
                    Some(NudgeMode::Warp) => "  [WARP]",
                    _ => "  [JUMP]",
                };
                let vinyl_label = if vinyl_mode { "  [VINYL]" } else { "  [BEAT]" };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        format!("  zoom:{}s  lat:{}ms  fps:{}/{}/{}{}{}",
                            zoom_secs, audio_latency_ms,
                            fps_display.0, fps_display.1, fps_display.2,
                            nudge_label, vinyl_label),
                        Style::default().fg(Color::DarkGray),
                    ))),
                    area_detail_info,
                );
            }

            let label_style = Style::default().fg(Color::Rgb(40, 60, 100));
            let notif_bg    = Style::default().bg(Color::Rgb(20, 20, 38));

            // Extract tick viewport slices for both shared tick rows.
            let tick_w = area_tick_ab.width as usize;
            let tick_centre = ((tick_w as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
                .clamp(0, tick_w.saturating_sub(1));
            let pos_a = render[0].as_ref().map(|rs| rs.display_pos_samp).unwrap_or(0);
            let pos_b = render[1].as_ref().map(|rs| rs.display_pos_samp).unwrap_or(0);
            let pos_c = render[2].as_ref().map(|rs| rs.display_pos_samp).unwrap_or(0);
            let tick_a = extract_tick_viewport(&buf_a, pos_a, tick_centre, tick_w);
            let tick_b = extract_tick_viewport(&buf_b, pos_b, tick_centre, tick_w);
            let tick_c = extract_tick_viewport(&buf_c, pos_c, tick_centre, tick_w);
            render_shared_tick_row(frame, area_tick_ab, &tick_a, &tick_b);
            render_shared_tick_row(frame, area_tick_bc, &tick_b, &tick_c);

            // ---- Deck 1 ----
            if let (Some(deck), Some(rs)) = (&mut d0, &render[0]) {
                let content = notification_line_for_deck(deck, area_notif_a.width.saturating_sub(2) as usize, vinyl_mode);
                let num1_style = if selected_deck == 0 && !browser_open { Style::default().fg(Color::Yellow) } else { label_style };
                let mut spans = vec![Span::styled("1", num1_style), Span::styled(" ", label_style)];
                spans.extend(content.spans);
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_a);
                let info = info_line_for_deck(deck, frame_count, rs.beat_on, rs.spinner_active, label_style, area_info_a.width, vinyl_mode);
                frame.render_widget(Paragraph::new(info), area_info_a);
                deck.display.overview_rect = area_overview_a;
                refresh_overview_for_deck(deck, area_overview_a, rs.display_samp, rs.analysing, rs.warning_active, rs.warn_beat_on);
                if let Some(ref cached) = deck.display.overview_cache {
                    frame.render_widget(&cached.paragraph, area_overview_a);
                }
                render_detail_waveform(frame, &buf_a, deck, area_detail_a, &display_cfg, rs.display_pos_samp, deck.display.palette);
            } else {
                let num1_style = if selected_deck == 0 && !browser_open { Style::default().fg(Color::Yellow) } else { label_style };
                let mut spans = vec![Span::styled("1", num1_style), Span::styled(" ", label_style)];
                if let Some(ref s) = loading_label[0] {
                    spans.push(Span::styled(s.clone(), Style::default().fg(Color::DarkGray)));
                } else {
                    spans.extend(notification_line_empty().spans);
                }
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_a);
                frame.render_widget(Paragraph::new(info_line_empty(area_info_a.width)), area_info_a);
                frame.render_widget(Paragraph::new(overview_empty(area_overview_a, 0)), area_overview_a);
                render_detail_empty(frame, area_detail_a, 0);
            }

            // ---- Deck 2 ----
            if let (Some(deck), Some(rs)) = (&mut d1, &render[1]) {
                let content = notification_line_for_deck(deck, area_notif_b.width.saturating_sub(2) as usize, vinyl_mode);
                let num2_style = if selected_deck == 1 && !browser_open { Style::default().fg(Color::Yellow) } else { label_style };
                let mut spans = vec![Span::styled("2", num2_style), Span::styled(" ", label_style)];
                spans.extend(content.spans);
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_b);
                let info = info_line_for_deck(deck, frame_count, rs.beat_on, rs.spinner_active, label_style, area_info_b.width, vinyl_mode);
                frame.render_widget(Paragraph::new(info), area_info_b);
                deck.display.overview_rect = area_overview_b;
                refresh_overview_for_deck(deck, area_overview_b, rs.display_samp, rs.analysing, rs.warning_active, rs.warn_beat_on);
                if let Some(ref cached) = deck.display.overview_cache {
                    frame.render_widget(&cached.paragraph, area_overview_b);
                }
                render_detail_waveform(frame, &buf_b, deck, area_detail_b, &display_cfg, rs.display_pos_samp, deck.display.palette);
            } else {
                let num2_style = if selected_deck == 1 && !browser_open { Style::default().fg(Color::Yellow) } else { label_style };
                let mut spans = vec![Span::styled("2", num2_style), Span::styled(" ", label_style)];
                if let Some(ref s) = loading_label[1] {
                    spans.push(Span::styled(s.clone(), Style::default().fg(Color::DarkGray)));
                } else {
                    spans.extend(notification_line_empty().spans);
                }
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_b);
                frame.render_widget(Paragraph::new(info_line_empty(area_info_b.width)), area_info_b);
                frame.render_widget(Paragraph::new(overview_empty(area_overview_b, 1)), area_overview_b);
                render_detail_empty(frame, area_detail_b, 1);
            }

            // ---- Deck 3 ----
            if let (Some(deck), Some(rs)) = (&mut d2, &render[2]) {
                let content = notification_line_for_deck(deck, area_notif_c.width.saturating_sub(2) as usize, vinyl_mode);
                let num3_style = if selected_deck == 2 && !browser_open { Style::default().fg(Color::Yellow) } else { label_style };
                let mut spans = vec![Span::styled("3", num3_style), Span::styled(" ", label_style)];
                spans.extend(content.spans);
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_c);
                let info = info_line_for_deck(deck, frame_count, rs.beat_on, rs.spinner_active, label_style, area_info_c.width, vinyl_mode);
                frame.render_widget(Paragraph::new(info), area_info_c);
                deck.display.overview_rect = area_overview_c;
                if deck.loop_state.active {
                    render_loop_panels(frame, deck, area_overview_c, rs.display_pos_samp, deck.display.palette);
                } else {
                    refresh_overview_for_deck(deck, area_overview_c, rs.display_samp, rs.analysing, rs.warning_active, rs.warn_beat_on);
                    if let Some(ref cached) = deck.display.overview_cache {
                        frame.render_widget(&cached.paragraph, area_overview_c);
                    }
                }
                render_detail_waveform(frame, &buf_c, deck, area_detail_c, &display_cfg, rs.display_pos_samp, deck.display.palette);
            } else {
                let num3_style = if selected_deck == 2 && !browser_open { Style::default().fg(Color::Yellow) } else { label_style };
                let mut spans = vec![Span::styled("3", num3_style), Span::styled(" ", label_style)];
                if let Some(ref s) = loading_label[2] {
                    spans.push(Span::styled(s.clone(), Style::default().fg(Color::DarkGray)));
                } else {
                    spans.extend(notification_line_empty().spans);
                }
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_c);
                frame.render_widget(Paragraph::new(info_line_empty(area_info_c.width)), area_info_c);
                frame.render_widget(Paragraph::new(overview_empty(area_overview_c, 2)), area_overview_c);
                render_detail_empty(frame, area_detail_c, 2);
            }

            // ---- Global status bar ----
            {
                if pending_quit.map_or(false, |e| Instant::now() > e) { pending_quit = None; }
                let notification_bar = |msg: &str, expires: Instant, fg: Color, bg: Color, countdown_fg: Color| {
                    let secs = expires.saturating_duration_since(Instant::now()).as_secs();
                    let countdown = format!("[{}]", secs);
                    let w = area_global.width as usize;
                    let inner = w.saturating_sub(countdown.len());
                    let pad = inner.saturating_sub(msg.len()) / 2;
                    let centred = format!("{:pad$}{msg}", "");
                    let fill = inner.saturating_sub(pad + msg.len());
                    let line = Line::from(vec![
                        Span::styled(format!("{centred}{:fill$}", ""), Style::default().fg(fg)),
                        Span::styled(countdown, Style::default().fg(countdown_fg)),
                    ]);
                    (line, Style::default().bg(bg))
                };
                let (global_line, bar_style) = if let Some(quit_expires) = pending_quit {
                    notification_bar("Track is playing — quit?  [y] quit   [Esc/n] cancel", quit_expires,
                        Color::Rgb(255, 180, 180), Color::Rgb(100, 20, 20), Color::Rgb(200, 120, 120))
                } else if let Some(ref n) = global_notification {
                    match n.style {
                        NotificationStyle::Error =>
                            notification_bar(&n.message, n.expires,
                                Color::Rgb(255, 180, 180), Color::Rgb(100, 20, 20), Color::Rgb(200, 120, 120)),
                        NotificationStyle::Warning =>
                            notification_bar(&n.message, n.expires,
                                Color::Rgb(255, 220, 120), Color::Rgb(80, 60, 0), Color::Rgb(200, 160, 80)),
                        NotificationStyle::Info =>
                            notification_bar(&n.message, n.expires,
                                Color::Rgb(160, 200, 255), Color::Rgb(20, 40, 80), Color::Rgb(100, 140, 200)),
                        NotificationStyle::Success =>
                            notification_bar(&n.message, n.expires,
                                Color::Rgb(140, 230, 160), Color::Rgb(10, 60, 30), Color::Rgb(80, 170, 100)),
                    }
                } else {
                    let version = format!(" {} ", env!("CARGO_PKG_VERSION"));
                    let dir     = format!("  {}", browser_dir.display());
                    let w       = area_global.width as usize;
                    let pad     = w.saturating_sub(dir.len() + version.len());
                    let line    = Line::from(vec![
                        Span::styled(dir, Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{:pad$}", ""), Style::default()),
                        Span::styled(version, Style::default().fg(Color::DarkGray)),
                    ]);
                    (line, notif_bg)
                };
                frame.render_widget(Paragraph::new(global_line).style(bar_style), area_global);
            }

            // ---- Browser + context panel (permanent 70/30 split) ----
            if let Some(ref bs) = browser_state {
                let area = if c[16].height >= 8 {
                    c[16]
                } else {
                    frame.render_widget(ratatui::widgets::Clear, inner);
                    inner
                };
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .split(area);
                render_browser(frame, cols[0], bs);
                if let Some(ref editor) = tag_editor {
                    // Editing a track's tags — the panel hosts the editor; browser dims.
                    render::dim_area(frame, cols[0]);
                    render::render_tag_editor_panel(frame, cols[1], editor);
                } else {
                    // Dim the browser while the playlist list is the active target.
                    if panel.dim_browser() {
                        render::dim_area(frame, cols[0]);
                    }
                    let playing_of = |p: &PlaylistPanel| decks.iter().flatten()
                        .find_map(|d| d.playlist.as_ref().filter(|a| a.path == p.path).map(|a| a.index));
                    render::render_panel(frame, cols[1], &panel, &playing_of);
                }
            } else if c[16].height >= 3 && art_bright_idx < 2 {
                let brightness = [1.0f32, 0.35, 0.0][art_bright_idx as usize];
                // 1-row top margin separates art from deck 2 above.
                let vert = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(0)])
                    .split(c[16]);
                let art_row = vert[1];
                // 1-column gaps between the three panels.
                let art_areas = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Fill(1), Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1), Constraint::Fill(1)])
                    .split(art_row);
                for (idx, deck_opt) in [&mut d0, &mut d1, &mut d2].iter_mut().enumerate() {
                    let panel_idx = idx * 2; // indices 0 and 2; index 1 is the gap
                    if let Some(deck) = deck_opt {
                        if let Some(ref bytes) = deck.cover_art {
                            let a = art_areas[panel_idx];
                            let cached = deck.cover_art_cache.get_or_insert_with(|| {
                                (a.width, a.height, art_bright_idx,
                                 Paragraph::new(halfblock_art(bytes, a.width, a.height, brightness)))
                            });
                            if cached.0 != a.width || cached.1 != a.height || cached.2 != art_bright_idx {
                                *cached = (a.width, a.height, art_bright_idx,
                                           Paragraph::new(halfblock_art(bytes, a.width, a.height, brightness)));
                            }
                            frame.render_widget(&cached.3, a);
                        }
                    }
                }
            }

            // Unified help overlay — drawn on top of art; skipped when browser is open
            if help_open && browser_state.is_none() {
                render_keyboard_help(frame, c[16]);
            }

            // Tag editor overlay — a standalone modal only when the browser is closed
            // (the load-time rename offer); with the browser open it's in the panel.
            if browser_state.is_none() {
                if let Some(ref editor) = tag_editor {
                    render_tag_editor(frame, editor, area);
                }
            }

        })?;
        let draw_dur = draw_start.elapsed();

        // Put all three decks back after render.
        decks[0] = d0;
        decks[1] = d1;
        decks[2] = d2;

        // Single event handler — all actions work regardless of which deck is loaded.
        while event::poll(Duration::ZERO)? {
            match event::read()? {
            Event::Mouse(mouse_event) => {
                if browser_state.is_some() { continue; }
                if tag_editor.is_some() { continue; }
                if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
                    let col = mouse_event.column as usize;
                    let row = mouse_event.row as usize;
                    for slot in 0..3 {
                        if let Some(ref d) = decks[slot] {
                            let rect = d.display.overview_rect;
                            if col >= rect.x as usize && col < (rect.x + rect.width) as usize
                                && row >= rect.y as usize && row < (rect.y + rect.height) as usize
                            {
                                let click_col = col - rect.x as usize;
                                let target_secs = if vinyl_mode {
                                    d.total_duration * click_col as f64 / rect.width as f64
                                } else {
                                    d.display.last_bar_cols.iter()
                                        .zip(d.display.last_bar_times.iter())
                                        .filter(|(c, _)| **c <= click_col)
                                        .last()
                                        .map(|(_, t)| *t)
                                        .unwrap_or(0.0)
                                };
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(target_secs);
                                } else {
                                    d.audio.seek_handle.seek_to(target_secs);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            Event::Key(key) => {
                // Esc steps up one level per physical press, so only the initial press acts.
                // A held Esc repeats every ~30 ms, which would otherwise race through every
                // level of the cascade — deselecting a playlist and closing the browser in
                // one hold. Other keys keep their repeats; holding j to scroll is wanted.
                if key.code == KeyCode::Esc && key.kind != KeyEventKind::Press {
                    continue;
                }
                // Ctrl-C: unconditional quit.
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    for slot in 0..3 {
                        if let Some(ref d) = decks[slot] {
                            d.audio.player.stop();
                            if let Some(ref hash) = d.tempo.analysis_hash {
                                track_data.set(hash.clone(), cache_entry_for_deck(d));
                            }
                        }
                    }
                    track_data.save();
                    session.save();
                    return Ok(());
                }
                // Tag editor — a standalone modal; intercepts all keys while open.
                if tag_editor.is_some() {
                    if let KeyEventKind::Press = key.kind {
                        let saved = handle_tag_editor_key(&mut tag_editor, &mut decks, &mut browser_state, &mut global_notification, &mut compliance_cache, key);
                        if saved {
                            // A fix: advance from the resume anchor next frame.
                            cleanup_advance_to = edit_resume_anchor.take();
                        } else if tag_editor.is_none() {
                            // Editor closed without saving (cancelled) — don't advance.
                            edit_resume_anchor = None;
                        }
                    }
                    continue; // block all other key handling while editor is open
                }
                // Load-confirm: the target deck was playing. y/Enter loads; any other
                // key cancels (so a stray press can never leave input stuck).
                if browser_state.is_some() && browser_load_confirm.is_some() {
                    if key.kind == KeyEventKind::Press {
                        if key.code == KeyCode::Enter {
                            let (load, deck) = browser_load_confirm.take().unwrap();
                            global_notification = None;
                            if let Some(bs) = browser_state.as_ref() { *browser_dir = bs.cwd.clone(); }
                            session.set_last_browser_path(browser_dir);
                            let workspace = session.workspace().map(|p| p.to_path_buf());
                            if let Some(n) = apply_browser_load(load, deck, &mut decks, &mut pending_loads, workspace.as_deref()) {
                                global_notification = Some(n);
                            }
                            browser_state = None;
                            preview_output = None;
                        } else {
                            // Any other key cancels — no wedge, and it matches the prompt.
                            browser_load_confirm = None;
                            global_notification = None;
                        }
                    }
                    continue;
                }
                // Context panel state machine over the browser. Preview passes keys through
                // (with playlist transitions on `l`/`e`/Enter); Browse and Edit intercept.
                if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    let cmd_mode = browser_state.as_ref().map_or(false, |bs| bs.mode == BrowserMode::Command && bs.name_prompt.is_none());
                    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                    let target_deck = browser_state.as_ref().map_or(0, |bs| bs.target_deck);
                    let workspace = session.workspace().map(|p| p.to_path_buf());
                    let mut transition: Option<Panel> = None;
                    let mut consumed = false;
                    match &mut panel {
                        Panel::Preview(_) if cmd_mode => {
                            if let Some((path, EntryKind::Playlist)) = browser_state.as_ref().and_then(|bs| bs.highlighted_entry()) {
                                match key.code {
                                    KeyCode::Char('l') | KeyCode::Enter => {
                                        transition = Some(Panel::Browse(PlaylistPanel::open(path, workspace.as_deref())));
                                        consumed = true;
                                    }
                                    KeyCode::Char('e') => {
                                        let pp = PlaylistPanel::open(path, workspace.as_deref());
                                        transition = Some(Panel::Edit { panel: pp, focus: EditFocus::Playlist });
                                        consumed = true;
                                    }
                                    _ => {} // fall through to the browser
                                }
                            }
                        }
                        Panel::Browse(pp) => {
                            consumed = true;
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('h') => transition = Some(Panel::Preview(Preview::Empty)),
                                KeyCode::Up | KeyCode::Char('k') => pp.cursor_up(),
                                KeyCode::Down | KeyCode::Char('j') => pp.cursor_down(),
                                // Enter: play a resolved entry, open the picker on a needs-confirmation
                                // one (it can't be played), or nudge for a workspace on an unavailable one.
                                KeyCode::Enter => match pp.status_at(pp.cursor) {
                                    EntryStatus::Found => {
                                        if let Some(n) = play_panel_entry(pp, pp.cursor, target_deck, &mut decks, &mut pending_loads, workspace.as_deref()) {
                                            global_notification = Some(n);
                                        }
                                    }
                                    EntryStatus::NeedsConfirmation => {
                                        let dir = pp.path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
                                        let library = WorkspaceLibrary::new(workspace.as_deref());
                                        if let playlist::Resolution::NeedsConfirmation { candidates } = playlist::resolve(&pp.playlist.entries[pp.cursor], &dir, &library) {
                                            transition = Some(Panel::Confirm { panel: pp.clone(), entry: pp.cursor, candidates, cursor: 0, layout: Rc::new(RefCell::new(ConfirmLayout::default())) });
                                        }
                                    }
                                    EntryStatus::Unavailable => {
                                        if workspace.is_none() {
                                            global_notification = Some(notification("Set a workspace (@) to find candidates for missing tracks", NotificationStyle::Warning));
                                        }
                                    }
                                },
                                KeyCode::Char('e') => {
                                    transition = Some(Panel::Edit { panel: pp.clone(), focus: EditFocus::Playlist });
                                }
                                _ => {} // swallow
                            }
                        }
                        Panel::Confirm { panel: pp, entry, candidates, cursor, layout } => {
                            consumed = true;
                            // `cursor` is a line offset; scroll by line and adopt the card at the top.
                            // The renderer publishes the variable-height card layout into `layout`.
                            let published = layout.borrow();
                            let max_offset = published.total_lines.saturating_sub(1);
                            let active = confirm_active_card(*cursor, &published.card_starts);
                            drop(published);
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('h') => transition = Some(Panel::Browse(pp.clone())),
                                KeyCode::Up | KeyCode::Char('k') => *cursor = cursor.saturating_sub(1),
                                KeyCode::Down | KeyCode::Char('j') => *cursor = (*cursor + 1).min(max_offset),
                                KeyCode::Enter => {
                                    if let Some(cand) = candidates.get(active) {
                                        let cand_path = cand.path.clone();
                                        let dir = pp.path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
                                        match track_facts(&cand_path) {
                                            Some(facts) => {
                                                let mut new_pp = pp.clone();
                                                match playlist::adopt_candidate(&mut new_pp.playlist.entries[*entry], &cand_path, &dir, &facts) {
                                                    Ok(()) => {
                                                        commit_playlist(&new_pp, &mut decks);
                                                        new_pp.recompute_status(workspace.as_deref(), false);
                                                        global_notification = Some(notification("Track re-linked", NotificationStyle::Success));
                                                        transition = Some(Panel::Browse(new_pp));
                                                    }
                                                    Err(e) => global_notification = Some(notification(format!("re-link failed: {e:?}"), NotificationStyle::Error)),
                                                }
                                            }
                                            None => global_notification = Some(notification("couldn't read candidate file", NotificationStyle::Error)),
                                        }
                                    }
                                }
                                _ => {} // swallow
                            }
                        }
                        Panel::Edit { panel: pp, focus, .. } => {
                            match (*focus, key.code) {
                                (_, KeyCode::Enter) => {
                                    commit_playlist(pp, &mut decks);
                                    // Return the browser to the playlist file and enter Browse, so
                                    // it's not lost and is ready to load a track.
                                    let dir = pp.path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
                                    let rpl = pp.path.clone();
                                    if let Some(bs) = browser_state.as_mut() { let _ = bs.go_to(dir, Some(&rpl), String::new()); }
                                    transition = Some(Panel::Browse(pp.clone()));
                                    consumed = true;
                                }
                                (_, KeyCode::Esc) => {
                                    // Abort: discard the buffer, but still land back on the playlist.
                                    let dir = pp.path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
                                    let rpl = pp.path.clone();
                                    if let Some(bs) = browser_state.as_mut() { let _ = bs.go_to(dir, Some(&rpl), String::new()); }
                                    transition = Some(Panel::Preview(Preview::Empty));
                                    consumed = true;
                                }
                                (_, KeyCode::Tab) => { *focus = if *focus == EditFocus::Browser { EditFocus::Playlist } else { EditFocus::Browser }; consumed = true; }
                                (EditFocus::Playlist, KeyCode::Char('h')) => { *focus = EditFocus::Browser; consumed = true; }
                                (EditFocus::Browser, KeyCode::Char('l')) => { *focus = EditFocus::Playlist; consumed = true; }
                                // Insert the highlighted browser entry (track, or another playlist's
                                // entries) after (`a`) or before (`A`) the cursor, Helix-style like
                                // paste p/P — `a` on the last entry appends.
                                (EditFocus::Browser, KeyCode::Char(c @ ('a' | 'A'))) if cmd_mode => {
                                    if let Some((path, kind)) = browser_state.as_ref().and_then(|bs| bs.highlighted_entry()) {
                                        let dir = pp.path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
                                        match gather_insert_entries(&path, kind, &dir) {
                                            Ok(entries) if !entries.is_empty() => {
                                                let at = if c == 'a' && !pp.playlist.entries.is_empty() { pp.cursor + 1 } else { pp.cursor };
                                                pp.insert_at(at, entries);
                                                pp.recompute_status(workspace.as_deref(), false);
                                            }
                                            Ok(_) => {}
                                            Err(e) => global_notification = Some(notification(e, NotificationStyle::Error)),
                                        }
                                    }
                                    consumed = true;
                                }
                                (EditFocus::Playlist, KeyCode::Up) if shift => { pp.move_up(); consumed = true; }
                                (EditFocus::Playlist, KeyCode::Down) if shift => { pp.move_down(); consumed = true; }
                                (EditFocus::Playlist, KeyCode::Char('K')) => { pp.move_up(); consumed = true; }
                                (EditFocus::Playlist, KeyCode::Char('J')) => { pp.move_down(); consumed = true; }
                                (EditFocus::Playlist, KeyCode::Up) | (EditFocus::Playlist, KeyCode::Char('k')) => { pp.cursor_up(); consumed = true; }
                                (EditFocus::Playlist, KeyCode::Down) | (EditFocus::Playlist, KeyCode::Char('j')) => { pp.cursor_down(); consumed = true; }
                                (EditFocus::Playlist, KeyCode::Char('x')) | (EditFocus::Playlist, KeyCode::Delete) => { pp.remove_at_cursor(); consumed = true; }
                                // Edit-Playlist swallows the rest; Edit-Browser lets nav fall through to the browser.
                                _ => if *focus == EditFocus::Playlist { consumed = true; }
                            }
                        }
                        _ => {} // Preview in search mode: fall through to the browser
                    }
                    if let Some(t) = transition {
                        if matches!(t, Panel::Preview(_)) { panel_source = None; }
                        panel = t;
                    }
                    if consumed { continue; }
                }
                // Browser — intercepts all key events when open.
                if let Some(bs) = browser_state.as_mut() {
                    // # previews the highlighted audio file — only in Command mode,
                    // where letters aren't filter input.
                    if key.kind == KeyEventKind::Press
                        && key.code == KeyCode::Char('#')
                        && bs.mode == BrowserMode::Command
                    {
                        if let Some(ref po) = preview_output {
                            if let Some(path) = bs.highlighted_audio_path() {
                                po.play(&path);
                            }
                        }
                        continue;
                    }

                    // Any other key stops an active preview before being handled normally.
                    if let Some(ref po) = preview_output {
                        po.stop();
                    }

                    // Remember the primary mode so the browser reopens where it left off.
                    last_browser_mode = bs.primary_mode();

                    // A selected track/playlist can't be loaded inside the `bs` borrow;
                    // capture the intent and apply it after the match releases `bs`.
                    let mut browser_selection_outcome: Option<(BrowserLoad, usize, PathBuf)> = None;
                    let mut create_playlist_request: Option<(String, PathBuf)> = None;
                    match handle_browser_key(bs, key)? {
                        Some(BrowserResult::ReturnToPlayer) => {
                            *browser_dir = bs.cwd.clone();
                            session.set_last_browser_path(browser_dir);
                            browser_state = None;
                            preview_output = None;
                        }
                        Some(BrowserResult::Selected(path)) => {
                            browser_selection_outcome = Some((BrowserLoad::Track(path), bs.target_deck, bs.cwd.clone()));
                        }
                        Some(BrowserResult::PlaylistSelected(path)) => {
                            browser_selection_outcome = Some((BrowserLoad::Playlist(path), bs.target_deck, bs.cwd.clone()));
                        }
                        Some(BrowserResult::CreatePlaylist(name)) => {
                            create_playlist_request = Some((name, bs.cwd.clone()));
                        }
                        Some(BrowserResult::EditRequested(path)) => {
                            tag_editor = Some(TagEditorState::for_track(&path));
                            // Cleanup mode: remember the entry below this one so a save
                            // resumes there (stable across the rename), wrapping to top.
                            if bs.compliance_on {
                                edit_resume_anchor = bs.entries.get(bs.cursor + 1)
                                    .or_else(|| bs.entries.first())
                                    .map(|e| e.path.clone());
                            }
                        }
                        Some(BrowserResult::DirectoryChosen(dir)) => {
                            // Move the carried source file, sync any deck loaded from
                            // it, and stay in the browser (back to Command) refreshed.
                            if let Some(source) = bs.move_source.take() {
                                let (notif, new_path) = move_file_to_directory(&source, &dir);
                                if let Some(ref new_path) = new_path {
                                    sync_deck_path(&mut decks, &source, new_path, None);
                                }
                                global_notification = Some(notif);
                            }
                            bs.mode = BrowserMode::Command;
                            let _ = bs.refresh();
                        }
                        Some(BrowserResult::CycleLocation) => {
                            // Locations: the opening directory (home), then each loaded
                            // deck's track directory (with the track highlighted).
                            let mut locations: Vec<(PathBuf, Option<PathBuf>, String)> =
                                vec![(browser_dir.clone(), None, "Working directory".to_string())];
                            for slot in 0..3 {
                                if let Some(ref d) = decks[slot] {
                                    if let Some(parent) = d.path.parent() {
                                        locations.push((parent.to_path_buf(), Some(d.path.clone()), format!("Deck {} directory", slot + 1)));
                                    }
                                }
                            }
                            location_cycle = (location_cycle + 1) % locations.len();
                            let (dir, highlight, label) = &locations[location_cycle];
                            let _ = bs.go_to(dir.clone(), highlight.as_deref(), label.clone());
                        }
                        Some(BrowserResult::WorkspaceSet(path)) => {
                            session.set_workspace(&path);
                            // Adopt the newly-attached library's travelling database.
                            track_data.set_mirror(Some(&path));
                            track_data.sync_with_mirror();
                            // Heal open playlists: relocate moved tracks against the new library.
                            let ws = Some(path.as_path());
                            let mut healed = false;
                            for d in decks.iter_mut().flatten() {
                                if let Some(active) = d.playlist.as_mut() {
                                    healed |= heal_playlist(&mut active.playlist, &active.path, ws);
                                }
                            }
                            if let Some(pp) = panel.playlist_mut() {
                                heal_playlist(&mut pp.playlist, &pp.path, ws);
                                pp.recompute_status(ws, false);
                            }
                            if healed {
                                global_notification = Some(notification("Relocated moved tracks in open playlists", NotificationStyle::Success));
                            }
                        }
                        Some(BrowserResult::WorkspaceCleared) => {
                            session.clear_workspace();
                            track_data.set_mirror(None);
                        }
                        None => {}
                    }
                    if let Some((load, deck, cwd)) = browser_selection_outcome {
                        *browser_dir = cwd;
                        session.set_last_browser_path(browser_dir);
                        let playing = decks[deck].as_ref().is_some_and(|d| !d.audio.player.is_paused());
                        if playing {
                            // Defer behind a confirmation; the browser stays open.
                            browser_load_confirm = Some((load, deck));
                            global_notification = Some(notification(
                                format!("Deck {} is playing — Enter to load, any other key cancels", deck + 1),
                                NotificationStyle::Error,
                            ));
                        } else {
                            let workspace = session.workspace().map(|p| p.to_path_buf());
                            if let Some(n) = apply_browser_load(load, deck, &mut decks, &mut pending_loads, workspace.as_deref()) {
                                global_notification = Some(n);
                            }
                            browser_state = None;
                            preview_output = None;
                        }
                    }
                    if let Some((name, dir)) = create_playlist_request {
                        match create_playlist_file(&dir, &name) {
                            // Highlight the new file; the hover preview then opens its editor.
                            Ok(path) => if let Some(bs) = browser_state.as_mut() {
                                let _ = bs.go_to(dir.clone(), Some(&path), format!("new playlist: {name}"));
                            },
                            Err(e) => global_notification = Some(notification(e, NotificationStyle::Error)),
                        }
                    }
                    continue; // block all player key handling while browser is open
                }
                // A chord fires while either modifier is held: Alt (advertised, arrives
                // as a reliable per-press bit) or Space (legacy, tracked below).
                let alt = key.modifiers.contains(KeyModifiers::ALT);
                // Space modifier: track held state for chords.
                if key.code == KeyCode::Char(' ') {
                    space_saw_event_this_frame = true;
                    match key.kind {
                        KeyEventKind::Press | KeyEventKind::Repeat => {
                            if !space_repeat_suppressed { space_held = true; }
                        }
                        KeyEventKind::Release => {
                            space_held = false;
                            space_repeat_suppressed = false;
                        }
                    }
                }
                // Nudge and mode toggle — handled for all key kinds (Release must be detected).
                match key.kind {
                    KeyEventKind::Press
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::NudgeModeToggle) =>
                    {
                        let new_mode = decks.iter().flatten().next()
                            .map(|d| match d.nudge_mode {
                                NudgeMode::Jump => NudgeMode::Warp,
                                NudgeMode::Warp => NudgeMode::Jump,
                            })
                            .unwrap_or(NudgeMode::Jump);
                        if new_mode == NudgeMode::Warp && !release_events_supported {
                            global_notification = Some(Notification {
                                message: "Warp nudge unavailable — terminal can't report key releases".to_string(),
                                style: NotificationStyle::Error,
                                expires: Instant::now() + NOTIFICATION_TIMEOUT,
                            });
                        } else {
                            for slot in 0..3 {
                                if let Some(ref mut d) = decks[slot] {
                                    if d.nudge != 0 {
                                        d.nudge = 0;
                                        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                    }
                                    d.nudge_mode = new_mode;
                                }
                            }
                        }
                    }
                    // Cue play comes before nudge so that chord+nudge-key resolves to
                    // cue (via Chord lookup) rather than nudge.
                    // Press guard requires a chord modifier to avoid firing on bare nudge-key presses.
                    KeyEventKind::Press
                        if (space_held || alt) && keymap.get(&KeyBinding::Chord(key.code)) == Some(&Action::CuePlay) =>
                    {
                        if let Some(ref mut d) = decks[selected_deck] {
                            if let Some(cue_samp) = d.cue_sample {
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                    d.display.smooth_display_samp = cue_samp as f64;
                                } else {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    let target_samp = (cue_samp + latency_samps).min(d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize);
                                    d.audio.seek_handle.seek_to(target_samp as f64 / d.audio.sample_rate as f64);
                                }
                            }
                            if space_held { space_held = false; space_repeat_suppressed = true; }
                        }
                        continue 'tui;
                    }
                    // Nudge (selected deck) — guards exclude both chord modifiers so chord+nudge-key
                    // resolves cleanly to its Chord action without also nudging.
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if !space_held && !alt && keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::NudgeBackward) =>
                    {
                        let scrub_spc = [scrub_spc_a, scrub_spc_b, scrub_spc_c][selected_deck];
                        if let Some(ref mut d) = decks[selected_deck] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    if d.audio.player.is_paused() {
                                        let target = (current - 0.010).max(0.0);
                                        d.audio.seek_handle.set_position(target);
                                        d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc);
                                    } else {
                                        let bump_secs = d.audio.seek_handle.seek_relative_faded(-0.010, d.total_duration);
                                        d.display.smooth_display_samp += bump_secs * d.audio.sample_rate as f64;
                                    }
                                }
                                NudgeMode::Warp => {
                                    if release_events_supported {
                                        d.nudge = -1;
                                        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 0.9);
                                    }
                                }
                            }
                        }
                    }
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if !space_held && !alt && keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::NudgeForward) =>
                    {
                        let scrub_spc = [scrub_spc_a, scrub_spc_b, scrub_spc_c][selected_deck];
                        if let Some(ref mut d) = decks[selected_deck] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    if d.audio.player.is_paused() {
                                        let target = (current + 0.010).min(d.total_duration);
                                        d.audio.seek_handle.set_position(target);
                                        d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc);
                                    } else {
                                        let bump_secs = d.audio.seek_handle.seek_relative_faded(0.010, d.total_duration);
                                        d.display.smooth_display_samp += bump_secs * d.audio.sample_rate as f64;
                                    }
                                }
                                NudgeMode::Warp => {
                                    if release_events_supported {
                                        d.nudge = 1;
                                        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 1.1);
                                    }
                                }
                            }
                        }
                    }
                    KeyEventKind::Release
                        if matches!(keymap.get(&KeyBinding::Key(key.code)),
                            Some(&Action::NudgeBackward) | Some(&Action::NudgeForward)) =>
                    {
                        if let Some(ref mut d) = decks[selected_deck] {
                            if d.nudge_mode == NudgeMode::Warp {
                                d.nudge = 0;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            }
                        }
                    }
                    _ => {}
                }
                // Base BPM ramp — fires on Press and Repeat with time-based step size.
                // The ramp resets only when no base-BPM key has been seen for >500 ms,
                // so a quick release-and-repress continues at the current tier.
                if !vinyl_mode
                    && !space_held && !alt
                    && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                    && matches!(keymap.get(&KeyBinding::Key(key.code)),
                        Some(&Action::BaseBpmIncrease) | Some(&Action::BaseBpmDecrease))
                {
                    let gap = bpm_ramp_last.map_or(Duration::MAX, |t| t.elapsed());
                    if gap > Duration::from_millis(80) {
                        bpm_ramp_started = Some(Instant::now());
                    }
                    bpm_ramp_last = Some(Instant::now());
                    let elapsed = bpm_ramp_started.map_or(Duration::ZERO, |t| t.elapsed());
                    let step: f32 = if elapsed >= Duration::from_secs(3) { 0.05 }
                                    else { 0.01 };
                    let sign = if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::BaseBpmIncrease) { 1.0f32 } else { -1.0f32 };
                    if let Some(ref mut d) = decks[selected_deck] {
                        d.tempo.base_bpm = (d.tempo.base_bpm + sign * step).clamp(40.0, 240.0);
                        d.tempo.bpm_established = true;
                        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                        shared_renderer.store_speed_ratio(selected_deck, d.tempo.bpm, d.tempo.base_bpm);
                        anchor_beat_grid_to_cue(d);
                        if let Some(ref hash) = d.tempo.analysis_hash {
                            track_data.set(hash.clone(), cache_entry_for_deck(d));
                        }
                    }
                }
                // All other actions fire on Press only.
                if key.kind == KeyEventKind::Press {
                    // Esc closes the help overlay.
                    if help_open && key.code == KeyCode::Esc {
                        help_open = false;
                        continue 'tui;
                    }
                    // Quit confirmation intercept — y/Enter confirms, anything else cancels.
                    if pending_quit.is_some() {
                        pending_quit = None;
                        if matches!(key.code, KeyCode::Char('y') | KeyCode::Enter) {
                            for slot in 0..3 {
                                if let Some(ref d) = decks[slot] {
                                    d.audio.player.stop();
                                    if let Some(ref hash) = d.tempo.analysis_hash {
                                        track_data.set(hash.clone(), cache_entry_for_deck(d));
                                    }
                                }
                            }
                            session.set_latency(audio_latency_ms);
                            track_data.save();
                            session.save();
                            return Ok(());
                        }
                        continue 'tui;
                    }
                    // Esc dismisses any active global notification.
                    if global_notification.is_some() && key.code == KeyCode::Esc {
                        global_notification = None;
                        continue 'tui;
                    }
                    // BPM confirmation intercept — check both decks.
                    let mut bpm_intercepted = false;
                    for slot in 0..3 {
                        if let Some(ref mut d) = decks[slot] {
                            if let Some((hash, p_bpm, p_offset, _)) = d.tempo.pending_bpm.take() {
                                if matches!(key.code, KeyCode::Char('y') | KeyCode::Enter) {
                                    d.tempo.bpm = p_bpm;
                                    d.tempo.base_bpm = p_bpm;
                                    d.tempo.offset_ms = (p_offset as f64 / 10.0).round() as i64 * 10;
                                    d.tempo.bpm_established = true;
                                    d.audio.player.set_speed(1.0);
                                    shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
                                    d.tempo.offset_established = true;
                                    track_data.set(hash.clone(), cache_entry_for_deck(d));
                                    d.tempo.analysis_hash = Some(hash);
                                }
                                // Any key dismisses the confirmation.
                                bpm_intercepted = true;
                                break;
                            }
                        }
                    }
                    if bpm_intercepted { continue 'tui; }

                    // Rename offer — 'y' and 'h' are intercepted when offer is visible;
                    // any other key dismisses the offer and falls through to normal handling.
                    let mut rename_offer_consumed = false;
                    for slot in 0..3 {
                        if let Some(ref mut d) = decks[slot] {
                            if d.rename_offer_active() {
                                match key.code {
                                    KeyCode::Char('y') => {
                                        tag_editor = Some(TagEditorState::for_track(&d.path));
                                        d.rename_offer_started = None;
                                        rename_offer_consumed = true;
                                    }
                                    _ => {
                                        // Key performs normally; offer stays.
                                    }
                                }
                                break;
                            }
                        }
                    }
                    if rename_offer_consumed { continue 'tui; }

                    let action = if (space_held || alt) && key.code != KeyCode::Char(' ') {
                        if let Some(a) = keymap.get(&KeyBinding::Chord(key.code)) {
                            if space_held { space_held = false; space_repeat_suppressed = true; }
                            Some(a)
                        } else {
                            keymap.get(&KeyBinding::Key(key.code))
                        }
                    } else {
                        keymap.get(&KeyBinding::Key(key.code))
                    };
                    match action {
                    Some(Action::Quit) => {
                        let any_playing = decks.iter().flatten().any(|d| !d.audio.player.is_paused());
                        if any_playing && pending_quit.is_none() {
                            pending_quit = Some(Instant::now() + Duration::from_secs(5));
                            continue 'tui;
                        }
                        for slot in 0..3 {
                            if let Some(ref d) = decks[slot] {
                                d.audio.player.stop();
                                if let Some(ref hash) = d.tempo.analysis_hash {
                                    track_data.set(hash.clone(), cache_entry_for_deck(d));
                                }
                            }
                        }
                        session.set_latency(audio_latency_ms);
                        track_data.save();
                        session.save();
                        return Ok(());
                    }
                    Some(Action::SelectDeck1) => { selected_deck = 0; }
                    Some(Action::SelectDeck2) => { selected_deck = 1; }
                    Some(Action::SelectDeck3) => { selected_deck = 2; }
                    Some(Action::SelectNextDeck) => { selected_deck = (selected_deck + 1) % 3; }
                    Some(Action::SelectPrevDeck) => { selected_deck = (selected_deck + 2) % 3; }
                    Some(Action::PlaylistNext) => {
                        let ws = session.workspace().map(|p| p.to_path_buf());
                        play_playlist_step(selected_deck, true, &mut decks, &mut pending_loads, ws.as_deref());
                    }
                    Some(Action::PlaylistPrev) => {
                        let ws = session.workspace().map(|p| p.to_path_buf());
                        play_playlist_step(selected_deck, false, &mut decks, &mut pending_loads, ws.as_deref());
                    }
                    Some(Action::OpenBrowser) => {
                        // Opening never interrupts anything; the load target defaults to
                        // the least-disruptive deck and is adjustable in the browser.
                        let workspace = session.workspace().map(|p| p.to_path_buf());
                        let mut bs = BrowserState::new(browser_dir.clone(), workspace)?;
                        bs.mode = last_browser_mode;
                        bs.target_deck = default_target_deck(&decks, selected_deck);
                        // Opens at the working directory (cycle position 0); show its label.
                        bs.location_label = Some("Working directory".to_string());
                        browser_state = Some(bs);
                        preview_output = Some(PreviewOutput::new(mixer));
                        location_cycle = 0;
                    }
                    Some(Action::PlayPause) => {
                        if let Some(ref d) = decks[selected_deck] {
                            if d.audio.player.is_paused() {
                                if d.mixer.filter_offset != 0 {
                                    d.audio.filter_state_reset.store(true, Ordering::Relaxed);
                                }
                                d.audio.player.play();
                            } else {
                                d.audio.player.pause();
                            }
                        }
                    }
                    Some(Action::Deck1LevelUp)   => { if let Some(ref mut d) = decks[0] { d.mixer.volume = (d.mixer.volume + 0.05).min(1.0); d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 0 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck1LevelDown)  => { if let Some(ref mut d) = decks[0] { d.mixer.volume = (d.mixer.volume - 0.05).max(0.0); d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 0 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck1LevelMax)   => { if let Some(ref mut d) = decks[0] { d.mixer.volume = 1.0; d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 0 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck1LevelMin)   => { if let Some(ref mut d) = decks[0] { d.mixer.volume = 0.0; d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 0 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck2LevelUp)    => { if let Some(ref mut d) = decks[1] { d.mixer.volume = (d.mixer.volume + 0.05).min(1.0); d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 1 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck2LevelDown)  => { if let Some(ref mut d) = decks[1] { d.mixer.volume = (d.mixer.volume - 0.05).max(0.0); d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 1 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck2LevelMax)   => { if let Some(ref mut d) = decks[1] { d.mixer.volume = 1.0; d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 1 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck2LevelMin)   => { if let Some(ref mut d) = decks[1] { d.mixer.volume = 0.0; d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 1 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck3LevelUp)    => { if let Some(ref mut d) = decks[2] { d.mixer.volume = (d.mixer.volume + 0.05).min(1.0); d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 2 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck3LevelDown)  => { if let Some(ref mut d) = decks[2] { d.mixer.volume = (d.mixer.volume - 0.05).max(0.0); d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 2 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck3LevelMax)   => { if let Some(ref mut d) = decks[2] { d.mixer.volume = 1.0; d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 2 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::Deck3LevelMin)   => { if let Some(ref mut d) = decks[2] { d.mixer.volume = 0.0; d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 2 { d.audio.player.set_volume(d.mixer.volume); } } }
                    Some(Action::PflOnOff) => {
                        if pfl_active_deck.load(Ordering::Relaxed) == selected_deck {
                            if let Some(ref mut d) = decks[selected_deck] { d.mixer.pfl_level = 0; d.audio.pfl_level.store(0, Ordering::Relaxed); d.audio.player.set_volume(d.mixer.volume); }
                            pfl_active_deck.store(usize::MAX, Ordering::Relaxed);
                        } else {
                            for other in 0..3 { if other != selected_deck { if let Some(ref mut d) = decks[other] { d.mixer.pfl_level = 0; d.audio.pfl_level.store(0, Ordering::Relaxed); d.audio.player.set_volume(d.mixer.volume); } } }
                            if let Some(ref mut d) = decks[selected_deck] { d.mixer.pfl_level = 100; d.audio.pfl_level.store(100, Ordering::Relaxed); d.audio.player.set_volume(1.0); }
                            pfl_active_deck.store(selected_deck, Ordering::Relaxed);
                        }
                    }
                    Some(Action::PflLevelUp) => {
                        let activate = if let Some(ref mut d) = decks[selected_deck] {
                            if d.mixer.pfl_level < 100 {
                                let was_zero = d.mixer.pfl_level == 0;
                                d.mixer.pfl_level = d.mixer.pfl_level.saturating_add(20).min(100);
                                d.audio.pfl_level.store(d.mixer.pfl_level, Ordering::Relaxed);
                                was_zero
                            } else { false }
                        } else { false };
                        if activate {
                            for other in 0..3 { if other != selected_deck { if let Some(ref mut od) = decks[other] { od.mixer.pfl_level = 0; od.audio.pfl_level.store(0, Ordering::Relaxed); od.audio.player.set_volume(od.mixer.volume); } } }
                            if let Some(ref mut d) = decks[selected_deck] { d.audio.player.set_volume(1.0); }
                            pfl_active_deck.store(selected_deck, Ordering::Relaxed);
                        }
                    }
                    Some(Action::PflLevelDown) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            if d.mixer.pfl_level > 0 {
                                d.mixer.pfl_level = d.mixer.pfl_level.saturating_sub(20);
                                d.audio.pfl_level.store(d.mixer.pfl_level, Ordering::Relaxed);
                                if d.mixer.pfl_level == 0 {
                                    d.audio.player.set_volume(d.mixer.volume);
                                    pfl_active_deck.store(usize::MAX, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                    Some(Action::PflLevelReset) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            d.mixer.pfl_level = 0;
                            d.audio.pfl_level.store(0, Ordering::Relaxed);
                            d.audio.player.set_volume(d.mixer.volume);
                        }
                        if pfl_active_deck.load(Ordering::Relaxed) == selected_deck {
                            pfl_active_deck.store(usize::MAX, Ordering::Relaxed);
                        }
                    }
                    Some(Action::Deck1GainIncrease) => {
                        if let Some(ref mut d) = decks[0] {
                            d.mixer.gain_db = (d.mixer.gain_db + 1).min(12);
                            d.audio.gain_linear.store(10f32.powf(d.mixer.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                track_data.set(hash.clone(), cache_entry_for_deck(d));
                            }
                        }
                    }
                    Some(Action::Deck1GainDecrease) => {
                        if let Some(ref mut d) = decks[0] {
                            d.mixer.gain_db = (d.mixer.gain_db - 1).max(-12);
                            d.audio.gain_linear.store(10f32.powf(d.mixer.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                track_data.set(hash.clone(), cache_entry_for_deck(d));
                            }
                        }
                    }
                    Some(Action::Deck2GainIncrease) => {
                        if let Some(ref mut d) = decks[1] {
                            d.mixer.gain_db = (d.mixer.gain_db + 1).min(12);
                            d.audio.gain_linear.store(10f32.powf(d.mixer.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                track_data.set(hash.clone(), cache_entry_for_deck(d));
                            }
                        }
                    }
                    Some(Action::Deck2GainDecrease) => {
                        if let Some(ref mut d) = decks[1] {
                            d.mixer.gain_db = (d.mixer.gain_db - 1).max(-12);
                            d.audio.gain_linear.store(10f32.powf(d.mixer.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                track_data.set(hash.clone(), cache_entry_for_deck(d));
                            }
                        }
                    }
                    Some(Action::Deck3GainIncrease) => {
                        if let Some(ref mut d) = decks[2] {
                            d.mixer.gain_db = (d.mixer.gain_db + 1).min(12);
                            d.audio.gain_linear.store(10f32.powf(d.mixer.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                track_data.set(hash.clone(), cache_entry_for_deck(d));
                            }
                        }
                    }
                    Some(Action::Deck3GainDecrease) => {
                        if let Some(ref mut d) = decks[2] {
                            d.mixer.gain_db = (d.mixer.gain_db - 1).max(-12);
                            d.audio.gain_linear.store(10f32.powf(d.mixer.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                track_data.set(hash.clone(), cache_entry_for_deck(d));
                            }
                        }
                    }
                    Some(Action::SwapDeck1Deck2) => {
                        decks.swap(0, 1);
                        shared_renderer.swap_slots(0, 1);
                        if selected_deck == 0 { selected_deck = 1; } else if selected_deck == 1 { selected_deck = 0; }
                    }
                    Some(Action::SwapDeck2Deck3) => {
                        decks.swap(1, 2);
                        shared_renderer.swap_slots(1, 2);
                        if selected_deck == 1 { selected_deck = 2; } else if selected_deck == 2 { selected_deck = 1; }
                    }
                    Some(Action::PitchUp) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            d.pitch_semitones = (d.pitch_semitones + 1).min(6);
                            d.audio.pitch_semitones.store(d.pitch_semitones, Ordering::Relaxed);
                        }
                    }
                    Some(Action::PitchDown) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            d.pitch_semitones = (d.pitch_semitones - 1).max(-6);
                            d.audio.pitch_semitones.store(d.pitch_semitones, Ordering::Relaxed);
                        }
                    }
                    Some(Action::PitchReset) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            d.pitch_semitones = 0;
                            d.audio.pitch_semitones.store(0, Ordering::Relaxed);
                        }
                    }
                    Some(Action::MetronomeToggle) => {
                        if !vinyl_mode {
                            if let Some(ref mut d) = decks[selected_deck] {
                                d.metronome_mode = !d.metronome_mode;
                                d.last_metro_beat = if d.metronome_mode {
                                    let beat_period = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
                                    let ns = (d.display.smooth_display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
                                        - d.tempo.offset_ms as i128 * 1_000_000;
                                    Some(ns.div_euclid(beat_period.as_nanos() as i128))
                                } else { None };
                            }
                        }
                    }
                    Some(Action::DetectBpm) => {
                        if !vinyl_mode {
                        if let Some(ref mut d) = decks[selected_deck] {
                            if d.tempo.pending_bpm.is_some() {
                                d.tempo.pending_bpm = None;
                            } else if d.tempo.redetecting {
                                let (_, dead_rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                d.tempo.background_rx = Some(std::mem::replace(&mut d.tempo.bpm_rx, dead_rx));
                                d.tempo.redetecting = false;
                                d.tempo.analysis_hash = d.tempo.redetect_saved_hash.take();
                            } else if d.tempo.analysis_hash.is_some() {
                                if let Some(bg_rx) = d.tempo.background_rx.take() {
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.bpm_rx = bg_rx;
                                    d.tempo.redetecting = true;
                                } else {
                                    let mono_bg = Arc::clone(&d.audio.mono);
                                    let (tx, rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                    let hash_bg = d.tempo.analysis_hash.clone().unwrap_or_default();
                                    let sr_bg = d.audio.sample_rate;
                                    thread::spawn(move || {
                                        if let Ok(bpm) = detect_bpm(&mono_bg, sr_bg) {
                                            let _ = tx.send((hash_bg, bpm, 0, true));
                                        }
                                    });
                                    d.tempo.bpm_rx = rx;
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.redetecting = true;
                                }
                            }
                        }
                        }
                    }
                    Some(Action::Help)            => { help_open = !help_open; }
                    Some(Action::VinylModeToggle) => {
                        vinyl_mode = !vinyl_mode;
                        for slot in 0..3 {
                            if let Some(ref mut d) = decks[slot] {
                                if vinyl_mode {
                                    // Entering vinyl mode: capture current speed as vinyl_speed;
                                    // clear tap state and stop metronome.
                                    d.tempo.vinyl_speed = d.tempo.bpm / d.tempo.base_bpm;
                                    d.audio.player.set_speed(d.tempo.vinyl_speed);
                                    shared_renderer.store_speed_ratio(slot, d.tempo.vinyl_speed, 1.0);
                                    d.tap.tap_times.clear();
                                    d.tap.last_tap_wall = None;
                                    d.metronome_mode = false;
                                    d.last_metro_beat = None;
                                } else {
                                    // Leaving vinyl mode: convert vinyl_speed to BPM adjustment.
                                    d.tempo.bpm = (d.tempo.base_bpm * d.tempo.vinyl_speed).clamp(40.0, 240.0);
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                    shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
                                    anchor_beat_grid_to_cue(d);
                                }
                            }
                        }
                        session.set_vinyl_mode(vinyl_mode);
                    }
                    Some(Action::LatencyDecrease)  => {
                        audio_latency_ms = (audio_latency_ms - 10).max(0);
                        session.set_latency(audio_latency_ms);
                    }
                    Some(Action::LatencyIncrease)  => {
                        audio_latency_ms = (audio_latency_ms + 10).min(250);
                        session.set_latency(audio_latency_ms);
                    }
                    Some(Action::FpsIncrease) => {
                        if let Some(pos) = FPS_LEVELS.iter().position(|&l| l == target_fps) {
                            if pos + 1 < FPS_LEVELS.len() { target_fps = FPS_LEVELS[pos + 1]; }
                        } else {
                            target_fps = snap_to_fps_level(target_fps);
                        }
                        fps_display.2 = target_fps;
                    }
                    Some(Action::FpsDecrease) => {
                        if let Some(pos) = FPS_LEVELS.iter().position(|&l| l == target_fps) {
                            if pos > 0 { target_fps = FPS_LEVELS[pos - 1]; }
                        } else {
                            target_fps = snap_to_fps_level(target_fps);
                        }
                        fps_display.2 = target_fps;
                    }
                    Some(Action::Deck1FilterIncrease) => { if let Some(ref mut d) = decks[0] { d.mixer.filter_offset = (d.mixer.filter_offset + 1).min(16);  d.audio.filter_offset_shared.store(d.mixer.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck1FilterDecrease) => { if let Some(ref mut d) = decks[0] { d.mixer.filter_offset = (d.mixer.filter_offset - 1).max(-16); d.audio.filter_offset_shared.store(d.mixer.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck1FilterReset)    => { if let Some(ref mut d) = decks[0] { d.mixer.filter_offset = 0; d.audio.filter_offset_shared.store(0, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterIncrease) => { if let Some(ref mut d) = decks[1] { d.mixer.filter_offset = (d.mixer.filter_offset + 1).min(16);  d.audio.filter_offset_shared.store(d.mixer.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterDecrease) => { if let Some(ref mut d) = decks[1] { d.mixer.filter_offset = (d.mixer.filter_offset - 1).max(-16); d.audio.filter_offset_shared.store(d.mixer.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterReset)    => { if let Some(ref mut d) = decks[1] { d.mixer.filter_offset = 0; d.audio.filter_offset_shared.store(0, Ordering::Relaxed); } }
                    Some(Action::Deck1FilterSlopeIncrease) => { if let Some(ref mut d) = decks[0] { if d.mixer.filter_poles < 4 { d.mixer.filter_poles += 2; d.audio.filter_poles.store(d.mixer.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::Deck1FilterSlopeDecrease) => { if let Some(ref mut d) = decks[0] { if d.mixer.filter_poles > 2 { d.mixer.filter_poles -= 2; d.audio.filter_poles.store(d.mixer.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::Deck2FilterSlopeIncrease) => { if let Some(ref mut d) = decks[1] { if d.mixer.filter_poles < 4 { d.mixer.filter_poles += 2; d.audio.filter_poles.store(d.mixer.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::Deck2FilterSlopeDecrease) => { if let Some(ref mut d) = decks[1] { if d.mixer.filter_poles > 2 { d.mixer.filter_poles -= 2; d.audio.filter_poles.store(d.mixer.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::Deck3FilterIncrease) => { if let Some(ref mut d) = decks[2] { d.mixer.filter_offset = (d.mixer.filter_offset + 1).min(16);  d.audio.filter_offset_shared.store(d.mixer.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck3FilterDecrease) => { if let Some(ref mut d) = decks[2] { d.mixer.filter_offset = (d.mixer.filter_offset - 1).max(-16); d.audio.filter_offset_shared.store(d.mixer.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck3FilterReset)    => { if let Some(ref mut d) = decks[2] { d.mixer.filter_offset = 0; d.audio.filter_offset_shared.store(0, Ordering::Relaxed); } }
                    Some(Action::Deck3FilterSlopeIncrease) => { if let Some(ref mut d) = decks[2] { if d.mixer.filter_poles < 4 { d.mixer.filter_poles += 2; d.audio.filter_poles.store(d.mixer.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::Deck3FilterSlopeDecrease) => { if let Some(ref mut d) = decks[2] { if d.mixer.filter_poles > 2 { d.mixer.filter_poles -= 2; d.audio.filter_poles.store(d.mixer.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::PaletteCycle) => {
                        scheme_idx = (scheme_idx + 1) % PALETTE_SCHEMES.len();
                        for slot in 0..3 {
                            if let Some(ref mut d) = decks[slot] {
                                d.display.palette = if slot == 0 { PALETTE_SCHEMES[scheme_idx].1 } else { PALETTE_SCHEMES[scheme_idx].2 };
                            }
                        }
                    }
                    Some(Action::ArtCycle) => {
                        art_bright_idx = [2u8, 0, 1][art_bright_idx as usize]; // dim→bright→off→dim
                        session.set_art_bright_idx(art_bright_idx);
                    }
                    Some(Action::OffsetIncrease) => {
                        if !vinyl_mode { if let Some(ref mut d) = decks[selected_deck] { apply_offset_step(d, 5); } }
                    }
                    Some(Action::OffsetDecrease) => {
                        if !vinyl_mode { if let Some(ref mut d) = decks[selected_deck] { apply_offset_step(d, -5); } }
                    }
                    Some(Action::ZoomOut) => { if zoom_idx > 0 { zoom_idx -= 1; } }
                    Some(Action::ZoomIn)  => { if zoom_idx + 1 < ZOOM_LEVELS.len() { zoom_idx += 1; } }
                    Some(Action::HeightDecrease) => { if detail_height > DET_MIN as usize { detail_height -= 1; } }
                    Some(Action::HeightIncrease) => { if detail_height < max_det_h { detail_height += 1; } }
                    Some(Action::BpmIncrease) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            if vinyl_mode || !d.tempo.bpm_established {
                                d.tempo.vinyl_speed = (d.tempo.vinyl_speed + 0.001).clamp(0.1, 4.0);
                                d.audio.player.set_speed(d.tempo.vinyl_speed);
                                shared_renderer.store_speed_ratio(selected_deck, d.tempo.vinyl_speed, 1.0);
                            } else {
                                d.tempo.bpm = (d.tempo.bpm + 0.1).min(240.0);
                                d.tempo.bpm_established = true;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                shared_renderer.store_speed_ratio(selected_deck, d.tempo.bpm, d.tempo.base_bpm);
                                anchor_beat_grid_to_cue(d);
                            }
                        }
                    }
                    Some(Action::BpmDecrease) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            if vinyl_mode || !d.tempo.bpm_established {
                                d.tempo.vinyl_speed = (d.tempo.vinyl_speed - 0.001).clamp(0.1, 4.0);
                                d.audio.player.set_speed(d.tempo.vinyl_speed);
                                shared_renderer.store_speed_ratio(selected_deck, d.tempo.vinyl_speed, 1.0);
                            } else {
                                d.tempo.bpm = (d.tempo.bpm - 0.1).max(40.0);
                                d.tempo.bpm_established = true;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                shared_renderer.store_speed_ratio(selected_deck, d.tempo.bpm, d.tempo.base_bpm);
                                anchor_beat_grid_to_cue(d);
                            }
                        }
                    }
                    Some(Action::JumpForward1bt)  => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,    0.5); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,    1); } } }
                    Some(Action::JumpBackward1bt) => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,   -0.5); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,   -1); } } }
                    Some(Action::JumpForward1b)   => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,    2.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,    4); } } }
                    Some(Action::JumpBackward1b)  => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,   -2.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,   -4); } } }
                    Some(Action::JumpForward4b)   => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,    8.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,   16); } } }
                    Some(Action::JumpBackward4b)  => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,   -8.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  -16); } } }
                    Some(Action::JumpForward8b)   => {
                        // Override: while loop mode is active on deck 3, this key trims loop_start +1ms.
                        if selected_deck == 2 && decks[2].as_ref().map_or(false, |d| d.loop_state.active) {
                            if let Some(ref mut d) = decks[2] {
                                let delta = (d.audio.sample_rate as usize) / 1000;
                                let candidate = d.loop_state.start_sample.saturating_add(delta);
                                d.loop_state.start_sample = candidate.min(d.loop_state.end_sample.saturating_sub(1));
                                let ch = d.audio.seek_handle.channels as usize;
                                d.audio.loop_start.store(d.loop_state.start_sample * ch, Ordering::SeqCst);
                                shared_renderer.store_loop(2, true, d.loop_state.start_sample, d.loop_state.end_sample);
                            }
                        } else if let Some(ref d) = decks[selected_deck] {
                            if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,   16.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,   32); }
                        }
                    }
                    Some(Action::JumpBackward8b)  => {
                        // Override: while loop mode is active on deck 3, this key trims loop_start −1ms.
                        if selected_deck == 2 && decks[2].as_ref().map_or(false, |d| d.loop_state.active) {
                            if let Some(ref mut d) = decks[2] {
                                let delta = (d.audio.sample_rate as usize) / 1000;
                                d.loop_state.start_sample = d.loop_state.start_sample.saturating_sub(delta);
                                let ch = d.audio.seek_handle.channels as usize;
                                d.audio.loop_start.store(d.loop_state.start_sample * ch, Ordering::SeqCst);
                                shared_renderer.store_loop(2, true, d.loop_state.start_sample, d.loop_state.end_sample);
                            }
                        } else if let Some(ref d) = decks[selected_deck] {
                            if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  -16.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  -32); }
                        }
                    }
                    Some(Action::JumpForward16b)  => {
                        // Override: while loop mode is active on deck 3, this key trims loop_end +1ms.
                        if selected_deck == 2 && decks[2].as_ref().map_or(false, |d| d.loop_state.active) {
                            if let Some(ref mut d) = decks[2] {
                                let delta = (d.audio.sample_rate as usize) / 1000;
                                d.loop_state.end_sample = d.loop_state.end_sample.saturating_add(delta);
                                let ch = d.audio.seek_handle.channels as usize;
                                d.audio.loop_end.store(d.loop_state.end_sample * ch, Ordering::SeqCst);
                                shared_renderer.store_loop(2, true, d.loop_state.start_sample, d.loop_state.end_sample);
                            }
                        } else if let Some(ref d) = decks[selected_deck] {
                            if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,   32.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,   64); }
                        }
                    }
                    Some(Action::JumpBackward16b) => {
                        // Override: while loop mode is active on deck 3, this key trims loop_end −1ms.
                        if selected_deck == 2 && decks[2].as_ref().map_or(false, |d| d.loop_state.active) {
                            if let Some(ref mut d) = decks[2] {
                                let delta = (d.audio.sample_rate as usize) / 1000;
                                let candidate = d.loop_state.end_sample.saturating_sub(delta);
                                d.loop_state.end_sample = candidate.max(d.loop_state.start_sample + 1);
                                let ch = d.audio.seek_handle.channels as usize;
                                d.audio.loop_end.store(d.loop_state.end_sample * ch, Ordering::SeqCst);
                                shared_renderer.store_loop(2, true, d.loop_state.start_sample, d.loop_state.end_sample);
                            }
                        } else if let Some(ref d) = decks[selected_deck] {
                            if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  -32.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  -64); }
                        }
                    }
                    Some(Action::JumpForward32b)  => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,   64.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  128); } } }
                    Some(Action::JumpBackward32b) => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  -64.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -128); } } }
                    Some(Action::JumpForward64b)  => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  128.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  256); } } }
                    Some(Action::JumpBackward64b) => { if let Some(ref d) = decks[selected_deck] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, -128.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -256); } } }
                    Some(Action::SpeedReset) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            d.tempo.vinyl_speed = 1.0;
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.audio.player.set_speed(1.0);
                            if vinyl_mode {
                                shared_renderer.store_speed_ratio(selected_deck, 1.0, 1.0);
                            } else {
                                shared_renderer.store_speed_ratio(selected_deck, d.tempo.bpm, d.tempo.base_bpm);
                            }
                        }
                    }
                    Some(Action::BpmTap) => {
                        if !vinyl_mode {
                            if let Some(ref mut d) = decks[selected_deck] {
                                if !d.audio.player.is_paused() {
                                    let now = Instant::now();
                                    if let Some(last) = d.tap.last_tap_wall {
                                        if now.duration_since(last).as_secs_f64() > 2.0 { d.tap.tap_times.clear(); }
                                    }
                                    let display_samp = render[selected_deck].as_ref().map_or(d.display.smooth_display_samp, |rs| rs.display_samp);
                                    d.tap.tap_times.push(display_samp / d.audio.sample_rate as f64);
                                    d.tap.last_tap_wall = Some(now);
                                    if d.tap.tap_times.len() >= 8 {
                                        let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
                                        let tapped_offset = (tapped_offset_raw as f64 / 10.0).round() as i64 * 10;
                                        let speed_ratio = d.tempo.bpm / d.tempo.base_bpm;
                                        d.tempo.base_bpm = tapped_bpm;
                                        d.tempo.bpm = (d.tempo.base_bpm * speed_ratio).clamp(40.0, 240.0);
                                        d.tempo.offset_ms = tapped_offset;
                                        d.tempo.bpm_established = true;
                                        d.tempo.offset_established = true;
                                        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                        shared_renderer.store_speed_ratio(selected_deck, d.tempo.bpm, d.tempo.base_bpm);
                                    }
                                }
                            }
                        }
                    }
                    Some(Action::Cue) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            if d.audio.player.is_paused() {
                                let raw_samp = d.display.smooth_display_samp as usize;
                                d.cue_sample = Some(raw_samp);
                                anchor_beat_grid_to_cue(d);
                                if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                    track_data.set(hash.clone(), cache_entry_for_deck(d));
                                }
                            }
                        }
                    }
                    Some(Action::CuePlay) => {
                        if let Some(ref mut d) = decks[selected_deck] {
                            if let Some(cue_samp) = d.cue_sample {
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                    d.display.smooth_display_samp = cue_samp as f64;
                                } else {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    let target_samp = (cue_samp + latency_samps).min(d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize);
                                    d.audio.seek_handle.seek_to(target_samp as f64 / d.audio.sample_rate as f64);
                                }
                            }
                        }
                    }
                    Some(Action::NudgeBackward) | Some(Action::NudgeForward)
                    | Some(Action::NudgeModeToggle)
                    | Some(Action::BaseBpmIncrease) | Some(Action::BaseBpmDecrease) => {}
                    Some(Action::LoopTap) => {
                        // PoC: loop mode lives on deck 3 only. Pure tap key — no exit overload.
                        if selected_deck == 2 {
                            if let Some(ref mut d) = decks[selected_deck] {
                                if !d.loop_state.active && !d.audio.player.is_paused() {
                                    let now = Instant::now();
                                    // Two-second safety reset if the last tap was a while ago.
                                    if let Some(last) = d.loop_tap.last_tap_wall {
                                        if now.duration_since(last).as_secs_f64() > 2.0 {
                                            d.loop_tap.tap_times.clear();
                                        }
                                    }
                                    let display_samp = render[selected_deck]
                                        .as_ref()
                                        .map_or(d.display.smooth_display_samp, |rs| rs.display_samp);
                                    if d.loop_tap.tap_times.is_empty() {
                                        // First tap marks the loop start (mono frame index).
                                        d.loop_state.start_sample = display_samp as usize;
                                    }
                                    d.loop_tap.tap_times.push(display_samp / d.audio.sample_rate as f64);
                                    d.loop_tap.last_tap_wall = Some(now);
                                }
                            }
                        }
                    }
                    Some(Action::LoopExit) => {
                        // PoC: deck 3 only. Exit loop mode.
                        if selected_deck == 2 {
                            if let Some(ref mut d) = decks[selected_deck] {
                                if d.loop_state.active {
                                    d.loop_state.active = false;
                                    d.loop_tap.tap_times.clear();
                                    d.audio.loop_active.store(false, Ordering::SeqCst);
                                    shared_renderer.store_loop(2, false, 0, 0);
                                }
                            }
                        }
                    }
                    None => {}
                    }
                } // end if Press
            }
            _ => {}
            }
        }

        let sleep_start = Instant::now();
        thread::sleep(frame_dur.saturating_sub(frame_start.elapsed()));
        if let Some(rec) = recorder.as_mut() {
            rec.record_frame(frame_start, service_dur, draw_dur, frame_dur, sleep_start.elapsed());
        }
    }
}

fn service_deck_frame(
    slot: usize,
    decks: &mut [Option<Deck>; 3],
    col_secs: f64,
    elapsed: f64,
    elapsed_uncapped: f64,
    mixer: &rodio::mixer::Mixer,
    shared_renderer: &SharedDetailRenderer,
    track_data: &mut TrackDatabase,
    audio_latency_ms: i64,
    vinyl_mode: bool,
) {
    let Some(ref mut d) = decks[slot] else { return; };

    shared_renderer.store_gain(slot, f32::from_bits(d.audio.gain_linear.load(Ordering::Relaxed)));

    // Auto-reject pending BPM confirmation after 15 seconds.
    if let Some((_, _, _, received_at)) = &d.tempo.pending_bpm {
        if received_at.elapsed().as_secs() >= 15 {
            d.tempo.pending_bpm = None;
        }
    }

    // Expire per-deck active notification.
    if d.active_notification.as_ref().map_or(false, |n| Instant::now() >= n.expires) {
        d.active_notification = None;
    }

    // Poll BPM detection results.
    if let Ok((hash, new_bpm, new_offset, is_fresh)) = d.tempo.bpm_rx.try_recv() {
        if !is_fresh || !d.tempo.bpm_established {
            d.tempo.bpm      = new_bpm;
            d.tempo.base_bpm = new_bpm;
            shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
            d.tempo.offset_ms = (new_offset as f64 / 10.0).round() as i64 * 10;
            if hash.is_empty() {
                // Unhashable track (content identity unavailable): it plays, but persists
                // nothing and can't be referenced by a playlist. The load thread already
                // recorded an error report; warn in this deck's notification row.
                d.cue_sample = None;
                d.tempo.offset_established = false;
                d.mixer.gain_db = 0;
                d.audio.gain_linear.store(1.0f32.to_bits(), Ordering::Relaxed);
                d.active_notification = Some(notification(
                    "Content identity unavailable — not saved, unusable in playlists",
                    NotificationStyle::Error,
                ));
                d.tempo.analysis_hash = None;
            } else {
                // Restore cue_sample, offset_established, and gain_db from the track database if present.
                d.cue_sample = track_data.get(hash.as_str()).and_then(|e| e.cue_sample);
                d.tempo.offset_established = track_data.get(hash.as_str()).map_or(false, |e| e.offset_established);
                d.mixer.gain_db = track_data.get(hash.as_str()).map_or(0, |e| e.gain_db);
                d.audio.gain_linear.store(10f32.powf(d.mixer.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                if !vinyl_mode {
                    if let Some(cue_samp) = d.cue_sample {
                        let cue_secs = cue_samp as f64 / d.audio.sample_rate as f64;
                        d.audio.seek_handle.seek_direct(cue_secs);
                    }
                }
                track_data.set(hash.clone(), cache_entry_for_deck(d));
                d.tempo.analysis_hash = Some(hash);
            }
            d.tempo.analysis_settled   = true;
            if !is_fresh || d.tempo.redetecting { d.tempo.bpm_established = true; }
            d.tempo.redetecting        = false;
            d.tempo.redetect_saved_hash = None;
            d.tempo.background_rx      = None;
        } else {
            d.tempo.analysis_hash      = Some(hash.clone());
            d.tempo.analysis_settled   = true;
            d.tempo.redetecting        = false;
            d.tempo.redetect_saved_hash = None;
            d.tempo.background_rx      = None;
            d.tempo.pending_bpm        = Some((hash, new_bpm, new_offset, Instant::now()));
        }
    }

    // Actual playback position from TrackingSource — used for end-of-track detection.
    let pos_raw  = d.audio.seek_handle.position.load(Ordering::Relaxed);
    let pos_samp = pos_raw / d.audio.seek_handle.channels as usize;
    let total_mono_samps = d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize;

    // End-of-track: pause and reset to cue point if set, otherwise start.
    let at_end = pos_samp >= total_mono_samps;
    if at_end && !d.audio.player.is_paused() {
        d.audio.player.pause();
        let (reset_secs, reset_samp) = d.cue_sample
            .map(|s| (s as f64 / d.audio.sample_rate as f64, s as f64))
            .unwrap_or((0.0, 0.0));
        d.audio.seek_handle.seek_direct(reset_secs);
        d.display.smooth_display_samp = reset_samp;
        // On a playlist deck with a further entry, signal the main loop to advance.
        if let Some(pl) = d.playlist.as_mut() {
            if pl.index + 1 < pl.playlist.entries.len() {
                pl.advance_requested = true;
            }
        }
        return;
    }

    // Advance smooth display position.
    if !d.audio.player.is_paused() {
        // Include warp-nudge speed factor so the display tracks the audio speed exactly.
        let base_speed = if vinyl_mode { d.tempo.vinyl_speed as f64 } else { (d.tempo.bpm / d.tempo.base_bpm) as f64 };
        let speed = base_speed * (1.0 + d.nudge as f64 * 0.1);
        // Integrate the measured frame interval. Each interval is frame_start minus the
        // previous frame_start, so the sum telescopes to exact wall time — sleep jitter
        // cannot compound. Adding to the running value (rather than recomputing from an
        // anchor) lets the drift damper below accumulate frame to frame.
        d.display.smooth_display_samp += elapsed_uncapped * d.audio.sample_rate as f64 * speed;
    } else if d.nudge != 0 {
        // Paused with warp nudge: drift display and sync actual audio position for scrubbing.
        d.display.smooth_display_samp = (d.display.smooth_display_samp
            + elapsed * d.audio.sample_rate as f64 * d.nudge as f64 * 0.1)
            .clamp(0.0, total_mono_samps as f64);
        d.audio.seek_handle.set_position(d.display.smooth_display_samp / d.audio.sample_rate as f64);
        // Fire a scrub snippet once per half-column advance.
        let scrub_spc = match slot {
            0 => shared_renderer.shared_a.lock().unwrap().samples_per_col,
            1 => shared_renderer.shared_b.lock().unwrap().samples_per_col,
            _ => shared_renderer.shared_c.lock().unwrap().samples_per_col,
        };
        let half_samples_per_col = (scrub_spc / 2).max(1);
        if scrub_spc > 0
            && (d.display.smooth_display_samp - d.display.last_scrub_samp).abs() >= half_samples_per_col as f64
        {
            scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                        d.audio.sample_rate, d.display.smooth_display_samp as usize, half_samples_per_col);
            d.display.last_scrub_samp = d.display.smooth_display_samp;
        }
    }

    // Drift correction — use output_position to avoid the 512-sample batch-read jumps in
    // TrackingSource when pitch is active, which would otherwise cause the display to snap.
    let display_pos_samp = d.audio.seek_handle.output_position.load(Ordering::Relaxed)
        / d.audio.seek_handle.channels as usize;
    let drift = d.display.smooth_display_samp - display_pos_samp as f64;
    // 0.1s: far above damper steady-state drift and audio batch-read noise (< ~25 ms), but
    // below the smallest deliberate seek (a 1-beat jump at 240 BPM is 0.25 s), so every seek
    // snaps immediately instead of gliding in through the damper.
    let large_drift = drift.abs() > d.audio.sample_rate as f64 * 0.1;
    let paused_snap  = d.audio.player.is_paused() && d.nudge == 0 && drift.abs() > 1.0;
    if large_drift || paused_snap {
        // Snap to nearest half-column so sub_col is stable after seeks.
        let speed = if vinyl_mode { d.tempo.vinyl_speed as f64 } else { (d.tempo.bpm / d.tempo.base_bpm) as f64 };
        let col_samp_f64 = col_secs * d.audio.sample_rate as f64 * speed;
        let half_col = col_samp_f64 / 2.0;
        d.display.smooth_display_samp = if half_col > 0.0 {
            (display_pos_samp as f64 / half_col).round() * half_col
        } else {
            display_pos_samp as f64
        };
    } else if !d.audio.player.is_paused() {
        // The correction persists: the integration above adds to the running value rather than
        // recomputing it, so damping accumulates and system-clock vs audio-clock skew settles
        // at rate × skew / (0.002 × fps) — well under a millisecond at typical ppm-level skew.
        // A larger factor would amplify audio-device batch-read noise into visible flicker.
        d.display.smooth_display_samp -= drift * 0.002;
    }

    // Metronome: fire from buffer write position so the click arrives at the speaker on the beat.
    let beat_period = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
    let metro_beat_index = {
        let ns = (d.display.smooth_display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
            - d.tempo.offset_ms as i128 * 1_000_000;
        ns.div_euclid(beat_period.as_nanos() as i128)
    };
    if d.metronome_mode && !d.audio.player.is_paused() {
        if d.last_metro_beat != Some(metro_beat_index) {
            play_click_tone(mixer, d.audio.sample_rate);
            d.last_metro_beat = Some(metro_beat_index);
        }
    } else {
        d.last_metro_beat = None;
    }

    // Tap session timeout: finalise BPM when the user stops tapping.
    let tap_active_now = !d.tap.tap_times.is_empty()
        && d.tap.last_tap_wall.map_or(false, |t| t.elapsed().as_secs_f64() < 2.0);
    if d.tap.was_tap_active && !tap_active_now && d.tap.tap_times.len() >= 8 {
        let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
        let tapped_offset  = (tapped_offset_raw as f64 / 10.0).round() as i64 * 10;
        let speed_ratio    = d.tempo.bpm / d.tempo.base_bpm;
        d.tempo.base_bpm   = tapped_bpm;
        d.tempo.bpm        = (d.tempo.base_bpm * speed_ratio).clamp(40.0, 240.0);
        d.tempo.offset_ms  = tapped_offset;
        d.tempo.bpm_established = true;
        d.tempo.offset_established = true;
        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
        shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
        if let Some(ref hash) = d.tempo.analysis_hash {
            track_data.set(hash.clone(), cache_entry_for_deck(d));
        }
    }
    d.tap.was_tap_active = tap_active_now;

    // Loop tap session timeout: activate loop mode (deck 3 PoC) when tapping stops.
    // Timeout: 1.0 s of inactivity since the last tap.
    let loop_tap_active_now = !d.loop_tap.tap_times.is_empty()
        && d.loop_tap.last_tap_wall.map_or(false, |t| t.elapsed().as_secs_f64() < 1.0);
    if d.loop_tap.was_tap_active && !loop_tap_active_now && slot == 2 {
        let tap_count = d.loop_tap.tap_times.len();
        if tap_count >= 4 {
            let (tapped_bpm, _) = compute_tap_bpm_offset(&d.loop_tap.tap_times);
            let beat_period_samples = (60.0 / tapped_bpm as f64 * d.audio.sample_rate as f64) as usize;
            // Round up to nearest power of 2 bars; loop length in beats = bars × 4.
            let bars = ((tap_count as f64) / 4.0).ceil() as usize;
            let bars_pow2 = bars.next_power_of_two().max(1);
            let loop_beats = bars_pow2 * 4;
            let loop_len_mono = loop_beats * beat_period_samples;
            d.loop_state.end_sample = d.loop_state.start_sample + loop_len_mono;
            d.loop_state.active = true;
            // Push to audio thread as interleaved-sample indices (frame × channels).
            let ch = d.audio.seek_handle.channels as usize;
            d.audio.loop_start.store(d.loop_state.start_sample * ch, Ordering::SeqCst);
            d.audio.loop_end.store(d.loop_state.end_sample * ch, Ordering::SeqCst);
            d.audio.loop_active.store(true, Ordering::SeqCst);
            shared_renderer.store_loop(2, true, d.loop_state.start_sample, d.loop_state.end_sample);
        }
        d.loop_tap.tap_times.clear();
    }
    d.loop_tap.was_tap_active = loop_tap_active_now;

    // Spectrum analyser: chars every half beat, background glow every 8 beats.
    let analysing   = !d.tempo.analysis_settled || d.tempo.redetecting;
    let half_period = if analysing { Duration::from_millis(250) } else { beat_period / 4 };
    let bar_period  = beat_period * 8;
    let chars_due   = d.spectrum.last_update.map_or(true,    |t| t.elapsed() >= half_period);
    let bg_due      = d.spectrum.last_bg_update.map_or(true, |t| t.elapsed() >= bar_period);
    if chars_due || bg_due {
        let latency_correction = if d.audio.player.is_paused() { 0.0 } else { audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0 };
        let display_pos_samp = (d.display.smooth_display_samp - latency_correction).max(0.0) as usize;
        let spectrum_start = Instant::now();
        let (new_chars, new_bg) = compute_spectrum(&d.audio.mono, display_pos_samp, d.audio.sample_rate, d.mixer.filter_offset);
        frame_stats::note_spectrum(spectrum_start.elapsed());
        if chars_due {
            d.spectrum.chars = new_chars;
            for i in 0..16 { d.spectrum.bg_accum[i] |= new_bg[i]; }
            d.spectrum.bg = d.spectrum.bg_accum;
            d.spectrum.last_update = Some(Instant::now());
        }
        if bg_due {
            d.spectrum.bg_accum = [false; 16];
            d.spectrum.last_bg_update = Some(Instant::now());
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// A normal tag write must not change the content identity — otherwise the
    /// check would raise a false incident on every edit. Verifies no incident and
    /// no leftover staging.
    #[test]
    fn tag_write_preserves_identity() {
        let tmp = std::env::temp_dir().join(format!("deck-idcheck-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // SAFETY: single-threaded test setup; no other test reads XDG_STATE_HOME.
        unsafe { std::env::set_var("XDG_STATE_HOME", &tmp); }

        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/resilient-playlists/corpus/clean.flac");
        let file = tmp.join("track.flac");
        std::fs::copy(src, &file).unwrap();

        let fields: Vec<(String, usize)> = ["Artist", "Title", "", "", "", "", ""]
            .iter().map(|s| (s.to_string(), 0)).collect();
        let incident = write_tags_verified(&file, &fields).unwrap();
        assert_eq!(incident, None, "a normal tag write must not change identity");

        let base = error_reports::dir();
        let entries: Vec<_> = std::fs::read_dir(&base).map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).collect()).unwrap_or_default();
        assert!(entries.is_empty(), "staging/incident left behind: {entries:?}");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Demonstration of the unhappy path: forces a mismatch and prints the incident
    /// folder it produces. Run with `cargo test demo_identity_incident -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn demo_identity_incident() {
        let tmp = std::env::temp_dir().join(format!("deck-iddemo-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // SAFETY: run alone (ignored); no other test observes these vars.
        unsafe {
            std::env::set_var("XDG_STATE_HOME", &tmp);
            std::env::set_var("DECK_SIMULATE_IDENTITY_FAULT", "1");
        }
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/resilient-playlists/corpus/clean.flac");
        let file = tmp.join("Some Track.flac");
        std::fs::copy(src, &file).unwrap();
        let fields: Vec<(String, usize)> = ["Artist", "Title", "", "", "", "", ""]
            .iter().map(|s| (s.to_string(), 0)).collect();

        let incident = write_tags_verified(&file, &fields).unwrap()
            .expect("fault injection should produce an incident");
        println!("\n=== incident folder: {} ===", incident.display());
        for e in std::fs::read_dir(&incident).unwrap().filter_map(|e| e.ok()) {
            let len = e.metadata().map(|m| m.len()).unwrap_or(0);
            println!("  {}  ({len} bytes)", e.file_name().to_string_lossy());
        }
        println!("--- details.txt ---\n{}", std::fs::read_to_string(incident.join("details.txt")).unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
