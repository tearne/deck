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
