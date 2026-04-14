# Application

[Down](#deck)
[Down](#browser)
[Down](#mixer)
[Down](#keymap)
[Down](#settings)
[Down](#album-art)

A minimal terminal DJ player. Modern DJ equipment does so much it removes the fun — deck blends the convenience of software with the skill of beat-matching and mixing. Three decks, real-time waveform visualisation, nudge/speed adjustment, unified filter, optional BPM.

Deliberately excludes loops, effects, jump points, samples, and track recommendations.

Designed to perform well on slow hardware. Three recurring constraints:

- **Never block the audio thread** — all communication via lock-free atomics. Any blocking causes audible glitches.

- **Do work once, reuse** — full decode at load, pre-rendered waveform buffers, pre-computed overview peaks.

- **Recompute only on change** — dirty flags, drift thresholds, adaptive frame timing. Most iterations are no-ops.


# Deck

[Up](#application)
[Down](#waveform-colour)
[Down](#overview-waveform)
[Down](#detail-waveform)
[Down](#transport)

A loaded track with waveform visualisation and transport controls. One deck is active at a time (receives input). Each deck has its own track, playback state, BPM, and audio output.

The waveforms are the primary visual feedback — the DJ reads track structure, position, and phase from them. Three layers: a colour encoding representing frequency content, an overview showing the full track, and a detail view showing the area around the playhead at high zoom.

> [!DECISION] Three deck instances share one conceptual model. The map describes the deck, not each instance.


# Waveform Colour

[Up](#deck)

Both waveforms use the same colour scheme: a spectral gradient driven by bass ratio, giving a visual sense of the frequency content at each point in the track. Low bass is one end of the palette, high bass the other. Applied per column in both overview and detail views.

**Detail**

Bass ratio computed per column from mono sample data within that column's sample range. Box-smoothed with radius 3 to prevent flicker at wide zoom. Interpolated across a 4-stop linear palette, from treble (bass=0) to bass (bass=1):

1. Cyan `(0, 255, 255)`
2. Teal `(0, 255, 120)`
3. Gold `(220, 255, 0)`
4. Amber `(255, 120, 0)`

Interpolation is linear between adjacent stops, with brightness scaling that normalises to max channel = 255 to preserve saturation at all brightness levels.


# Overview Waveform

[Up](#deck)

*TODO — full-track view, position indicator, beat grid overlay, click-to-seek.*


# Detail Waveform

[Up](#deck)
[Down](#zoom)
[Down](#wide-buffer)
[Down](#sliding-viewport)
[Down](#sub-column-smoothing)

Smooth scrolling without per-frame redraw. Four mechanisms work together: zoom controls the time scale, the wide buffer provides pre-rendered content, the viewport slides across it, and sub-column smoothing gives half-character precision.

**See also**

- [Waveform Colour](#waveform-colour) — both waveform types share the same colour encoding


# Zoom

[Up](#detail-waveform)

The user steps through discrete zoom levels controlling how many seconds of audio are visible on screen. Zooming triggers a buffer recompute. BPM-adjusted playback is accounted for so that beat grids align visually across decks at the same effective tempo.

**Detail**

Zoom levels: 1.0, 2.0, 4.0, 8.0, 16.0, 32.0 seconds per screen width. Samples per column: `(zoom_secs * sample_rate * speed_ratio) / screen_cols`.


# Wide Buffer

[Up](#detail-waveform)

A pre-rendered waveform image, much wider than the screen, computed in the background, before the UI needs it. A dedicated background thread maintains the buffer. It polls for changes and recomputes only when needed. Most iterations are no-ops. The buffer is a grid of braille characters, anchored at a sample position at a specific zoom level.

> [!DECISION] Pre-render rather than render each frame to avoid distracting waveform "wiggle" in motion due to fractional rounding.

**Detail**

- Buffer width: 5x screen width in columns.
- Background thread: OS thread with 8ms sleep per iteration (~125Hz).
- Recompute trigger: playhead drifts past 75% of buffer width, or zoom/BPM/track/window changes.
- Buffer content: 2D grid of braille characters (each char is a 2x4 dot matrix). One column = one sample range determined by zoom level.
- Buffer swap: computed into a new buffer, then swapped in under a mutex. The mutex is held only for the pointer swap, not during computation.


# Sliding Viewport

[Up](#detail-waveform)

Each UI frame: read the playhead position (atomic, lock-free), locate it in the buffer, extract a screen-width slice. Per-frame cost is negligible.

**Detail**

- Playhead position read via atomic load (Relaxed ordering) from the audio thread's output position counter.
- Viewport offset calculated as sample-space delta from buffer anchor, converted to column + half-column.
- Display position is time-anchored (wall clock from a fixed sample/time pair) rather than accumulated per-frame, to avoid drift and snap oscillations.
- Drift correction: factor of 0.002 when display and audio positions diverge, snapping at 0.3s divergence.


# Sub-Column Smoothing

[Up](#detail-waveform)

When the playhead falls between character boundaries, braille bit manipulation shifts the display by half a character width, giving sub-character precision without re-rendering.

> [!ASSUMPTION] Char colour on half-col shift — since braille can move by half a column but colour characters can't, we assume that colour changes are gradual enough to avoid obvious misalignment.

**Detail**

- Half-column detection: `delta_half = (delta / half_col_samp).round()`, sub-column when `delta_half % 2 != 0`.
- Braille shift: moves right dot-column bits (positions 3, 4, 5, 7) to left dot-column (positions 0, 1, 2, 6) of the adjacent character.


# Transport

[Up](#deck)
[Down](#click-free-beat-jump)

Play, pause, seek, beat jump, nudge, speed adjustment.


# Click-free Beat Jump

[Up](#transport)
[Down](#quiet-frame-search)
[Down](#fade-envelope)
[Down](#pipeline-flush)

When jumping to a beat position during playback, three independent layers prevent audible clicks. Each catches a different artifact: the fade handles raw discontinuity, the quiet-frame search minimises what the fade has to hide, and the pipeline flush prevents downstream processing from reintroducing what the fade already dealt with.


# Quiet-Frame Search

[Up](#click-free-beat-jump)

The target is nudged to the quietest sample within a small window around the requested beat, landing on a naturally low-amplitude moment.

Search window: +/-10ms around the requested target position. Selects the frame with lowest amplitude within that window.


# Fade Envelope

[Up](#click-free-beat-jump)

The audio thread fades out at the old position, jumps, then fades in at the new position. The crossfade is very short — inaudible but eliminates the discontinuity.

**Detail**

- Fade duration: 256 samples (~5.8ms at 44.1kHz).
- Audio thread (TrackingSource) detects a pending seek target via atomic swap. Fades out over 256 samples, jumps, fades in over 256 samples.
- When paused: direct position write (seek_direct) with no fade — instant repositioning since there's no audible output.


# Pipeline Flush

[Up](#click-free-beat-jump)

The pitch shifter and filter carry internal state (buffered samples, filter history). On a seek, these are flushed or crossfaded independently so stale audio doesn't bleed through.

**Detail**

- **Pitch shifter (PitchSource):** internal SoundTouch buffers flushed on seek. Processes in 512-frame chunks; output buffered in a VecDeque.
- **Filter (FilterSource):** crossfades from previous filter state to new state over 256 samples. Biquad history (per-channel) is pre-allocated — no allocations on the audio path.


# Browser

[Up](#application)

*TODO — file navigation, track selection, preview playback.*


# Mixer

[Up](#application)

*TODO — how deck outputs combine, volume, filtering, PFL routing.*


# Keymap

[Up](#application)

*TODO — keyboard layout design, ergonomic rationale, modifier system.*


# Settings

[Up](#application)

*TODO — user configuration and track cache.*


# Album Art

[Up](#application)

*TODO — cover art display.*
