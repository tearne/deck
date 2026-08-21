use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI8, AtomicU8, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use ratatui::layout::Rect;
use rodio::Player;

use crate::audio::{butterworth_biquad, SeekHandle, WaveformData, FILTER_CUTOFFS_HZ};
use crate::cache::{CacheEntry, DeckSnapshot, Grid};
use crate::config::{Action, KeyBinding};
use crossterm::event::KeyCode;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub(crate) type Rgb = (u8, u8, u8);
/// Four-stop spectral palette: (treble, mid-treble, mid-bass, bass).
/// Stops are ordered from treble (bass_ratio=0) to bass (bass_ratio=1).
pub(crate) type SpecPalette = (Rgb, Rgb, Rgb, Rgb);

/// Colour schemes: (name, deck_a_palette, deck_b_palette).
/// The first scheme is the default; cycling rotates through all.
pub(crate) const PALETTE_SCHEMES: &[(&str, SpecPalette, SpecPalette)] = &[
    ("amber/cyan",
     ((  0, 255, 255), (  0, 255, 120), (220, 255,   0), (255, 120,   0)),  // cyan → teal → gold → amber
     ((  0, 255, 255), (  0, 255, 120), (220, 255,   0), (255, 120,   0))), // B: same
];


pub(crate) struct DeckAudio {
    pub(crate) player: Player,
    pub(crate) seek_handle: SeekHandle,
    pub(crate) mono: Arc<Vec<f32>>,
    pub(crate) waveform: Arc<WaveformData>,
    pub(crate) sample_rate: u32,
    pub(crate) filter_offset_shared: Arc<AtomicI32>,
    pub(crate) filter_state_reset: Arc<AtomicBool>,
    pub(crate) pfl_level: Arc<AtomicU8>,
    /// Deck fader volume as f32 bits; read by FilterSource on the right channel when PFL is active.
    pub(crate) deck_volume_atomic: Arc<AtomicU32>,
    /// Gain trim as f32 bits (linear multiplier); applied pre-fader in FilterSource.
    pub(crate) gain_linear: Arc<AtomicU32>,
    /// Filter slope: 2 = 12 dB/oct, 4 = 24 dB/oct.
    pub(crate) filter_poles: Arc<AtomicU8>,
    /// Pitch shift in semitones (±6); shared with PitchSource on the audio thread.
    pub(crate) pitch_semitones: Arc<AtomicI8>,
}

pub(crate) struct TempoState {
    pub(crate) bpm: f32,
    pub(crate) base_bpm: f32,
    pub(crate) offset_ms: i64,
    pub(crate) bpm_rx: std::sync::mpsc::Receiver<(String, f32, i64, bool, Option<String>)>,
    pub(crate) analysis_hash: Option<String>,
    /// True once the initial load analysis has returned (identified or unhashable).
    /// Drives the "analysing" spinner independently of whether a key was produced,
    /// so an unhashable track settles instead of spinning forever.
    pub(crate) analysis_settled: bool,
    pub(crate) bpm_established: bool,
    /// Absolute playback speed multiplier (1.0 = nominal). Used in Playback mode and
    /// when no BPM is established. Independent of BPM state; passed directly to `player.set_speed`.
    pub(crate) playback_speed: f32,
}

pub(crate) struct TapState {
    pub(crate) tap_times: Vec<f64>,
    pub(crate) last_tap_wall: Option<Instant>,
    pub(crate) was_tap_active: bool,
}

/// Everything the rendered overview depends on. The overview is rebuilt only
/// when this key changes — a few times per second (playhead column, flash
/// states) rather than at frame rate.
#[derive(PartialEq)]
pub(crate) struct OverviewKey {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) playhead_col: usize,
    pub(crate) cue_col: Option<usize>,
    pub(crate) ghosts: Vec<(usize, char)>,
    pub(crate) analysing: bool,
    pub(crate) warning_active: bool,
    pub(crate) warn_beat_on: bool,
    pub(crate) gain_db: i8,
    pub(crate) base_bpm_bits: u32,
    pub(crate) offset_ms: i64,
    pub(crate) palette: SpecPalette,
}

pub(crate) struct OverviewCache {
    pub(crate) key: OverviewKey,
    pub(crate) paragraph: ratatui::widgets::Paragraph<'static>,
}

pub(crate) struct DisplayState {
    pub(crate) smooth_display_samp: f64,
    pub(crate) last_scrub_samp: f64,
    pub(crate) last_viewport_start: usize,
    pub(crate) overview_rect: Rect,
    pub(crate) last_bar_cols: Vec<usize>,
    pub(crate) last_bar_times: Vec<f64>,
    pub(crate) palette: SpecPalette,
    pub(crate) overview_cache: Option<OverviewCache>,
}

pub(crate) struct SpectrumState {
    pub(crate) chars: [char; SPECTRUM_CHARS],
    pub(crate) bg: [bool; SPECTRUM_CHARS],
    pub(crate) bg_accum: [bool; SPECTRUM_CHARS],
    pub(crate) last_update: Option<Instant>,
    pub(crate) last_bg_update: Option<Instant>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum NudgeMode { Jump, Warp }

/// A deck's operating mode. Playback ignores the beat grid entirely; Beat
/// unlocks BPM-relative jumps and displays. Clip joins with clip-mode-core.
/// Per deck, remembered per track.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DeckMode { Playback, Beat }

pub(crate) const TAG_FIELD_LABELS: &[&str] = &[
    " Artist", "  Title", "  Album", "   Year", "  Track", "  Genre", "Comment",
];

pub(crate) struct TagEditorState {
    pub(crate) fields:          Vec<(String, usize)>,
    pub(crate) active_field:    usize,
    /// Whether saving also renames the file to match the tags. On by default;
    /// off makes the save tags-only. The ninth focus stop toggles it.
    pub(crate) rename_enabled:  bool,
    /// Directory of the file being edited, so save can write and rename without
    /// reference to any deck.
    pub(crate) dir:             std::path::PathBuf,
    pub(crate) current_stem:    String,
    pub(crate) extension:       String,
    pub(crate) collision_error: Option<String>,
}

impl TagEditorState {
    /// Open the editor for an existing track, seeded from its tags and filename.
    /// `None` when the tags can't be read — opening with blank fields would invite
    /// the operator to save those blanks over the file's real tags.
    pub(crate) fn for_track(path: &std::path::Path) -> Option<Self> {
        let fields = crate::tags::read_tags_for_editor(path)?
            .into_iter()
            .map(|v| (v, 0))
            .collect();
        let dir = path.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf();
        let current_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string();
        Some(TagEditorState { fields, active_field: 0, rename_enabled: true, dir, current_stem, extension, collision_error: None })
    }

    /// The file's current full path (before any pending rename).
    pub(crate) fn current_path(&self) -> std::path::PathBuf {
        if self.extension.is_empty() {
            self.dir.join(&self.current_stem)
        } else {
            self.dir.join(format!("{}.{}", self.current_stem, self.extension))
        }
    }

    /// The full path the current field values would rename the file to.
    pub(crate) fn target_path(&self) -> std::path::PathBuf {
        let stem = self.preview();
        if self.extension.is_empty() {
            self.dir.join(stem)
        } else {
            self.dir.join(format!("{stem}.{}", self.extension))
        }
    }

    pub(crate) fn active_field_mut(&mut self) -> (&mut String, &mut usize) {
        let (val, cur) = &mut self.fields[self.active_field];
        (val, cur)
    }
    pub(crate) fn preview(&self) -> String {
        let a = self.fields[0].0.trim();
        let t = self.fields[1].0.trim();
        format!("{t} - {a}")
    }
}

pub(crate) struct Mixer {
    pub(crate) volume: f32,
    pub(crate) gain_db: i8,
    pub(crate) pfl_level: u8,
    pub(crate) filter_offset: i32,
    pub(crate) filter_poles: u8,
}

/// A playlist loaded on a deck: the parsed `.rpl`, the file it came from (for
/// resilient rewrites), and the index of the entry currently on the deck.
pub(crate) struct ActivePlaylist {
    pub(crate) playlist: crate::playlist::Playlist,
    pub(crate) path: std::path::PathBuf,
    pub(crate) index: usize,
    /// How many entries couldn't be played as of the last resolution — the deck's
    /// answer to "is there a problem in this set?". The browser says which.
    pub(crate) unplayable: usize,
    /// One-shot: set at end-of-track when a next entry exists, consumed by the
    /// main loop to trigger the auto-advance load.
    pub(crate) advance_requested: bool,
}

pub(crate) struct Deck {
    pub(crate) filename: String,
    pub(crate) path:     std::path::PathBuf,
    pub(crate) track_name: String,
    pub(crate) total_duration: f64,
    pub(crate) pitch_semitones: i8,
    pub(crate) nudge: i8,
    pub(crate) nudge_mode: NudgeMode,
    pub(crate) mode: DeckMode,
    pub(crate) metronome_mode: bool,
    pub(crate) last_metro_beat: Option<i128>,
    pub(crate) cue_sample: Option<usize>,
    /// The grid's phase datum, in mono samples — pinned in the detached view,
    /// persisted per track. Supersedes the cue as datum; offset derives from it.
    pub(crate) anchor_sample: Option<usize>,
    pub(crate) rename_hint: Option<String>,
    pub(crate) rename_offer_started: Option<Instant>,
    pub(crate) rename_accepted: Option<String>,
    pub(crate) cover_art: Option<Vec<u8>>,
    pub(crate) cover_art_cache: Option<(u16, u16, u8, ratatui::widgets::Paragraph<'static>)>, // (cols, rows, bright_idx, rendered art)
    /// Set when the deck was loaded from a playlist; drives auto-advance and the
    /// position indicator. Dropped when a standalone track is loaded or the deck clears.
    pub(crate) playlist: Option<ActivePlaylist>,
    /// A session restore's transport state, waiting for the load-time grid result
    /// so speed lands on the right BPM and position outlives the cue seek.
    pub(crate) restore_transport: Option<RestoreTransport>,

    pub(crate) audio: DeckAudio,
    pub(crate) mixer: Mixer,
    pub(crate) tempo: TempoState,
    pub(crate) tap: TapState,
    pub(crate) display: DisplayState,
    pub(crate) spectrum: SpectrumState,
}

impl Deck {
    pub(crate) fn new(
        filename: String,
        path: std::path::PathBuf,
        track_name: String,
        total_duration: f64,
        rename_hint: Option<String>,
        audio: DeckAudio,
        bpm_rx: std::sync::mpsc::Receiver<(String, f32, i64, bool, Option<String>)>,
    ) -> Self {
        Deck {
            filename,
            path,
            track_name,
            total_duration,
            mixer: Mixer { volume: 1.0, gain_db: 0, pfl_level: 0, filter_offset: 0, filter_poles: 2 },
            pitch_semitones: 0,
            nudge: 0,
            nudge_mode: NudgeMode::Jump,
            mode: DeckMode::Beat,
            metronome_mode: false,
            last_metro_beat: None,
            cue_sample: None,
            anchor_sample: None,
            rename_offer_started: rename_hint.as_ref().map(|_| Instant::now()),
            rename_hint,
            rename_accepted: None,
            cover_art: None,
            cover_art_cache: None,
            playlist: None,
            restore_transport: None,
            audio,
            tempo: TempoState {
                bpm: 120.0,
                base_bpm: 120.0,
                offset_ms: 0,
                bpm_rx,
                analysis_hash: None,
                analysis_settled: false,
                bpm_established: false,
                playback_speed: 1.0,
            },
            tap: TapState {
                tap_times: Vec::new(),
                last_tap_wall: None,
                was_tap_active: false,
            },
            display: DisplayState {
                smooth_display_samp: 0.0,
                last_scrub_samp: -1.0,
                last_viewport_start: 0,
                overview_rect: Rect::default(),
                last_bar_cols: Vec::new(),
                last_bar_times: Vec::new(),
                palette: PALETTE_SCHEMES[0].1, // corrected to slot-specific palette on load
                overview_cache: None,
            },
            spectrum: SpectrumState {
                chars: ['\u{2800}'; SPECTRUM_CHARS],
                bg: [false; SPECTRUM_CHARS],
                bg_accum: [false; SPECTRUM_CHARS],
                last_update: None,
                last_bg_update: None,
            },
        }
    }

    pub(crate) fn rename_offer_active(&self) -> bool {
        self.rename_offer_started.is_some()
            && self.rename_hint.is_some()
            && self.rename_accepted.is_none()
    }
}

/// Compute BPM and phase offset from a list of tap times (track position in seconds).
/// BPM = linear regression slope across all taps (beat index vs time), which converges
/// as taps accumulate — later taps add leverage and reduce variance.
/// Outlier taps (residual > half a beat period) are dropped before the final regression.
/// Offset = mean residual anchored to the first tap, avoiding phase drift from imprecise period.
pub(crate) fn compute_tap_bpm_offset(tap_times: &[f64]) -> (f32, i64) {
    let n = tap_times.len();
    if n < 2 { return (120.0, 0); }

    // First pass: regression over all taps to get a rough period for outlier detection.
    let beat_period = linear_regression_period(tap_times);
    if beat_period <= 0.0 { return (120.0, 0); }

    // Drop taps whose residual from the regression line exceeds half a beat period.
    let t0 = tap_times[0];
    let filtered: Vec<f64> = tap_times.iter().enumerate()
        .filter(|&(i, &t)| {
            let expected = t0 + i as f64 * beat_period;
            (t - expected).abs() < beat_period / 2.0
        })
        .map(|(_, &t)| t)
        .collect();
    let taps = if filtered.len() >= 2 { &filtered[..] } else { tap_times };

    // Second pass: refined regression on filtered taps.
    let beat_period = linear_regression_period(taps);
    if beat_period <= 0.0 { return (120.0, 0); }
    let bpm = (60.0 / beat_period) as f32;

    // Anchor residuals to the first tap so deltas are small.
    // Computing t % beat_period on large absolute positions causes phase drift when
    // beat_period is even slightly imprecise — error accumulates with distance from zero.
    let t0 = taps[0];
    let mean_residual = taps.iter()
        .map(|&t| { let d = t - t0; d - (d / beat_period).round() * beat_period })
        .sum::<f64>() / taps.len() as f64;
    let offset_secs = (t0 + mean_residual).rem_euclid(beat_period);
    let offset_ms = (offset_secs * 1000.0).round() as i64;
    (bpm.clamp(40.0, 240.0), offset_ms)
}

pub(crate) fn linear_regression_period(tap_times: &[f64]) -> f64 {
    let n = tap_times.len();
    let x_mean = (n - 1) as f64 / 2.0;
    let y_mean = tap_times.iter().sum::<f64>() / n as f64;
    let num: f64 = tap_times.iter().enumerate()
        .map(|(i, &y)| (i as f64 - x_mean) * (y - y_mean))
        .sum();
    let den: f64 = (0..n).map(|i| (i as f64 - x_mean).powi(2)).sum();
    if den <= 0.0 { return 0.0; }
    num / den
}

/// After a BPM change, re-anchor `offset_ms` so the beat grid stays aligned to
/// the cue position. With no cue set this is a no-op.
/// Re-derive the grid's phase from its datum: the anchor when pinned (1 ms
/// precision — it is the phase source), else the cue (10 ms convention), else
/// leave the offset be. Called whenever the base BPM changes.
pub(crate) fn rederive_grid_phase(d: &mut Deck) {
    let beat_period_ms = 60_000.0 / d.tempo.base_bpm as f64;
    if let Some(anchor) = d.anchor_sample {
        let anchor_ms = anchor as f64 / d.audio.sample_rate as f64 * 1000.0;
        d.tempo.offset_ms = anchor_ms.rem_euclid(beat_period_ms).round() as i64;
    } else if let Some(cue_samp) = d.cue_sample {
        let cue_ms = cue_samp as f64 / d.audio.sample_rate as f64 * 1000.0;
        d.tempo.offset_ms = (cue_ms.rem_euclid(beat_period_ms) / 10.0).round() as i64 * 10;
    }
}

/// Apply a ±10ms offset step and keep the display position in sync when paused.
///
/// The display delta uses the raw `delta_ms` step — never `new_offset - old_offset`.
/// Those two values differ when `rem_euclid` wraps the offset across a beat boundary,
/// which would shift `smooth_display_samp` by nearly a full period and trigger a
/// spurious waveform rerender.
pub(crate) fn apply_offset_step(d: &mut Deck, delta_ms: i64) {
    if let Some(anchor) = d.anchor_sample {
        let delta_samp = delta_ms as f64 / 1000.0 * d.audio.sample_rate as f64;
        d.anchor_sample = Some((anchor as i64 + delta_samp as i64).max(0) as usize);
    }
    d.tempo.offset_ms += delta_ms;
    let period = (60_000.0 / d.tempo.base_bpm as f64 / 10.0).round() as i64 * 10;
    d.tempo.offset_ms = d.tempo.offset_ms.rem_euclid(period);
    if d.audio.player.is_paused() {
        let delta_samp = delta_ms as f64 / 1000.0 * d.audio.sample_rate as f64;
        d.display.smooth_display_samp = (d.display.smooth_display_samp + delta_samp).max(0.0);
        d.audio.seek_handle.set_position(d.display.smooth_display_samp / d.audio.sample_rate as f64);
    }
}

/// Compute 16 braille spectrum characters from mono samples at `pos`.
/// Uses the Goertzel algorithm on 32 log-spaced bins, 20 Hz – 20 kHz.
/// Spectrum analyser width in braille characters (two bins per character).
pub(crate) const SPECTRUM_CHARS: usize = 16;
const SPECTRUM_BINS: usize = SPECTRUM_CHARS * 2;

pub(crate) fn compute_spectrum(mono: &[f32], pos: usize, sample_rate: u32, filter_offset: i32) -> ([char; SPECTRUM_CHARS], [bool; SPECTRUM_CHARS]) {
    const N: usize = 4096;
    const LEFT_MASKS:  [u8; 5] = [0x00, 0x40, 0x44, 0x46, 0x47];
    const RIGHT_MASKS: [u8; 5] = [0x00, 0x80, 0xA0, 0xB0, 0xB8];

    // Log-spaced centre frequencies: 20 Hz … 20 kHz.
    let freqs: [f64; SPECTRUM_BINS] = std::array::from_fn(|i| {
        20.0 * (1000.0_f64).powf(i as f64 / (SPECTRUM_BINS - 1) as f64)
    });

    // Hann window coefficients — computed once and reused across all calls.
    static HANN: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
    let hann = HANN.get_or_init(|| {
        (0..N)
            .map(|n| 0.5 * (1.0 - (2.0 * std::f64::consts::PI * n as f64 / (N - 1) as f64).cos()) as f32)
            .collect()
    });

    // Pre-filter the window if a filter is active.
    let filtered: Vec<f32> = if filter_offset != 0 {
        let idx = (filter_offset.unsigned_abs() as usize - 1).min(15);
        let is_lpf = filter_offset < 0;
        let fc = if is_lpf { FILTER_CUTOFFS_HZ[idx] } else { FILTER_CUTOFFS_HZ[15 - idx] };
        let (b0, b1, b2, a1, a2) = butterworth_biquad(fc, sample_rate, is_lpf);
        let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        (0..N).map(|i| {
            let x = mono.get(pos + i).copied().unwrap_or(0.0);
            let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            x2 = x1; x1 = x; y2 = y1; y1 = y;
            y
        }).collect()
    } else {
        Vec::new()
    };

    let sr = sample_rate as f64;
    let mut heights = [0usize; SPECTRUM_BINS];
    let mut raw_heights = [0.0f32; SPECTRUM_BINS];

    for (k, &f) in freqs.iter().enumerate() {
        let coeff = 2.0 * (2.0 * std::f64::consts::PI * f / sr).cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for i in 0..N {
            let raw = if filter_offset != 0 {
                filtered[i]
            } else {
                mono.get(pos + i).copied().unwrap_or(0.0)
            };
            let sample = raw as f64 * hann[i] as f64;
            let s = sample + coeff * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
        let magnitude = power.max(0.0).sqrt();
        let db = if magnitude > 0.0 { 20.0 * magnitude.log10() } else { 0.0 };
        // +3 dB/octave tilt to compensate for the ~1/f (pink noise) rolloff of typical music,
        // making treble bins as visible as bass bins with equal perceptual loudness.
        // 20 Hz → 0 dB boost; 20 kHz → +30 dB boost (~10 octaves × 3 dB).
        let tilt_db = (f / 20.0).log2() * 3.0;
        let raw = (db + tilt_db - 10.0) / 12.5;
        heights[k] = raw.round().clamp(0.0, 4.0) as usize;
        raw_heights[k] = raw as f32;
    }

    // Background active when raw energy exceeds 1/4 of the single-dot threshold (0.5).
    const BG_THRESH: f32 = 0.5 / 4.0;
    let chars: [char; SPECTRUM_CHARS] = std::array::from_fn(|c| {
        let l = heights[c * 2];
        let r = heights[c * 2 + 1];
        char::from_u32(0x2800 | LEFT_MASKS[l] as u32 | RIGHT_MASKS[r] as u32).unwrap_or(' ')
    });
    let has_bg: [bool; SPECTRUM_CHARS] = std::array::from_fn(|c| {
        raw_heights[c * 2] > BG_THRESH || raw_heights[c * 2 + 1] > BG_THRESH
    });
    (chars, has_bg)
}

/// Beat-jump helper. Positive `beats` = forward, negative = backward.
///
/// While playing: swallow jumps that would hit either end-stop (preserves beat alignment).
/// Forward guard keeps at least one jump-size from the end.
/// While paused: clamp to boundaries so the user can navigate to the start or end deliberately.
/// Position and speed from a session snapshot, applied once the grid is known.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RestoreTransport {
    pub(crate) position_secs: f64,
    pub(crate) bpm: f32,
    pub(crate) playback_speed: f32,
}

/// The deck as it is now, in the shape the session file keeps.
pub(crate) fn snapshot_of_deck(d: &Deck) -> DeckSnapshot {
    DeckSnapshot {
        path: d.path.to_string_lossy().into_owned(),
        position_secs: d.audio.seek_handle.current_pos().as_secs_f64(),
        playlist_path: d.playlist.as_ref().map(|p| p.path.to_string_lossy().into_owned()),
        playlist_index: d.playlist.as_ref().map_or(0, |p| p.index),
        bpm: d.tempo.bpm,
        playback_speed: d.tempo.playback_speed,
        volume: d.mixer.volume,
        pitch_semitones: d.pitch_semitones,
        filter_offset: d.mixer.filter_offset,
        filter_poles: d.mixer.filter_poles,
        pfl_level: d.mixer.pfl_level,
    }
}

/// Put a snapshot's mixer state onto a freshly built deck. PFL routing (which
/// deck the headphones follow) is the caller's, since it is global.
pub(crate) fn apply_mixer_snapshot(d: &mut Deck, snap: &DeckSnapshot) {
    d.mixer.volume = snap.volume.clamp(0.0, 1.0);
    d.audio.deck_volume_atomic.store(d.mixer.volume.to_bits(), Ordering::Relaxed);
    d.pitch_semitones = snap.pitch_semitones.clamp(-6, 6);
    d.audio.pitch_semitones.store(d.pitch_semitones, Ordering::Relaxed);
    d.mixer.filter_offset = snap.filter_offset.clamp(-16, 16);
    d.audio.filter_offset_shared.store(d.mixer.filter_offset, Ordering::Relaxed);
    d.mixer.filter_poles = if snap.filter_poles >= 4 { 4 } else { 2 };
    d.audio.filter_poles.store(d.mixer.filter_poles, Ordering::Relaxed);
    d.mixer.pfl_level = snap.pfl_level.min(100);
    d.audio.pfl_level.store(d.mixer.pfl_level, Ordering::Relaxed);
    d.audio.player.set_volume(if d.mixer.pfl_level > 0 { 1.0 } else { d.mixer.volume });
}

/// Put a snapshot's transport onto the deck now that its grid is settled:
/// speed in the mode's own terms, then the saved position. Paused throughout.
pub(crate) fn apply_restore_transport(d: &mut Deck, t: RestoreTransport) {
    match d.mode {
        DeckMode::Beat => {
            if d.tempo.bpm_established && t.bpm > 0.0 {
                d.tempo.bpm = t.bpm.clamp(40.0, 240.0);
            }
            d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
        }
        DeckMode::Playback => {
            d.tempo.playback_speed = t.playback_speed.clamp(0.1, 4.0);
            d.audio.player.set_speed(d.tempo.playback_speed);
        }
    }
    let position = t.position_secs.clamp(0.0, d.total_duration);
    d.audio.seek_handle.seek_direct(position);
}

/// The beat-jump sizes, largest first: the forward and backward actions and the
/// distance in beats. Playback mode reads the same table as fixed time, half a
/// second per beat. Shared by the jump keys, the detached cursor, and ghosts.
pub(crate) const JUMP_SIZES: [(Action, Action, i32); 7] = [
    (Action::JumpForward64b, Action::JumpBackward64b, 256),
    (Action::JumpForward32b, Action::JumpBackward32b, 128),
    (Action::JumpForward16b, Action::JumpBackward16b, 64),
    (Action::JumpForward8b,  Action::JumpBackward8b,  32),
    (Action::JumpForward4b,  Action::JumpBackward4b,  16),
    (Action::JumpForward1b,  Action::JumpBackward1b,  4),
    (Action::JumpForward1bt, Action::JumpBackward1bt, 1),
];

/// Signed beat count for a jump action, or None if `action` is not a jump.
pub(crate) fn jump_beats(action: Action) -> Option<i32> {
    JUMP_SIZES.iter().find_map(|&(fwd, back, beats)| {
        if action == fwd { Some(beats) } else if action == back { Some(-beats) } else { None }
    })
}

/// Seconds per beat in Playback mode, where jumps are fixed-time.
pub(crate) const PLAYBACK_JUMP_BEAT_SECS: f64 = 0.5;

/// Where a jump of `jump` seconds from `current` lands, or None where the key
/// refuses. Backward clamps to the start (refused while playing if it would
/// clamp); forward while playing is refused if a further jump would overrun
/// the end, so a landing always leaves room to jump on.
pub(crate) fn jump_landing(current: f64, jump: f64, playing: bool, track_end: f64) -> Option<f64> {
    if jump < 0.0 {
        let target = current + jump;
        if playing && target < 0.0 { return None; }
        Some(target.max(0.0))
    } else {
        let target = current + jump;
        if playing && target + jump > track_end { return None; }
        Some(target.min(track_end))
    }
}

/// Where one beat-jump key would land from the current position: the landing
/// sample and the bare key bound to it. Largest jumps first, so a view that
/// collapses two landings onto one column keeps the larger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GhostLanding {
    pub(crate) sample: usize,
    pub(crate) key: char,
}

/// Ghost playheads for a deck: each jump key's landing from `current_samp`,
/// under the live Beat Jump rules. Beat mode only — Playback draws none. The
/// single-beat jump is skipped: too close to the playhead to aim.
pub(crate) fn ghost_landings(
    deck: &Deck,
    current_samp: f64,
    keymap: &HashMap<KeyBinding, Action>,
) -> Vec<GhostLanding> {
    if deck.mode != DeckMode::Beat || deck.total_duration <= 0.0 { return Vec::new(); }
    let sr      = deck.audio.sample_rate as f64;
    let current = current_samp / sr;
    let playing = !deck.audio.player.is_paused();
    let mut out = Vec::with_capacity(JUMP_SIZES.len() * 2);
    for &(fwd, back, beats) in JUMP_SIZES.iter().filter(|&&(_, _, b)| b > 1) {
        let jump = beats as f64 * 60.0 / deck.tempo.base_bpm as f64;
        for (action, signed) in [(fwd, jump), (back, -jump)] {
            let Some(key) = bare_key_char(keymap, action) else { continue };
            if let Some(t) = jump_landing(current, signed, playing, deck.total_duration) {
                out.push(GhostLanding { sample: (t * sr) as usize, key });
            }
        }
    }
    out
}

/// The unchorded printable key bound to `action`, if any — the label a ghost wears.
fn bare_key_char(keymap: &HashMap<KeyBinding, Action>, action: Action) -> Option<char> {
    keymap.iter().find_map(|(binding, bound)| match binding {
        KeyBinding::Key(KeyCode::Char(c)) if *bound == action => Some(*c),
        _ => None,
    })
}

pub(crate) fn do_jump(seek_handle: &SeekHandle, player: &rodio::Player, bpm: f32, track_end: f64, beats: i32) {
    let jump    = beats as f64 * 60.0 / bpm as f64;
    let current = seek_handle.current_pos().as_secs_f64();
    let playing = !player.is_paused();
    if let Some(target) = jump_landing(current, jump, playing, track_end) {
        if playing { seek_handle.seek_to(target); } else { seek_handle.seek_direct(target); }
    }
}

/// Seek by a fixed number of seconds (Playback-mode jump).
pub(crate) fn do_time_jump(seek_handle: &SeekHandle, player: &Player, track_end: f64, secs: f64) {
    let current = seek_handle.current_pos().as_secs_f64();
    let playing  = !player.is_paused();
    if secs < 0.0 {
        let target = (current + secs).max(0.0);
        if playing { seek_handle.seek_to(target); } else { seek_handle.seek_direct(target); }
    } else {
        let target = current + secs;
        if playing && target + secs > track_end { return; }
        let clamped = target.min(track_end);
        if playing { seek_handle.seek_to(clamped); } else { seek_handle.seek_direct(clamped); }
    }
}

pub(crate) fn cache_entry_for_deck(d: &Deck) -> CacheEntry {
    CacheEntry {
        grid: d.tempo.bpm_established.then(|| Grid { bpm: d.tempo.base_bpm, offset_ms: d.tempo.offset_ms }),
        name: d.filename.clone(),
        cue_sample: d.cue_sample,
        gain_db: d.mixer.gain_db,
        mode: Some(d.mode),
        anchor_sample: d.anchor_sample,
    }
}

// Suppress the unused import warning — FilterSource is used in main.rs via build_deck
// which constructs it, but it's imported here for the type to be in scope for DeckAudio.
// The actual use of FilterSource is in main.rs::build_deck.
#[allow(unused_imports)]
pub(crate) use crate::audio::FilterSource as _FilterSourceReexport;

#[cfg(test)]
mod tests {
    #[test]
    fn grid_phase_derivation_maths() {
        // Anchor at 10.5 s, 120 BPM (500 ms period) → offset 10500 % 500 = 0.
        // Anchor at 10.75 s → 250 ms. 1 ms precision, no 10 ms rounding.
        let period_ms = 60_000.0 / 120.0f64;
        let derive = |anchor_ms: f64| anchor_ms.rem_euclid(period_ms).round() as i64;
        assert_eq!(derive(10_500.0), 0);
        assert_eq!(derive(10_750.0), 250);
        assert_eq!(derive(10_753.0), 253);
    }
}
