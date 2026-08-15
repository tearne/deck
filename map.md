# Application

[Down](#deck)
[Down](#browser)
[Down](#playlists)
[Down](#mixer)
[Down](#keymap)
[Down](#track-database)
[Down](#session-state)
[Down](#messages)
[Down](#fault-capture)
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
│ │ └ Renaming
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
│ ├ Loop Prototype
│ ├ Beat Grid
│ │ └ Cue Point
│ └ Audio Pipeline
│   ├ Filter
│   ├ Level & Gain
│   ├ Pitch Shift
│   └ Metronome
├ Browser
│ ├ Search
│ ├ Preview
│ ├ Load Target
│ ├ Jump to Loaded
│ ├ Tag Compliance
│ ├ Context Panel
│ └ File Operations
│   ├ Metadata Editor
│   └ Move
├ Playlists
│ ├ Loaded Playlist
│ ├ Playlist Editing
│ └ Finding Tracks
│   ├ Library Scan Cost
│   └ Candidate Picker
├ Mixer
│ └ PFL Monitor
├ Keymap
│ └ Key Reporting
├ Track Database
├ Session State
├ Messages
│ ├ History View
│ └ Event Log
├ Fault Capture
├ Album Art (TODO)
└ Audio Latency
```


# Deck

[Up](#application)
[Down](#deck-selection)
[Down](#track-loading)
[Down](#spectral-colour)
[Down](#overview-waveform)
[Down](#spectrum-analyser)
[Down](#detail-waveform)
[Down](#transport)
[Down](#loop-prototype)
[Down](#beat-grid)
[Down](#audio-pipeline)

A loaded track with waveform visualisation and transport controls. Each deck has its own track, playback state, BPM, and audio output.

The waveforms are the primary visual feedback — the DJ reads track structure, position, and phase from them. Three layers: a colour encoding representing frequency content, an overview showing the full track, and a detail view showing the area around the playhead at high zoom. The strip is nothing but waveform — the deck's title and readouts overlay the overview's corners.

Three deck instances share one conceptual model. The map describes the deck, not each instance.

**See also**

- [Loaded Playlist](#loaded-playlist) — a deck can be carrying a whole running order, not just a track


# Deck Selection

[Up](#deck)

Each deck is independent, one selected at a time. The selected deck receives all deck-specific input — transport, BPM, cue, nudge, pitch. The mixer controls — level, gain, and filter — are the exception: they address each deck directly, whichever is selected.

The selected deck is marked by a yellow accent bar in the left gutter, beside both its detail waveform and its overview.

Two decks can be swapped wholesale — their entire state trades places, and selection follows the content so the operator keeps controlling the same track. The others start empty, loaded by selecting them and opening the browser. Audio latency is a single global value shared across all decks.


# Track Loading

[Up](#deck)
[Down](#renaming)

Decoding runs in the background while the UI stays responsive; a progress screen tracks it, and the deck arrives **loaded but paused** — the operator starts playback deliberately. Hashing and BPM analysis follow on a further background pass (see Beat Grid).

Supported formats: FLAC, MP3, OGG, WAV, AAC, OPUS.

**See also**

- [Loaded Playlist](#loaded-playlist) — a set queues the next track up this way, still paused


# Renaming

[Up](#track-loading)

Keeps track filenames matching their tags. The convention is `Title - Artist` (an optional `(suffix)` allowed), checked against the raw filename stem at load. A conforming file loads silently; a non-conforming one raises a rename offer that counts down in the deck's readout corner — accept it to open the metadata editor, or carry on and it shrinks to an amber `⚠` beside the track title until dealt with. This is the automatic prompt to fix a non-conforming name; the same editor can also be opened deliberately from File Operations.

**See also**

- [Metadata Editor](#metadata-editor) — the modal the offer opens
- [File Operations](#file-operations) — editing the same file on demand, from the browser
- [Keymap](#keymap) — fixed rename/editor keys


# File Operations

[Up](#browser)
[Down](#metadata-editor)
[Down](#move)

The browser's command mode acts on the highlighted file: `e` edits its tags and name, `m` moves it to another directory. Any audio file you can navigate to, not just one loaded on a deck — the browser is a file manager. If a touched file is currently loaded on a deck, that deck follows it (its path and name update).

**See also**

- [Renaming](#renaming) — the load-time offer that also opens the editor
- [Keymap](#keymap) — the fixed command-mode file-op keys


# Metadata Editor

[Up](#file-operations)

The modal that does the renaming, by way of editing the track's metadata. Seven fields — Artist, Title, Album, Year, Track, Genre, Comment — are seeded from the file and shown with a live preview of the resulting filename. Confirming writes the edited metadata back to the file and renames it to the sanitised `Title - Artist`; Artist and Title are required (they form the name), and the rename aborts rather than overwrite an existing file. Cancelling leaves the file untouched. While open it captures all input.

Reached two ways: by `e` on a highlighted file in the browser, or via the load-time rename offer (see Renaming).

> [!IMPORTANT] A tag edit must never change the track's **content identity** — the hash of the audio payload, with tags excluded by design, so only tag/container bytes may shift, never the payload itself. The editor enforces this as a **required safeguard**: it computes the identity (over the audio payload) before and after every write and, on any change, raises a critical alert and preserves the original and edited files under the shared `~/.local/state/deck/error_reports/` (dated, type-tagged) for analysis. A changed identity silently breaks every playlist referencing the track, so this is not optional and never auto-undone (the original is kept for recovery).

**Detail**

- Tags read via symphonia (ID3v2 preferred over container tags), written via `lofty` (symphonia is read-only).
- Filename-illegal characters `/ \ : * ? " < > |` become `-`; renamed only when the proposed stem differs.
- Identity check: a mismatch is a critical fault (most likely a byte-range extraction bug), surfaced and preserved rather than reverted — the audio-payload-only hash is the same invariant the playlist format's conformance relies on (tags excluded from identity).

**See also**

- [Renaming](#renaming) — the load-time offer that also opens this
- [Keymap](#keymap) — editor keys (fixed, not configurable)
- [Fault Capture](#fault-capture) — where a payload-change mismatch is preserved


# Move

[Up](#file-operations)

Relocates the highlighted file to another directory — for sorting tracks into folders while listening. Pressing `m` in command mode enters the browser's move mode: a folder-focused view where folders read in clear blue and are the only thing selectable, tracks and other files shown but dimmed and inert. Navigate to the target folder and `y` moves the file there. If the file is loaded on a deck, that deck follows it; playback is unaffected since the audio is already decoded in memory.

**Detail**

- A same-filesystem rename; a cross-device move is refused with a notification rather than falling back to a copy. Moving into the same directory, or onto an existing filename, is refused too.

**See also**

- [Browser](#browser) — move is one of its command-mode modes


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

Its corners carry the deck's text on a navy backing: top-left the title (deck number, play state, playlist badge, track name); bottom-right the readout — tempo and offset │ level and gain │ bar interval, spectrum and filter. Bottom-left holds only transient state (tap counter, nudge arrows), and a countdown prompt briefly displaces the readout. A narrow terminal costs waveform, never title or readout.

Rendered at half-column braille resolution: each character encodes two adjacent audio columns, doubling horizontal detail within the terminal width.

In beat mode, bar markers overlay the track as thin vertical lines at every N bars. The interval defaults to 4 bars and doubles until no two adjacent markers are closer than 4 characters, adapting to both BPM and screen width. The current interval shows as `Nbr` in the readout. When remaining playback time drops below a configurable threshold (default 30 s), the bar markers flash — alternating between a muted reddish tone and near-invisible on each beat, active only during playback. In vinyl mode, bar markers and the warning flash are suppressed.

**See also**

- [Spectral Colour](#spectral-colour) — the colour encoding both views share
- [Needle Drop](#needle-drop) — seeking via the overview
- [Beat Grid](#beat-grid) — the BPM and offset that position the bar markers


# Spectrum Analyser

[Up](#deck)

A compact real-time frequency display at the tail of the deck's readout — 16 braille characters wide (32 logarithmically spaced bins, 20 Hz to 20 kHz), one braille row tall. Each character encodes two adjacent bins as a bottom-up bar chart. Active whenever a track is loaded.

The display is beat-synced: it updates 4 times per beat, falling back to 250 ms intervals during BPM analysis. A background glow lights character cells with sub-threshold activity and resets on a 2-bar accumulation window.

When a filter is active, the attenuated region is shaded with a grey background — LPF from the right, HPF from the left — with each of the 16 filter steps corresponding to one character.

**Detail**

Goertzel algorithm over a 4096-sample Hann-windowed window at the current playback position. Amplitude mapped on a dB scale (~10 dB floor, ~60 dB ceiling, ~12.5 dB per dot row) with a +3 dB/octave perceptual tilt to equalise bass and treble visibility.

**See also**

- [Filter](#filter) — the shaded region tracks filter position
- [Overview Waveform](#overview-waveform) — the readout it sits in, overlaid bottom-right
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
- Changes to operator-adjusted values (BPM, offset, gain, loop bounds, speed) rebuild at most ~10 times a second while the value keeps changing, then once more ~50 ms after it stops. A held key therefore shows the waveform updating live, and the final state is exact. Drift, resize, zoom, and load rebuild immediately.
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

- [Session State](#session-state) — stores and restores the active mode


# Nudge

[Up](#transport)

Fine position adjustment with two sub-modes (jump and warp):

- **While playing** — jump mode seeks ±10ms per press; warp mode applies a continuous ±10% speed offset while held, returning to normal on release
- **While paused** — both modes play a short audio snippet at the new position so the DJ can hear where they are. Jump fires on each press; warp fires continuously at half-column intervals as the position drifts

Warp needs the terminal to report key releases; where it can't, the mode toggle refuses warp with a notification — a warp that can't see the release would latch on with no way to end it.

**See also**

- [Click-free Seek](#click-free-seek) — shared seek mechanism used by nudge and beat jump
- [Key Reporting](#key-reporting) — what makes release detection available, and the fallback when it isn't


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

Nudge seeks skip the quiet-frame search: the fade alone eliminates the click, the search only softens the contrast across the seam — negligible for a ±10 ms hop — and its ±10 ms window is as large as the nudge step itself, so a searched landing would make repeated steps uneven. Under key-repeat, nudge seeks also chain onto the still-pending target and aim past the samples the fade consumes, so the display and the heard position stay in lockstep.

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


# Loop Prototype

[Up](#deck)

Experimental, deck 3 only: capture a seamless loop by tapping, then refine it by ear. `g` tapped on the beat — the first tap marks the loop start — and stopping after ≥4 taps activates it: the tapped tempo sets the period, the tap count rounds up to a power-of-two bars of length. The audio thread cycles the bounds; the overview gives way to three loop panels for close-up trimming, `4`/`r` and `5`/`t` nudging start and end by ±1 ms. `H` exits.

Independent of the beat grid — the loop's tempo comes from its own taps.

**Detail**

- Taps: 2 s inter-tap reset; 1 s silence ends the session; <4 taps discards; deck must be playing.
- Length: tap count → bars (÷4, ceiling) → next power of two, × 4 beats.
- Trim keys are the 8/16-bar jump keys, overridden only while the loop is active.

**See also**

- [Keymap](#keymap) — `loop_tap` (`g`) and `loop_exit` (`H`) are configurable


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
- [Track Database](#track-database) — BPM and offset persisted per track by audio hash


# Cue Point

[Up](#beat-grid)

A single saved position per deck with two distinct actions:

- **Cue set** (`cue`) — only works while paused; stores the current position and snaps the beat grid so a tick falls on the cue
- **Cue play** (`cue_play`) — jumps to the cue and maintains current play state (playing continues, paused stays paused)

Persisted to cache alongside BPM and offset.

**See also**

- [Click-free Seek](#click-free-seek) — cue play uses the same seek mechanism
- [Track Database](#track-database) — cue position persisted per track
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
- [Track Database](#track-database) — gain persisted per track; level is session-only


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
[Down](#file-operations)
[Down](#load-target)
[Down](#tag-compliance)
[Down](#jump-to-loaded)
[Down](#context-panel)

A file navigator for loading tracks. It opens over the player at any time (`open_browser`) and never interrupts playback. Entries are listed alphabetically — audio files highlighted and selectable, everything else shown but inert.

The last-visited directory is remembered between sessions, so the browser reopens where you left off (a command-line path wins the first open only). `Enter` loads the highlighted track into the current Load Target.

The browser is modal, like a modal text editor. **Command mode** navigates (`j`/`k` or arrows) and issues commands, including editing (`e`) and moving (`m`) the highlighted file (see File Operations); **search mode** filters the listing by typing; **move mode** picks a destination directory (see Move). `Tab` toggles command and search — the two primary modes, of which the last used is restored on reopen. `Esc` backs out one level — clearing an active search filter in place (mode unchanged, so exiting from search still restores it on reopen), then exiting the browser. Each mode carries its own accent colour and a status bar naming it and its keys, so which mode you're in is unmistakable.

**See also**

- [Session State](#session-state) — where the last-visited directory and workspace persist
- [File Operations](#file-operations) — editing and moving files from command mode
- [Keymap](#keymap) — navigation and action keys


# Search

[Up](#browser)

Fuzzy track-finding, entered as **search mode** (`Tab` or `/` from command mode). With a **workspace** set — a directory nominated as the search root (`@` sets the current directory, `'` clears it) — typing searches recursively beneath the root, each match shown relative to it, best-match-first. With no workspace, typing filters the current directory's own listing instead. Clearing the term restores the full listing.

The workspace persists between sessions and is silently dropped if it no longer exists, prompting for a new one.

**See also**

- [Session State](#session-state) — where the workspace persists
- [Keymap](#keymap) — workspace and search keys
- [Finding Tracks](#finding-tracks) — the same workspace is the search root for relocating a playlist's tracks


# Preview

[Up](#browser)

A quick listen to the highlighted track without loading it. `#` plays it from 20% of the way in (or 30 s if the duration isn't known) through the main output, independent of the decks, so it doesn't disturb what's loaded. `#` again restarts; any other key stops it and then does its normal job; closing the browser stops it too.

**See also**

- [Keymap](#keymap) — the preview key


# Load Target

[Up](#browser)

Where `Enter` sends the highlighted track. The browser isn't bound to a deck — the target floats, shown as a chip. It defaults to the least-disruptive deck: an empty one, else a loaded-but-not-playing one, else the selected deck. Adjust it with `[`/`]` in any mode, or `1`/`2`/`3` in command mode. Loading into a deck that is playing asks to confirm first — `Enter` loads, any other key cancels.


# Jump to Loaded

[Up](#browser)

`` ` `` in command mode rotates the browser through the directories of the tracks currently loaded on the decks, highlighting each track, and loops back to the directory it opened at — a quick way to return to where a loaded track lives while browsing. A subtle chip at the top names each stop: "Working directory", then "Deck N directory".


# Tag Compliance

[Up](#browser)

A cleanup mode (toggle `T`) for finding and fixing badly-named files. When on, the current directory's audio files are checked in the background — a file is flagged when its name doesn't match its tags (the `Title - Artist` convention) — and flagged entries show an amber `⚠` marker with a count. The same amber marks the load-time rename offer on a deck, so the signal is consistent.

Fix them in sequence: `j`/`k` to the first flagged, `e` to edit it, and the cursor auto-advances to the next flagged file below (wrapping). Only a save advances — cancelling leaves the cursor put.

**Detail**

- The check opens and probes each file for tags, so it runs on a background thread and never blocks the interface. Results are cached per session (keyed by path), so revisiting a directory is instant and only new files are scanned; navigating cancels the current scan and restarts for the new directory.

**See also**

- [File Operations](#file-operations) — `e` opens the editor that fixes a flagged file
- [Renaming](#renaming) — the same non-conformance check, at load time


# Context Panel

[Up](#browser)

The browser's right-hand pane, showing whatever the highlight implies: nothing, a track's tags, or a playlist's entries. It follows the highlight passively until the operator focuses it.

Focusing it makes it the active list. On a highlighted playlist, `l` opens it for browsing — the cursor moves independently of the browser, and `Enter` sends the chosen entry to a deck, the second way onto a deck and the one that picks any entry rather than starting from the first. `e` opens the same playlist for editing instead, and an entry needing confirmation opens the candidate picker. While the panel has focus the browser dims, so which list is being driven is never in doubt.

**See also**

- [Playlist Editing](#playlist-editing) — the edit state, and what a commit does

- [Candidate Picker](#candidate-picker) — the state an unconfirmed entry opens

- [Loaded Playlist](#loaded-playlist) — where an entry sent from browsing arrives


# Playlists

[Up](#application)
[Down](#loaded-playlist)
[Down](#playlist-editing)
[Down](#finding-tracks)

A saved running order. An `.rpl` file lists its tracks by **content identity** — the hash of the audio payload — and keeps the file path only as a hint, so a set survives its tracks being moved, renamed, or retagged.

Three things happen to a set: it **plays** on a deck, it is **edited** in the browser's context panel, and its entries are **resolved** to real files whenever either of those happens. Resolution is the load-bearing one — an entry that can't be matched to a file can't be played, and the operator has to be told before they reach it mid-mix.

Deck implements the format; it doesn't define it. The `.rpl` spec — byte ranges, hashing, the write procedure and its backups — has its own map.

**See also**

- [Resilient Playlists](resilient-playlists/map.md#resilient-playlists) — the format itself: identity rules, resolution steps, write procedure
- [Track Database](#track-database) — the same content identity, keyed per track for BPM and cue


# Loaded Playlist

[Up](#playlists)

A set attached to a deck. Loading an `.rpl` plays the first entry that resolves and attaches the rest, so the deck works through the running order without further browsing. A `≡ x/y` badge before the track name gives the position in the set.

At end of track the deck loads the next entry that resolves, skipping any it can't play; `alt+n` / `alt+p` step the selected deck the same way. Each arrives **loaded but paused**, like any other load — the running order queues the next track up, it never starts it. The entry on the deck is tracked by identity rather than by index, so an edit committed in the browser doesn't lose the deck's place.

A set carrying entries the deck can't play says so on load and turns the badge amber, and it stays amber until they are fixed. That is the whole of the deck's answer — which entries, and why, is the browser's.

**See also**

- [Finding Tracks](#finding-tracks) — what "can't play" means, and when it is decided

- [Playlist Editing](#playlist-editing) — a committed edit reaches every deck carrying the set

- [Keymap](#keymap) — the skip keys


# Playlist Editing

[Up](#playlists)

Building and reordering a set, in the browser's context panel. `n` creates an empty `.rpl` in the current directory; `e` on a highlighted one opens it for editing.

Editing is **transactional** — a working buffer that only reaches the file on commit. `Enter` commits and writes; `Esc` drops the buffer and the set is untouched. Focus moves between the browser, where entries are picked, and the list, where they are reordered and removed. Inserting from the browser takes a track, or splices in another playlist's entries wholesale.

A commit reaches decks as well as the file: any deck carrying the same set adopts the new running order in place, keeping the track it is playing.

**See also**

- [Context Panel](#context-panel) — the pane this happens in, and its other states

- [Loaded Playlist](#loaded-playlist) — what a committed set does to a deck already carrying it

- [Keymap](#keymap) — the editor keys


# Finding Tracks

[Up](#playlists)
[Down](#library-scan-cost)
[Down](#candidate-picker)

Turning an entry into a file on disk. Every entry resolves to one of three outcomes: **found** and playable, **needs confirmation**, or **unavailable**. Only found entries play.

Resolution needs somewhere to look. With no **workspace** set an entry can only be checked where its hint says it is — relative to the `.rpl` itself, so a set kept alongside its music still resolves. The workspace is the search root that makes relocating a *moved* track possible at all. Setting one re-resolves every open set, repairing what it can and leaving the rest to be reported.

Repairs found on the way are written back to the `.rpl`.

**See also**

- [File Resolution](resilient-playlists/map.md#file-resolution) — the matching rules themselves: hint, then size, then hash

- [Search](#search) — where the workspace is set, and what else it serves


# Library Scan Cost

[Up](#finding-tracks)

Searching is the expensive part: a walk of the library and a probe per file. It happens **at most once per operation**, and only when something actually needs looking for — a set whose tracks are all where it left them never triggers it.

> [!IMPORTANT] The unit is the operation, not the entry. Resolving fifty entries screens the library once; anything that resolves per entry instead pays that cost fifty times.

**Detail**

A whole set resolves when it is opened on a deck or in the panel, when a workspace is set, and after an edit. Stepping to the next entry on a deck resolves lazily instead, one entry at a time.


# Candidate Picker

[Up](#finding-tracks)

Where the operator settles an entry the rules can't. When nothing in the library hashes to the entry's identity the track was most likely re-encoded — new bytes, so no hash can ever match it again — and the fallback offers library files that resemble it by duration and description. They appear as cards, closest first; choosing one re-links the entry to that file.

Confirming rewrites the entry's identity to the chosen file's, so the set follows the new encoding from then on. That is why it is never automatic: nothing but the operator can tell a re-encode of the same track from a different recording of it.

**See also**

- [Descriptive Fallback](resilient-playlists/map.md#descriptive-fallback) — how candidates are found and ranked

- [Context Panel](#context-panel) — the pane the picker occupies


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
[Down](#key-reporting)

Three input layers on a split keyboard: plain keys, Shift-modified, and Alt-chorded. The left block controls the selected deck — transport, BPM, pitch, nudge, cue, PFL. The right block addresses each deck's mixer directly — level, gain, filter — so the operator can adjust any deck without switching selection.

Alt is the chord modifier: holding it and pressing another key fires a chord action. Alt arrives as a reliable per-keypress modifier bit, so — unlike a held Space — there's no held-state to track. By convention, chords are reserved for one-time actions (set cue, open browser, select/cycle deck); continuous actions like nudge or fader movement use the plain or Shift layers. Space still fires the same chords as an unadvertised legacy modifier. Ctrl-C always quits unconditionally.

Esc steps up one level per press — dismiss the overlay, leave the panel, close the browser — one physical tap at a time. Repeats and releases are ignored, so holding it doesn't race through the levels.

Most keys are configurable via `config.toml` as action-name → key-string mappings. A small set are fixed: browser command-mode and search keys, tag editor input, and confirmation prompts.

**See also**

- [keybindings.md](keybindings.md) — full action table, keyboard layout, fixed keys, config format


# Key Reporting

[Up](#keymap)

Terminals differ in how much they say about a keystroke. On startup deck configures two things: that key events carry their kind — press, repeat or release — and that keys are encoded unambiguously, rather than in the legacy forms that collide with escape sequences.

Two features rest on this. Warp nudge ends on release, so without release reporting it would latch on with no way to stop it. Esc's legacy encoding is the bare escape byte, which has no room for a kind, so its release would arrive as an identical second press and one tap would act twice.

**Detail**

The kitty keyboard protocol; support is detected at startup. Where it is absent, warp nudge is refused with a notification and Esc arrives as a single unlabelled press per tap.

**See also**

- [Nudge](#nudge) — warp mode is the feature that needs release reporting


# Track Database

[Up](#application)

Per-track memory, keyed by the track's **content identity** — the Blake3 hash of its encoded audio payload with tags excluded, the same identity playlists and the tag editor use. So it follows the music across renaming, retagging, and re-containering, and one identity spans the whole app. Each entry holds the detected BPM, phase offset, cue point, and gain trim (plus whether the offset was deliberately placed, and the filename as a human-readable hint).

Stored at `~/.local/share/deck/track-data.json` (XDG data home) — durable user data, not a regenerable cache. When a workspace is set, the database also mirrors to a copy in the workspace root (`track-data.json`, same filename), so it travels with the music.

**Detail**

- A JSON file: an `_about` header that explains itself to a stranger, then the `tracks` map of `{identity: entry}`, deterministically sorted. Mutation marks it dirty; a flush runs after ~1 s idle, and quit always flushes, so a crash loses at most the last second of trims. Untouched keys are never rewritten.
- The two copies reconcile whenever a workspace becomes active — at start-up if one is already configured, and when the operator sets or changes it: the workspace copy wins on shared identities, local-only entries are pushed out, and both are written at once. Every later save writes both, so the analysis follows the library between machines.
- A track whose content identity can't be computed still loads and plays but persists nothing here — it's unsupported app-wide (no playlist can reference it), and the failure is surfaced as an error event carrying the cause (see [Fault Capture](#fault-capture)).

**See also**

- [Session State](#session-state) — the other persisted store, in the state dir
- [Metadata Editor](#metadata-editor) — defines content identity and its safeguard


# Session State

[Up](#application)

Global player state remembered between runs: last-visited browser directory, search workspace, audio latency, vinyl/beat mode, and cover-art brightness.

Stored at `~/.local/state/deck/session.json` (XDG state home), alongside the panic log and error reports (see [Fault Capture](#fault-capture)).

**Detail**

- Same save discipline as the [Track Database](#track-database): dirty-on-mutation, ~1 s idle flush, flush on quit, atomic temp-file rename.

**See also**

- [Track Database](#track-database) — the other persisted store, in the data dir


# Messages

[Up](#application)
[Down](#history-view)
[Down](#event-log)

Application messages appear in three places:

- The **global bar** — top row, always visible. Shows one thing at a time; at idle, it shows directory and version.

- **Deck overlays** — on each deck's own overview, for things about that deck.

- The **history** — everything worth keeping, one thing seen two ways: the [History View](#history-view) is its UI, the [Event Log](#event-log) its persistence.

Three kinds of message pass through them:

- **Prompts** await a keypress beside what they ask about: deck prompts (BPM confirmation, rename offer) on the deck overlay, app prompts (quit and load confirmations) on the bar. Never recorded.

- **Events** are things that happened — loads, moves, warnings, identity alerts. Every event enters the history; those needing attention also show on the bar, named ("Deck 2: …"), and leaving the bar loses nothing.

- **Hints** are transient guidance, on the bar — "No track loaded — Alt+F opens the file browser". Displayed, recorded nowhere.

**Detail**

- Bar precedence: prompts outrank events, events outrank hints; Esc dismisses whatever it shows (events stay in the history).
- Severity (info/success/warning/error) colours the bar; display time is per-event — 5 s usual, 30 s for the identity alert.


# History View

[Up](#messages)

The look-back over every event, this session and previous ones. `N` opens it over the album-art space (mutually exclusive with the browser, like help); each line is clock time plus the event, severity-coloured, long lines wrapping under a hanging indent. `k`/`j` scroll older/newer — the header counts what lies beyond each edge — and Esc or `N` closes it.

Sessions read as one continuous scroll: on startup the log file's retained history seeds the view, and every session opens with a "deck v… started" line and closes with "deck quit", so the boundaries are visible. Seeded history never appears on the global bar.

**Detail**

- The header names the log file's path.
- Copying text out is the terminal's own selection — Shift+drag bypasses the app's mouse capture.

**See also**

- [Keymap](#keymap) — `message_history` is configurable; `N` is the default


# Event Log

[Up](#messages)

The persistent record: every event appends a line to `messages.log` in the state dir, beside the panic log and error reports. Human-readable — `2026-08-11 14:03 warn deck2 3 tracks unavailable…` — local time, written and flushed as it happens, so a crash loses nothing. This file seeds the History View, and it's the one to read when diagnosing after the fact.

The file prunes itself at startup: lines older than the retention window (`[messages] retention_days`, default 90) are dropped, as are lines that no longer parse. One file, no rotation suffixes.

**Detail**

- Line format: `YYYY-MM-DD HH:MM:SS <severity> <source>  <text>`; sources are `deck1`–`deck3`, `playlist`, `tags`, `files`, `app`. Text is single-line by construction.
- Local time uses the zone offset queried from the system once at startup (`date +%z`), falling back to UTC.

**See also**

- [Fault Capture](#fault-capture) — the log is fault capture's first layer: mismatch events name their report folders, and a session missing its closing "deck quit" raises the abnormal-end warning at next startup


# Fault Capture

[Up](#application)

Three layers, by what a fault needs to leave behind:

- **The log** is the primary record — every fault is an event, in history and on disk with its detail. See [Event Log](#event-log).

- **`error_reports/`** preserves artefacts a log line can't hold. One kind today: **identity-mismatch** — a tag edit changed the audio payload; a dated folder keeps the original, edited, and details for recovery.

- **`panic.log`** catches the crash itself, written as the process dies. At next startup, a session that never logged its "deck quit" raises an abnormal-end warning, naming panic.log if present — crashes join the narrative.

All three live in `~/.local/state/deck/`. Report folders are named `YYYY-MM-DD_HHMMSS-<kind>-<label>` so a listing reads chronologically, and the mismatch event names its folder — the log indexes the directory.

**See also**

- [Metadata Editor](#metadata-editor) — the identity safeguard that writes mismatch reports
- [Event Log](#event-log) — layer one, the chronological record
- [Event Log](#event-log) — the identity-mismatch event names its report, so the log indexes this directory chronologically


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
- [Session State](#session-state) — latency persisted as a global value
