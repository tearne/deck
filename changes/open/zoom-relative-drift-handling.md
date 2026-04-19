# Zoom-Relative Drift Handling

## Intent

The partial drift correction factor (0.002) and the drift-snap threshold (0.3 s) are currently absolute, applied against raw sample-count drift without reference to what the user can actually see. At any given zoom level, the visible impact of drift is a function of how many samples a single column represents — so a fixed absolute correction over-reacts at one zoom and under-reacts at another.

The user would like to reframe both drift mechanisms as **zoom-relative**: expressed as a fraction of the samples represented by a single column character. The hypothesis is that tying correction to what's actually visible will yield smoother scrolling across the full zoom range and make the knobs easier to reason about.

Supersedes `partial-drift-correction-factor.md`, whose absolute-factor experiment produced no visible difference across its candidate range.
