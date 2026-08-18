# Keybindings Reference

Action-to-key reference for deck. Map nodes reference actions by their config name (e.g. `bpm_tap`); this document maps each to its default key.


## Keyboard Layout

The spatial layout below matches the in-app help overlay (`?`). Left block = selected-deck controls; right block = per-deck mixer.

```
  ╭         ╭         ╭         ╭ +32b    ╭ +64b    ┆   ╭  ╭  ╭ +Slope
  1 +1bt    2 +1b     3 +4b     4 +8b     5 +16b    ┆   7  8  9 HPF
  ╰         ╰         ╰         ╰         ╰         ┆   ╰  ╰  ╰ Flt=
    ╭         ╭         ╭         ╭ -32b    ╭ -64b    ┆   ╭  ╭  ╭ -Slope
    Q -1bt    W -1b     E -4b     R -8b     T -16b    ┆   U  I  O LPF
    ╰         ╰         ╰         ╰         ╰         ┆   ╰  ╰  ╰ Flt=
      ╭         ╭         ╭ +Tick   ╭ -BsBPM  ╭ CueJp   ┆   ╭  ╭  ╭ +Gain
      A +Ptch   S +PFL    D +Ndge   F -BPM    G         ┆   J  K  L +Lvl
      ╰ =Ptch   ╰ Rst     ╰ PFLTog  ╰ Brows   ╰ Play    ┆   ╰  ╰  ╰ 100%
        ╭         ╭         ╭ -Tick   ╭ +BsBPM  ╭ CueSt   ┆   ╭  ╭  ╭ -Gain
        Z -Ptch   X -PFL    C -Ndge   V +BPM    B Tap     ┆   M  ,  . -Lvl
        ╰ =Ptch   ╰ Rst     ╰ SpRst   ╰ Metro   ╰ BDtct   ┆   ╰  ╰  ╰ 0%
───────────────────────────────────────────────────────────────────── ╭ [Shift]
` vinyl   ¬ nudge   -/= zoom   {/} height   [/] latency   Esc quit   │ [Bare]
/ art   p palette   Sp+= swap1↔2   Sp+- swap2↔3   Alt+j/k deck      ╰ [Space]
```

Per-cell format: `╭ Shift-action` / `Key plain-action` / `╰ Space-action`. Empty modifier cells = no binding on that layer.

Not shown in the overlay: `?` help toggle.


## Config Actions

Configurable via `config.toml` under `[keys]`. Format: `action_name = "key"` or `action_name = ["key1", "key2"]`.

### Global

| Action | Default | Description |
|--------|---------|-------------|
| `quit` | `esc` | Quit (confirmation only while a deck is playing) |
| `help` | `?` | Toggle keyboard help overlay |
| `message_history` | `N` | Toggle message history overlay (k/j scroll, Esc close) |
| `vinyl_mode_toggle` | `` ` `` | Toggle vinyl/beat mode |
| `nudge_mode_toggle` | `¬` | Toggle nudge jump/warp |
| `zoom_in` | `-` | Zoom in |
| `zoom_out` | `=` | Zoom out |
| `height_increase` | `}` | Increase detail waveform height |
| `height_decrease` | `{` | Decrease detail waveform height |
| `latency_increase` | `]` | Audio latency +10 ms |
| `latency_decrease` | `[` | Audio latency −10 ms |
| `fps_increase` | `^` | Increase FPS cap |
| `fps_decrease` | `Y` | Decrease FPS cap |
| `palette_cycle` | `p` | Cycle colour palette |
| `art_cycle` | `/` | Cycle album art brightness |

### Deck Selection & Swap

| Action | Default | Description |
|--------|---------|-------------|
| `select_next_deck` | `alt+j` | Cycle to next deck (wraps; works in the browser too) |
| `select_prev_deck` | `alt+k` | Cycle to previous deck (wraps; works in the browser too) |
| `playlist_next` | `space+n` | Selected deck: skip to next playlist track |
| `playlist_prev` | `space+p` | Selected deck: skip to previous playlist track |
| `swap_deck1_deck2` | `space+=` | Swap decks 1 ↔ 2 |
| `swap_deck2_deck3` | `space+-` | Swap decks 2 ↔ 3 |

### Transport

| Action | Default | Description |
|--------|---------|-------------|
| `play_pause` | `space+g` | Play/pause |
| `open_browser` | `space+f` | Open file browser |

### BPM & Beat Grid

| Action | Default | Description |
|--------|---------|-------------|
| `bpm_tap` | `b` | Tap BPM |
| `detect_bpm` | `space+b` | Trigger BPM detection |
| `bpm_increase` | `v` | Playback BPM +0.1 |
| `bpm_decrease` | `f` | Playback BPM −0.1 |
| `base_bpm_increase` | `V` | Native BPM +0.01 |
| `base_bpm_decrease` | `F` | Native BPM −0.01 |
| `speed_reset` | `space+c` | Reset playback speed to nominal |
| `offset_increase` | `D` | Phase offset +10 ms |
| `offset_decrease` | `C` | Phase offset −10 ms |
| `metronome_toggle` | `space+v` | Toggle metronome |

### Nudge

| Action | Default | Description |
|--------|---------|-------------|
| `nudge_forward` | `d` | Nudge forward |
| `nudge_backward` | `c` | Nudge backward |

### Cue

| Action | Default | Description |
|--------|---------|-------------|
| `cue` | `B` | Set cue point (paused only) |
| `cue_play` | `G` | Jump to cue point |

### Pitch

| Action | Default | Description |
|--------|---------|-------------|
| `pitch_up` | `a` | Pitch +1 semitone |
| `pitch_down` | `z` | Pitch −1 semitone |
| `pitch_reset` | `space+a`, `space+z` | Reset pitch to 0 |

### PFL

| Action | Default | Description |
|--------|---------|-------------|
| `pfl_up` | `s` | PFL level +20 |
| `pfl_down` | `x` | PFL level −20 |
| `pfl_reset` | `space+s`, `space+x` | PFL level to 0 |
| `pfl_on_off` | `space+d` | Toggle PFL on (100) / off (0) |

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
| `jump_forward_16b` | `5` | Jump +16 bars |
| `jump_backward_16b` | `t` | Jump −16 bars |
| `jump_forward_32b` | `$` | Jump +32 bars |
| `jump_backward_32b` | `R` | Jump −32 bars |
| `jump_forward_64b` | `%` | Jump +64 bars |
| `jump_backward_64b` | `T` | Jump −64 bars |

### Mixer (per deck)

Actions are prefixed `deck1_`, `deck2_`, `deck3_`. Default keys are shown left to right (deck 1 / 2 / 3).

| Action template | Deck 1 | Deck 2 | Deck 3 | Description |
|-----------------|--------|--------|--------|-------------|
| `deckN_level_up` | `j` | `k` | `l` | Level +5% |
| `deckN_level_down` | `m` | `,` | `.` | Level −5% |
| `deckN_level_max` | `space+j` | `space+k` | `space+l` | Level to 100% |
| `deckN_level_min` | `space+m` | `space+,` | `space+.` | Level to 0% |
| `deckN_gain_increase` | `J` | `K` | `L` | Gain +1 dB |
| `deckN_gain_decrease` | `M` | `<` | `>` | Gain −1 dB |
| `deckN_filter_increase` | `7` | `8` | `9` | Filter toward HPF |
| `deckN_filter_decrease` | `u` | `i` | `o` | Filter toward LPF |
| `deckN_filter_reset` | `space+7`, `space+u` | `space+8`, `space+i` | `space+9`, `space+o` | Filter to flat |
| `deckN_filter_slope_increase` | `&` | `*` | `(` | Filter slope up |
| `deckN_filter_slope_decrease` | `U` | `I` | `O` | Filter slope down |

### Loop (experimental)

| Action | Default | Description |
|--------|---------|-------------|
| `loop_tap` | `g` | Tap loop entry |
| `loop_exit` | `H` | Exit loop |


## Fixed Keys

Not configurable — hardcoded in the application.

### Browser

| Key | Action |
|-----|--------|
| `Up` / `Down` / `j` / `k` | Navigate entries |
| `Enter` | Load highlighted track to the selected deck, or open highlighted playlist |
| `Backspace` / `Left` | Go up a directory |
| `@` | Set current directory as workspace |
| `'` | Clear workspace |
| `#` | Preview highlighted track |
| `n` | New playlist (prompts for a name) |
| `Esc` | Close browser / clear search term |
| `q` | Close browser (when not searching) |
| Printable chars | Search (when workspace is set) |

### Playlist editor (right pane; opens on hovering a `.rpl`)

| Key | Action |
|-----|--------|
| `Tab` / `l` / `e` | Focus the playlist pane; `h` / `Esc` back to the browser |
| `Enter` / `a` | (browser focus) append highlighted track |
| `j` / `k` | (playlist focus) move cursor |
| `K` / `J` | (playlist focus) reorder entry up / down |
| `x` / `Delete` | (playlist focus) remove entry |

### Tag Editor

| Key | Action |
|-----|--------|
| `Tab` / `Down` | Next field |
| `Shift+Tab` / `Up` | Previous field |
| `Left` / `Right` | Move cursor |
| `Home` / `End` | Cursor to start / end |
| `Backspace` / `Delete` | Delete character |
| `Enter` | Confirm and rename |
| `Esc` | Cancel |

### Confirmations

| Key | Context | Action |
|-----|---------|--------|
| `y` | BPM detection, quit, load-to-playing-deck | Confirm (only `y` — a reflexive Enter dismisses) |
| any other key | BPM detection, quit, load-to-playing-deck | Cancel |
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
