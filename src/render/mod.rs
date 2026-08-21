use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::audio::WaveformData;
use crate::deck::{
    Deck, DeckMode, SpecPalette,
    TAG_FIELD_LABELS,
};
use crate::messages::{Event, Severity};

/// The grid-work accent for the fixed furniture: ticks, tag, and anchor.
pub(crate) const GRID_BLUE: ratatui::style::Color = ratatui::style::Color::Rgb(40, 100, 210);
/// The movable cursor keeps a lighter cyan, standing out against the furniture.
pub(crate) const GRID_CURSOR: ratatui::style::Color = ratatui::style::Color::Rgb(60, 150, 255);

pub(crate) const ZOOM_LEVELS: &[f32] = &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
pub(crate) const DEFAULT_ZOOM_IDX: usize = 2; // 4 seconds

/// Interpolate a four-stop spectral palette at `bass` ∈ [0,1] and scale by `brightness`.
/// The interpolated colour is normalised so its dominant channel is always 255 before
/// brightness is applied — this preserves full saturation at all bass ratios and ensures
/// the hue is clearly identifiable even at low brightness.
/// Spectral palette quantised to 32 levels. Adjacent columns whose smoothed bass
/// ratios land on the same level share an identical `Color`, so run-length span
/// merging collapses them into one styled span — one SGR sequence instead of one
/// per column. 32 levels are indistinguishable from the continuous gradient at
/// the box-smoothed rate the bass ratio actually changes.
pub(crate) const SPECTRAL_LEVELS: usize = 32;

pub(crate) struct SpectralLut {
    colors: [ratatui::style::Color; SPECTRAL_LEVELS],
}

impl SpectralLut {
    pub(crate) fn new(pal: SpecPalette, brightness: f32) -> Self {
        Self {
            colors: std::array::from_fn(|i| {
                indexed(spectral_color(pal, i as f32 / (SPECTRAL_LEVELS - 1) as f32, brightness))
            }),
        }
    }

    pub(crate) fn color(&self, bass: f32) -> ratatui::style::Color {
        let idx = (bass.clamp(0.0, 1.0) * (SPECTRAL_LEVELS - 1) as f32).round() as usize;
        self.colors[idx.min(SPECTRAL_LEVELS - 1)]
    }
}

/// Nearest xterm-256 colour (6×6×6 cube 16–231, grayscale ramp 232–255) for an
/// RGB colour. An indexed SGR sequence is ~11 bytes against truecolor's ~19, and
/// the coarser palette collapses more adjacent waveform columns into shared
/// spans — both cut the escape volume the terminal emulator has to parse.
fn indexed(color: ratatui::style::Color) -> ratatui::style::Color {
    let ratatui::style::Color::Rgb(r, g, b) = color else { return color };
    const CUBE: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let level = |v: u8| (0..6).min_by_key(|&i| (CUBE[i] - v as i32).abs()).unwrap();
    let (ri, gi, bi) = (level(r), level(g), level(b));
    let dist = |cr: i32, cg: i32, cb: i32| {
        (cr - r as i32).pow(2) + (cg - g as i32).pow(2) + (cb - b as i32).pow(2)
    };
    let cube_dist = dist(CUBE[ri], CUBE[gi], CUBE[bi]);
    let gray_step = (((r as i32 + g as i32 + b as i32) / 3 - 8).clamp(0, 230) / 10).min(23);
    let gray = 8 + gray_step * 10;
    if dist(gray, gray, gray) < cube_dist {
        ratatui::style::Color::Indexed((232 + gray_step) as u8)
    } else {
        ratatui::style::Color::Indexed((16 + 36 * ri + 6 * gi + bi) as u8)
    }
}

pub(crate) fn spectral_color(pal: SpecPalette, bass: f32, brightness: f32) -> ratatui::style::Color {
    let stops = [pal.0, pal.1, pal.2, pal.3];
    let t = (bass * 3.0).clamp(0.0, 3.0);
    let seg = (t.floor() as usize).min(stops.len() - 2);
    let f   = t.fract();
    let (a, b) = (stops[seg], stops[seg + 1]);
    let r = a.0 as f32 + f * (b.0 as f32 - a.0 as f32);
    let g = a.1 as f32 + f * (b.1 as f32 - a.1 as f32);
    let b_ = a.2 as f32 + f * (b.2 as f32 - a.2 as f32);
    // Normalise to max channel = 255 so saturation is always 1.0 before dimming.
    let max_ch = r.max(g).max(b_).max(1.0);
    let norm = 255.0 / max_ch * brightness;
    ratatui::style::Color::Rgb(
        (r  * norm).round().clamp(0.0, 255.0) as u8,
        (g  * norm).round().clamp(0.0, 255.0) as u8,
        (b_ * norm).round().clamp(0.0, 255.0) as u8,
    )
}

/// Pre-rendered braille buffer wider than the visible area, enabling smooth scrolling.
/// The UI thread pans a viewport through this stable buffer rather than requesting
/// a full recompute every time the playhead advances by one column.
pub(crate) struct BrailleBuffer {
    pub(crate) grid:            Vec<Vec<u8>>, // rows × buf_cols braille bytes
    pub(crate) bass_ratio:      Vec<f32>,     // per-column bass ratio in [0,1]: 1=bass, 0=treble
    pub(crate) tick:            Vec<u8>,      // per-column tick byte: 0x47=left sub-col, 0xB8=right, 0=none
    pub(crate) cue_buf_col:     Option<usize>,// buffer column of cue point, None if unset or out of range
    pub(crate) buf_cols:        usize,        // total buffer width (= 5 × screen_cols)
    pub(crate) anchor_sample:   usize,        // mono-sample index at the buffer centre
    pub(crate) samples_per_col: usize,        // mono samples represented by each buffer column
}

impl BrailleBuffer {
    pub(crate) fn empty() -> Self {
        Self { grid: Vec::new(), bass_ratio: Vec::new(), tick: Vec::new(), cue_buf_col: None, buf_cols: 0, anchor_sample: 0, samples_per_col: 1 }
    }
}

/// A single background thread that produces three `BrailleBuffer`s — one per
/// deck — each at a `col_samp` scaled by that deck's `bpm / base_bpm` ratio.
/// Scaling by the playback speed means ticks placed at `base_bpm` sample
/// spacing appear at `bpm`-spaced columns, so the tick grids of two decks at
/// the same effective BPM are visually identical.
pub(crate) struct SharedDetailRenderer {
    pub(crate) cols:           Arc<AtomicUsize>,
    pub(crate) rows:           Arc<AtomicUsize>,
    pub(crate) zoom_at:        Arc<AtomicUsize>,
    pub(crate) sample_rate_a:  Arc<AtomicUsize>,
    pub(crate) sample_rate_b:  Arc<AtomicUsize>,
    pub(crate) sample_rate_c:  Arc<AtomicUsize>,
    /// `(bpm / base_bpm) × 65536`, updated on every BPM-changing action.
    pub(crate) speed_ratio_a:  Arc<AtomicUsize>,
    pub(crate) speed_ratio_b:  Arc<AtomicUsize>,
    pub(crate) speed_ratio_c:  Arc<AtomicUsize>,
    pub(crate) waveform_a:     Arc<Mutex<Option<Arc<WaveformData>>>>,
    pub(crate) waveform_b:     Arc<Mutex<Option<Arc<WaveformData>>>>,
    pub(crate) waveform_c:     Arc<Mutex<Option<Arc<WaveformData>>>>,
    pub(crate) display_pos_a:  Arc<AtomicUsize>,
    pub(crate) display_pos_b:  Arc<AtomicUsize>,
    pub(crate) display_pos_c:  Arc<AtomicUsize>,
    pub(crate) channels_a:     Arc<AtomicUsize>,
    pub(crate) channels_b:     Arc<AtomicUsize>,
    pub(crate) channels_c:     Arc<AtomicUsize>,
    /// Incremented each time a new track is loaded into the slot; signals the
    /// background thread to recompute immediately rather than waiting for drift.
    pub(crate) load_gen_a:     Arc<AtomicUsize>,
    pub(crate) load_gen_b:     Arc<AtomicUsize>,
    pub(crate) load_gen_c:     Arc<AtomicUsize>,
    /// `base_bpm` as f32 bits; 0 when analysing or unloaded.
    pub(crate) bpm_a:          Arc<AtomicU32>,
    pub(crate) bpm_b:          Arc<AtomicU32>,
    pub(crate) bpm_c:          Arc<AtomicU32>,
    pub(crate) offset_ms_a:    Arc<AtomicI64>,
    pub(crate) offset_ms_b:    Arc<AtomicI64>,
    pub(crate) offset_ms_c:    Arc<AtomicI64>,
    /// Cue point in mono samples; -1 when unset.
    pub(crate) cue_sample_a:   Arc<AtomicI64>,
    pub(crate) cue_sample_b:   Arc<AtomicI64>,
    pub(crate) cue_sample_c:   Arc<AtomicI64>,
    /// Gain trim as f32 bits; 1.0 when unset. Peaks in the buffer are pre-scaled
    /// by this value so the detail waveform height tracks gain visually.
    pub(crate) gain_a:         Arc<AtomicU32>,
    pub(crate) gain_b:         Arc<AtomicU32>,
    pub(crate) gain_c:         Arc<AtomicU32>,
    pub(crate) shared_a:       Arc<Mutex<Arc<BrailleBuffer>>>,
    pub(crate) shared_b:       Arc<Mutex<Arc<BrailleBuffer>>>,
    pub(crate) shared_c:       Arc<Mutex<Arc<BrailleBuffer>>>,
    _stop_guard:    StopOnDrop,
}

struct StopOnDrop(Arc<AtomicBool>);
impl Drop for StopOnDrop {
    fn drop(&mut self) { self.0.store(true, Ordering::Relaxed); }
}

impl SharedDetailRenderer {
    pub(crate) fn new(zoom_idx: usize) -> Self {
        let cols           = Arc::new(AtomicUsize::new(0));
        let rows           = Arc::new(AtomicUsize::new(0));
        let zoom_at        = Arc::new(AtomicUsize::new(zoom_idx));
        let sample_rate_a  = Arc::new(AtomicUsize::new(44100));
        let sample_rate_b  = Arc::new(AtomicUsize::new(44100));
        let sample_rate_c  = Arc::new(AtomicUsize::new(44100));
        let speed_ratio_a  = Arc::new(AtomicUsize::new(65536)); // 1.0 × 65536
        let speed_ratio_b  = Arc::new(AtomicUsize::new(65536));
        let speed_ratio_c  = Arc::new(AtomicUsize::new(65536));
        let waveform_a     = Arc::new(Mutex::new(None::<Arc<WaveformData>>));
        let waveform_b     = Arc::new(Mutex::new(None::<Arc<WaveformData>>));
        let waveform_c     = Arc::new(Mutex::new(None::<Arc<WaveformData>>));
        let display_pos_a  = Arc::new(AtomicUsize::new(0));
        let display_pos_b  = Arc::new(AtomicUsize::new(0));
        let display_pos_c  = Arc::new(AtomicUsize::new(0));
        let channels_a     = Arc::new(AtomicUsize::new(1));
        let channels_b     = Arc::new(AtomicUsize::new(1));
        let channels_c     = Arc::new(AtomicUsize::new(1));
        let load_gen_a     = Arc::new(AtomicUsize::new(0));
        let load_gen_b     = Arc::new(AtomicUsize::new(0));
        let load_gen_c     = Arc::new(AtomicUsize::new(0));
        let bpm_a          = Arc::new(AtomicU32::new(0));
        let bpm_b          = Arc::new(AtomicU32::new(0));
        let bpm_c          = Arc::new(AtomicU32::new(0));
        let offset_ms_a    = Arc::new(AtomicI64::new(0));
        let offset_ms_b    = Arc::new(AtomicI64::new(0));
        let offset_ms_c    = Arc::new(AtomicI64::new(0));
        let cue_sample_a   = Arc::new(AtomicI64::new(-1));
        let cue_sample_b   = Arc::new(AtomicI64::new(-1));
        let cue_sample_c   = Arc::new(AtomicI64::new(-1));
        let gain_a         = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let gain_b         = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let gain_c         = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let shared_a: Arc<Mutex<Arc<BrailleBuffer>>> =
            Arc::new(Mutex::new(Arc::new(BrailleBuffer::empty())));
        let shared_b: Arc<Mutex<Arc<BrailleBuffer>>> =
            Arc::new(Mutex::new(Arc::new(BrailleBuffer::empty())));
        let shared_c: Arc<Mutex<Arc<BrailleBuffer>>> =
            Arc::new(Mutex::new(Arc::new(BrailleBuffer::empty())));
        let stop          = Arc::new(AtomicBool::new(false));
        let stop_guard    = StopOnDrop(Arc::clone(&stop));

        {
            let cols_bg      = Arc::clone(&cols);
            let rows_bg      = Arc::clone(&rows);
            let zoom_bg      = Arc::clone(&zoom_at);
            let sr_a_bg      = Arc::clone(&sample_rate_a);
            let sr_b_bg      = Arc::clone(&sample_rate_b);
            let sr_c_bg      = Arc::clone(&sample_rate_c);
            let ratio_a_bg   = Arc::clone(&speed_ratio_a);
            let ratio_b_bg   = Arc::clone(&speed_ratio_b);
            let ratio_c_bg   = Arc::clone(&speed_ratio_c);
            let wf_a_bg      = Arc::clone(&waveform_a);
            let wf_b_bg      = Arc::clone(&waveform_b);
            let wf_c_bg      = Arc::clone(&waveform_c);
            let pos_a_bg     = Arc::clone(&display_pos_a);
            let pos_b_bg     = Arc::clone(&display_pos_b);
            let pos_c_bg     = Arc::clone(&display_pos_c);
            let ch_a_bg      = Arc::clone(&channels_a);
            let ch_b_bg      = Arc::clone(&channels_b);
            let ch_c_bg      = Arc::clone(&channels_c);
            let gen_a_bg     = Arc::clone(&load_gen_a);
            let gen_b_bg     = Arc::clone(&load_gen_b);
            let gen_c_bg     = Arc::clone(&load_gen_c);
            let bpm_a_bg     = Arc::clone(&bpm_a);
            let bpm_b_bg     = Arc::clone(&bpm_b);
            let bpm_c_bg     = Arc::clone(&bpm_c);
            let off_ms_a_bg  = Arc::clone(&offset_ms_a);
            let off_ms_b_bg  = Arc::clone(&offset_ms_b);
            let off_ms_c_bg  = Arc::clone(&offset_ms_c);
            let cue_a_bg     = Arc::clone(&cue_sample_a);
            let cue_b_bg     = Arc::clone(&cue_sample_b);
            let cue_c_bg     = Arc::clone(&cue_sample_c);
            let gain_a_bg    = Arc::clone(&gain_a);
            let gain_b_bg    = Arc::clone(&gain_b);
            let gain_c_bg    = Arc::clone(&gain_c);
            let shared_a_bg  = Arc::clone(&shared_a);
            let shared_b_bg  = Arc::clone(&shared_b);
            let shared_c_bg  = Arc::clone(&shared_c);
            let stop_bg      = Arc::clone(&stop);
            thread::spawn(move || {
                let sr_at   = [sr_a_bg, sr_b_bg, sr_c_bg];
                let ratio   = [ratio_a_bg, ratio_b_bg, ratio_c_bg];
                let wf      = [wf_a_bg, wf_b_bg, wf_c_bg];
                let pos_at  = [pos_a_bg, pos_b_bg, pos_c_bg];
                let ch_at   = [ch_a_bg, ch_b_bg, ch_c_bg];
                let gen_at  = [gen_a_bg, gen_b_bg, gen_c_bg];
                let bpm_at  = [bpm_a_bg, bpm_b_bg, bpm_c_bg];
                let off_at  = [off_ms_a_bg, off_ms_b_bg, off_ms_c_bg];
                let cue_at  = [cue_a_bg, cue_b_bg, cue_c_bg];
                let gain_at = [gain_a_bg, gain_b_bg, gain_c_bg];
                let shared  = [shared_a_bg, shared_b_bg, shared_c_bg];

                /// The rebuild inputs a slot's buffer content is a pure function of
                /// (besides dimensions, zoom, and the playhead anchor).
                #[derive(Clone, Copy, PartialEq)]
                struct SlotParams {
                    col_samp: usize,
                    bpm_raw:  u32,
                    off_ms:   i64,
                    cue_raw:  i64,
                    gain_raw: u32,
                }

                fn compute_cue_buf_col(cue_raw: i64, anchor: usize, col_samp: usize, buf_cols: usize) -> Option<usize> {
                    if cue_raw < 0 || col_samp == 0 { return None; }
                    let delta = cue_raw - anchor as i64;
                    let col = buf_cols as i64 / 2 + delta.div_euclid(col_samp as i64);
                    if col >= 0 && (col as usize) < buf_cols { Some(col as usize) } else { None }
                }
                fn scale_peaks(peaks: Vec<(f32, f32)>, g: f32) -> Vec<(f32, f32)> {
                    peaks.into_iter().map(|(mn, mx)| (mn * g, mx * g)).collect()
                }

                // Parameter-driven rebuilds wait for the inputs to sit still, so a held
                // key repeating at ~30 Hz doesn't cause a rebuild storm — but during a
                // sustained ramp the slot still rebuilds every THROTTLE so the operator
                // sees tick marks move against the wave (needed for beat-grid alignment).
                // Drift, resize, zoom, and track load rebuild immediately.
                const SETTLE:   Duration = Duration::from_millis(50);
                const THROTTLE: Duration = Duration::from_millis(100);

                let mut last_cols = 0usize;
                let mut last_rows = 0usize;
                let mut last_zoom = usize::MAX;
                let mut last_gen: [usize; 3] = [usize::MAX; 3];
                // Params and anchor of each slot's live buffer, None before first build.
                let mut built: [Option<SlotParams>; 3] = [None; 3];
                let mut built_anchor: [usize; 3] = [0; 3];
                // Most recently observed params and when they were first seen unchanged.
                let mut seen: [Option<(SlotParams, std::time::Instant)>; 3] = [None, None, None];
                let mut last_rebuild: [std::time::Instant; 3] = [std::time::Instant::now(); 3];

                loop {
                    if stop_bg.load(Ordering::Relaxed) { break; }

                    let cols = cols_bg.load(Ordering::Relaxed);
                    let rows = rows_bg.load(Ordering::Relaxed);
                    if cols == 0 || rows == 0 {
                        thread::sleep(Duration::from_millis(8));
                        continue;
                    }

                    let zoom      = zoom_bg.load(Ordering::Relaxed).min(ZOOM_LEVELS.len() - 1);
                    let zoom_secs = ZOOM_LEVELS[zoom] as f64;
                    let shared_changed = cols != last_cols || rows != last_rows || zoom != last_zoom;
                    let buf_cols = cols * 5;

                    for slot in 0..3 {
                        let sr    = sr_at[slot].load(Ordering::Relaxed);
                        let ratio = ratio[slot].load(Ordering::Relaxed) as f64 / 65536.0;
                        // col_samp scaled by speed ratio so column grid is in playback-time space.
                        let col_samp = ((zoom_secs * sr as f64 * ratio) as usize / cols).max(1);
                        let ch  = ch_at[slot].load(Ordering::Relaxed).max(1);
                        let pos = pos_at[slot].load(Ordering::Relaxed) / ch;
                        let load_gen = gen_at[slot].load(Ordering::Relaxed);
                        let params = SlotParams {
                            col_samp,
                            bpm_raw:  bpm_at[slot].load(Ordering::Relaxed),
                            off_ms:   off_at[slot].load(Ordering::Relaxed),
                            cue_raw:  cue_at[slot].load(Ordering::Relaxed),
                            gain_raw: gain_at[slot].load(Ordering::Relaxed),
                        };

                        let first_seen = match seen[slot] {
                            Some((p, t)) if p == params => t,
                            _ => {
                                let now = std::time::Instant::now();
                                seen[slot] = Some((params, now));
                                now
                            }
                        };

                        let drift_cols = match built[slot] {
                            Some(bp) => pos.abs_diff(built_anchor[slot]) / bp.col_samp,
                            None => usize::MAX,
                        };
                        let immediate = shared_changed
                            || load_gen != last_gen[slot]
                            || drift_cols >= cols * 3 / 4;
                        let changed = built[slot] != Some(params);
                        let settled = changed && first_seen.elapsed() >= SETTLE;
                        let ramping = changed && last_rebuild[slot].elapsed() >= THROTTLE;
                        if !immediate && !settled && !ramping { continue; }

                        let rebuild_start = std::time::Instant::now();
                        let wf: Option<Arc<WaveformData>> = wf[slot].lock().unwrap().clone();
                        let anchor = (pos / col_samp) * col_samp;
                        let tick_view_start = anchor as f64 - (buf_cols / 2) as f64 * col_samp as f64;
                        let gain = f32::from_bits(params.gain_raw);
                        let buf = Arc::new(BrailleBuffer {
                            grid: render_braille(
                                &scale_peaks(peaks_for_slot(&wf, anchor, col_samp, buf_cols), gain),
                                rows, buf_cols,
                            ),
                            bass_ratio:      spectral_for_slot(&wf, anchor, col_samp, buf_cols, sr as u32),
                            tick:            compute_tick_display(buf_cols, col_samp, tick_view_start,
                                                 params.bpm_raw == 0, f32::from_bits(params.bpm_raw), sr as u32, params.off_ms),
                            cue_buf_col:     compute_cue_buf_col(params.cue_raw, anchor, col_samp, buf_cols),
                            buf_cols,
                            anchor_sample:   anchor,
                            samples_per_col: col_samp,
                        });
                        *shared[slot].lock().unwrap() = buf;
                        crate::frame_stats::note_rebuild(slot, rebuild_start.elapsed());
                        last_rebuild[slot] = std::time::Instant::now();
                        built[slot] = Some(params);
                        built_anchor[slot] = anchor;
                        last_gen[slot] = load_gen;
                    }

                    last_cols = cols;
                    last_rows = rows;
                    last_zoom = zoom;

                    thread::sleep(Duration::from_millis(8));
                }
            });
        }

        SharedDetailRenderer {
            cols, rows, zoom_at,
            sample_rate_a, sample_rate_b, sample_rate_c,
            speed_ratio_a, speed_ratio_b, speed_ratio_c,
            waveform_a, waveform_b, waveform_c,
            display_pos_a, display_pos_b, display_pos_c,
            channels_a, channels_b, channels_c,
            load_gen_a, load_gen_b, load_gen_c,
            bpm_a, bpm_b, bpm_c,
            offset_ms_a, offset_ms_b, offset_ms_c,
            cue_sample_a, cue_sample_b, cue_sample_c,
            gain_a, gain_b, gain_c,
            shared_a, shared_b, shared_c,
            _stop_guard: stop_guard,
        }
    }

    pub(crate) fn set_deck(&self, slot: usize, wf: Arc<WaveformData>, channels: u16, sample_rate: u32) {
        match slot {
            0 => {
                *self.waveform_a.lock().unwrap() = Some(wf);
                self.channels_a.store(channels as usize, Ordering::Relaxed);
                self.sample_rate_a.store(sample_rate as usize, Ordering::Relaxed);
                self.speed_ratio_a.store(65536, Ordering::Relaxed); // reset to 1.0 on load
                self.load_gen_a.fetch_add(1, Ordering::Relaxed);
            }
            1 => {
                *self.waveform_b.lock().unwrap() = Some(wf);
                self.channels_b.store(channels as usize, Ordering::Relaxed);
                self.sample_rate_b.store(sample_rate as usize, Ordering::Relaxed);
                self.speed_ratio_b.store(65536, Ordering::Relaxed);
                self.load_gen_b.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                *self.waveform_c.lock().unwrap() = Some(wf);
                self.channels_c.store(channels as usize, Ordering::Relaxed);
                self.sample_rate_c.store(sample_rate as usize, Ordering::Relaxed);
                self.speed_ratio_c.store(65536, Ordering::Relaxed);
                self.load_gen_c.fetch_add(1, Ordering::Relaxed);
            }
        }
    }


    pub(crate) fn store_speed_ratio(&self, slot: usize, bpm: f32, base_bpm: f32) {
        let ratio = ((bpm / base_bpm) as f64 * 65536.0) as usize;
        match slot {
            0 => self.speed_ratio_a.store(ratio, Ordering::Relaxed),
            1 => self.speed_ratio_b.store(ratio, Ordering::Relaxed),
            _ => self.speed_ratio_c.store(ratio, Ordering::Relaxed),
        }
    }

    pub(crate) fn store_cue(&self, slot: usize, cue_sample: Option<usize>) {
        let raw = cue_sample.map_or(-1, |s| s as i64);
        match slot {
            0 => self.cue_sample_a.store(raw, Ordering::Relaxed),
            1 => self.cue_sample_b.store(raw, Ordering::Relaxed),
            _ => self.cue_sample_c.store(raw, Ordering::Relaxed),
        }
    }

    pub(crate) fn store_tempo(&self, slot: usize, base_bpm: f32, offset_ms: i64, analysing: bool) {
        let bpm_raw = if analysing { 0.0f32 } else { base_bpm }.to_bits();
        match slot {
            0 => {
                self.bpm_a.store(bpm_raw, Ordering::Relaxed);
                self.offset_ms_a.store(offset_ms, Ordering::Relaxed);
            }
            1 => {
                self.bpm_b.store(bpm_raw, Ordering::Relaxed);
                self.offset_ms_b.store(offset_ms, Ordering::Relaxed);
            }
            _ => {
                self.bpm_c.store(bpm_raw, Ordering::Relaxed);
                self.offset_ms_c.store(offset_ms, Ordering::Relaxed);
            }
        }
    }

    pub(crate) fn store_gain(&self, slot: usize, gain_linear: f32) {
        match slot {
            0 => self.gain_a.store(gain_linear.to_bits(), Ordering::Relaxed),
            1 => self.gain_b.store(gain_linear.to_bits(), Ordering::Relaxed),
            _ => self.gain_c.store(gain_linear.to_bits(), Ordering::Relaxed),
        }
    }

    /// Swap all rendering state between two deck slots. Called when the user swaps decks.
    pub(crate) fn swap_slots(&self, slot_x: usize, slot_y: usize) {
        let swap_atomic_usize = |a: &AtomicUsize, b: &AtomicUsize| {
            let va = a.load(Ordering::Relaxed);
            let vb = b.load(Ordering::Relaxed);
            a.store(vb, Ordering::Relaxed);
            b.store(va, Ordering::Relaxed);
        };
        let swap_atomic_u32 = |a: &AtomicU32, b: &AtomicU32| {
            let va = a.load(Ordering::Relaxed);
            let vb = b.load(Ordering::Relaxed);
            a.store(vb, Ordering::Relaxed);
            b.store(va, Ordering::Relaxed);
        };
        let swap_atomic_i64 = |a: &AtomicI64, b: &AtomicI64| {
            let va = a.load(Ordering::Relaxed);
            let vb = b.load(Ordering::Relaxed);
            a.store(vb, Ordering::Relaxed);
            b.store(va, Ordering::Relaxed);
        };
        let swap_waveform = |wa: &Mutex<Option<Arc<WaveformData>>>, wb: &Mutex<Option<Arc<WaveformData>>>| {
            let mut ga = wa.lock().unwrap();
            let mut gb = wb.lock().unwrap();
            std::mem::swap(&mut *ga, &mut *gb);
        };
        let swap_buffer = |ba: &Mutex<Arc<BrailleBuffer>>, bb: &Mutex<Arc<BrailleBuffer>>| {
            let mut ga = ba.lock().unwrap();
            let mut gb = bb.lock().unwrap();
            std::mem::swap(&mut *ga, &mut *gb);
        };

        let (sr_x, sr_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.sample_rate_a, &self.sample_rate_b),
            (1, 2) | (2, 1) => (&self.sample_rate_b, &self.sample_rate_c),
            (0, 2) | (2, 0) => (&self.sample_rate_a, &self.sample_rate_c),
            _ => return,
        };
        let (ratio_x, ratio_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.speed_ratio_a, &self.speed_ratio_b),
            (1, 2) | (2, 1) => (&self.speed_ratio_b, &self.speed_ratio_c),
            _ => (&self.speed_ratio_a, &self.speed_ratio_c),
        };
        let (wf_x, wf_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.waveform_a, &self.waveform_b),
            (1, 2) | (2, 1) => (&self.waveform_b, &self.waveform_c),
            _ => (&self.waveform_a, &self.waveform_c),
        };
        let (pos_x, pos_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.display_pos_a, &self.display_pos_b),
            (1, 2) | (2, 1) => (&self.display_pos_b, &self.display_pos_c),
            _ => (&self.display_pos_a, &self.display_pos_c),
        };
        let (ch_x, ch_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.channels_a, &self.channels_b),
            (1, 2) | (2, 1) => (&self.channels_b, &self.channels_c),
            _ => (&self.channels_a, &self.channels_c),
        };
        let (gen_x, gen_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.load_gen_a, &self.load_gen_b),
            (1, 2) | (2, 1) => (&self.load_gen_b, &self.load_gen_c),
            _ => (&self.load_gen_a, &self.load_gen_c),
        };
        let (bpm_x, bpm_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.bpm_a, &self.bpm_b),
            (1, 2) | (2, 1) => (&self.bpm_b, &self.bpm_c),
            _ => (&self.bpm_a, &self.bpm_c),
        };
        let (off_x, off_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.offset_ms_a, &self.offset_ms_b),
            (1, 2) | (2, 1) => (&self.offset_ms_b, &self.offset_ms_c),
            _ => (&self.offset_ms_a, &self.offset_ms_c),
        };
        let (cue_x, cue_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.cue_sample_a, &self.cue_sample_b),
            (1, 2) | (2, 1) => (&self.cue_sample_b, &self.cue_sample_c),
            _ => (&self.cue_sample_a, &self.cue_sample_c),
        };
        let (gain_x, gain_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.gain_a, &self.gain_b),
            (1, 2) | (2, 1) => (&self.gain_b, &self.gain_c),
            _ => (&self.gain_a, &self.gain_c),
        };
        let (buf_x, buf_y) = match (slot_x, slot_y) {
            (0, 1) | (1, 0) => (&self.shared_a, &self.shared_b),
            (1, 2) | (2, 1) => (&self.shared_b, &self.shared_c),
            _ => (&self.shared_a, &self.shared_c),
        };

        swap_atomic_usize(sr_x, sr_y);
        swap_atomic_usize(ratio_x, ratio_y);
        swap_waveform(wf_x, wf_y);
        swap_atomic_usize(pos_x, pos_y);
        swap_atomic_usize(ch_x, ch_y);
        swap_atomic_usize(gen_x, gen_y);
        swap_atomic_u32(bpm_x, bpm_y);
        swap_atomic_i64(off_x, off_y);
        swap_atomic_i64(cue_x, cue_y);
        swap_atomic_u32(gain_x, gain_y);
        swap_buffer(buf_x, buf_y);
    }
}

/// Play state, badge, and track name for the overview's top-left overlay; a
/// lingering rename offer marks the title it concerns with an amber ⚠.
pub(crate) fn overview_title_line(deck: &Deck, frame_count: usize, beat_on: bool, analysing: bool) -> Line<'static> {
    let mut spans = tempo_spans(deck, frame_count, beat_on, analysing);
    spans.push(Span::raw(" "));
    let (badge, _) = playlist_badge(deck);
    spans.extend(badge);
    spans.push(Span::styled(
        deck.track_name.clone(),
        Style::default().fg(spectral_color(deck.display.palette, 0.0, 0.85)),
    ));
    if deck.rename_offer_active() && deck.rename_offer_started.unwrap().elapsed().as_secs() >= 10 {
        spans.push(Span::styled(" ⚠", Style::default().fg(Color::Rgb(230, 170, 60))));
    }
    Line::from(spans)
}

/// A countdown prompt that momentarily displaces the meters corner: the BPM
/// confirmation, or the rename offer's active phase. The meters return when
/// the prompt resolves.
pub(crate) fn countdown_prompt_line(deck: &Deck) -> Option<Line<'static>> {
    if deck.rename_offer_active() {
        let elapsed = deck.rename_offer_started.unwrap().elapsed().as_secs();
        if elapsed < 10 {
            let secs_left = 10 - elapsed;
            // Same amber as the browser's non-compliant marker.
            return Some(Line::from(Span::styled(
                format!("⚠ rename? [y]  ({secs_left}s)"),
                Style::default().fg(Color::Rgb(230, 170, 60)),
            )));
        }
    }
    None
}

/// Overlay `line` on the top-left of `area`, backed by `bg` only behind the
/// text, so the rest of the row stays waveform.
pub(crate) fn overlay_top_left(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, line: Line<'static>, bg: Style) {
    if area.height == 0 || area.width == 0 { return; }
    let w = (line.width() as u16).min(area.width);
    let rect = ratatui::layout::Rect { x: area.x, y: area.y, width: w, height: 1 };
    frame.render_widget(Paragraph::new(line).style(bg), rect);
}

/// A one-column vertical marker over `area` at `col` — grid anchors on the
/// overview. Drawn after the cached waveform, so no cache invalidation.
pub(crate) fn overlay_marker_column(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, col: u16, style: Style) {
    if area.height == 0 || col >= area.width { return; }
    let rect = ratatui::layout::Rect { x: area.x + col, y: area.y, width: 1, height: area.height };
    let lines: Vec<Line> = (0..area.height).map(|_| Line::from(Span::styled("┃", style))).collect();
    frame.render_widget(Paragraph::new(lines), rect);
}

/// As [`overlay_top_left`], anchored to the bottom-left corner.
pub(crate) fn overlay_bottom_left(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, line: Line<'static>, bg: Style) {
    if area.height == 0 || area.width == 0 { return; }
    let w = (line.width() as u16).min(area.width);
    let rect = ratatui::layout::Rect { x: area.x, y: area.y + area.height - 1, width: w, height: 1 };
    frame.render_widget(Paragraph::new(line).style(bg), rect);
}

/// As [`overlay_top_left`], anchored to the bottom-right corner.
pub(crate) fn overlay_bottom_right(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, line: Line<'static>, bg: Style) {
    if area.height == 0 || area.width == 0 { return; }
    let w = (line.width() as u16).min(area.width);
    let rect = ratatui::layout::Rect { x: area.x + area.width - w, y: area.y + area.height - 1, width: w, height: 1 };
    frame.render_widget(Paragraph::new(line).style(bg), rect);
}

/// The `≡ x/y` position badge shown before the track name when a playlist is
/// active on the deck. Returns the spans and their display width.
fn playlist_badge(deck: &Deck) -> (Vec<Span<'static>>, usize) {
    match &deck.playlist {
        Some(pl) => {
            let text = format!("≡ {}/{}  ", pl.index + 1, pl.playlist.entries.len());
            let width = text.chars().count();
            // Amber when the set carries tracks the deck can't play — the same amber the
            // browser's non-compliant marker uses.
            let color = if pl.unplayable > 0 {
                Color::Rgb(230, 170, 60)
            } else {
                Color::Rgb(120, 210, 180)
            };
            (vec![Span::styled(text, Style::default().fg(color))], width)
        }
        None => (Vec::new(), 0),
    }
}

pub(crate) fn title_empty_span() -> Span<'static> {
    Span::styled("no track", Style::default().fg(Color::Rgb(60, 60, 60)))
}

/// Tempo readouts: BPM or percentage, pitch, metronome, grid offset — the
/// heart of the old info row's left group.
fn tempo_spans(
    deck: &Deck,
    frame_count: usize,
    beat_on: bool,
    analysing: bool,
) -> Vec<Span<'static>> {
    let playback = deck.mode == DeckMode::Playback;
    const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let dim = Style::default().fg(Color::DarkGray);
    if analysing {
        return vec![Span::styled(
            format!("[analysing {}]", SPINNER[(frame_count / 6) % SPINNER.len()]),
            dim,
        )];
    }
    // Beat flash requires Beat mode and an established BPM.
    let beat_active = !playback && beat_on && deck.tempo.bpm_established;
    let beat_style = if beat_active {
        Style::default().fg(Color::Yellow).bg(Color::Rgb(60, 50, 0))
    } else {
        dim
    };
    // Percentage display: Playback mode always; Beat mode when no BPM established.
    let show_percentage = playback || !deck.tempo.bpm_established;

    let left_spans: Vec<Span<'static>> = {
        let mut spans = Vec::new();
        if show_percentage {
            let pct = if playback || !deck.tempo.bpm_established {
                (deck.tempo.playback_speed - 1.0) * 100.0
            } else {
                (deck.tempo.bpm / deck.tempo.base_bpm - 1.0) * 100.0
            };
            let rounded = (pct * 10.0).round() / 10.0;
            let pct_str = if rounded == 0.0 {
                "0.0%".to_string()
            } else if rounded > 0.0 {
                format!("+{:.1}%", rounded)
            } else {
                format!("{:.1}%", rounded)
            };
            spans.push(Span::styled(pct_str, beat_style));
            if deck.pitch_semitones != 0 {
                let pitch_str = if deck.pitch_semitones > 0 {
                    format!("+{}st", deck.pitch_semitones)
                } else {
                    format!("{}st", deck.pitch_semitones)
                };
                let pitch_color = spectral_color(deck.display.palette, 0.0, 0.55);
                spans.push(Span::styled(" (", dim));
                spans.push(Span::styled(pitch_str, Style::default().fg(pitch_color)));
                spans.push(Span::styled(")", dim));
            }
            if !playback && deck.tempo.bpm_established {
                if deck.metronome_mode {
                    spans.push(Span::styled("\u{266A}", Style::default().fg(Color::Red)));
                }
            }
        } else {
            // base_bpm adjusts in 0.01 steps → 2dp; playback bpm adjusts in 0.1 steps → 1dp.
            let adjusted = (deck.tempo.bpm - deck.tempo.base_bpm).abs() >= 0.05;
            let pitched  = deck.pitch_semitones != 0;
            if adjusted || pitched {
                spans.push(Span::styled(format!("{:.2} ", deck.tempo.base_bpm), dim));
                spans.push(Span::styled("(", dim));
                if adjusted {
                    spans.push(Span::styled(format!("{:.1}", deck.tempo.bpm), beat_style));
                }
                if adjusted && pitched {
                    spans.push(Span::styled("  ", dim));
                }
                if pitched {
                    let pitch_str = if deck.pitch_semitones > 0 {
                        format!("+{}st", deck.pitch_semitones)
                    } else {
                        format!("{}st", deck.pitch_semitones)
                    };
                    let pitch_color = spectral_color(deck.display.palette, 0.0, 0.55);
                    spans.push(Span::styled(pitch_str, Style::default().fg(pitch_color)));
                }
                spans.push(Span::styled(")", dim));
            } else {
                spans.push(Span::styled(format!("{:.2}", deck.tempo.base_bpm), beat_style));
            }
            if deck.metronome_mode {
                spans.push(Span::styled("\u{266A}", Style::default().fg(Color::Red)));
            }
        }
        spans
    };
    left_spans
}

/// Level, gain, and PFL — the meters segment of the readout line.
fn meter_spans(deck: &Deck) -> Vec<Span<'static>> {
    let dim = Style::default().fg(Color::DarkGray);
    let mut spans: Vec<Span<'static>> = Vec::new();
    if deck.mixer.pfl_level > 0 {
        spans.push(Span::styled("PFL ", Style::default().fg(Color::Cyan)));
    }
    const LEVEL_BARS: [char; 8] = ['▁','▂','▃','▄','▅','▆','▇','█'];
    let level_idx = ((deck.mixer.volume * 7.0).round() as usize).min(7);
    let t = level_idx as f32 / 7.0;
    let level_style = Style::default()
        .fg(Color::Rgb((60.0 + 195.0 * t).round() as u8, (50.0 + 165.0 * t).round() as u8, 0))
        .bg(Color::Rgb((40.0 * t).round() as u8, (33.0 * t).round() as u8, 0));
    let bracket_style = Style::default().fg(Color::Rgb(140, 140, 140));
    spans.push(Span::styled("lvl:", dim));
    spans.push(Span::styled("\u{2595}", bracket_style));
    spans.push(Span::styled(LEVEL_BARS[level_idx].to_string(), level_style));
    spans.push(Span::styled("\u{258F}", bracket_style));
    {
        const GAIN_CHARS: [char; 7] = ['▁','▂','▃','▄','▅','▆','▇'];
        let idx = ((deck.mixer.gain_db as i32 + 12) * 6 / 24).clamp(0, 6) as usize;
        let gain_style = if deck.mixer.gain_db == 0 {
            Style::default().fg(Color::Rgb(45, 45, 45))
        } else {
            Style::default().fg(Color::Rgb(180, 140, 0))
        };
        spans.push(Span::styled(GAIN_CHARS[idx].to_string(), gain_style));
    }
    spans
}

/// The readout's tail segment: bar interval, spectrum, filter shading, and
/// slope field. The filter's 16 steps map onto the character count (ceiling,
/// so the first step always shades something).
fn spectrum_filter_spans(deck: &Deck, overview_width: usize, analysing: bool) -> Vec<Span<'static>> {
    use crate::deck::SPECTRUM_CHARS;
    let dim = Style::default().fg(Color::DarkGray);
    let mut spans: Vec<Span<'static>> = Vec::new();
    if !analysing {
        let (_, _, bars_per_tick) = bar_tick_cols(deck.tempo.base_bpm as f64, deck.tempo.offset_ms, deck.total_duration, overview_width);
        spans.push(Span::styled(format!("{bars_per_tick}br"), dim));
    }
    let stopband: Option<(bool, usize)> = if deck.mixer.filter_offset != 0 {
        let n = deck.mixer.filter_offset.unsigned_abs() as usize;
        let scaled = (n * SPECTRUM_CHARS).div_ceil(16);
        let is_lpf = deck.mixer.filter_offset < 0;
        let cutoff_char = if is_lpf { SPECTRUM_CHARS - scaled } else { scaled };
        Some((is_lpf, cutoff_char))
    } else {
        None
    };
    spans.push(Span::styled("\u{2595}".to_string(), dim));
    for i in 0..SPECTRUM_CHARS {
        let ch = deck.spectrum.chars[i].to_string();
        let in_stopband = stopband.map_or(false, |(is_lpf, cutoff_char)| {
            if is_lpf { i >= cutoff_char } else { i < cutoff_char }
        });
        let style = if in_stopband {
            if ch != "\u{2800}" {
                Style::default().fg(Color::Rgb(120, 100, 0)).bg(Color::Rgb(50, 50, 50))
            } else {
                Style::default().bg(Color::Rgb(50, 50, 50))
            }
        } else if ch != "\u{2800}" {
            Style::default().fg(Color::Yellow).bg(Color::Rgb(40, 33, 0))
        } else if deck.spectrum.bg[i] {
            Style::default().bg(Color::Rgb(40, 33, 0))
        } else {
            Style::default()
        };
        spans.push(Span::styled(ch, style));
    }
    spans.push(Span::styled("\u{258F}".to_string(), dim));
    // dB/oct indicator — present only while the filter is active.
    let slope_str = if deck.mixer.filter_offset != 0 { match deck.mixer.filter_poles { 4 => "24", _ => "12" } } else { "" };
    spans.push(Span::styled(slope_str, dim));
    spans
}

/// Bottom-right corner: the deck's whole readout in one `│`-separated line —
/// tempo and offset, level and gain, bar interval + spectrum and filter.
/// Anchored bottom-right so a narrow terminal
/// costs waveform, never the title or the stats; a countdown prompt displaces
/// it for its seconds-long life.
pub(crate) fn readout_corner_line(
    deck: &Deck,
    overview_width: usize,
    analysing: bool,
    grid_status: Option<String>,
) -> Line<'static> {
    let sep = Span::styled("│", Style::default().fg(Color::Rgb(70, 70, 90)));
    let (mode_tag, tag_style) = match grid_status {
        Some(status) => (status, Style::default().fg(GRID_BLUE)),
        None => (match deck.mode {
            DeckMode::Playback => "PLAY",
            DeckMode::Beat     => "BEAT",
        }.to_string(), Style::default().fg(Color::Rgb(110, 110, 130))),
    };
    let mut spans = vec![Span::styled(mode_tag, tag_style), sep.clone()];
    if deck.mode == DeckMode::Beat && deck.tempo.bpm_established {
        spans.push(Span::styled(format!("{:+}ms", deck.tempo.offset_ms), Style::default().fg(Color::DarkGray)));
        spans.push(sep.clone());
    }
    spans.extend(meter_spans(deck));
    spans.push(sep);
    spans.extend(spectrum_filter_spans(deck, overview_width, analysing));
    Line::from(spans)
}

/// Bottom-left corner, transient content only: the tap counter while tapping
/// and the nudge arrows while nudging. `None` in every steady state, so the
/// waveform stays clear.
pub(crate) fn bottom_transient_line(deck: &Deck) -> Option<Line<'static>> {
    let dim = Style::default().fg(Color::DarkGray);
    let mut spans: Vec<Span<'static>> = Vec::new();
    match deck.nudge {
        1  => spans.push(Span::styled("▶nudge", dim)),
        -1 => spans.push(Span::styled("◀nudge", dim)),
        _  => {}
    }
    let tap_active = !deck.tap.tap_times.is_empty()
        && deck.tap.last_tap_wall.map_or(false, |t| t.elapsed().as_secs_f64() < 2.0);
    if tap_active {
        if !spans.is_empty() { spans.push(Span::raw("  ")); }
        let tap_flash_on = deck.tap.last_tap_wall.map_or(false, |t| t.elapsed().as_millis() < 150);
        let style = if tap_flash_on {
            Style::default().fg(Color::Yellow).bg(Color::Rgb(60, 50, 0))
        } else {
            dim
        };
        spans.push(Span::styled(format!("tap:{}", deck.tap.tap_times.len()), style));
    }
    if spans.is_empty() { None } else { Some(Line::from(spans)) }
}

/// Rebuild the deck's overview if any input it depends on has changed, storing the
/// result (and the bar columns/times used by needle drop) on the deck. Most frames
/// this is a key comparison and nothing else — the playhead column moves every few
/// seconds, the flash states a few times per second.
pub(crate) fn refresh_overview_for_deck(
    deck: &mut Deck,
    rect: ratatui::layout::Rect,
    display_samp: f64,
    analysing: bool,
    warning_active: bool,
    warn_beat_on: bool,
) {
    use crate::deck::{OverviewCache, OverviewKey};
    let overview_width  = rect.width  as usize;
    let playhead_frac = if deck.total_duration == 0.0 {
        0.0
    } else {
        (display_samp / deck.audio.sample_rate as f64 / deck.total_duration).clamp(0.0, 1.0)
    };
    let playhead_col = ((playhead_frac * overview_width as f64).round() as usize)
        .min(overview_width.saturating_sub(1));
    let cue_col: Option<usize> = deck.cue_sample.map(|samp| {
        let frac = (samp as f64 / deck.audio.sample_rate as f64
            / deck.total_duration).clamp(0.0, 1.0);
        ((frac * overview_width as f64).round() as usize)
            .min(overview_width.saturating_sub(1))
    });
    let key = OverviewKey {
        width: overview_width,
        height: rect.height as usize,
        playhead_col,
        cue_col,
        analysing,
        warning_active,
        warn_beat_on,
        gain_db: deck.mixer.gain_db,
        base_bpm_bits: deck.tempo.base_bpm.to_bits(),
        offset_ms: deck.tempo.offset_ms,
        palette: deck.display.palette,
    };
    if deck.display.overview_cache.as_ref().map_or(false, |c| c.key == key) {
        return;
    }
    let (lines, bar_cols, bar_times) =
        overview_lines(deck, rect, playhead_col, cue_col, analysing, warning_active, warn_beat_on);
    deck.display.last_bar_cols  = bar_cols;
    deck.display.last_bar_times = bar_times;
    deck.display.overview_cache = Some(OverviewCache { key, paragraph: Paragraph::new(lines) });
}

fn overview_lines(
    deck: &Deck,
    rect: ratatui::layout::Rect,
    playhead_col: usize,
    cue_col: Option<usize>,
    analysing: bool,
    warning_active: bool,
    warn_beat_on: bool,
) -> (Vec<Line<'static>>, Vec<usize>, Vec<f64>) {
    let overview_width  = rect.width  as usize;
    let overview_height = rect.height as usize;
    let total_peaks = deck.audio.waveform.peaks.len();

    let gain_linear = 10f32.powf(deck.mixer.gain_db as f32 / 20.0);
    let hires: Vec<((f32, f32), f32)> = (0..overview_width * 2)
        .map(|col| {
            let idx = (col * total_peaks / (overview_width * 2).max(1)).min(total_peaks.saturating_sub(1));
            let (min_v, max_v) = deck.audio.waveform.peaks[idx];
            let bass = deck.audio.waveform.bass_ratio[idx];
            ((min_v * gain_linear, max_v * gain_linear), bass)
        })
        .collect();
    let ov_peaks_hires: Vec<(f32, f32)> = hires.iter().map(|(p, _)| *p).collect();
    let ov_bass_hires: Vec<f32>          = hires.iter().map(|(_, b)| *b).collect();
    let hires_buf = render_braille(&ov_peaks_hires, overview_height, overview_width * 2);
    let ov_braille: Vec<Vec<u8>> = hires_buf.iter()
        .map(|row| (0..overview_width).map(|c| (row[c * 2] & 0x47) | (row[c * 2 + 1] & 0xB8)).collect())
        .collect();
    let ov_bass: Vec<f32> = (0..overview_width)
        .map(|c| (ov_bass_hires[c * 2] + ov_bass_hires[c * 2 + 1]) / 2.0)
        .collect();
    let (bar_cols, bar_times, _bars_per_tick): (Vec<usize>, Vec<f64>, u32) = if !analysing {
        bar_tick_cols(deck.tempo.base_bpm as f64, deck.tempo.offset_ms, deck.total_duration, overview_width)
    } else {
        (Vec::new(), Vec::new(), 4)
    };
    let is_bar_col = {
        let mut mask = vec![false; overview_width];
        for &c in &bar_cols { mask[c] = true; }
        mask
    };
    let lut = SpectralLut::new(deck.display.palette, 0.8);

    let ov_lines: Vec<Line<'static>> = ov_braille
        .into_iter()
        .enumerate()
        .map(|(r, row)| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut run = String::new();
            let mut run_color = Color::Reset;
            for (c, byte) in row.into_iter().enumerate() {
                let (color, ch) = if c == playhead_col && cue_col == Some(c) {
                    if r == 0 || r + 1 == overview_height {
                        (Color::Rgb(255, 0, 255), '\u{28FF}')
                    } else {
                        (Color::Rgb(255, 255, 255), '\u{28FF}')
                    }
                } else if c == playhead_col {
                    (Color::Rgb(255, 255, 255), '\u{28FF}')
                } else if cue_col == Some(c) {
                    (Color::Rgb(255, 0, 255), '\u{28FF}')
                } else if is_bar_col[c] {
                    if warn_beat_on {
                        (Color::Rgb(120, 60, 60), '│')
                    } else if warning_active {
                        (Color::Rgb(40, 20, 20), '│')
                    } else {
                        (Color::DarkGray, '│')
                    }
                } else {
                    (lut.color(ov_bass[c]), char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
                };
                if color != run_color {
                    if !run.is_empty() {
                        spans.push(Span::styled(
                            std::mem::take(&mut run),
                            Style::default().fg(run_color),
                        ));
                    }
                    run_color = color;
                }
                run.push(ch);
            }
            if !run.is_empty() {
                spans.push(Span::styled(run, Style::default().fg(run_color)));
            }
            Line::from(spans)
        })
        .collect();

    (ov_lines, bar_cols, bar_times)
}

fn empty_deck_mesh_line(w: usize, bg: Color, fg: Color) -> Line<'static> {
    // U+2895 has dots 1,3,5,8 — a checkerboard within the 2×4 cell (•./. •/•./. •).
    // Tiling identical characters continues the alternating pattern seamlessly in
    // both directions, so no per-column logic is needed.
    let s: String = std::iter::repeat('\u{2895}').take(w).collect();
    Line::from(Span::styled(s, Style::default().fg(fg).bg(bg)))
}

pub(crate) fn overview_empty(rect: ratatui::layout::Rect, deck_slot: usize) -> Vec<Line<'static>> {
    let w = rect.width as usize;
    let h = rect.height as usize;
    let bg = Color::Rgb(11, 11, 15);
    let fg = if deck_slot == 0 {
        Color::Rgb(26, 26, 36)
    } else {
        Color::Rgb(17, 17, 24)
    };
    vec![empty_deck_mesh_line(w, bg, fg); h]
}

pub(crate) fn render_detail_empty(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    deck_slot: usize,
) {
    let w = area.width as usize;
    let h = area.height as usize;
    let bg = Color::Rgb(11, 11, 15);
    let fg = if deck_slot == 0 {
        Color::Rgb(26, 26, 36)
    } else {
        Color::Rgb(17, 17, 24)
    };
    let lines: Vec<Line<'static>> = vec![empty_deck_mesh_line(w, bg, fg); h];
    frame.render_widget(Paragraph::new(lines), area);
}

/// Render embedded cover art as half-block characters (`▀`), filling the area.
/// The image is scaled to cover the full cols×rows cell area (cropping the longer
/// axis symmetrically). `brightness` (0.0–1.0) dims the pixel values uniformly.
pub(crate) fn halfblock_art(bytes: &[u8], cols: u16, rows: u16, brightness: f32) -> Vec<Line<'static>> {
    use image::imageops::FilterType;
    if cols == 0 || rows == 0 { return vec![]; }
    let img = match image::load_from_memory(bytes) {
        Ok(i) => i,
        Err(_) => return vec![],
    };
    let pixel_w = cols as u32;
    let pixel_h = rows as u32 * 2;
    // Scale to a square large enough to cover both dimensions, then crop to the panel.
    let art_px = pixel_w.max(pixel_h);
    let x_off  = (art_px - pixel_w) / 2;
    let y_off  = (art_px - pixel_h) / 2;
    let rgb = img.resize_exact(art_px, art_px, FilterType::Triangle).to_rgb8();
    let dim = |c: u8| (c as f32 * brightness) as u8;
    (0..rows).map(|row| {
        let spans: Vec<Span<'static>> = (0..cols).map(|col| {
            let px      = col as u32 + x_off;
            let py_top  = row as u32 * 2 + y_off;
            let py_bot  = (py_top + 1).min(art_px - 1);
            let top = rgb.get_pixel(px, py_top);
            let bot = rgb.get_pixel(px, py_bot);
            Span::styled("▀", Style::default()
                .fg(Color::Rgb(dim(top[0]), dim(top[1]), dim(top[2])))
                .bg(Color::Rgb(dim(bot[0]), dim(bot[1]), dim(bot[2]))))
        }).collect();
        Line::from(spans)
    }).collect()
}

pub(crate) fn render_editor_field(label: &'static str, text: &str, active: bool, cursor: usize, text_width: usize) -> Vec<Line<'static>> {
    let label_style  = Style::default().fg(Color::Rgb(90, 110, 150));
    let text_style   = Style::default().fg(if active { Color::White } else { Color::Rgb(200, 220, 255) });
    let cursor_style = Style::default().fg(Color::Black).bg(Color::Yellow);
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());

    // Split into visual rows; always at least one so an empty field still renders.
    let rows: Vec<&[char]> = if chars.is_empty() {
        vec![&[]]
    } else {
        chars.chunks(text_width).collect()
    };

    rows.iter().enumerate().map(|(row_idx, row_chars)| {
        let start      = row_idx * text_width;
        let row_len    = row_chars.len();
        let is_last    = row_idx == rows.len() - 1;
        let prefix     = if row_idx == 0 { format!("{label}: ") } else { " ".repeat(9) };
        let cursor_here = active && (
            (cursor >= start && cursor < start + row_len) ||
            (is_last && cursor == chars.len())
        );
        if cursor_here {
            let local = cursor - start;
            let before: String = row_chars[..local].iter().collect();
            let (at_cur, after): (String, String) = if local < row_len {
                (row_chars[local].to_string(), row_chars[local + 1..].iter().collect())
            } else {
                (" ".to_string(), String::new())
            };
            Line::from(vec![
                Span::styled(prefix, label_style),
                Span::styled(before, text_style),
                Span::styled(at_cur, cursor_style),
                Span::styled(after, text_style),
            ])
        } else {
            Line::from(vec![
                Span::styled(prefix, label_style),
                Span::styled(row_chars.iter().collect::<String>(), text_style),
            ])
        }
    }).collect()
}

pub(crate) fn section_divider(label: &'static str, inner_width: usize) -> Line<'static> {
    let fill = "─".repeat(inner_width.saturating_sub(4 + label.len()));
    Line::from(vec![
        Span::styled("── ", Style::default().fg(Color::Rgb(70, 90, 130))),
        Span::styled(label, Style::default().fg(Color::Rgb(120, 140, 175))),
        Span::styled(format!(" {fill}"), Style::default().fg(Color::Rgb(70, 90, 130))),
    ])
}

pub(crate) fn compose_shared_tick_row(tick_a: &[u8], tick_b: &[u8], width: usize) -> Vec<u8> {
    let mut row = vec![0u8; width];
    for c in 0..width {
        let a = tick_a.get(c).copied().unwrap_or(0);
        let b = tick_b.get(c).copied().unwrap_or(0);
        if a != 0 {
            let (c_pat, c1_pat): (u8, u8) = if a & 0x01 != 0 {
                (0x1A, 0x02) // left sub-col: tip=row1-col1, base=row2 cols 0,1 + col0 of c+1
            } else {
                (0x10, 0x13) // right sub-col: base=row2 col1 + cols 0,1 of c+1, tip=row1-col0 of c+1
            };
            row[c] |= c_pat;
            if c + 1 < width { row[c + 1] |= c1_pat; }
        }
        if b != 0 {
            let (c_pat, c1_pat): (u8, u8) = if b & 0x01 != 0 {
                (0xA4, 0x04) // left sub-col: base=row3 cols 0,1 + col0 of c+1, tip=row4-col1
            } else {
                (0x20, 0x64) // right sub-col: base=row3 col1 + cols 0,1 of c+1, tip=row4-col0 of c+1
            };
            row[c] |= c_pat;
            if c + 1 < width { row[c + 1] |= c1_pat; }
        }
    }
    row
}

/// Extract a screen-width slice of tick data from a pre-rendered buffer, applying the
/// same half-column viewport transform as the waveform so ticks stay locked to peaks.
/// When `sub_col` is true (the viewport is offset by one half-column), tick bytes are
/// shifted: right sub-col (0xB8) at buffer column c becomes left sub-col (0x47) at
/// screen column c, and left sub-col (0x47) at buffer column c becomes right sub-col
/// (0xB8) at screen column c−1.
/// Screen column of `sample` in the detail viewport, through the same
/// buffer-anchored arithmetic the waveform and ticks render with — overlay
/// markers computed here can never drift against them. Right half-columns
/// round rightward, matching the glyph-tick centring.
pub(crate) fn sample_screen_col(
    buf: &BrailleBuffer,
    view_pos: usize,
    centre_col: usize,
    sample: usize,
) -> Option<i64> {
    if buf.samples_per_col == 0 || buf.buf_cols == 0 { return None; }
    let half_col   = buf.samples_per_col as f64 / 2.0;
    let delta      = view_pos as i64 - buf.anchor_sample as i64;
    let delta_half = (delta as f64 / half_col).round() as i64;
    let delta_cols = delta_half.div_euclid(2);
    let sub_col    = delta_half.rem_euclid(2) != 0;
    let viewport_off = buf.buf_cols as i64 / 2 + delta_cols - centre_col as i64;
    let view_start = buf.anchor_sample as f64 - (buf.buf_cols / 2) as f64 * buf.samples_per_col as f64;
    let disp_half  = ((sample as f64 - view_start) / half_col).round() as i64;
    let screen_half = disp_half - 2 * viewport_off - (sub_col as i64);
    Some((screen_half + 1).div_euclid(2))
}

pub(crate) fn extract_tick_viewport(
    buf:        &BrailleBuffer,
    display_pos: usize,
    centre_col: usize,
    width:      usize,
) -> Vec<u8> {
    if buf.samples_per_col == 0 || buf.tick.is_empty() {
        return vec![0u8; width];
    }
    let half_col      = buf.samples_per_col as f64 / 2.0;
    let delta         = display_pos as i64 - buf.anchor_sample as i64;
    let delta_half    = (delta as f64 / half_col).round() as i64;
    let delta_cols    = delta_half.div_euclid(2);
    let sub_col       = delta_half.rem_euclid(2) != 0;
    let viewport_off  = buf.buf_cols as i64 / 2 + delta_cols - centre_col as i64;
    let need          = if sub_col { width + 1 } else { width };
    if viewport_off < 0 || (viewport_off as usize) + need > buf.buf_cols {
        return vec![0u8; width];
    }
    let start = viewport_off as usize;
    if !sub_col {
        buf.tick[start..start + width].to_vec()
    } else {
        // With a half-column shift: 0xB8 (right) at buf col → 0x47 (left) at same screen col;
        // 0x47 (left) at buf col → 0xB8 (right) at previous screen col.
        let mut out = vec![0u8; width];
        for c in 0..=width {
            let b = buf.tick[start + c];
            if b == 0xB8 && c < width { out[c]     = 0x47; }
            if b == 0x47 && c > 0     { out[c - 1] = 0xB8; }
        }
        out
    }
}

pub(crate) fn compute_tick_display(
    detail_width:    usize,
    samples_per_col: usize,
    marker_view_start: f64,
    analysing:       bool,
    base_bpm:        f32,
    sample_rate:     u32,
    offset_ms:       i64,
) -> Vec<u8> {
    if analysing || samples_per_col == 0 {
        return vec![0u8; detail_width];
    }
    let mut row = vec![0u8; detail_width];
    let samples_per_col    = samples_per_col as f64;
    let half_samples_per_col = samples_per_col / 2.0;
    let beat_period_samp   = 60.0 / base_bpm as f64 * sample_rate as f64;
    let offset_samp        = offset_ms as f64 / 1000.0 * sample_rate as f64;
    let view_end           = marker_view_start + detail_width as f64 * samples_per_col;
    let n_start            = ((marker_view_start - offset_samp) / beat_period_samp).floor() as i64 - 1;
    let mut t_samp         = offset_samp + n_start as f64 * beat_period_samp;
    while t_samp <= view_end {
        let disp_half = ((t_samp - marker_view_start) / half_samples_per_col).round() as i64;
        if disp_half >= 0 {
            let col = (disp_half / 2) as usize;
            if col < detail_width {
                row[col] = if disp_half % 2 != 0 { 0xB8 } else { 0x47 };
            }
        }
        t_samp += beat_period_samp;
    }
    row
}

pub(crate) fn peaks_for_slot(
    wf: &Option<Arc<WaveformData>>,
    anchor: usize,
    col_samp: usize,
    buf_cols: usize,
) -> Vec<(f32, f32)> {
    let Some(wf) = wf else {
        return vec![(0.0, 0.0); buf_cols];
    };
    let mono = &wf.mono;
    (0..buf_cols).map(|c| {
        let offset    = c as i64 - (buf_cols / 2) as i64;
        let raw_start = anchor as i64 + offset * col_samp as i64;
        if raw_start < 0 {
            return (1.0, -1.0);
        }
        let samp_start = raw_start as usize;
        let samp_end   = (samp_start + col_samp).min(mono.len());
        if samp_start >= mono.len() {
            return (1.0, -1.0);
        }
        let chunk = &mono[samp_start..samp_end];
        let mn = chunk.iter().cloned().fold(f32::INFINITY,     f32::min);
        let mx = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        (mn.max(-1.0), mx.min(1.0))
    }).collect()
}

/// Compute per-column bass ratio directly from raw samples using an IIR low-pass,
/// smoothed with a box filter to avoid sharp colour transitions at wide zoom.
pub(crate) fn spectral_for_slot(
    wf: &Option<Arc<WaveformData>>,
    anchor: usize,
    col_samp: usize,
    buf_cols: usize,
    sample_rate: u32,
) -> Vec<f32> {
    let Some(wf) = wf else {
        return vec![0.5; buf_cols];
    };
    let mono  = &wf.mono;
    let alpha = {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * 250.0);
        let dt = 1.0 / sample_rate as f32;
        dt / (rc + dt)
    };
    let bass_raw: Vec<f32> = (0..buf_cols).map(|c| {
        let offset    = c as i64 - (buf_cols / 2) as i64;
        let raw_start = anchor as i64 + offset * col_samp as i64;
        if raw_start < 0 || raw_start as usize >= mono.len() {
            return 0.5;
        }
        let samp_start = raw_start as usize;
        let chunk = &mono[samp_start..(samp_start + col_samp).min(mono.len())];
        if chunk.is_empty() { return 0.5; }
        let total_energy: f32 = chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32;
        let mut lp = 0.0f32;
        let lp_energy: f32 = chunk.iter().map(|&s| { lp += alpha * (s - lp); lp * lp })
            .sum::<f32>() / chunk.len() as f32;
        (lp_energy / (total_energy + 1e-10)).clamp(0.0, 1.0)
    }).collect();
    box_smooth(&bass_raw, 3)
}


pub(crate) fn box_smooth(v: &[f32], radius: usize) -> Vec<f32> {
    let n = v.len();
    (0..n).map(|i| {
        let lo = i.saturating_sub(radius);
        let hi = (i + radius + 1).min(n);
        v[lo..hi].iter().sum::<f32>() / (hi - lo) as f32
    }).collect()
}

/// Takes the right dot-column of `a` (bits 3,4,5,7) as the new left column (bits 0,1,2,6)
/// and the left dot-column of `b` (bits 0,1,2,6) as the new right column (bits 3,4,5,7).
pub(crate) fn shift_braille_half(a: u8, b: u8) -> u8 {
    let left  = ((a >> 3) & 0x07) | ((a >> 1) & 0x40);
    let right = ((b & 0x07) << 3) | ((b & 0x40) << 1);
    left | right
}

pub(crate) fn render_braille(peaks: &[(f32, f32)], rows: usize, cols: usize) -> Vec<Vec<u8>> {
    // Bit mask for left+right dots at each of the 4 dot-rows within a Braille cell.
    // Layout: dot1(bit0)/dot4(bit3), dot2(bit1)/dot5(bit4), dot3(bit2)/dot6(bit5), dot7(bit6)/dot8(bit7)
    const DOT_BITS: [u8; 4] = [0x09, 0x12, 0x24, 0xC0];

    let mut grid = vec![vec![0u8; cols]; rows];
    if rows == 0 || cols == 0 {
        return grid;
    }
    let total_dots = rows * 4;

    let mut set_dot = |c: usize, d: usize| {
        let br = d / 4;
        let dr = d % 4;
        if br < rows {
            grid[br][c] |= DOT_BITS[dr];
        }
    };

    for (c, &(min_val, max_val)) in peaks.iter().take(cols).enumerate() {
        let clamped_max = max_val.min(1.0);
        let clamped_min = min_val.max(-1.0);
        if clamped_min > clamped_max { continue; }
        // Map y ∈ [-1, 1] → dot row ∈ [0, total_dots); y=1 is top (row 0).
        let top_dot = ((1.0 - clamped_max) / 2.0 * total_dots as f32) as usize;
        let bot_dot = {
            let raw = (((1.0 - clamped_min) / 2.0 * total_dots as f32) as usize)
                .min(total_dots - 1);
            if raw > top_dot && raw + top_dot >= total_dots { raw - 1 } else { raw }
        };
        for d in top_dot..=bot_dot { set_dot(c, d); }
    }
    grid
}

/// Return the column indices of bar-tick lines within the overview, and the bars-per-tick interval.
///
/// Starts at 4 bars and doubles until all adjacent ticks are at least 2 columns apart
/// (leaving at least 1 blank character gap between every pair of markers).
pub(crate) fn bar_tick_cols(bpm: f64, offset_ms: i64, total_secs: f64, cols: usize) -> (Vec<usize>, Vec<f64>, u32) {
    if bpm <= 0.0 || total_secs <= 0.0 || cols == 0 {
        return (Vec::new(), Vec::new(), 4);
    }
    let beat_secs = 60.0 / bpm;
    let offset_secs = offset_ms as f64 / 1000.0;
    let mut bars: u32 = 4;
    loop {
        let bar_period = bars as f64 * 4.0 * beat_secs; // bars × 4 beats/bar × secs/beat
        let n_start = (-offset_secs / bar_period).ceil() as i64;
        let mut result: Vec<(usize, f64)> = Vec::new();
        let mut t = offset_secs + n_start as f64 * bar_period;
        while t <= total_secs {
            let col = ((t / total_secs) * cols as f64).round() as usize;
            if col < cols {
                result.push((col, t.max(0.0)));
            }
            t += bar_period;
        }
        let min_gap = result.windows(2)
            .map(|w| w[1].0.saturating_sub(w[0].0))
            .min()
            .unwrap_or(usize::MAX);
        if min_gap >= 4 || bars >= 512 {
            let cols_vec = result.iter().map(|&(c, _)| c).collect();
            let times_vec = result.iter().map(|&(_, t)| t).collect();
            return (cols_vec, times_vec, bars);
        }
        bars *= 2;
    }
}

pub(crate) fn render_detail_waveform(
    frame: &mut ratatui::Frame,
    buf: &Arc<BrailleBuffer>,
    deck: &mut Deck,
    detail_area: ratatui::layout::Rect,
    display_cfg: &crate::config::DisplayConfig,
    display_pos_samp: usize,
    palette: SpecPalette,
) {
    let detail_width      = detail_area.width  as usize;
    let detail_panel_rows = detail_area.height as usize;
    let buf = Arc::clone(buf);
    let centre_col = ((detail_width as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
        .clamp(0, detail_width.saturating_sub(1));

    let lut = SpectralLut::new(palette, 1.0);
    let half_col_samp: f64 = buf.samples_per_col as f64 / 2.0;
    let mut sub_col = false;
    let viewport_start: Option<usize> = if buf.buf_cols >= detail_width && buf.samples_per_col > 0 {
        let delta = display_pos_samp as i64 - buf.anchor_sample as i64;
        let delta_half = (delta as f64 / half_col_samp).round() as i64;
        sub_col = delta_half % 2 != 0;
        let delta_cols = delta_half.div_euclid(2);
        let viewport_offset = buf.buf_cols as i64 / 2 + delta_cols - centre_col as i64;
        let need = if sub_col { detail_width + 1 } else { detail_width };
        if viewport_offset >= 0 && (viewport_offset as usize) + need <= buf.buf_cols {
            let start = viewport_offset as usize;
            deck.display.last_viewport_start = start;
            Some(start)
        } else {
            None
        }
    } else {
        None
    };

    // Cue column is pre-computed by the background thread in buffer space, using the
    // same anchor and samples_per_col as the waveform and ticks. Map to screen here
    // via viewport_start — identical to how ticks are handled.
    let cue_screen_col: Option<usize> = viewport_start.and_then(|vs| {
        buf.cue_buf_col.and_then(|cbc| {
            if cbc >= vs && cbc < vs + detail_width { Some(cbc - vs) } else { None }
        })
    });

    let waveform_rows = detail_panel_rows;

    let detail_lines: Vec<Line<'static>> = (0..waveform_rows)
        .map(|r| {
            // buf_r maps directly: row 0 → buffer row 0.
            let buf_r = r;
            let shifted: Option<Vec<u8>>;
            let row_slice: Option<&[u8]>;
            shifted = if sub_col {
                viewport_start.and_then(|start| {
                    buf.grid.get(buf_r).map(|row| {
                        (0..detail_width).map(|c| shift_braille_half(row[start + c], row[start + c + 1])).collect()
                    })
                })
            } else { None };
            row_slice = if sub_col {
                shifted.as_deref()
            } else {
                viewport_start.and_then(|start| buf.grid.get(buf_r).map(|row| &row[start..start + detail_width]))
            };
            let _ = &shifted;
            let actual_rows = buf.grid.len().min(waveform_rows);
            let is_edge_row = r == 0 || r + 1 == actual_rows;
            let row = match row_slice {
                None => return Line::from(Span::raw("\u{2800}".repeat(detail_width))),
                Some(s) => s,
            };
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut run = String::new();
            let mut run_color = Color::Reset;
            for (c, &byte) in row.iter().enumerate() {
                let buf_col  = viewport_start.unwrap_or(0) + c;
                let bass     = buf.bass_ratio.get(buf_col).copied().unwrap_or(0.5);
                let spectral = lut.color(bass);
                let (color, ch) = if c == centre_col && cue_screen_col == Some(c) {
                    if is_edge_row {
                        (Color::Rgb(255, 0, 255), '\u{28FF}')
                    } else {
                        (Color::Rgb(255, 255, 255), '\u{28FF}')
                    }
                } else if c == centre_col {
                    (Color::Rgb(255, 255, 255), '\u{28FF}')
                } else if cue_screen_col == Some(c) {
                    (Color::Rgb(255, 0, 255), '\u{28FF}')
                } else {
                    (spectral, char::from_u32(0x2800 | byte as u32).unwrap_or(' '))
                };
                if color != run_color {
                    if !run.is_empty() {
                        spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(run_color)));
                    }
                    run_color = color;
                }
                run.push(ch);
            }
            if !run.is_empty() {
                spans.push(Span::styled(run, Style::default().fg(run_color)));
            }
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(detail_lines), detail_area);
}

pub(crate) fn render_shared_tick_row(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    tick_a: &[u8],
    tick_b: &[u8],
    a_detached: bool,
    b_detached: bool,
) {
    let w = area.width as usize;
    let display_row = compose_shared_tick_row(tick_a, tick_b, w);
    // A detached deck's ticks read blue — the grid-work accent — so the
    // alignment instrument itself signals the mode. Only that deck's ticks
    // change; its row-mate keeps gray.
    let blue = Style::default().fg(GRID_BLUE);
    let gray = Style::default().fg(Color::Gray);
    let mut cells: Vec<(char, Style)> = display_row.iter().map(|&byte| {
        let ch = if byte != 0 { char::from_u32(0x2800 | byte as u32).unwrap_or(' ') } else { ' ' };
        (ch, gray)
    }).collect();
    // A detached deck's ticks become three-wide glyph markers centred on the
    // tick's column — the braille half-column form reads off-centre once the
    // detached view snaps to whole columns. The stem points at the deck the
    // ticks belong to (`┴` above the row, `┬` below); the row-mate's braille
    // ticks stay as they are.
    let mut glyphs = |ticks: &[u8], stem: char| {
        for (c, &byte) in ticks.iter().enumerate() {
            if byte == 0 { continue; }
            let centre = if byte == 0xB8 { (c + 1).min(w.saturating_sub(1)) } else { c };
            for (col, ch) in [(centre.wrapping_sub(1), '─'), (centre, stem), (centre + 1, '─')] {
                if col < w {
                    cells[col] = (ch, blue);
                }
            }
        }
    };
    if a_detached { glyphs(tick_a, '┴'); }
    if b_detached { glyphs(tick_b, '┬'); }
    let spans: Vec<Span> = cells.into_iter().map(|(ch, style)| Span::styled(ch.to_string(), style)).collect();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub(crate) fn render_keyboard_help(frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    const TEXT_W: u16 = 79;
    const TEXT_H: u16 = 15;
    const H_PAD:  u16 = 2;
    const V_PAD:  u16 = 1;

    // The box sits one row below the top of the spacer to clear the deck-2 margin.
    // available_h accounts for that offset so the rect never overflows the buffer.
    let available_h = area.height.saturating_sub(1);
    if available_h == 0 { return; }

    let outer_w = (TEXT_W + H_PAD * 2).min(area.width);
    let outer_h = (TEXT_H + V_PAD * 2).min(available_h);
    let x = area.x + area.width.saturating_sub(outer_w) / 2;
    let outer = ratatui::layout::Rect { x, y: area.y + 1, width: outer_w, height: outer_h };

    // Clear replaces halfblock art characters with spaces; Block then fills with dark bg.
    // Without Clear, set_style leaves the art ▀ characters in place, producing a comb edge.
    let bg = Style::default().bg(Color::Rgb(15, 15, 15));
    frame.render_widget(Clear, outer);
    frame.render_widget(Block::default().style(bg), outer);

    let inner = ratatui::layout::Rect {
        x:      outer.x + H_PAD.min(outer.width / 2),
        y:      outer.y + V_PAD.min(outer.height / 2),
        width:  outer.width.saturating_sub(H_PAD * 2),
        height: outer.height.saturating_sub(V_PAD * 2),
    };

    let sh = Style::default().fg(Color::Rgb(130, 100,  50));  // Shift layer: dim warm amber
    let ba = Style::default().fg(Color::Rgb(170, 170, 170));  // Bare layer:  medium gray
    let sp = Style::default().fg(Color::Rgb( 60, 100, 160));  // Space (chord) layer: dim cool blue
    let gr = Style::default().fg(Color::Rgb( 80, 140,  70));  // nudge / BPM: muted sage
    let wh = Style::default().fg(Color::White);               // F-key bracket exception

    // Row 7: Shift — F's ╭ bracket stays white
    let row7 = Line::from(vec![
        Span::styled("    ╭         ╭         ╭ +Tick   ", sh),
        Span::styled("╭", wh),
        Span::styled(" -BsBPM  ╭ CueJp   ┆   ╭  ╭  ╭ +Gain", sh),
    ]);
    // Row 8: Bare — F key name stays white; +Ndge / -BPM use muted sage
    let row8 = Line::from(vec![
        Span::styled("    A +Ptch   S +PFL    D ", ba),
        Span::styled("+Ndge", gr),
        Span::styled("   ", ba),
        Span::styled("F", wh),
        Span::styled(" ", ba),
        Span::styled("-BPM", gr),
        Span::styled("    G Grid    ┆   J  K  L +Lvl", ba),
    ]);
    // Row 9: Space (chord) — F's ╰ bracket stays white
    let row9 = Line::from(vec![
        Span::styled("    ╰ =Ptch   ╰ Rst     ╰ PFLTog  ", sp),
        Span::styled("╰", wh),
        Span::styled(" Brows   ╰ Play    ┆   ╰  ╰  ╰ 100%", sp),
    ]);
    // Row 11: Bare — -Ndge / +BPM use muted sage
    let row11 = Line::from(vec![
        Span::styled("      Z -Ptch   X -PFL    C ", ba),
        Span::styled("-Ndge", gr),
        Span::styled("   V ", ba),
        Span::styled("+BPM", gr),
        Span::styled("    B Tap     ┆   M  ,  . -Lvl", ba),
    ]);
    // Row 13: separator — modifier legend as vertical ╭│╰ box flush-right
    let row13 = Line::from(vec![
        Span::styled("───────────────────────────────────────────────────────────────────── ", ba),
        Span::styled("╭ [Shift]", sh),
    ]);
    // Row 15: second footer line — ╰ [Space] flush-right
    let row15 = Line::from(vec![
        Span::styled("/ art   Sp+= swap1↔2   Sp+- swap2↔3   Alt+j/k deck                    ", ba),
        Span::styled("╰ [Space]", sp),
    ]);

    let lines: Vec<Line<'static>> = vec![
        Line::styled("╭         ╭         ╭         ╭ +32b    ╭ +64b    ┆   ╭  ╭  ╭ +Slope", sh),
        Line::styled("1 +1bt    2 +1b     3 +4b     4 +8b     5 +16b    ┆   7  8  9 HPF", ba),
        Line::styled("╰ SelD1   ╰ SelD2   ╰ SelD3   ╰         ╰         ┆   ╰  ╰  ╰ Flt=", sp),
        Line::styled("  ╭         ╭         ╭         ╭ -32b    ╭ -64b    ┆   ╭  ╭  ╭ -Slope", sh),
        Line::styled("  Q -1bt    W -1b     E -4b     R -8b     T -16b    ┆   U  I  O LPF", ba),
        Line::styled("  ╰         ╰         ╰         ╰         ╰         ┆   ╰  ╰  ╰ Flt=", sp),
        row7,
        row8,
        row9,
        Line::styled("      ╭         ╭         ╭ -Tick   ╭ +BsBPM  ╭ CueSt   ┆   ╭  ╭  ╭ -Gain", sh),
        row11,
        Line::styled("      ╰ =Ptch   ╰ Rst     ╰ SpRst   ╰ Metro   ╰         ┆   ╰  ╰  ╰ 0%", sp),
        row13,
        Line::from(vec![
            Span::styled("` mode   ¬ nudge  -/= zoom  {/} height  [/] latency ", ba),
            Span::styled("N msgs", sh),
            Span::styled("  Esc quit  │ [Bare] ", ba),
        ]),
        row15,
    ];
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Rgb(15, 15, 15))),
        inner,
    );
}

/// The message-history overlay: the stream's log over the album-art area,
/// newest at the bottom, long messages wrapped with a hanging indent.
/// `scroll_from_tail` is how many display lines the view has been scrolled
/// back toward the start (0 = showing the latest); the value actually used
/// is clamped to the content and returned so the caller can adopt it.
pub(crate) fn render_message_history(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    entries: &[Event],
    scroll_from_tail: usize,
    utc_offset_secs: i64,
    log_path: &str,
) -> usize {
    const H_MARGIN: u16 = 2;

    // Like the help overlay, sit one row below the top of the spacer to clear
    // the deck-3 margin.
    let available_h = area.height.saturating_sub(1);
    if available_h == 0 { return scroll_from_tail; }

    let outer_w = area.width.saturating_sub(H_MARGIN * 2);
    let outer = ratatui::layout::Rect { x: area.x + H_MARGIN, y: area.y + 1, width: outer_w, height: available_h };

    // Clear replaces halfblock art characters with spaces; Block then fills with
    // dark bg (without Clear the art ▀ characters would comb through).
    let bg = Style::default().bg(Color::Rgb(15, 15, 15));
    frame.render_widget(Clear, outer);
    frame.render_widget(Block::default().style(bg), outer);

    let inner = ratatui::layout::Rect {
        x:      outer.x + 1,
        y:      outer.y,
        width:  outer.width.saturating_sub(2),
        height: outer.height,
    };
    let rows = inner.height.saturating_sub(1) as usize; // header takes one
    let width = inner.width as usize;

    let severity_fg = |severity| match severity {
        Severity::Error   => Color::Rgb(255, 180, 180),
        Severity::Warning => Color::Rgb(255, 220, 120),
        Severity::Info    => Color::Rgb(160, 200, 255),
        Severity::Success => Color::Rgb(140, 230, 160),
    };

    // Every display line, oldest first: "HH:MM:SS  text", wrapped continuations
    // indented under the text column.
    const INDENT: usize = 10; // "HH:MM:SS  "
    let body_width = width.saturating_sub(INDENT).max(8);
    let mut all: Vec<Line> = Vec::new();
    for m in entries {
        let fg = severity_fg(m.severity);
        for (i, segment) in wrap_words(&m.display_text(), body_width).into_iter().enumerate() {
            let prefix = if i == 0 { format!("{}  ", m.clock_time(utc_offset_secs)) } else { " ".repeat(INDENT) };
            all.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                Span::styled(segment, Style::default().fg(fg)),
            ]));
        }
    }

    // The window ends `scroll` display lines before the newest.
    let total = all.len();
    let scroll = scroll_from_tail.min(total.saturating_sub(rows));
    let end = total - scroll;
    let start = end.saturating_sub(rows);
    let position = match (start, scroll) {
        (0, 0) => String::new(),
        (older, 0) => format!("  ({older} older)"),
        (older, newer) => format!("  ({older} older, {newer} newer)"),
    };
    let header = format!("Messages — k/j scroll, Esc close{position}  ·  {log_path}");
    let mut lines = vec![Line::from(Span::styled(header, Style::default().fg(Color::Rgb(110, 110, 130))))];
    lines.extend(all.drain(start..end));
    if entries.is_empty() {
        lines.push(Line::from(Span::styled("no messages yet", Style::default().fg(Color::Rgb(60, 60, 60)))));
    }
    frame.render_widget(Paragraph::new(lines).style(bg), inner);
    scroll
}

/// Dim every cell in `area` in place — used to de-emphasise the browser half while
/// the playlist pane holds focus.

/// The permanent context panel. Renders whichever state it's in: an empty frame,
/// a track's metadata, or a playlist (preview / browse / edit).
pub(crate) fn render_panel(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    panel: &crate::Panel,
    playing_of: &dyn Fn(&crate::PlaylistPanel) -> Option<usize>,
) {
    use crate::{EditFocus, Panel, Preview};
    frame.render_widget(Clear, area);
    match panel {
        Panel::Preview(Preview::Empty) => {
            let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(55, 62, 78)));
            frame.render_widget(Paragraph::new("").block(block), area);
        }
        Panel::Preview(Preview::Track { fields, current_name, proposed_name }) => {
            let owned: Vec<(String, usize)> = fields.iter().map(|f| (f.clone(), 0)).collect();
            render_metadata_panel(frame, area, &owned, None, current_name, proposed_name.as_deref(), None, None);
        }
        Panel::Preview(Preview::Playlist(pp)) => render_playlist_panel(frame, area, pp, playing_of(pp), PanelKind::Preview),
        Panel::Browse(pp) => render_playlist_panel(frame, area, pp, playing_of(pp), PanelKind::Browse),
        Panel::Edit { panel: pp, focus } => {
            let kind = match focus { EditFocus::Playlist => PanelKind::EditList, EditFocus::Browser => PanelKind::EditBrowser };
            render_playlist_panel(frame, area, pp, playing_of(pp), kind);
        }
        Panel::Confirm { panel: pp, entry, candidates, cursor, layout } => render_confirm(frame, area, pp, *entry, candidates, *cursor, layout),
    }
}

/// A byte count rendered compactly (MB/KB/B) for the picker's size fields.
fn human_size(bytes: u64) -> String {
    const MB: f64 = 1_048_576.0;
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Greedy word-wrap into lines of at most `width` columns. Words longer than
/// `width` (paths, typically) are hard-split rather than overflowing their line.
fn wrap_words(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let chunks = text.split_whitespace().flat_map(|word| {
        word.chars().collect::<Vec<_>>()
            .chunks(width)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
    });
    let mut lines = Vec::new();
    let mut line = String::new();
    for chunk in chunks {
        if line.is_empty() {
            line = chunk;
        } else if line.chars().count() + 1 + chunk.chars().count() <= width {
            line.push(' ');
            line.push_str(&chunk);
        } else {
            lines.push(std::mem::take(&mut line));
            line = chunk;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// The descriptive-fallback candidate picker: a frozen header for the original entry, then a
/// scrolling list of variable-height candidate cards for the operator to confirm a re-link.
fn render_confirm(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    pp: &crate::PlaylistPanel,
    entry: usize,
    candidates: &[crate::playlist::Candidate],
    cursor: usize,
    layout: &std::cell::RefCell<crate::ConfirmLayout>,
) {
    use ratatui::layout::{Constraint, Layout};
    let rows = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(area);

    let dim = Style::default().fg(Color::Rgb(120, 140, 175));
    let label = Style::default().fg(Color::Rgb(90, 110, 150));
    let val = Style::default().fg(Color::Rgb(200, 220, 255));
    let amber_bold = Style::default().fg(Color::Rgb(230, 190, 100)).add_modifier(Modifier::BOLD);
    let match_green = Style::default().fg(Color::Rgb(120, 210, 150));
    let miss = Style::default().fg(Color::Rgb(150, 130, 120));
    let orig = pp.playlist.entries.get(entry);

    // Place the block first so the list width (which the path wraps to) is known before the
    // header height is computed — the width doesn't depend on the header, only the height does.
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(230, 190, 100))).title(" confirm re-link ");
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    let scrollbar_col = 1u16;
    let path_indent = 6usize;
    let path_width = (inner.width.saturating_sub(scrollbar_col) as usize).saturating_sub(path_indent);

    // A field line marked ✓ (matches the original) or · (differs), showing the value.
    let field_line = |name: &str, orig_val: &str, cand_val: &str| {
        let matches = !orig_val.is_empty() && orig_val.eq_ignore_ascii_case(cand_val);
        let style = if matches { match_green } else { miss };
        let shown = if cand_val.is_empty() { "—" } else { cand_val };
        Line::from(vec![Span::styled(format!("    {} {name:<7} ", if matches { "✓" } else { "·" }), style), Span::styled(shown.to_string(), style)])
    };
    // Hard-wrap a path across as many lines as it needs (at least one) so it stays fully
    // visible; the card simply grows to fit.
    let wrap_path = |path: &str| -> Vec<String> {
        let chars: Vec<char> = path.chars().collect();
        if path_width == 0 || chars.is_empty() {
            return vec![path.to_string()];
        }
        chars.chunks(path_width).map(|c| c.iter().collect()).collect()
    };

    // Build each candidate as a variable-height card, recording its start line so the input's
    // line-scroll can map an offset back to a card. Heights differ: album/year lines appear only
    // when present, and the path wraps to however many lines it needs.
    let mut clines: Vec<Line> = Vec::new();
    let mut card_starts: Vec<usize> = Vec::new();
    for (i, c) in candidates.iter().enumerate() {
        card_starts.push(clines.len());
        let (orig_year, orig_title, orig_artist, orig_album, orig_dur, orig_size) = match orig {
            Some(e) => (e.description.year.as_str(), e.description.title.as_str(), e.description.artist.as_str(), e.description.album.as_str(), e.identity.duration_secs, e.hints.file_size_bytes),
            None => ("", "", "", "", c.duration_secs, c.file_size_bytes),
        };
        clines.push(Line::from(Span::styled(format!("  candidate {}", i + 1), Style::default().fg(Color::Rgb(180, 200, 230)))));
        clines.push(field_line("title", orig_title, &c.description.title));
        clines.push(field_line("artist", orig_artist, &c.description.artist));
        if !orig_album.is_empty() || !c.description.album.is_empty() {
            clines.push(field_line("album", orig_album, &c.description.album));
        }
        if !orig_year.is_empty() || !c.description.year.is_empty() {
            clines.push(field_line("year", orig_year, &c.description.year));
        }
        let delta = (c.duration_secs - orig_dur).abs();
        let len_ok = delta <= 2.0;
        clines.push(Line::from(Span::styled(format!("    {} length  {:.0}s (Δ{:.1}s)", if len_ok { "✓" } else { "·" }, c.duration_secs, delta), if len_ok { match_green } else { miss })));
        // Size is the cheap proxy for the audio payload: a re-encode changes it (·), so it's the
        // visible evidence of why confirmation is needed even when every tag field matches.
        let size_ok = orig_size != 0 && c.file_size_bytes == orig_size;
        clines.push(Line::from(Span::styled(format!("    {} {:<7} {}", if size_ok { "✓" } else { "·" }, "size", human_size(c.file_size_bytes)), if size_ok { match_green } else { miss })));
        // Show the path relative to the playlist dir — the same basis as the stored hint, and
        // exactly what the hint becomes on adopt — rather than the raw absolute workspace path.
        let rel = crate::playlist::relative_to(pp.path.parent().unwrap_or(std::path::Path::new(".")), &c.path);
        for path_line in wrap_path(&rel) {
            clines.push(Line::from(Span::styled(format!("{:indent$}{path_line}", "", indent = path_indent), dim)));
        }
    }
    let total = clines.len();

    // The active card is the topmost one whose header line is still on screen; highlight its head.
    let active = crate::confirm_active_card(cursor, &card_starts);
    if let Some(&start) = card_starts.get(active) {
        clines[start] = Line::from(Span::styled(format!("▶ candidate {}", active + 1), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
    }
    // Publish the layout so the input handler's line-scroll can map its offset back to a card.
    *layout.borrow_mut() = crate::ConfirmLayout { card_starts, total_lines: total };

    // Frozen header: a note on why confirmation is needed, the original entry, then a label.
    let note_style = Style::default().fg(Color::Rgb(230, 190, 100));
    let note = "Audio fingerprint changed — confirm the right file.";
    let mut header: Vec<Line> = wrap_words(note, inner.width.saturating_sub(1) as usize)
        .into_iter()
        .map(|l| Line::from(Span::styled(l, note_style)))
        .collect();
    header.push(Line::from(""));
    header.push(Line::from(Span::styled("Original entry", amber_bold)));
    if let Some(e) = orig {
        header.push(Line::from(vec![Span::styled("  title  ", label), Span::styled(e.description.title.clone(), val)]));
        header.push(Line::from(vec![Span::styled("  artist ", label), Span::styled(e.description.artist.clone(), val)]));
        if !e.description.album.is_empty() {
            header.push(Line::from(vec![Span::styled("  album  ", label), Span::styled(e.description.album.clone(), val)]));
        }
        if !e.description.year.is_empty() {
            header.push(Line::from(vec![Span::styled("  year   ", label), Span::styled(e.description.year.clone(), val)]));
        }
        header.push(Line::from(vec![Span::styled("  hint   ", label), Span::styled(e.hints.relative_path.clone(), dim)]));
        header.push(Line::from(vec![Span::styled("  size   ", label), Span::styled(human_size(e.hints.file_size_bytes), dim)]));
        header.push(Line::from(vec![Span::styled("  length ", label), Span::styled(format!("{:.0}s", e.identity.duration_secs), dim)]));
    }
    header.push(Line::from(""));
    header.push(Line::from(vec![
        Span::styled(format!("Candidates ({})", candidates.len()), amber_bold),
        Span::styled(if candidates.is_empty() { String::new() } else { format!("   #{} of {}", active + 1, candidates.len()) }, dim),
    ]));
    let header_h = header.len() as u16;

    let split = Layout::vertical([Constraint::Length(header_h), Constraint::Min(1)]).split(inner);
    frame.render_widget(Paragraph::new(header), split[0]);

    if candidates.is_empty() {
        frame.render_widget(Paragraph::new(Span::styled("  (no candidates)", dim)), split[1]);
    } else {
        let cols = Layout::horizontal([Constraint::Min(1), Constraint::Length(scrollbar_col)]).split(split[1]);
        // Scroll straight to the line offset; cards fall off the top/bottom edges by line.
        let offset = cursor.min(total.saturating_sub(1));
        frame.render_widget(Paragraph::new(clines).scroll((offset as u16, 0)), cols[0]);
        let mut sb = ScrollbarState::new(total).position(offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).begin_symbol(None).end_symbol(None),
            cols[1],
            &mut sb,
        );
    }

    frame.render_widget(
        Paragraph::new("j/k select · Enter confirm · Esc cancel").style(dim).wrap(ratatui::widgets::Wrap { trim: true }),
        rows[1],
    );
}

enum PanelKind { Preview, Browse, EditList, EditBrowser }

/// A playlist in the panel: entries with ▶ playing / ⇢ next-up markers and status.
/// A wrapping hint sits below the frame; the border colour marks the active side.
fn render_playlist_panel(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    pp: &crate::PlaylistPanel,
    playing: Option<usize>,
    kind: PanelKind,
) {
    let next_up = pp.next_up(playing);
    let show_cursor = !matches!(kind, PanelKind::Preview);
    let name = pp.path.file_stem().and_then(|s| s.to_str()).unwrap_or("playlist").to_string();

    // Reserve two rows below the frame for the (wrapping) hint text.
    let rows = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Min(2),
        ratatui::layout::Constraint::Length(2),
    ]).split(area);

    use crate::EntryStatus;
    let items: Vec<ListItem> = pp.playlist.entries.iter().enumerate().map(|(i, entry)| {
        let status = pp.status_at(i);
        let marker = if Some(i) == playing { "▶" } else if Some(i) == next_up { "⇢" } else { " " };
        let desc = format!("{} - {}", entry.description.title, entry.description.artist);
        let shown = if desc == " - " { entry.hints.relative_path.clone() } else { desc };
        let (tail, mut style) = match status {
            EntryStatus::Found => (String::new(), Style::default().fg(Color::Rgb(200, 220, 255))),
            EntryStatus::NeedsConfirmation => ("   ? confirm".to_string(), Style::default().fg(Color::Rgb(230, 190, 100))),
            EntryStatus::Unavailable => ("   unavailable".to_string(), Style::default().fg(Color::Rgb(90, 90, 110))),
        };
        if show_cursor && i == pp.cursor {
            style = style.bg(Color::Rgb(40, 50, 80)).add_modifier(Modifier::BOLD);
        }
        ListItem::new(format!("{marker} {:>2}. {shown}{tail}", i + 1)).style(style)
    }).collect();

    // Border colour marks which side is active: bright on the focused side, dim otherwise.
    let (border, hint) = match kind {
        PanelKind::Preview     => (Color::Rgb(70, 90, 110),   "l browse · e edit"),
        PanelKind::Browse      => (Color::Rgb(120, 210, 180), "j/k move · Enter load · e edit · Esc back"),
        PanelKind::EditList    => (Color::Rgb(240, 180, 60),  "K/J reorder · x remove · h/Tab → browser · Enter commit · Esc abort"),
        PanelKind::EditBrowser => (Color::Rgb(130, 100, 40),  "browse left, then a insert-after (append) · A insert-before · l/Tab → list · Enter commit · Esc abort"),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(format!(" ♫ {name}  ({}) ", pp.playlist.entries.len()));
    // Select the cursor so the List scrolls to keep it visible when entries overflow the panel.
    let mut state = ListState::default();
    state.select((!pp.playlist.entries.is_empty()).then(|| pp.cursor.min(pp.playlist.entries.len() - 1)));
    frame.render_stateful_widget(List::new(items).block(block), rows[0], &mut state);
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::Rgb(110, 120, 140))).wrap(ratatui::widgets::Wrap { trim: true }),
        rows[1],
    );
}

/// The tag editor rendered in the context panel: labelled fields with a caret on
/// the active one, the resulting filename, and a hint. Input is the same handler
/// as the full-screen modal.
/// The one metadata panel: identical form for passive preview and active
/// editing — the symmetric dim carries the state difference, so the RHS never
/// changes shape at the moment of focus. Colours follow the browser family
/// over the navy-and-blue frame.
pub(crate) fn render_metadata_panel(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    fields: &[(String, usize)],
    active_field: Option<usize>,
    current_name: &str,
    proposed_name: Option<&str>,
    collision_error: Option<&str>,
    rename_toggle: Option<(bool, bool)>, // (enabled, focused) — edit mode only
) {
    let inner_width = (area.width as usize).saturating_sub(2).max(12);
    let text_width  = inner_width.saturating_sub(9);
    let label = Style::default().fg(Color::Rgb(90, 110, 150));
    let mut lines: Vec<Line<'static>> = std::iter::once(section_divider("Tags", inner_width))
        .chain(TAG_FIELD_LABELS.iter().enumerate()
            .flat_map(|(i, &lab)| {
                let (val, cur) = &fields[i];
                render_editor_field(lab, val, active_field == Some(i), *cur, text_width)
            }))
        .collect();
    // The decision is the section header: `── [x] Rename File ──` — in edit
    // mode the divider itself is the toggle (Tab to it, Space flips).
    let rename_on = rename_toggle.map_or(true, |(enabled, _)| enabled);
    match rename_toggle {
        Some((enabled, focused)) => {
            let text = format!("[{}] Rename File", if enabled { "x" } else { " " });
            let toggle_style = if focused {
                Style::default().fg(Color::White).bg(Color::Rgb(60, 80, 130))
            } else {
                Style::default().fg(Color::Rgb(120, 140, 175))
            };
            let frame_style = Style::default().fg(Color::Rgb(70, 90, 130));
            let fill = "─".repeat(inner_width.saturating_sub(4 + text.chars().count()));
            lines.push(Line::from(vec![
                Span::styled("── ", frame_style),
                Span::styled(text, toggle_style),
                Span::styled(format!(" {fill}"), frame_style),
            ]));
        }
        None => lines.push(section_divider("Filename", inner_width)),
    }
    lines.push(Line::from(vec![
        Span::styled(" Current: ", label),
        Span::styled(current_name.to_string(), Style::default().fg(Color::Rgb(200, 220, 255))),
    ]));
    if let Some(proposed) = proposed_name {
        let (label_style, value_style) = if rename_on {
            (label, Style::default().fg(Color::Yellow))
        } else {
            let dim = Style::default().fg(Color::Rgb(70, 70, 80));
            (dim, dim)
        };
        lines.push(Line::from(vec![
            Span::styled("Proposed: ", label_style),
            Span::styled(proposed.to_string(), value_style),
        ]));
    }
    if let Some(err) = collision_error {
        lines.push(Line::from(Span::styled(format!(" \u{26a0} {err}"), Style::default().fg(Color::Red))));
    }
    if active_field.is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Enter to confirm  Esc to cancel", label)));
    }
    let navy = Style::default().bg(Color::Rgb(20, 20, 38));
    let blue = Color::Rgb(40, 60, 100);
    let title = if active_field.is_some() { " Edit tags and rename file " } else { " Tags " };
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::Yellow)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(blue))
        .style(navy);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

pub(crate) fn render_tag_editor_panel(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, editor: &crate::deck::TagEditorState) {
    let with_ext = |stem: &str| -> String {
        if editor.extension.is_empty() { stem.to_string() } else { format!("{stem}.{}", editor.extension) }
    };
    let proposed = editor.preview();
    render_metadata_panel(
        frame, area,
        &editor.fields,
        Some(editor.active_field),
        &with_ext(&editor.current_stem),
        Some(&with_ext(&proposed)),
        editor.collision_error.as_deref(),
        Some((editor.rename_enabled, editor.active_field == TAG_FIELD_LABELS.len())),
    );
}
pub(crate) fn dim_area(frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    // Blend target and strengths. Foregrounds keep more of themselves than
    // backgrounds: pastel palettes lose perceived hue much faster than
    // saturated ones, and the fg carries the colour identity.
    const TARGET: (f32, f32, f32) = (45.0, 45.0, 55.0);
    const KEEP_FG: f32 = 0.55;
    const KEEP_BG: f32 = 0.35;
    fn to_rgb(c: Color) -> Option<(u8, u8, u8)> {
        Some(match c {
            Color::Rgb(r, g, b) => (r, g, b),
            Color::Black => (0, 0, 0),
            Color::Red => (205, 49, 49),
            Color::Green => (13, 188, 121),
            Color::Yellow => (229, 229, 16),
            Color::Blue => (36, 114, 200),
            Color::Magenta => (188, 63, 188),
            Color::Cyan => (17, 168, 205),
            Color::Gray => (200, 200, 200),
            Color::DarkGray => (102, 102, 102),
            Color::LightRed => (241, 76, 76),
            Color::LightGreen => (35, 209, 139),
            Color::LightYellow => (245, 245, 67),
            Color::LightBlue => (59, 142, 234),
            Color::LightMagenta => (214, 112, 214),
            Color::LightCyan => (41, 184, 219),
            Color::White => (229, 229, 229),
            Color::Indexed(i) => xterm_rgb(i),
            Color::Reset => return None,
        })
    }
    fn xterm_rgb(i: u8) -> (u8, u8, u8) {
        match i {
            0..=15 => [(0,0,0),(205,49,49),(13,188,121),(229,229,16),(36,114,200),(188,63,188),(17,168,205),(229,229,229),
                       (102,102,102),(241,76,76),(35,209,139),(245,245,67),(59,142,234),(214,112,214),(41,184,219),(255,255,255)][i as usize],
            16..=231 => {
                let v = i - 16;
                let level = |n: u8| if n == 0 { 0 } else { 55 + 40 * n };
                (level(v / 36), level((v / 6) % 6), level(v % 6))
            }
            _ => { let g = 8 + 10 * (i - 232); (g, g, g) }
        }
    }
    fn blend(c: Color, keep: f32) -> Color {
        match to_rgb(c) {
            Some((r, g, b)) => Color::Rgb(
                (r as f32 * keep + TARGET.0 * (1.0 - keep)) as u8,
                (g as f32 * keep + TARGET.1 * (1.0 - keep)) as u8,
                (b as f32 * keep + TARGET.2 * (1.0 - keep)) as u8,
            ),
            None => Color::Rgb(TARGET.0 as u8, TARGET.1 as u8, TARGET.2 as u8),
        }
    }
    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            let style = cell.style();
            let dimmed = Style::default()
                .fg(blend(style.fg.unwrap_or(Color::Reset), KEEP_FG))
                .bg(blend(style.bg.unwrap_or(Color::Reset), KEEP_BG));
            cell.set_style(dimmed);
        }
    }
}
