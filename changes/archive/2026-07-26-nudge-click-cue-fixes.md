# Nudge, Click, and Cue Fixes

**Mode:** Formal

## Intent

Three small transport/display defects, found in use:

1. **Warp nudge latches in terminals without key-release reporting.** Releasing the nudge key is what ends a warp, but release events only exist under the kitty keyboard protocol — in GNOME Terminal (VTE) the ±10% speed offset sticks on with no way to clear it. Make warp nudge safe in such terminals rather than removing the feature.

2. **Playing nudge-jumps click.** The `d`/`c` ±10 ms jumps bypass the click-free seek layers while playing — the position is stored raw and any in-flight fade is cancelled. Route playing nudge-jumps through the fade so they are as click-free as beat jumps.

3. **Cue mark hidden in vinyl mode.** The detail waveform deliberately suppresses the cue marker in vinyl mode (the overview already shows it). Show the cue mark on the detail waveform in vinyl mode too.

4. **Waveform freezes during base-BPM adjustment.** Aligning the beat grid needs the tick marks moving relative to the wave while the key is held, but the settle debounce on parameter-driven rebuilds means the waveform only updates after the hold ends. Provide live (if throttled) feedback during the ramp without reintroducing the rebuild storm the debounce was added to prevent.


## Approach

### Warp gated on key-release support

At startup (raw mode is already active) the terminal is queried for kitty keyboard-protocol support via crossterm's `supports_keyboard_enhancement`. Where unsupported, the nudge-mode toggle refuses to enter warp and shows a notification saying the terminal can't report key releases; warp activation is also guarded defensively. Preferred over a repeat-silence auto-release because key-repeat's initial delay would make releases lag ~500 ms — a held speed offset that overshoots its release is worse than not offering it.

### Fade-only seek for playing nudge-jumps

Playing nudge-jumps switch from the raw position store to the fade path. A new seek variant fades and flushes the pipeline but skips the quiet-frame search: the search window (±10 ms) is as large as the nudge step itself, so it could cancel or double a step, making repeated nudges uneven. Landing exactly on target also keeps the display bump consistent with the audio. The paused path is unchanged.

### Cue suppression removed

The detail renderer receives the cue sample regardless of vinyl mode. Tick marks stay suppressed in vinyl — only the cue changes.

### Settle debounce becomes throttle-with-trailing-edge

The renderer thread rebuilds a slot when its parameters have been stable for 50 ms (as today) **or** when they differ from the built buffer and the slot's last rebuild is more than ~100 ms old. During a held ramp this yields ~10 rebuilds/s — live tick motion at a bounded cost (measured rebuild cost is single-digit ms on a background thread) — and the trailing settle still delivers the final exact state. Drift, resize, zoom, and load stay immediate.


## Plan

- [x] Query key-release support at startup and thread the flag to the input loop.
- [x] Nudge-mode toggle refuses warp with a notification where releases are unavailable; warp activation arms guarded too.
- [x] Add a fade-only seek variant (fade + pipeline flush, no quiet-frame search) to the seek handle.
- [x] Playing nudge-jumps use the fade-only variant; paused path unchanged.
- [x] Stop suppressing the detail cue column in vinyl mode.
- [x] Renderer thread: track per-slot last-rebuild time; rebuild on 50 ms settle or >100 ms staleness with changed params.
- [x] Bump Cargo patch (0.11.6 → 0.11.7).


## Log

- Hand-back found the display losing sync and snapping during repeated playing nudges: the fade-out consumes FADE_SAMPLES before the jump applies, so each faded nudge moved the audio less than the display bump; key-repeat accumulated the discrepancy past the 0.1 s snap threshold. Fixed in 0.11.8 by aiming the seek target past the fade's consumption so perceived jump and display bump match exactly. Beat jump shares the (one-shot, negligible) error — left as is.
- 0.11.8 still overshot forward under key-repeat: presses arrive faster than the audio thread applies pending seeks, so each repeat recomputed its target from the not-yet-moved position — seeks collapsed into one jump while the display walked ahead, then snapped back. 0.11.9 replaces the exact-target seek with `seek_relative_faded`, which extends the pending target when a seek is still in flight and centralises the fade compensation; both nudge arms now just add the returned bump.


## Conclusion

Completed at 0.11.9 — patch confirmed, with two extra hand-back iterations for playing-nudge display sync (mechanisms in the Log). Map catch-up pending on three nodes: Wide Buffer (settle debounce now throttle-with-trailing-edge), Nudge (playing jumps now fade; warp unavailable without key-release reporting), Click-free Seek (nudge deliberately skips the quiet-frame search).
