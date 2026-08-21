# Hue-Preserving Dim

**Mode:** Formal

## Intent

*(Captured during browser-split-resize review.)*

Dimming an area (browser behind the tag editor, browser behind a focused panel) overwrites every cell's foreground with one flat gray — a jarring loss of colour. Dim by scaling each cell's existing RGB toward dark instead, so the area keeps its basic shades at lower brightness. Cells with non-RGB colours (indexed/named) need a reasonable mapping or fallback.

## Approach

`dim_area` becomes a per-cell blend: fg and bg resolve to RGB (named ANSI via table, indexed via the xterm-256 formula) and blend 65% toward a dark blue-gray (45,45,55) — per the user, toward gray rather than black, so the dimmed area recedes without going lightless. `Reset` colours land on the target itself. The `DIM` modifier and flat-gray overwrite are gone; both call sites inherit the change.

## Plan

- [x] Per-cell hue-preserving blend, named/indexed resolution, both call sites
- [x] Symmetric focus dimming: the panel dims while it merely previews, so exactly one side is ever bright

## Log

- Foreground retention raised to 55% (backgrounds stay 35%) after the panel's pastel palette dimmed perceptually harder than the browser's saturated one — same maths, different starting chroma.

## Conclusion

Completed at v0.29.3; patch bump confirmed. Hand-back extended the change with symmetric dimming (panel fades when passive). One map touch at wrap-up: Context Panel's "the browser dims" sentence becomes two-way.
