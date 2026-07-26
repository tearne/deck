# Application

[Down](#deck)
[Down](#browser)
[Down](#mixer)
[Down](#keymap)
[Down](#cache)
[Down](#album-art)
[Down](#audio-latency)

A minimal terminal DJ player. Modern DJ equipment does so much it removes the fun — deck blends the convenience of software with the skill of beat-matching and mixing. Three decks, real-time waveform visualisation, nudge/speed adjustment, unified filter, optional BPM.

Deliberately excludes loops, effects, jump points, samples, and track recommendations.

Designed to perform well on slow hardware. Three recurring constraints:

- **Never block the audio thread** — all communication via lock-free atomics. Any blocking causes audible glitches.

- **Do work once, reuse** — full decode at load, pre-rendered waveform buffers, pre-computed overview peaks.

- **Recompute only on change** — dirty flags, drift thresholds, adaptive frame timing. Most iterations are no-ops.

These hold across several threads: decode runs in the background so the UI stays live and shows progress, then hashing and BPM detection follow on another pass; waveform rasterisation has its own thread (see Wide Buffer), and audio playback its own. State crosses threads through lock-free or low-contention primitives, so the audio thread never stalls.

```
Application
├ Deck
│ ├ Deck Selection
│ ├ Track Loading
│ ├ Renaming
│ │ └ Metadata Editor
│ ├ Spectral Colour
│ ├ Overview Waveform
│ ├ Spectrum Analyser
│ ├ Detail Waveform
│ │ ├ Zoom
│ │ ├ Wide Buffer
│ │ ├ Sliding Viewport
│ │ │ ├ Drift Snap
│ │ │ └ Partial Drift Correction
│ │ └ Sub-Column Smoothing
│ ├ Transport
│ │ ├ Mode
│ │ ├ Nudge
│ │ ├ Beat Jump
│ │ ├ Needle Drop
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
├ Browser
│ ├ Search
│ └ Preview
├ Mixer
│ └ PFL Monitor
├ Keymap
├ Cache
├ Album Art (TODO)
└ Audio Latency
```


# Deck

[Up](#application)
[Down](#deck-selection)
[Down](#track-loading)
[Down](#renaming)
[Down](#spectral-colour)
[Down](#overview-waveform)
[Down](#spectrum-analyser)
[Down](#detail-waveform)
[Down](#transport)
[Down](#beat-grid)
[Down](#audio-pipeline)

A loaded track with waveform visualisation and transport controls. Each deck has its own track, playback state, BPM, and audio output.

The waveforms are the primary visual feedback — the DJ reads track structure, position, and phase from them. Three layers: a colour encoding representing frequency content, an overview showing the full track, and a detail view showing the area around the playhead at high zoom.

Three deck instances share one conceptual model. The map describes the deck, not each instance.


# Deck Selection

[Up](#deck)

Each deck is independent, one selected at a time. The selected deck receives all deck-specific input — transport, BPM, cue, nudge, pitch. The mixer controls — level, gain, and filter — are the exception: they address each deck directly, whichever is selected.

Two decks can be swapped wholesale — their entire state trades places, and selection follows the content so the operator keeps controlling the same track. The others start empty, loaded by selecting them and opening the browser. Audio latency is a single global value shared across all decks.


# Track Loading

[Up](#deck)

Decoding runs in the background while the UI stays responsive; a progress screen tracks it, and the deck arrives **loaded but paused** — the operator starts playback deliberately. Hashing and BPM analysis follow on a further background pass (see Beat Grid).

Supported formats: FLAC, MP3, OGG, WAV, AAC, OPUS.


# Renaming

[Up](#deck)
[Down](#metadata-editor)

Keeps track filenames matching their tags. The convention is `Title - Artist` (an optional `(suffix)` allowed), checked against the raw filename stem at load. A conforming file loads silently; a non-conforming one raises a rename offer in the deck's notification row — accept it to open the metadata editor, or carry on and the offer fades but lingers. The editor is reachable only through this offer: the feature exists to fix names that don't conform, not as a general metadata editor.

**See also**

- [Track Loading](#track-loading) — the offer fires at load
- [Keymap](#keymap) — fixed rename/editor keys


# Metadata Editor

[Up](#renaming)

The modal that does the renaming, by way of editing the track's metadata. Seven fields — Artist, Title, Album, Year, Track, Genre, Comment — are seeded from the file and shown with a live preview of the resulting filename. Confirming writes the edited metadata back to the file and renames it to the sanitised `Title - Artist`; Artist and Title are required (they form the name), and the rename aborts rather than overwrite an existing file. Cancelling leaves the file untouched. While open it captures all input.

**Detail**

- Tags read via symphonia (ID3v2 preferred over container tags), written via `lofty` (symphonia is read-only).
- Filename-illegal characters `/ \ : * ? " < > |` become `-`; renamed only when the proposed stem differs.

**See also**

- [Keymap](#keymap) — editor keys (fixed, not configurable)


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

At render time the bass ratio is quantised to 32 levels through a per-palette lookup table — the underlying data keeps full precision, but adjacent columns land on shared colours, so their escape sequences merge into runs and terminal output drops several-fold at no visible cost.

The looked-up colours are emitted as xterm-256 indexed colours rather than truecolor, to cut the bandwidth of colour escape codes the terminal has to parse while the waveform scrolls: an indexed sequence is roughly half the bytes, and the coarser palette merges still more columns into shared runs. Accent uses of the palette elsewhere in the UI (track titles, pitch readouts) stay truecolor.


# Overview Waveform

[Up](#deck)

The full-track waveform — a miniature map of the whole song. Playhead and cue point shown as vertical lines; spectral colour is shared with the detail view.

Rendered at half-column braille resolution: each character encodes two adjacent audio columns, doubling horizontal detail within the terminal width.

In beat mode, bar markers overlay the track as thin vertical lines at every N bars. The interval defaults to 4 bars and doubles until no two adjacent markers are closer than 4 characters, adapting to both BPM and screen width. A legend in the top-right corner shows the current interval. When remaining playback time drops below a configurable threshold (default 30 s), the bar markers flash — alternating between a muted reddish tone and near-invisible on each beat, active only during playback. In vinyl mode, bar markers and the warning flash are suppressed.

**See also**

- [Spectral Colour](#spectral-colour) — the colour encoding both views share
- [Needle Drop](#needle-drop) — seeking via the overview
- [Beat Grid](#beat-grid) — the BPM and offset that position the bar markers


# Spectrum Analyser

[Up](#deck)

A compact real-time frequency display in the info bar — 16 braille characters wide (32 logarithmically spaced bins, 20 Hz to 20 kHz), one braille row tall. Each character encodes two adjacent bins as a bottom-up bar chart. Active whenever a track is loaded.

The display is beat-synced: it updates 4 times per beat, falling back to 250 ms intervals during BPM analysis. A background glow lights character cells with sub-threshold activity and resets on a 2-bar accumulation window.

When a filter is active, the attenuated region is shaded with a grey background — LPF from the right, HPF from the left — with each of the 16 filter steps corresponding to one character.

**Detail**

Goertzel algorithm over a 4096-sample Hann-windowed window at the current playback position. Amplitude mapped on a dB scale (~10 dB floor, ~60 dB ceiling, ~12.5 dB per dot row) with a +3 dB/octave perceptual tilt to equalise bass and treble visibility.

**See also**

- [Filter](#filter) — the shaded region tracks filter position
- [Keymap](#keymap) — no config actions; the analyser is always on


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

> [!IMPORTANT] Pre-render rather than render each frame to avoid distracting waveform "wiggle" in motion due to fractional rounding.

**Detail**

- Buffer width: 5x screen width in columns.
- Background thread: OS thread with 8ms sleep per iteration (~125Hz).
- Recompute triggers are per deck: playhead drift past 75% of buffer width, track load, and parameter changes (BPM, offset, cue, gain, loop) rebuild only that deck's buffer; resize and zoom rebuild all three.
- Rebuilds triggered by operator-adjusted values (BPM, offset, gain, loop bounds, speed) wait until the value has been stable for ~50 ms, so a key-repeat burst causes one rebuild when the hold ends rather than one per repeat; drift, resize, zoom, and load rebuild immediately.
- Buffer content: 2D grid of braille characters (each char is a 2x4 dot matrix). One column = one sample range determined by zoom level.
- Buffer swap: computed into a new buffer, then swapped in under a mutex. The mutex is held only for the pointer swap, not during computation.


# Sliding Viewport

[Up](#detail-waveform)
[Down](#drift-snap)
[Down](#partial-drift-correction)

Each UI frame advances the displayed position by the **measured** frame interval × *sample_rate × speed*, accumulated into a running value. Measured intervals telescope to exact wall time, so timer jitter cannot compound into drift — advancing at an *expected* rate would.

Position jumps (seek, jump-nudge) shift the running value directly; speed changes simply take effect in the next frame's multiplication. The current position is then located in the buffer and a screen-width slice extracted.

Alignment with the audio's true position is maintained by two separate mechanisms: [Drift Snap](#drift-snap) for large divergence, and [Partial Drift Correction](#partial-drift-correction) for continuous small corrections that smooth audio-batch noise.

**Detail**

- The audio thread's sample counter — read via atomic load (Relaxed ordering) — is the reference point for drift detection. It counts samples emitted to the OS, so it leads what the user hears by the hardware buffer depth.
- Viewport offset — the column at which the playhead should appear — is calculated as *(current playhead sample − wide buffer start sample)*, converted to a column position plus half-column for sub-character precision.

**See also**

- [Wide Buffer](#wide-buffer) — the buffer the screen-width slice is extracted from


# Drift Snap

[Up](#sliding-viewport)

When the displayed position diverges too far from the audio's true output position, a snap immediately re-aligns the display. This prevents accumulated lag between the cursor and what the user hears.

**Detail**

- While playing, the threshold is 0.1 s — far above steady-state drift and audio-batch noise (sub-millisecond and ~25 ms respectively since the drift damper fix), while catching short seeks like a 1-beat jump at high BPM that the old 0.3 s threshold left to crawl in via partial correction.
- While paused and not nudging, the threshold tightens to a single sample — without motion to mask it, any gap is obvious.
- After the snap, the display position is rounded to the nearest half-column.


# Partial Drift Correction

[Up](#sliding-viewport)

Between snaps, each frame pulls the running display position a small fraction of the way toward the audio's true position. This absorbs the step noise from the audio thread's batched position reads, which would otherwise show up as flicker if the display fully tracked the audio sample-by-sample.

The correction is applied to the running value itself, so it accumulates frame to frame — continuous clock skew between the system and audio clocks settles at a small constant offset instead of growing until a snap fires.

**Detail**

- Factor: 0.002 per frame — position −= 0.002 × (position − audio position).
- Steady-state offset under clock skew ≈ rate × skew / (0.002 × fps): sub-millisecond at typical ppm-level crystal skew.
- Applies only while playing and below the snap threshold.


# Sub-Column Smoothing

[Up](#detail-waveform)

When the playhead falls between character boundaries, braille bit manipulation shifts the display by half a character width, giving sub-character precision without re-rendering.

> [!IMPORTANT] Char colour on half-col shift — since braille can move by half a column but colour characters can't, we assume that colour changes are gradual enough to avoid obvious misalignment.

**Detail**

- Half-column detection: `delta_half = (delta / half_col_samp).round()`, sub-column when `delta_half % 2 != 0`.
- Braille shift: moves right dot-column bits (positions 3, 4, 5, 7) to left dot-column (positions 0, 1, 2, 6) of the adjacent character.


# Transport

[Up](#deck)
[Down](#mode)
[Down](#nudge)
[Down](#beat-jump)
[Down](#needle-drop)
[Down](#speed-control)
[Down](#click-free-seek)

Play, pause, seek, and position control. Reaching the end of the track pauses the transport and returns the playhead to the start, the view staying interactive. Two top-level modes shape behaviour:

- **Beat mode** — BPM-relative operations: beat jump by N beats, speed as BPM ratio
- **Vinyl mode** — hides BPM machinery, speed as percentage, beat jumps remapped to fixed time intervals

**See also**

- [Cue Point](#cue-point) — jump-to-cue is a transport action; the cue itself lives under Beat Grid


# Mode

[Up](#transport)

A global toggle that applies to all decks simultaneously. The mode determines how speed is represented and how beat jumps behave.

- **Beat mode** — speed is a BPM ratio (`bpm / base_bpm`), beat grid and tick marks visible, beat jumps land on beat boundaries
- **Vinyl mode** — speed is a percentage of nominal (`vinyl_speed`, 1.0 = nominal), beat grid hidden, beat jumps remapped to fixed time intervals (N × 0.5s); BPM analysis and re-detection are suppressed

Switching modes preserves audio speed — no audible change on toggle. Beat-to-vinyl converts the current BPM ratio to `vinyl_speed`; vinyl-to-beat converts `vinyl_speed` back to a BPM.

The active mode is stored in cache and restored on startup. Default is beat mode.

**See also**

- [Cache](#cache) — stores and restores the active mode


# Nudge

[Up](#transport)

Fine position adjustment with two sub-modes (jump and warp):

- **While playing** — jump mode seeks ±10ms per press; warp mode applies a continuous ±10% speed offset while held, returning to normal on release
- **While paused** — both modes play a short audio snippet at the new position so the DJ can hear where they are. Jump fires on each press; warp fires continuously at half-column intervals as the position drifts

**See also**

- [Click-free Seek](#click-free-seek) — shared seek mechanism used by nudge and beat jump


# Beat Jump

[Up](#transport)

Discrete position jumps in seven sizes — 1 beat, 1 bar, 4 bars, 8 bars, 16 bars, 32 bars, 64 bars — in each direction. Each jump lands exactly N beat periods ahead or behind.

- **Beat mode** — jump distance is `N × (60 / base_bpm)` audio seconds, landing precisely on the next tick mark
- **Vinyl mode** — remapped to fixed time intervals: N × 0.5s (the beat period at 120 BPM)

Backward past the start clamps to position 0. Forward past the end is a no-op.

**See also**

- [Click-free Seek](#click-free-seek) — fade/search/flush mechanism that prevents audible clicks during jumps


# Needle Drop

[Up](#transport)

A left-click on the overview waveform seeks to the start of the nearest bar at or left of the click, preserving play/pause state.

**See also**

- [Click-free Seek](#click-free-seek) — the fade/search/flush mechanism the seek uses


# Speed Control

[Up](#transport)

Adjusts playback speed like a turntable — faster playback raises pitch, slower lowers it. The representation depends on mode:

- **Beat mode** — two tiers: `base_bpm_increase`/`base_bpm_decrease` adjust the native BPM in 0.01 steps; `bpm_increase`/`bpm_decrease` adjust the playback BPM in 0.1 steps. Playback speed is the ratio `bpm / base_bpm`.
- **Vinyl mode** — the same keys adjust `vinyl_speed` in 0.001 steps (±0.1%). Speed is passed directly to the player.

Clamped to 40.0–240.0 BPM in beat mode. All underlying values retain full precision; rounding is display-only.

**See also**

- [Pitch Shift](#pitch-shift) — independent key adjustment without changing tempo
- [Keymap](#keymap) — keys bound to the speed adjustment actions


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

The rhythmic framework overlaid on the track — a BPM value (`base_bpm`) and a phase offset (`offset_ms`) that together determine where beat ticks fall. Everything that displays or acts on beats consumes these two values. Detection assumes a single constant tempo across the track.

**BPM** is established by one of:

- **Cache lookup** — on load, the audio is hashed (Blake3 over decoded mono samples) and looked up in cache. If found, BPM and offset are applied immediately. If not, a 120 BPM placeholder is used.
- **Tap** (`bpm_tap`) — press in time with the beat. After 8 taps, `base_bpm` and `offset_ms` are set via linear regression. Outlier taps (residual > half a beat period) are excluded.
- **Detection** (`detect_bpm`) — manually triggers BPM analysis on the decoded audio. Result goes through a confirmation step if a BPM is already established.
- **Manual adjust** — `base_bpm_increase`/`base_bpm_decrease` nudge the native BPM in 0.01 steps

**Offset** positions the grid relative to the audio. Adjusted in 10ms steps, snapped to multiples of 10ms, wrapped into `[0, beat_period_ms)`. The cue point acts as the grid's zero datum — when `base_bpm` changes, `offset_ms` is recalculated to keep a tick on the cue.

Cache is keyed by audio hash, making it invariant of filename, tags, and container format.

**See also**

- [Keymap](#keymap) — keys bound to BPM tap, re-detect, and manual adjust
- [Cache](#cache) — BPM and offset persisted per track by audio hash


# Cue Point

[Up](#beat-grid)

A single saved position per deck with two distinct actions:

- **Cue set** (`cue`) — only works while paused; stores the current position and snaps the beat grid so a tick falls on the cue
- **Cue play** (`cue_play`) — jumps to the cue and maintains current play state (playing continues, paused stays paused)

Persisted to cache alongside BPM and offset.

**See also**

- [Click-free Seek](#click-free-seek) — cue play uses the same seek mechanism
- [Cache](#cache) — cue position persisted per track
- [Keymap](#keymap) — keys bound to `cue` and `cue_play`


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


# Filter

[Up](#audio-pipeline)

A single `filter_offset` parameter (−16 to +16, default 0) controls a second-order Butterworth IIR filter. Negative values give low-pass, positive give high-pass, zero bypasses.

Cutoff frequencies are logarithmically spaced from ~40 Hz to ~18 kHz. Filter position is indicated by shading out characters of the spectrum analyser, with each step corresponding to one character.

Filter slope is switchable per deck between 12 dB/oct (2-pole) and 24 dB/oct (4-pole).

**Detail**

Config actions: `deckN_filter_increase`, `deckN_filter_decrease`, `deckN_filter_reset`, `deckN_filter_slope_increase`, `deckN_filter_slope_decrease`. Crossfades from previous filter state to new state over 256 samples. Biquad history (per-channel) is pre-allocated.

**See also**

- [Spectrum Analyser](#spectrum-analyser) — displays filter position as shaded region
- [Keymap](#keymap) — keys bound to the filter and slope actions


# Level & Gain

[Up](#audio-pipeline)

Two independent volume controls per deck:

- **Level** — playback volume in 5% steps (0–100%). Not persisted.
- **Gain** — trim in 1 dB steps (±12 dB) for matching track loudness. A soft-knee limiter engages near 0 dBFS to prevent clipping when gain is raised. Persisted to cache.

**Detail**

Config actions: `deckN_level_up`, `deckN_level_down`, `deckN_gain_increase`, `deckN_gain_decrease`.

The limiter is a soft-knee curve (cubic Hermite) over the zone [1.0 − 0.3, 1.0]: slope 1 at entry, slope 0 at the ceiling. Hard clip above 1.0. Applied per sample after gain scaling.

**See also**

- [PFL Monitor](#pfl-monitor) — the pre-fader monitor tap, taken raw ahead of this stage
- [Keymap](#keymap) — keys bound to level and gain actions
- [Cache](#cache) — gain persisted per track; level is session-only


# Pitch Shift

[Up](#audio-pipeline)

Adjusts the key of a track in semitone steps (±6) without changing tempo — for key matching between tracks. Not persisted between sessions.

Config: `pitch_up`, `pitch_down`.

**Detail**

Time-domain pitch shifting via SoundTouch. Processes in 512-frame chunks; output buffered in a VecDeque. Internal buffers flushed on seek to prevent stale audio bleeding through.

**See also**

- [Speed Control](#speed-control) — turntable-style speed change that affects both tempo and pitch
- [Keymap](#keymap) — keys bound to pitch up/down


# Metronome

[Up](#audio-pipeline)

A click tone on every beat, synced to `base_bpm` and `offset_ms`. Only fires during playback; no click sounds on the beat where it is switched on, clicks beginning from the next. The click is timed against the audio buffer write position (ahead of the speaker by `audio_latency_ms`), so it arrives at the speaker on the beat when latency is correctly calibrated.

Config: `metronome_toggle`. Resets to off on each new track load.

**See also**

- [Audio Latency](#audio-latency) — metronome timing depends on correct latency calibration
- [Keymap](#keymap) — the key bound to `metronome_toggle`


# Browser

[Up](#application)
[Down](#search)
[Down](#preview)

A file navigator for loading tracks. It opens over the player at any time (`open_browser`) with audio still playing; choosing an audio file loads it into the selected deck and returns to the player. Entries are listed alphabetically — audio files highlighted and selectable, everything else shown but inert.

The last-visited directory is remembered between sessions, so the browser reopens where you left off (a path on the command line wins for the first open only). If the target deck is already playing, opening asks for confirmation first, so a stray key can't interrupt a mix.

**See also**

- [Cache](#cache) — where the last-visited directory and workspace persist
- [Keymap](#keymap) — navigation and action keys


# Search

[Up](#browser)

Fuzzy track-finding across a whole library, not just the current directory. Searching needs a **workspace** — a directory nominated as the search root (`@` sets the current directory, `'` clears it). The workspace persists between sessions and is silently dropped if it no longer exists, prompting for a new one.

With a workspace set, typing builds a search term and the listing is replaced by audio files found recursively beneath the root, each shown relative to it and ordered best-match-first. Clearing the term restores the directory listing.

**See also**

- [Cache](#cache) — where the workspace persists
- [Keymap](#keymap) — workspace and search keys


# Preview

[Up](#browser)

A quick listen to the highlighted track without loading it. `#` plays it from 20% of the way in (or 30 s if the duration isn't known) through the main output, independent of the decks, so it doesn't disturb what's loaded. `#` again restarts; any other key stops it and then does its normal job; closing the browser stops it too.

**See also**

- [Keymap](#keymap) — the preview key


# Mixer

[Up](#application)
[Down](#pfl-monitor)

Sums the three deck audio pipeline outputs into a single stereo stream for the audio device. PFL routing splits the output: right channel always carries the main mix, left channel carries the PFL deck's pre-fader signal when active.


# PFL Monitor

[Up](#mixer)

Pre-fader listen: routes one deck's raw audio to the left output channel for headphone cueing, so the operator can preview a deck before mixing it in. Exclusive — only one deck is cued at a time, and cueing another releases the first.

Unlike the per-deck mixer controls, PFL acts on the **selected** deck. The tap is raw — before filter and fader — so the cue reflects the source regardless of how the deck is EQ'd or faded. Stereo only; mono tracks are unaffected.

**Detail**

- Level 0–100 in steps of 20 (`pfl_up` / `pfl_down`); `pfl_on_off` toggles on (100) / off (0); `pfl_reset` zeroes it. Dropping the level to 0 releases the monitor.
- Left channel carries the cued deck at PFL level and drops the main mix; the right always carries the full main mix.

**See also**

- [Keymap](#keymap) — keys bound to the PFL actions


# Keymap

[Up](#application)

Three input layers on a split keyboard: plain keys, Shift-modified, and Space-chorded. The left block controls the selected deck — transport, BPM, pitch, nudge, cue, PFL. The right block addresses each deck's mixer directly — level, gain, filter — so the operator can adjust any deck without switching selection.

Space acts as a modifier: holding it and pressing another key fires a chord action. Released alone it has no effect. Space-chord bindings are reserved for one-time actions (set cue, open browser, select deck) because terminals cannot reliably detect Space being held, so continuous actions like nudge or fader movement use plain or Shift layers. Ctrl-C always quits unconditionally.

Most keys are configurable via `config.toml` as action-name → key-string mappings. A small set are fixed: browser navigation, tag editor input, and confirmation prompts.

**See also**

- [keybindings.md](keybindings.md) — full action table, keyboard layout, fixed keys, config format


# Cache

[Up](#application)

A single JSON file (`~/.config/deck/cache.json`) that lets the player do expensive work once and remember user state between runs. Two kinds of content live in it.

**Per-track memory**, keyed by a Blake3 hash of the decoded audio — so it follows the music, not the file. Each entry holds the detected BPM, phase offset, cue point, and gain trim (plus whether the offset has been deliberately placed, and the filename as a human-readable hint). Because the key is the audio itself, renaming, retagging, or re-containering a track keeps its analysis.

**Global state** — last-visited browser directory, search workspace, audio latency, vinyl/beat mode, and cover-art brightness.

**Detail**

- Mutation marks the cache dirty; one flush runs after the cache has been idle for ~1 s, and quit always flushes — so a crash can lose at most the last second of trims. Keys the app didn't touch are never rewritten.


# Album Art

[Up](#application)

*TODO — cover art display.*


# Audio Latency

[Up](#application)

A single global calibration (0–250 ms) compensating for the delay between audio leaving the program and reaching the speaker. The visual playback position is shifted back by this amount so the waveform and beat markers line up with what's heard. Applied only during playback — paused, there's no output to compensate for, so the display sits at the true position.

**Detail**

- Adjusted in 10 ms steps (`latency_increase` / `latency_decrease`), clamped 0–250 ms; one global value in the cache, loaded at startup.
- Cue-play adds the latency to its target so the cued point is heard, not just displayed, on the beat.

**See also**

- [Metronome](#metronome) — click timing depends on this calibration
- [Keymap](#keymap) — keys bound to the latency actions
- [Cache](#cache) — latency persisted as a global value
