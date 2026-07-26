# Render Stutter Exploration

**Mode:** Explore

## Intent

After the waveform-render-efficiency change (v0.11.3), scrolling is improved but still shows occasional stutter on target hardware. Rather than guess, explore the remaining cost structure with measurement and attack what the numbers implicate. Candidate levers, roughly ranked:

- **Instrumentation first** — per-frame breakdown (draw vs terminal write vs sleep), bytes written per frame, background rebuild durations; establish whether the stutter is periodic (buffer rebuild), spiky (spectrum analyser), or diffuse (output volume / pacing).
- **Indexed colours** — emit 256-colour SGR (`38;5;N`, ~11 bytes) instead of truecolor (~19 bytes); map the spectral palette onto the 6×6×6 cube. A further colour reduction in the form terminals natively reward.
- **Coarser or block quantisation** — fewer spectral levels, or one colour per block of columns, guaranteeing long runs.
- **Buffered terminal writer** — wrap stdout in a `BufWriter` so each frame is one syscall.
- **Peak pyramid** — precompute min/max (and bass) at power-of-two block sizes at load, making wide-buffer rebuilds O(columns) instead of O(samples in window); removes the periodic multi-ms background scan that can starve the UI thread on 2-core boxes.
- **Spectrum stagger/off-thread** — the three decks' Goertzel passes currently run synchronously on the UI thread and can align on the same frame.
- **Frame pacing precision** — sleep-then-trim if measurement implicates sleep jitter in presentation.

Goal: measured understanding of where frame time and bytes go, and the stutter eliminated or bounded to causes we consciously accept.


## Approach

### Frame-stats recorder behind a CLI flag

Instrumentation is a per-frame stats recorder enabled by a `--frame-stats` CLI flag, writing one row per frame — service time, draw time, write/flush time, sleep, bytes emitted, and background rebuild events with durations — to a fixed filename in the working directory. A file capture beats an on-screen HUD because the stutter lives on target hardware: the user runs a capture there and we analyse the file offline. The existing frame loop already timestamps each frame, so the seams are natural.

### Counting writer under the terminal backend

Bytes per frame are measured by wrapping stdout in a counting writer beneath the ratatui backend. The same wrapper is where a `BufWriter` slots in, so the buffered-writer lever and its measurement are one experiment: A/B by construction.

### Measure first, then levers in implicated order

No lever is applied until a capture shows where the time goes. Cheap, visually invisible levers (buffered writer, spectrum stagger) may be applied within this change once implicated, each with a before/after capture. Reduced waveform colour depth (256-colour, coarser quantisation) is acceptable in principle if bytes dominate. The peak pyramid, if implicated, gets exploratory prodding here until we're confident in the diagnosis, then potentially spins off as its own change for the deep dive.

### Captures via the dev-build-run flow

Target-hardware captures use the existing container-build-and-pull flow; the stats flag is forwarded through the script's pass-through app arguments. No harness changes needed.


## Plan

**Topics**

- Frame-stats recorder: `--frame-stats` flag, per-frame rows to a fixed CWD filename, counting writer under the backend, rebuild durations from the renderer thread.

- Baseline capture on target hardware; characterise the stutter — periodic, spiky, or diffuse — and correlate stalls with rebuilds, spectrum passes, and bytes written.

- Buffered writer A/B via the counting-writer seam.

- Colour depth experiments (256-colour, coarser quantisation) if bytes dominate.

- Spectrum stagger or off-thread if stalls align with Goertzel passes.

- Rebuild-cost prodding (toward a peak pyramid) if rebuild durations are implicated; spin off when confident.

- Frame pacing (sleep-then-trim) if sleep jitter is implicated.

**Done when** every stall class visible in target-hardware captures is eliminated or attributed to a measured cause we consciously accept, with findings recorded in this document.


## Log

- Frame-stats recorder implemented (0.11.4): `--frame-stats` writes `frame-stats.csv` in the CWD, one row per frame — t, frame, service, spectrum, draw, write, bytes, budget, sleep, and per-slot rebuild µs/counts. Probes: metered stdout writer under the ratatui backend, spectrum timing at the Goertzel call, rebuild timing in the renderer thread. Smoke-tested via pty + null ALSA: rows and values sane.
- First capture (2026-07-26, 43 s, 5083 frames, 8.33 ms budget): app-side timing is clean. Frame spacing p50 8.41 ms, p99 8.51 ms, stdev 0.64 ms; exactly one hitch (52.8 ms at t=5.74 s, coinciding with a 131 kB full-screen draw + slot-0 rebuild — looks like a load/zoom event). Draw p50 2.1 ms, write p50 0.54 ms, spectrum ≤1 ms, rebuilds 20 in 43 s (max 5.4 ms, uncorrelated with slow frames). Output volume ~2.4 MB/s while scrolling. Stutter was visible during this run (Kitty, on target hardware) — so the suspicion moves downstream of the app: terminal emulator parse/raster load, or spatial artefacts.
- Experiment build 0.11.5: spectral LUT entries now map to nearest xterm-256 indexed colours (`38;5;N`, ~11 bytes vs truecolor's ~19; coarser palette also merges more spans). Frame rate needs no code — runtime `fps_increase`/`fps_decrease` actions (`^`/`Y`) already step the levels, so 120 vs 60 can be A/B'd live mid-capture.
- Experiment outcome: no major visible difference from indexed colour or 60 fps. Residual smoothness variation differs between the laptop's built-in display and an external USB-C monitor — pointing at the display path (monitor refresh handling / emulator raster), downstream of anything the app controls. Decision: stop here; keep indexed colour, make 60 fps the default.
- 0.11.6: default target_fps 120 → 60 (embedded config, DisplayConfig default, keybindings.md); help overlay footer gains `Y/^ fps` — the keys sit on the 6/Y column the overlay uses as its hand divider, so the footer legend is where they fit.


## Conclusion

Done-when met: frame timing measured clean on target hardware (p99 spacing within 2% of budget, one attributable hitch in 43 s), and the residual smoothness variation is attributed to the display path downstream of the app (built-in vs USB-C monitor difference) — accepted. Shipped at 0.11.6: indexed spectral colour, 60 fps default, `Y/^ fps` in the help overlay. The `--frame-stats` recorder stays in the binary for future diagnosis. Patch bumps 0.11.4→0.11.6 confirmed. Unexplored levers (buffered writer, block quantisation, peak pyramid, spectrum stagger) remain listed in the Intent should stutter return under different conditions. Map catch-up pending on Spectral Colour (indexed emission); keybindings.md already updated in-build.
