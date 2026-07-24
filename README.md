# deck

A minimal terminal DJ player.

## Rationale

Modern DJ equipment does so much of the work it removes the fun. `deck` blends the convenience of software with the skill of beat-matching and mixing — you don't need turntables, vinyl, and an analogue mixer to get the real-time experience.

## Features

- Three independent decks with real-time waveform visualisation
- Nudge and playback speed adjustment
- Unified high-pass / low-pass filter per deck
- BPM detection is optional — used only for beat jump; set by tapping or manual adjust
- Fuzzy file browser with library search
- Deliberately excludes loops, effects, jump points, samples, and track recommendations

## Key bindings

Three input layers: plain keys, **Shift** (uppercase), and **Space-chord** (hold Space, press another key). The left block controls the selected deck; the right block addresses each deck's mixer directly.

```
  ╭         ╭         ╭         ╭ +32b    ╭ +64b    ┆   ╭  ╭  ╭ +Slope
  1 +1bt    2 +1b     3 +4b     4 +8b     5 +16b    ┆   7  8  9 HPF
  ╰ SelD1   ╰ SelD2   ╰ SelD3   ╰         ╰         ┆   ╰  ╰  ╰ Flt=
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
/ art   ~ palette   Spc+= swap1↔2   Spc+- swap2↔3                     ╰ [Space]
```

Press `?` in-app for the full overlay. Full action reference: [keybindings.md](keybindings.md).

## Installation

Build dependencies (Linux):

- `pkg-config`
- ALSA development headers — `libasound2-dev` on Debian/Ubuntu; the plain `alsa` package doesn't include these
- A C++ compiler (`g++` or `clang++`) — used to build the bundled `soundtouch` library

```sh
sudo apt install pkg-config libasound2-dev g++   # Debian/Ubuntu
cargo build --release
cp target/release/deck ~/.local/bin/
```

Runtime dependency: ALSA or PipeWire (Linux). Supported audio formats: FLAC, MP3, OGG, WAV, AAC, OPUS.

```sh
deck [path]   # path can be a file or directory
```

## Attribution

- [symphonia](https://github.com/pdeljanov/Symphonia) — audio decoding
- [rodio](https://github.com/RustAudio/rodio) — audio playback
- [ratatui](https://github.com/ratatui/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm) — TUI
- [stratum-dsp](https://github.com/jamsocket/stratum) — BPM detection
- [lofty](https://github.com/Serial-ATA/lofty-rs) — tag read/write
