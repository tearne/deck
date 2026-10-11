# Keybindings Reference

Action-to-key reference for deck. Map nodes reference actions by their config name (e.g. `bpm_tap`); this document maps each to its default key.


## Keyboard Layout

The spatial layout below matches the default in-app help overlay (`?` or `h` with deck focus). Columns 1–4 hold selected-deck playing controls; columns 5–6 hold mode, grid, transport, and view controls. Columns 7–9 (U/I/O, J/K/L, M/,/.) address the three decks’ mixers directly. The extended keys show zoom, FPS, height, latency, and deck swaps. Alt is a global layer across the keyboard; its left/right pane actions apply only in the browser.

```
╭ +16b    ╭ +32b    ╭ +64b    ╭ +128b   ╭      ╭      ╭  ╭  ╭ +Slope ╭      ╭ -FPS  ╭ +FPS
1 +1bt    2 +1b     3 +4b     4 +8b     5 Mode 6      7  8  9 HPF    0      - Zm+   = Zm-
│ Ghost   │         │         │         │      │      │  │  │ Flt=   │      │ 2↔3   │ 1↔2
╰ Deck1   ╰ Deck2   ╰ Deck3   ╰         ╰      ╰      ╰  ╰  ╰        ╰      ╰       ╰

  ╭ -16b    ╭ -32b    ╭ -64b    ╭ -128b   ╭      ╭      ╭  ╭  ╭ -Slope ╭      ╭ -Ht   ╭ +Ht
  Q -1bt    W -1b     E -4b     R -8b     T      Y      U  I  O LPF    P      [ -Lat  ] +Lat
  │         │         │         │         │      │      │  │  │ Flt=   │      │       │
  ╰         ╰         ╰         ╰ Restore ╰      ╰      ╰  ╰  ╰        ╰      ╰       ╰

    ╭ CueJp   ╭         ╭ +Tick   ╭ -BsBPM  ╭      ╭      ╭  ╭  ╭ +Gain  ╭      ╭       ╭
    A +Ptch   S +PFL    D +Ndge   F -BPM    G Grid H Help J  K  L +Lvl   ;      '       #
    │         │         │         │ Browse  │ Play │ Art  │  │  │ 100%   │      │       │
    ╰         ╰         ╰         ╰         ╰      ╰ ←    ╰↓ ╰↑ ╰ →      ╰      ╰       ╰

      ╭ CueSt   ╭         ╭ -Tick   ╭ +BsBPM  ╭ Metro╭      ╭  ╭  ╭ -Gain  ╭ Help
      Z -Ptch   X -PFL    C -Ndge   V +BPM    B Tap  N      M  ,  . -Lvl   / Bright   ╭ [Shift]
      │ Rst     │ Rst/Tog │ NMode   │ Rst     │      │ Msgs │  │  │ 0%     │          │ [Bare]
      ╰         ╰         ╰         ╰         ╰      ╰      ╰  ╰  ╰        ╰          │ [Space]
Browser: ` jump to selected deck's track                                              ╰ [Alt]
```

Per-cell format: `╭ Shift-action` / `Key plain-action` / `│ Space-action` / `╰ Alt-action`. A blank line separates physical keyboard rows. The white F accent remains; Shift is amber, plain gray, Space blue, Alt plum, and nudge/tempo labels sage. Unlabelled cells have no default binding in this context. `b` means bars; `bt` means beats. `Rst` resets pitch on Space+Z and playback speed on Space+V. `Rst/Tog` on Space+X turns active PFL off at any level, or turns it on at 100%.

View shortcuts and Alt pane navigation also appear in contextual hints. Alt+Up/Down are deck-cycling alternatives; Alt+Left/Right resize browser panes. Tab focus, these arrow alternatives, and Esc are omitted from the map. With browser focus, `h` retains parent-directory navigation, `/` starts search, and `?` opens Help in command mode. The bottom-left note shows backtick: in browser command mode it jumps to and highlights the selected deck’s loaded track. Search and text-entry modes keep printable keys as text.

`Tab` is fixed (not configurable): it flips the keyboard between the decks and the showing bottom view (browser or messages; help and art take no input).


## Contextual Hints

A shared hint area across the bottom shows controls for the current keyboard context: browser mode, playlist, candidate selection, tag editing, messages, or decks. Groups describe their controls consistently: Navigate, Actions, Edit, Switch and Views. Related controls stay together and wrap to additional rows as the terminal narrows. Pane-specific groups share their pane’s colour: browser mode colours, playlist green, and tags violet; shared switching and view controls stay neutral. Configurable actions show their current binding; typing modes omit commands intercepted as text.

Pane names and the configured pane-switch keys remain above their content. Tags has no navigation hint and cannot take focus when the highlighted item has no editable track metadata. Redundant arrow alternatives are omitted from hints; arrows remain shown where they have distinct roles, such as search selection and text editing. Search and Move are labelled beside the Browser chip; command mode has no suffix. Tag warnings, location and search result count appear in the browser’s bottom border. The workspace is shown by `@` and its path in the top border. Active text input and the move destination have a separate shaded area at the top inside the frame, above the file list. Search and playlist-name input wrap, keeping the input tail visible on short screens. Confirmation and editing controls take priority when height limits the hint area; an overflow notice asks for more terminal space.

With deck focus, the hints show view/focus controls and the key for the full keyboard reference. Hints are always visible; there is no minimise action.


## Config Actions

Configurable via `config.toml` under `[keys]`. Format: `action_name = "key"` or `action_name = ["key1", "key2"]`. Shifted symbols use the UK layout: Shift+2 is `"`, Shift+3 is `£`. Bind the characters your keyboard emits if it differs.

Existing config files override the new defaults and are not rewritten. To adopt this layout, update their `[keys]` entries from [resources/config.toml](resources/config.toml), or back up the old config and let Deck create a fresh one. The help map shows defaults; contextual hints use live bindings.

### Global

| Action | Default | Description |
|--------|---------|-------------|
| `quit` | `esc` | Quit (confirmation only while a deck is playing) |
| `help` | `?`, `h` | Show/hide the help view |
| `message_history` | `space+n` | Show/hide the messages view (takes focus; k/j scroll, Esc unfocus) |
| `mode_cycle` | `5` | Cycle the selected deck's mode (Playback/Beat); `vinyl_mode_toggle` still parses as an alias |
| `nudge_mode_toggle` | `space+c` | Toggle nudge jump/warp |
| `zoom_in` | `-` | Zoom in |
| `zoom_out` | `=` | Zoom out |
| `height_increase` | `}` | Increase detail waveform height |
| `height_decrease` | `{` | Decrease detail waveform height |
| `latency_increase` | `]` | Audio latency +10 ms |
| `latency_decrease` | `[` | Audio latency −10 ms |
| `fps_increase` | `+` | Increase FPS cap |
| `fps_decrease` | `_` | Decrease FPS cap |
| `art_cycle` | `/` | Cycle album art brightness |
| `art_view` | `space+h` | Show the art view; further presses cycle its brightness |
| `grid_mode` | `g` | Open a Grid session on the selected deck (see Grid mode keys) |

### Deck Selection & Swap

| Action | Default | Description |
|--------|---------|-------------|
| `select_next_deck` | `alt+j`, `alt+down` | Cycle to next deck (wraps; works in the browser too) |
| `select_prev_deck` | `alt+k`, `alt+up` | Cycle to previous deck (wraps; works in the browser too) |
| `select_deck_<n>` | `alt+<n>` (n = 1–3) | Select that deck; summon it if it's the next one; on the selected deck, destroy it (empty at once, loaded-paused via `y`, playing refused; last deck stays). Works identically in the browser |
| `session_restore` | `alt+r` | Reload the decks as they were when deck last ran (offered while all decks are empty) |
| `pane_left` | `alt+h` | Walk pane activation left (Tags → Browser → Playlist); cancels out of the tag editor. Works in every browser mode |
| `pane_right` | `alt+l` | Walk pane activation right; walking into Tags opens the highlighted track's tag editor; stays in Browser when no editable track is highlighted |
| `panel_widen` | `alt+left` | Drag the active pane's right-hand boundary left (playlist: narrower; browser: tags wider). Widths persisted separately |
| `panel_narrow` | `alt+right` | Drag the active pane's right-hand boundary right (playlist: wider; browser: tags narrower) |
| `swap_deck1_deck2` | `space+=` | Swap decks 1 ↔ 2 (needs both to exist) |
| `swap_deck2_deck3` | `space+-` | Swap decks 2 ↔ 3 (needs both to exist) |

### Transport

| Action | Default | Description |
|--------|---------|-------------|
| `play_pause` | `space+g` | Play/pause |
| `open_browser` | `space+f` | Open file browser |

### BPM & Beat Grid

| Action | Default | Description |
|--------|---------|-------------|
| `bpm_tap` | `b` | Tap BPM |
| `bpm_increase` | `v` | Playback BPM +0.1 |
| `bpm_decrease` | `f` | Playback BPM −0.1 |
| `base_bpm_increase` | `V` | Native BPM +0.01 |
| `base_bpm_decrease` | `F` | Native BPM −0.01 |
| `speed_reset` | `space+v` | Reset playback speed to nominal |
| `offset_increase` | `D` | Phase offset +10 ms |
| `offset_decrease` | `C` | Phase offset −10 ms |
| `metronome_toggle` | `B` | Toggle metronome |

### Nudge

| Action | Default | Description |
|--------|---------|-------------|
| `nudge_forward` | `d` | Nudge forward |
| `nudge_backward` | `c` | Nudge backward |

### Cue

| Action | Default | Description |
|--------|---------|-------------|
| `cue` | `Z` | Set cue point (paused only) |
| `cue_play` | `A` | Jump to cue point |

### Pitch

| Action | Default | Description |
|--------|---------|-------------|
| `pitch_up` | `a` | Pitch +1 semitone |
| `pitch_down` | `z` | Pitch −1 semitone |
| `pitch_reset` | `space+z` | Reset pitch to 0 |

### PFL

| Action | Default | Description |
|--------|---------|-------------|
| `pfl_up` | `s` | PFL level +20 |
| `pfl_down` | `x` | PFL level −20 |
| `pfl_reset` | Unbound | PFL level to 0 |
| `pfl_on_off` | `space+x` | Reset any active PFL level to off (0); when off, enable at 100 |

### Beat Jump

| Action | Default | Description |
|--------|---------|-------------|
| `jump_forward_1bt` | `1` | Jump +1 beat |
| `jump_backward_1bt` | `q` | Jump −1 beat |
| `jump_forward_1b` | `2` | Jump +1 bar |
| `jump_backward_1b` | `w` | Jump −1 bar |
| `jump_forward_4b` | `3` | Jump +4 bars |
| `jump_backward_4b` | `e` | Jump −4 bars |
| `jump_forward_8b` | `4` | Jump +8 bars |
| `jump_backward_8b` | `r` | Jump −8 bars |
| `jump_forward_16b` | `!` | Jump +16 bars |
| `jump_backward_16b` | `Q` | Jump −16 bars |
| `jump_forward_32b` | `"` | Jump +32 bars |
| `jump_backward_32b` | `W` | Jump −32 bars |
| `jump_forward_64b` | `£` | Jump +64 bars |
| `jump_backward_64b` | `E` | Jump −64 bars |
| `jump_forward_128b` | `$` | Jump +128 bars |
| `jump_backward_128b` | `R` | Jump −128 bars |
| `ghosts_toggle` | `space+1` | Toggle ghost playheads (jump key labels at each landing, Beat mode) |

### Mixer (per deck)

Actions are prefixed `deck1_`, `deck2_`, `deck3_`. Default keys are shown left to right (deck 1 / 2 / 3).

| Action template | Deck 1 | Deck 2 | Deck 3 | Description |
|-----------------|--------|--------|--------|-------------|
| `deck<n>_level_up` | `j` | `k` | `l` | Level +5% |
| `deck<n>_level_down` | `m` | `,` | `.` | Level −5% |
| `deck<n>_level_max` | `space+j` | `space+k` | `space+l` | Level to 100% |
| `deck<n>_level_min` | `space+m` | `space+,` | `space+.` | Level to 0% |
| `deck<n>_gain_increase` | `J` | `K` | `L` | Gain +1 dB |
| `deck<n>_gain_decrease` | `M` | `<` | `>` | Gain −1 dB |
| `deck<n>_filter_increase` | `7` | `8` | `9` | Filter toward HPF |
| `deck<n>_filter_decrease` | `u` | `i` | `o` | Filter toward LPF |
| `deck<n>_filter_reset` | `space+7`, `space+u` | `space+8`, `space+i` | `space+9`, `space+o` | Filter to flat |
| `deck<n>_filter_slope_increase` | `&` | `*` | `(` | Filter slope up |
| `deck<n>_filter_slope_decrease` | `U` | `I` | `O` | Filter slope down |

## Fixed Keys

Not configurable — hardcoded in the application.

### Detached view (grid work; playback never interrupted)

Toggled with `grid_mode` (`g`). The detail waveform stops tracking the playhead and follows a free cursor; jumps and nudge steer the cursor instead of the audio, and tick marks render in any deck mode. All grid edits are the normal live ones (`V`/`F` base BPM, `D`/`C` offset, `b` tap).

| Key | Action |
|-----|--------|
| `a` | Pin the anchor at the cursor — the grid's persisted phase source |
| `←` / `→` | Jump the cursor to the anchor / the last grid tick |
| `Esc` or `g` | Re-attach the view |

### Browser

| Key | Action |
|-----|--------|
| `Up` / `Down` / `j` / `k` | Navigate entries |
| `h` | Go up a directory (command mode) |
| `l` | Enter highlighted directory; on a playlist, pin and open it (command mode; never loads tracks) |
| `Enter` | Load highlighted track to the selected deck; on a playlist, pin it as the record box |
| `Backspace` / `Left` | Go up a directory |
| `@` | Set current directory as workspace |
| `'` | Clear workspace |
| `#` | Preview highlighted track |
| `` ` `` | Jump to the selected deck's track |
| `n` | New playlist (prompts for a name) |
| `Esc` | Back out one level: clear search term → command mode → keyboard to the decks |
| `Tab` | Keyboard to the decks (browser stays showing) |
| Printable chars | Search (when workspace is set) |

### Panes (Playlist | Browser | Tags)

On a wide terminal (≥120 columns) all three panes show at once. Narrower, two show at a time — Playlist+Browser, or Browser+Tags; under 50 columns the active pane has the width to itself. The nav strip above the panes names all three, off-screen tips docked at the screen edges. Whenever the playlist is on screen it is the playlist-working mode; edits write to the file live (no commit/abort).

| Key | Action |
|-----|--------|
| `alt+h` / `alt+l` | Walk activation left / right across the panes — every layout, every browser mode. Walking into Tags opens the tag editor; `alt+h` cancels back out |
| `j` / `k` or `↓` / `↑` | (playlist active) move |
| `Enter` | (playlist active) send the entry to the selected deck; on an unresolved entry, open the candidate picker |
| `K` / `J` or `shift+↑` / `shift+↓` | (playlist active) reorder — writes immediately |
| `→` / `←` | (playlist / tags active) back to the browser, like Esc |
| `x` / `Delete` | (playlist active) remove — writes immediately |
| `a` / `A` | Insert the browser's highlighted track after/before the playlist cursor (browser or playlist active, playlist on screen) — writes immediately |
| `Esc` | Back to the browser pane |

### Tag Editor

| Key | Action |
|-----|--------|
| `Down` | Next field (the rename toggle is the last stop) |
| `Up` | Previous field |
| `Space` (on the toggle) | Flip the `[x] Rename File` section header (default on) |
| `Left` / `Right` | Move cursor |
| `Home` / `End` | Cursor to start / end |
| `Backspace` / `Delete` | Delete character |
| `Enter` | Confirm and rename |
| `Esc` | Cancel |

### Confirmations

| Key | Context | Action |
|-----|---------|--------|
| `y` | quit, load-to-playing-deck | Confirm (only `y` — a reflexive Enter dismisses) |
| any other key | quit, load-to-playing-deck | Cancel |
| `y` | Rename offer | Open tag editor |
| `Ctrl-C` | Any | Quit unconditionally |

### Mouse

| Input | Action |
|-------|--------|
| Left click on overview | Seek to nearest bar boundary |


## Configuration

### Config loading

Key bindings load from `config.toml` at startup — first from the binary's directory, then from `~/.config/deck/config.toml`. If neither is found, the embedded default is written to `~/.config/deck/config.toml`.

### Key-string format

Printable characters as-is (`q`, `+`, `H`). Special keys as lowercase names (`space`, `esc`, `up`, `down`, `left`, `right`, `enter`, `backspace`). Alt-modifier chords as `space+<key>` (a legacy `space+<key>` form still parses to the same chord).

### Display parameters

Declared under `[display]` in `config.toml`. Missing keys fall back to defaults.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `playhead_position` | `20` | Detail playhead position, 0–100% from left |
| `warning_threshold_secs` | `30` | Seconds before track end to activate bar-marker flash |
| `detail_height` | `5` | Rows per detail waveform (including 2-row tick area; min 4) |
| `target_fps` | `60` | Frame rate cap; snapped to nearest: 15, 20, 24, 30, 45, 60, 90, 120, 240 |
