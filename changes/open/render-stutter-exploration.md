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
