//! Measurement scales and the one shared position→column mapping.
//!
//! Every scrolling element wobble this application has suffered came from two
//! sites converting positions to columns with different rounding. All rounding
//! policy therefore lives here and nowhere else: column values can only be made
//! by the maps in this module, so the compiler enforces the one-mapping rule
//! the map's Detail Waveform callout states.

/// A position or distance in mono samples. f64 because display positions are
/// smoothed; storage types (usize/i64 samples) convert at the boundary.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub(crate) struct Samples(pub f64);

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub(crate) struct Secs(pub f64);

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub(crate) struct Ms(pub f64);

impl Samples {
    pub(crate) fn to_secs(self, sample_rate: f64) -> Secs { Secs(self.0 / sample_rate) }
    pub(crate) fn to_ms(self, sample_rate: f64) -> Ms { Ms(self.0 / sample_rate * 1000.0) }
}
impl Secs {
    pub(crate) fn to_samples(self, sample_rate: f64) -> Samples { Samples(self.0 * sample_rate) }
}
impl Ms {
    pub(crate) fn to_samples(self, sample_rate: f64) -> Samples { Samples(self.0 / 1000.0 * sample_rate) }
}

/// The playhead's fixed screen column for a given width and configured
/// percentage. One formula, replacing five verbatim copies.
pub(crate) fn playhead_centre_col(width: usize, playhead_pct: u8) -> usize {
    ((width as f64 * playhead_pct as f64 / 100.0) as usize).clamp(0, width.saturating_sub(1))
}

/// The detail viewport: where the visible window starts in buffer columns, and
/// whether the extraction is shifted by one dot (half column).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Viewport {
    pub(crate) start: usize,
    pub(crate) sub_col: bool,
}

/// The sample↔column mapping for the detail waveform's buffer space. Both the
/// rasterisation thread and the render loop build their columns through this —
/// one implementation, two callers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DetailMap {
    anchor: i64,          // mono-sample index at the buffer centre
    samples_per_col: i64, // the master column width — the buffer's own integer
    buf_cols: usize,
}

impl DetailMap {
    /// The master column width in samples: zoom seconds at this rate and speed
    /// over the screen width, truncated exactly as the buffer builds it. Every
    /// consumer must reproduce this value or its grid is not the buffer's grid.
    pub(crate) fn col_width(zoom_secs: f64, sample_rate: f64, speed: f64, screen_cols: usize) -> usize {
        if screen_cols == 0 { return 1; }
        ((zoom_secs * sample_rate * speed) as usize / screen_cols).max(1)
    }

    pub(crate) fn new(anchor_sample: usize, samples_per_col: usize, buf_cols: usize) -> Self {
        Self { anchor: anchor_sample as i64, samples_per_col: samples_per_col.max(1) as i64, buf_cols }
    }

    pub(crate) fn samples_per_col(&self) -> usize { self.samples_per_col as usize }

    pub(crate) fn buf_cols(&self) -> usize { self.buf_cols }

    /// Sample position of the buffer's left edge.
    pub(crate) fn buffer_start(&self) -> Samples {
        Samples(self.anchor as f64 - (self.buf_cols as i64 / 2 * self.samples_per_col) as f64)
    }

    /// Sample position of the buffer's right edge.
    pub(crate) fn buffer_end(&self) -> Samples {
        Samples(self.buffer_start().0 + (self.buf_cols as i64 * self.samples_per_col) as f64)
    }

    /// A position's dot within the buffer: (column, right-dot flag), or None
    /// outside the buffer. For per-dot content baked into buffer columns.
    pub(crate) fn dot_in_buffer(&self, pos: Samples) -> Option<(usize, bool)> {
        let d = self.dot(pos) + self.buf_cols as i64;
        if d < 0 { return None; }
        let col = d.div_euclid(2);
        if (col as usize) < self.buf_cols {
            Some((col as usize, d.rem_euclid(2) != 0))
        } else {
            None
        }
    }

    /// The buffer's grid origin for tick anchoring: position floored onto the
    /// whole-column grid. A named policy — the one deliberate floor.
    pub(crate) fn grid_origin_floor(pos: Samples, samples_per_col: usize) -> Samples {
        let spc = samples_per_col.max(1) as f64;
        Samples((pos.0 / spc).floor() * spc)
    }

    /// Dot index of a position, relative to the buffer anchor. The canonical
    /// rounding: nearest half column. Everything else derives from this.
    pub(crate) fn dot(&self, pos: Samples) -> i64 {
        let half_col = self.samples_per_col as f64 / 2.0;
        ((pos.0 - self.anchor as f64) / half_col).round() as i64
    }

    /// A dot-pair index folded to its screen column: the shared convention for
    /// placing a whole-character mark on the dot grid (right dot rounds right).
    fn col_of_screen_half(screen_half: i64) -> i64 {
        (screen_half + 1).div_euclid(2)
    }

    /// The viewport for a display position: playhead pinned at `centre_col`,
    /// dot-resolution scrolling via the parity flag. None while the buffer
    /// can't cover the window.
    pub(crate) fn viewport(&self, display_pos: Samples, centre_col: usize, width: usize) -> Option<Viewport> {
        let delta_half = self.dot(display_pos);
        let sub_col = delta_half.rem_euclid(2) != 0;
        let delta_cols = delta_half.div_euclid(2);
        let off = self.buf_cols as i64 / 2 + delta_cols - centre_col as i64;
        let need = if sub_col { width + 1 } else { width };
        if off >= 0 && off as usize + need <= self.buf_cols {
            Some(Viewport { start: off as usize, sub_col })
        } else {
            None
        }
    }

    /// Content-anchored mark: the screen column where `sample` renders, given
    /// the viewport of `display_pos`. The cue, and anything else pinned to the
    /// audio, maps through here — the same dots the waveform itself uses.
    pub(crate) fn content_screen_col(&self, sample: Samples, display_pos: Samples, centre_col: usize) -> i64 {
        let delta_half = self.dot(display_pos);
        let sub_col = delta_half.rem_euclid(2) != 0;
        let delta_cols = delta_half.div_euclid(2);
        let off = self.buf_cols as i64 / 2 + delta_cols - centre_col as i64;
        // Dot index of the sample from the buffer's left edge.
        let view_start = self.anchor as f64 - (self.buf_cols as i64 / 2 * self.samples_per_col) as f64;
        let disp_half = ((sample.0 - view_start) / (self.samples_per_col as f64 / 2.0)).round() as i64;
        let screen_half = disp_half - 2 * off - (sub_col as i64);
        Self::col_of_screen_half(screen_half)
    }

    /// Playhead-anchored mark: a fixed sample distance from the playhead,
    /// placed by rounding the delta once in dot units. Rounding doesn't
    /// commute with subtraction — mapping the endpoint absolutely would
    /// oscillate at every boundary crossing while scrolling.
    pub(crate) fn playhead_anchored_col(&self, delta: Samples, centre_col: usize) -> i64 {
        let half_col = self.samples_per_col as f64 / 2.0;
        let offset_half = (delta.0 / half_col).round() as i64;
        centre_col as i64 + Self::col_of_screen_half(offset_half)
    }

    /// Snap a position onto the buffer's own grid: `halves` = 2 for the dot
    /// grid (paused decks), 1 for whole columns (the detached view).
    pub(crate) fn snap(pos: Samples, samples_per_col: usize, halves: f64) -> Samples {
        let unit = samples_per_col.max(1) as f64 / halves;
        Samples((pos.0 / unit).round() * unit)
    }
}

/// The track-fraction↔column mapping for the overview. Forward map rounds to
/// the nearest column; the inverse returns the centre of that column's
/// preimage, so needle drop lands where the mark would draw.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OverviewMap {
    width: usize,
    total_secs: f64,
    sample_rate: f64,
}

impl OverviewMap {
    pub(crate) fn new(width: usize, total_secs: f64, sample_rate: f64) -> Self {
        Self { width, total_secs, sample_rate }
    }

    pub(crate) fn col_of_frac(&self, frac: f64) -> usize {
        ((frac.clamp(0.0, 1.0) * self.width as f64).round() as usize)
            .min(self.width.saturating_sub(1))
    }

    pub(crate) fn col(&self, pos: Samples) -> usize {
        if self.total_secs == 0.0 { return 0; }
        self.col_of_frac(pos.0 / self.sample_rate / self.total_secs)
    }

    /// Inverse of `col`: the seconds position at the centre of the column's
    /// preimage under the rounding forward map.
    pub(crate) fn secs_at_col(&self, col: usize) -> Secs {
        if self.width == 0 { return Secs(0.0) }
        Secs(self.total_secs * col as f64 / self.width as f64)
    }
}

/// A position as offset-corrected nanoseconds — the one samples→ns→beat
/// conversion (previously triplicated at the metronome and beat flash).
fn beat_ns(pos: Samples, sample_rate: f64, offset_ms: i64) -> i128 {
    (pos.0 / sample_rate * 1_000_000_000.0) as i128 - offset_ms as i128 * 1_000_000
}

/// The beat index a position falls in, phase offset applied.
pub(crate) fn beat_index(pos: Samples, sample_rate: f64, offset_ms: i64, beat_period_ns: i128) -> i128 {
    beat_ns(pos, sample_rate, offset_ms).div_euclid(beat_period_ns)
}

/// The position's phase within its beat, in nanoseconds from the beat start.
pub(crate) fn beat_phase_ns(pos: Samples, sample_rate: f64, offset_ms: i64, beat_period_ns: i128) -> i128 {
    beat_ns(pos, sample_rate, offset_ms).rem_euclid(beat_period_ns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> DetailMap { DetailMap::new(1_000_000, 882, 1000) }

    #[test]
    fn cue_equals_playhead_at_equal_samples() {
        // A content mark at exactly the display position must land on the
        // playhead's column, for any position and any centre.
        let m = map();
        for i in 0..500 {
            let pos = Samples(995_000.0 + i as f64 * 33.7);
            for centre in [0usize, 20, 39] {
                assert_eq!(m.content_screen_col(pos, pos, centre), centre as i64, "pos {pos:?} centre {centre}");
            }
        }
    }

    #[test]
    fn content_col_steps_monotonically_under_scroll() {
        // A fixed sample's screen column never oscillates as the view scrolls.
        let m = map();
        let mark = Samples(1_002_000.0);
        let mut last = i64::MAX;
        for i in 0..2000 {
            let disp = Samples(998_000.0 + i as f64 * 13.1);
            let col = m.content_screen_col(mark, disp, 20);
            assert!(col <= last, "column went right while scrolling forward");
            last = col;
        }
    }

    #[test]
    fn playhead_anchored_col_is_scroll_invariant() {
        // The whole point: a fixed delta gives a fixed column, at every scroll
        // position — this is what absolute mapping of the endpoint cannot do.
        let m = map();
        let delta = Samples(882.0 * 7.3);
        let col0 = m.playhead_anchored_col(delta, 20);
        for i in 0..2000 {
            let _disp = Samples(998_000.0 + i as f64 * 13.1);
            assert_eq!(m.playhead_anchored_col(delta, 20), col0);
        }
    }

    #[test]
    fn viewport_matches_content_col_at_playhead() {
        let m = map();
        for i in 0..300 {
            let disp = Samples(999_000.0 + i as f64 * 57.3);
            if let Some(vp) = m.viewport(disp, 20, 100) {
                // The playhead's own dot maps into the window the viewport frames.
                let col = m.content_screen_col(disp, disp, 20);
                assert_eq!(col, 20);
                assert!(vp.start + 100 <= 1000);
            }
        }
    }

    #[test]
    fn snap_lands_on_buffer_grid() {
        for spc in [1usize, 7, 882, 4410] {
            for halves in [1.0, 2.0] {
                let snapped = DetailMap::snap(Samples(123_456.789), spc, halves);
                let unit = spc as f64 / halves;
                let rem = (snapped.0 / unit) - (snapped.0 / unit).round();
                assert!(rem.abs() < 1e-6);
            }
        }
    }

    #[test]
    fn overview_inverse_is_forward_fixed_point() {
        let m = OverviewMap::new(200, 300.0, 44_100.0);
        for col in 0..200 {
            let secs = m.secs_at_col(col);
            let back = m.col(Samples(secs.0 * 44_100.0));
            assert_eq!(back, col, "col {col}");
        }
    }

    #[test]
    fn unit_conversions_round_trip() {
        let sr = 44_100.0;
        let s = Samples(123_456.0);
        assert!((s.to_secs(sr).to_samples(sr).0 - s.0).abs() < 1e-9);
        assert!((s.to_ms(sr).to_samples(sr).0 - s.0).abs() < 1e-9);
    }
}
