# Application

[Down](#deck)
[Down](#browser)
[Down](#mixer)
[Down](#keymap)
[Down](#settings)
[Down](#album-art)
[Down](#audio-latency)

A minimal terminal DJ player. Modern DJ equipment does so much it removes the fun — deck blends the convenience of software with the skill of beat-matching and mixing. Three decks, real-time waveform visualisation, nudge/speed adjustment, unified filter, optional BPM.

Deliberately excludes loops, effects, jump points, samples, and track recommendations.

Designed to perform well on slow hardware. Three recurring constraints:

- **Never block the audio thread** — all communication via lock-free atomics. Any blocking causes audible glitches.

- **Do work once, reuse** — full decode at load, pre-rendered waveform buffers, pre-computed overview peaks.

- **Recompute only on change** — dirty flags, drift thresholds, adaptive frame timing. Most iterations are no-ops.

```
Application
├ Deck
│ ├ Spectral Colour
│ ├ Overview Waveform (TODO)
│ ├ Detail Waveform
│ │ ├ Zoom
│ │ ├ Wide Buffer
│ │ ├ Sliding Viewport
│ │ └ Sub-Column Smoothing
│ ├ Transport
│ │ ├ Mode
│ │ ├ Nudge
│ │ ├ Beat Jump
│ │ ├ Speed Control
│ │ └ Click-free Seek
│ │   ├ Quiet-Frame Search
│ │   ├ Fade Envelope
│ │   └ Pipeline Flush
│ ├ Beat Grid
│ │ └ Cue Point
│ └ Audio Pipeline
│   ├ Filter
│   ├ Level & Gain
│   ├ Pitch Shift
│   └ Metronome
├ Browser (TODO)
├ Mixer
├ Keymap (TODO)
├ Settings (TODO)
├ Album Art (TODO)
└ Audio Latency (TODO)
```


# Deck

[Up](#application)
[Down](#spectral-colour)
[Down](#overview-waveform)
[Down](#detail-waveform)
[Down](#transport)
[Down](#beat-grid)
[Down](#audio-pipeline)

A loaded track with waveform visualisation and transport controls. One deck is active at a time (receives input). Each deck has its own track, playback state, BPM, and audio output.

The waveforms are the primary visual feedback — the DJ reads track structure, position, and phase from them. Three layers: a colour encoding representing frequency content, an overview showing the full track, and a detail view showing the area around the playhead at high zoom.

> [!DECISION] Three deck instances share one conceptual model. The map describes the deck, not each instance.


# Spectral Colour

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

- [Spectral Colour](#spectral-colour) — both waveform types share the same colour encoding


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
[Down](#mode)
[Down](#nudge)
[Down](#beat-jump)
[Down](#speed-control)
[Down](#click-free-seek)

Play, pause, seek, and position control. Two top-level modes shape behaviour:

- **Beat mode** — BPM-relative operations: beat jump by N beats, speed as BPM ratio
- **Vinyl mode** — hides BPM machinery, speed as percentage, beat jumps remapped to fixed time intervals

**See also**

- [Cue Point](#cue-point) — jump-to-cue is a transport action; the cue itself lives under Beat Grid


# Mode

[Up](#transport)

A global toggle that applies to all decks simultaneously. The mode determines how speed is represented and how beat jumps behave.

- **Beat mode** — speed is a BPM ratio (`bpm / base_bpm`), beat grid and tick marks visible, beat jumps land on beat boundaries
- **Vinyl mode** — speed is a percentage of nominal (`vinyl_speed`, 1.0 = nominal), beat grid hidden, beat jumps remapped to fixed time intervals (N × 0.5s)

Switching modes preserves audio speed — no audible change on toggle. Beat-to-vinyl converts the current BPM ratio to `vinyl_speed`; vinyl-to-beat converts `vinyl_speed` back to a BPM.

> [!DECISION] Session persistence — the active mode is stored in cache and restored on startup. Default is beat mode.


# Nudge

[Up](#transport)

Fine position adjustment with two sub-modes (jump and warp):

- **While playing** — jump mode seeks ±10ms per press; warp mode applies a continuous ±10% speed offset while held, returning to normal on release
- **While paused** — both modes play a short audio snippet at the new position so the DJ can hear where they are. Jump fires on each press; warp fires continuously at half-column intervals as the position drifts

**See also**

- [Click-free Seek](#click-free-seek) — shared seek mechanism used by nudge and beat jump


# Beat Jump

[Up](#transport)

Discrete position jumps in four sizes (1, 4, 16, 64 beats) in each direction. Each jump lands exactly N beat periods ahead or behind.

- **Beat mode** — jump distance is `N × (60 / base_bpm)` audio seconds, landing precisely on the next tick mark
- **Vinyl mode** — remapped to fixed time intervals: N × 0.5s (the beat period at 120 BPM)

Backward past the start clamps to position 0. Forward past the end is a no-op.

**See also**

- [Click-free Seek](#click-free-seek) — fade/search/flush mechanism that prevents audible clicks during jumps


# Speed Control

[Up](#transport)

Adjusts playback speed like a turntable — faster playback raises pitch, slower lowers it. The representation depends on mode:

- **Beat mode** — two tiers: `base_bpm_increase`/`base_bpm_decrease` adjust the native BPM in 0.01 steps; `bpm_increase`/`bpm_decrease` adjust the playback BPM in 0.1 steps. Playback speed is the ratio `bpm / base_bpm`.
- **Vinyl mode** — the same keys adjust `vinyl_speed` in 0.001 steps (±0.1%). Speed is passed directly to the player.

Clamped to 40.0–240.0 BPM in beat mode. All underlying values retain full precision; rounding is display-only.

**See also**

- [Pitch Shift](#pitch-shift) — independent key adjustment without changing tempo


# Click-free Seek

[Up](#transport)
[Down](#quiet-frame-search)
[Down](#fade-envelope)
[Down](#pipeline-flush)

When seeking during playback (beat jump or nudge), three independent layers prevent audible clicks. Each catches a different artifact: the fade handles raw discontinuity, the quiet-frame search minimises what the fade has to hide, and the pipeline flush prevents downstream processing from reintroducing what the fade already dealt with.

When paused, seeks use direct repositioning with no fade — there's no audible output to protect.


# Quiet-Frame Search

[Up](#click-free-seek)

The target is nudged to the quietest sample within a small window around the requested beat, landing on a naturally low-amplitude moment.

Search window: +/-10ms around the requested target position. Selects the frame with lowest amplitude within that window.


# Fade Envelope

[Up](#click-free-seek)

The audio thread fades out at the old position, jumps, then fades in at the new position. The crossfade is very short — inaudible but eliminates the discontinuity.

**Detail**

- Fade duration: 256 samples (~5.8ms at 44.1kHz).
- Audio thread (TrackingSource) detects a pending seek target via atomic swap. Fades out over 256 samples, jumps, fades in over 256 samples.
- When paused: direct position write (seek_direct) with no fade — instant repositioning since there's no audible output.


# Pipeline Flush

[Up](#click-free-seek)

The pitch shifter and filter carry internal state (buffered samples, filter history). On a seek, these are flushed or crossfaded independently so stale audio doesn't bleed through.

**Detail**

- **Pitch shifter (PitchSource):** internal SoundTouch buffers flushed on seek. Processes in 512-frame chunks; output buffered in a VecDeque.
- **Filter (FilterSource):** crossfades from previous filter state to new state over 256 samples. Biquad history (per-channel) is pre-allocated — no allocations on the audio path.


# Beat Grid

[Up](#deck)
[Down](#cue-point)

The rhythmic framework overlaid on the track — a BPM value (`base_bpm`) and a phase offset (`offset_ms`) that together determine where beat ticks fall. Everything that displays or acts on beats consumes these two values.

**BPM** is established by one of:

- **Cache lookup** — on load, the audio is hashed (Blake3 over decoded mono samples) and looked up in cache. If found, BPM and offset are applied immediately. If not, a 120 BPM placeholder is used.
- **Tap** (`bpm_tap`) — press in time with the beat. After 8 taps, `base_bpm` and `offset_ms` are set via linear regression. Outlier taps (residual > half a beat period) are excluded.
- **Detection** (`redetect_bpm`) — manually triggers BPM analysis on the decoded audio. Result goes through a confirmation step if a BPM is already established.
- **Manual adjust** — `base_bpm_increase`/`base_bpm_decrease` nudge the native BPM in 0.01 steps

**Offset** positions the grid relative to the audio. Adjusted in 10ms steps, snapped to multiples of 10ms, wrapped into `[0, beat_period_ms)`. The cue point acts as the grid's zero datum — when `base_bpm` changes, `offset_ms` is recalculated to keep a tick on the cue.

Cache is keyed by audio hash, making it invariant of filename, tags, and container format.


# Cue Point

[Up](#beat-grid)

A single saved position per deck with two distinct actions:

- **Cue set** (`cue`) — only works while paused; stores the current position and snaps the beat grid so a tick falls on the cue
- **Cue play** (`cue_play`) — jumps to the cue and maintains current play state (playing continues, paused stays paused)

Persisted to cache alongside BPM and offset.

**See also**

- [Click-free Seek](#click-free-seek) — cue play uses the same seek mechanism


# Audio Pipeline

[Up](#deck)
[Down](#filter)
[Down](#level--gain)
[Down](#pitch-shift)
[Down](#metronome)

Per-deck signal chain on the audio thread. Stages run in order — each transforms the signal before passing it on:

1. **Filter** — unified HPF/LPF
2. **Level & Gain** — volume, gain staging, PFL signal taken here
3. **Pitch shift** — key adjustment without changing tempo

The metronome injects click tones into the mixer output rather than sitting in the signal chain, but is grouped here as an audio-output concern.

> [!DECISION] Never block the audio thread — all parameter changes via lock-free atomics. No allocations on the audio path.


# Filter

[Up](#audio-pipeline)

A single `filter_offset` parameter (−16 to +16, default 0) controls a second-order Butterworth IIR filter. Negative values give low-pass, positive give high-pass, zero bypasses.

Cutoff frequencies are logarithmically spaced from ~40 Hz to ~18 kHz. Filter position is indicated by shading out characters of the spectrum analyser, with each step corresponding to one character.

Filter slope is also adjustable per deck.

**Detail**

Config actions: `deckN_filter_increase`, `deckN_filter_decrease`, `deckN_filter_reset`, `deckN_filter_slope_increase`, `deckN_filter_slope_decrease`. Crossfades from previous filter state to new state over 256 samples. Biquad history (per-channel) is pre-allocated.

**See also**

- [Spectrum Analyser](#spectrum-analyser) — displays filter position as shaded region


# Level & Gain

[Up](#audio-pipeline)

Two independent volume controls per deck:

- **Level** — playback volume in 5% steps (0–100%). Not persisted.
- **Gain** — trim in 1 dB steps (±12 dB) for matching track loudness. A soft-knee limiter engages near 0 dBFS to prevent clipping when gain is raised. Persisted to cache.

PFL (Pre-Fader Listen) signal is taken from this stage — before level and filter are applied — and routed to the left channel when active.

**Detail**

Config actions: `deckN_level_up`, `deckN_level_down`, `deckN_gain_increase`, `deckN_gain_decrease`.

The limiter is a soft-knee curve (cubic Hermite) over the zone [1.0 − 0.3, 1.0]: slope 1 at entry, slope 0 at the ceiling. Hard clip above 1.0. Applied per sample after gain scaling.


# Pitch Shift

[Up](#audio-pipeline)

Adjusts the key of a track in semitone steps (±6) without changing tempo — for key matching between tracks. Not persisted between sessions.

Config: `pitch_up`, `pitch_down`.

**Detail**

Time-domain pitch shifting via SoundTouch. Processes in 512-frame chunks; output buffered in a VecDeque. Internal buffers flushed on seek to prevent stale audio bleeding through.

**See also**

- [Speed Control](#speed-control) — turntable-style speed change that affects both tempo and pitch


# Metronome

[Up](#audio-pipeline)

A click tone on every beat, synced to `base_bpm` and `offset_ms`. Only fires during playback. The click is timed against the audio buffer write position (ahead of the speaker by `audio_latency_ms`), so it arrives at the speaker on the beat when latency is correctly calibrated.

Config: `metronome_toggle`. Resets to off on each new track load.

**See also**

- [Audio Latency](#audio-latency) — metronome timing depends on correct latency calibration


# Browser

[Up](#application)

*TODO — file navigation, track selection, preview playback.*


# Mixer

[Up](#application)

Sums the three deck audio pipeline outputs into a single stereo stream for the audio device. PFL routing splits the output: right channel always carries the main mix, left channel carries the PFL deck's pre-fader signal when active.


# Keymap

[Up](#application)

*TODO — keyboard layout design, ergonomic rationale, modifier system.*


# Settings

[Up](#application)

*TODO — user configuration and track cache.*


# Album Art

[Up](#application)

*TODO — cover art display.*


# Audio Latency

[Up](#application)

*TODO — global latency calibration, visual compensation, metronome timing.*
