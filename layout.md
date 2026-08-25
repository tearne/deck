# Layout Reference

Visual layout and presentation details for deck. Conceptual descriptions live in `map.md`; this document covers field formats, colour schemes, and rendering precedence.


## UI Structure

Sections in render order, top to bottom. Heights in rows; variable sections compress when screen space is limited.

| Section | Height | Notes |
|---------|--------|-------|
| Global status bar | 1 | Fixed — always rendered |
| Detail info bar | 1 | Shared across all three decks |
| Detail waveform A | `detail_height` | Collapses entirely if insufficient space |
| Shared tick row A/B | 1 | |
| Detail waveform B | `detail_height` | |
| Shared tick row B/C | 1 | |
| Detail waveform C | `detail_height` | |
| Notification bar A | 1 | |
| Info bar A | 1 | |
| Overview A | `overview_height` | Collapses entirely if insufficient space |
| Notification bar B | 1 | |
| Info bar B | 1 | |
| Overview B | `overview_height` | |
| Notification bar C | 1 | |
| Info bar C | 1 | |
| Overview C | `overview_height` | |
| Spacer | remaining | Bottom view: album art (ground), browser, help, or messages |


## Detail Info Bar

One shared row above all three detail waveforms. Always dim grey.

```
  zoom:4s  lat:0ms  fps:100/120/120  [JUMP]  [BEAT]
```

| Field | Format | Notes |
|-------|--------|-------|
| Zoom | `zoom:Xs` | Current zoom level in seconds |
| Latency | `lat:Xms` | Audio latency; always shown |
| FPS | `fps:A/B/C` | Measured / budget / cap |
| Nudge mode | `[JUMP]` or `[WARP]` | From the first loaded deck |
| Mode | `[VINYL]` or `[BEAT]` | Current playback mode |
| Space held | `[SPC]` | Shown only while Space is held |


## Notification Bar

One row per deck. Priority order (highest wins):

1. **BPM pending** — `BPM detected: 120.00  [y] accept  [n] reject  (Xs)`. Yellow; countdown turns red in the last 5 s. Expires after 15 s.
2. **Active notification** — transient message in style colour (see Colour Schemes). Expires after 5 s.
3. **Rename offer** — track name left-aligned; `rename? [y]` right-aligned. First 10 s: red with per-second countdown. After 10 s: dim. Offer lingers until dismissed or a track loads. While the browser view is showing, the offer renders as an amber banner over the context panel instead and the deck stays clean.
4. **Default** — track name (left) + cache indicators (right, 16 chars fixed).

### Cache Indicators

`[BPM][Tick][Cue]` — right-aligned, 16 chars fixed so the track name does not shift.

| Indicator | Lit when |
|-----------|----------|
| `[BPM]` | BPM established (cache, tap, or detection confirmed) |
| `[Tick]` | Offset explicitly placed, or cue point set |
| `[Cue]` | Cue point set |

**Lit** — palette treble colour at moderate brightness. **Unlit** — near-black `Rgb(50, 50, 50)`. **Vinyl mode** — all indicators show a dim version of the palette treble colour regardless of state.

Track name uses a muted treble-palette colour; notification text uses style colours.


## Info Bar

One row per deck. Two groups separated by a variable-width spacer; right group stays pinned to the right edge.

### Left Group

| Slot | Content |
|------|---------|
| Play icon | `▶` playing / `⏸` paused |
| Analysing | `[analysing ⠋]` with animated spinner — shown when BPM analysis running, vinyl mode active, or no BPM established |
| Speed (percentage) | Vinyl mode or no established BPM: `+0.3%` / `-1.2%` / `0.0%`. Pitch appended as `(+2st)` if non-zero. Phase offset appended in beat mode once BPM is established. |
| BPM (beat mode, established) | `120.00` with beat flash (yellow/amber bg). If playback BPM differs: `120.00 (124.4)`. If pitch shifted: shown inside the same parentheses. |
| Metronome | `♪` in red when active |
| Phase offset | `  +0ms` — always shown in beat mode when BPM established |
| Tap counter | `  tap:N` — shown during an active tap session; yellow flash (150 ms) on each tap |

### Right Group

| Slot | Content | Notes |
|------|---------|-------|
| Nudge direction | `  ▶nudge` or `  ◀nudge` | Shown only while a nudge is in progress |
| PFL active | `  PFL` in cyan | Shown when `pfl_level > 0` |
| Level | `  level:▕X▏` | X = 8-level block char (▁▂▃▄▅▆▇█); colour scales dark→amber with level |
| Gain | Single char `▁▂▃▄▅▆▇` immediately after `▏` | Amber when non-zero; near-black at 0 dB |
| Spectrum | `  ▕` + 16 braille chars + `▏` + 2-char dB/oct field | Filter shading applied within strip; `12`/`24` dB/oct shown when filter active, blanked otherwise |

### Empty Deck

- Notification bar: `no track — Space+F to open the file browser` (dim)
- Info bar: `⏸  ---  +0ms` (dim)


## Global Status Bar

One row pinned to the top of the screen. Priority order:

1. **Quit confirmation** — "Track is playing — quit?  [y] quit   [Esc/n] cancel"; centred with countdown. Error colours.
2. **Browser-blocked** — confirmation prompt when opening the browser over a playing deck. Error colours with countdown.
3. **Global notification** — centred message with countdown. Style colours (see Colour Schemes).
4. **Idle** — browser working directory (left) + version string (right), dim grey.

### Colour Schemes

| Style | Foreground | Background |
|-------|------------|------------|
| Error | `Rgb(255, 180, 180)` | `Rgb(100, 20, 20)` |
| Warning | `Rgb(255, 220, 120)` | `Rgb(80, 60, 0)` |
| Info | `Rgb(160, 200, 255)` | `Rgb(20, 40, 80)` |
| Success | `Rgb(140, 230, 160)` | `Rgb(10, 60, 30)` |

Per-deck notification bar uses foreground colours only (no background): Error=red, Warning=yellow, Info=dark grey, Success=green.


## Spacer Panel

The rows left over below Overview C. Shows exactly one bottom view; the choice persists across sessions.

1. **Album art** (ground state) — three panels (one per deck), 1-row top margin, 1-column gaps between panels. Rendered when `art_bright_idx < 2`. Brightness levels: full (index 0), dim 35% (index 1), off (index 2). `/` and repeat presses of `space+v` cycle the index.
2. **Browser** — fills the spacer. If the spacer has fewer than 8 rows, the browser expands to fill the full screen instead. Dimmed whole while the decks hold the keyboard.
3. **Keyboard help** — drawn over the art ground. Takes no input.
4. **Messages** — the event history. Dimmed while the decks hold the keyboard; focused, `k`/`j` scroll it.


## Column Coincidence (Detail Waveform)

When multiple elements occupy the same screen column, priority (highest wins):

1. **Playhead** — always rendered; distinct playhead colour
2. **Cue mark** — green `│`, full detail height
3. **Tick mark** — rendered in gaps where waveform dots are absent; does not override waveform dots
4. **Waveform** — braille dot pattern from the wide buffer
