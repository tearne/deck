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

Four input layers: plain keys, **Shift**, **Space chords**, and **Alt chords**. Columns 1–4 control the selected deck, columns 5–6 hold mode and shared controls, and columns 7–9 address the three decks’ mixers directly. Alt provides deck selection and browser pane navigation across the keyboard.

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

Press `?` or `h` with deck focus for the overlay. In the browser, `h` still navigates to the parent directory and `?` opens Help in command mode. The shifted number-row symbols follow a UK keyboard layout.

Full action reference and guidance for updating an existing config: [keybindings.md](keybindings.md).

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
