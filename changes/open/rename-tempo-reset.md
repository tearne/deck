# Rename and Bind tempo_reset

## Intent

`tempo_reset` resets playback speed to nominal (`bpm` → `base_bpm`, `vinyl_speed` → 1.0) but has no default key binding and its name implies BPM detection rather than speed. Rename the config action to `speed_reset` for accuracy, give it a default binding (Space+C is a candidate — currently unbound on the Space layer), and add it to the keyboard layout diagram (`SpRst` label).
