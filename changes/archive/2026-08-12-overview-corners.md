# Overview Corners

**Mode:** Formal

*(Follow-on from [deck-row-condense](2026-08-12-deck-row-condense.md), refining its overlay approach.)*

## Intent

The overview waveform becomes the deck's whole status surface, its four corners anchoring the readouts: top-left the title (as now), top-right the spectrum analyser, bottom-left transport and BPM, bottom-right the prompts. The info row is then deleted — its remaining content (mixer meters, PFL, nudge indicator) folds into the corners — freeing another screen line per deck. Experimental by intent: build it and see how it feels.

## Approach

### Corner assignments

- **Top-left** — deck number, badge, title (unchanged).
- **Top-right** — bar legend, then the spectrum analyser. The filter display travels with the spectrum (its shading is the filter readout).
- **Bottom-left** — the info row's transport group verbatim: play state, BPM/percentage, pitch, offset, metronome, tap, analysing spinner.
- **Bottom-right** — mixer meters (level, gain, PFL, nudge). A countdown prompt (BPM confirmation, rename offer's active phase) displaces them for its seconds-long life; the rename offer's lingering phase compresses to a lone amber `⚠` beside the meters, so meters stay visible in every steady state.

### Info row deleted

Fixed layout lines drop 7 → 4; the spacer indices shift again. Prompts stop overriding the info row (last change's mechanism) and live bottom-right instead.

### Overview height

OV_MAX rises 3 → 4: the overview is now the deck's status surface, and the extra row keeps waveform clear between the corner rows when space allows. At the 2-row minimum all four corners still render, waveform visible only between them — accepted for the experiment.

### Truncation

Where corners would collide, left content truncates to keep right content whole — titles are the variable-width part, the right groups are fixed-width.

## Approach — second iteration (after first hand-back)

Corner text on both waveform rows obscured the overview's shape. Everything steady-state consolidates onto the **top** row; the bottom row stays waveform except transient content.

- **Top-left** — deck number, play state, badge, title; a lingering rename offer's amber `⚠` sits after the title it concerns.
- **Top-right** — one `│`-separated readout line: BPM/percentage (pitch, metronome inline), offset │ level, gain, PFL │ bar legend, spectrum compacted 16 → 11 chars (22 bins), slope field. Filter shading maps its 16 steps onto the 11 chars, slightly coarser. Pan joins this segment when the pan change builds (noted there).
- **Bottom-left** — transient only: tap counter while tapping, nudge arrows while nudging.
- **Bottom-right** — transient only: the countdown prompts.

## Plan

- [x] Corner line builders — transport group (bottom-left), meters + lingering `⚠` (bottom-right), countdown-prompt line, legend + spectrum (top-right)
- [x] Remove the legend from the overview's braille content — it lives in the top-right overlay now
- [x] Delete the info rows from the layout; OV_MAX 3 → 4; spacer indices shift
- [x] Render the four corners per deck; countdown prompts displace the meters
- [x] Empty/loading decks — title corner only, others blank

Second iteration:

- [x] Spectrum compacted to 11 chars / 22 bins; filter shading rescaled
- [x] Top-right readout line — tempo group │ meters │ legend + spectrum
- [x] Play state and lingering `⚠` join the title corner
- [x] Bottom corners transient-only — tap/nudge bottom-left, countdown prompts bottom-right

Third iteration:

- [x] The **analyser** (named zone: bar legend, spectrum, filter shading, slope) moves to bottom-right; countdown prompts displace it
- [x] Spectrum restored to 16 chars / 32 bins
- [x] The entire readout (tempo │ meters │ analyser) anchors bottom-right as one line; top-right returns to pure waveform

## Log

- With the info rows gone, adjacent decks' overviews touch — each deck's title corner is now the visual divider between strips.

- The accent strip simplifies to two segments: detail waveform and overview.

- The legend keeps its vinyl-mode suppression for free: it renders only when the overview isn't in its analysing state, same flag as before.

- Spacer index shifted again (c[13] → c[10]).

Second iteration:

- The filter's 16 steps map to the 11 spectrum chars by ceiling division, so the first step always shades at least one character.

- The spectrum's bins fell 32 → 22 with the same 20 Hz–20 kHz log spacing — each character still covers two bins, they're just wider.

- The analysing spinner replaces only the tempo segment of the readout line; meters and spectrum stay live during analysis.

- Pan's future home in the meters segment is noted in the pan change.

Third iteration:

- The bottom row is no longer clear in steady state — the whole readout occupies bottom-right. The second iteration's "bottom stays waveform" principle traded away: title and readout each own a row edge, so nothing collides with the stats on narrow terminals — waveform pays instead.

- First cut misplaced only the analyser bottom-right (stats still top-right); corrected on the user's clarification to the whole readout.

- The spectrum-width parameterisation earned its keep: restoring 16 chars was one constant.

- `overlay_top_right` became unused and was removed.

- Whitespace compaction pass on the readout: bare `│` separators, `lvl:`, `Nbr` legend, no leading space on the analyser bracket, slope field only while the filter is active (the line's tail width varies by 2 chars with filter state — right-anchored, so the line just extends leftward).

## Conclusion

Completed at v0.20.6; minor bump confirmed. The experiment ran four visual iterations (the Log records each); the settled layout is title top-left, the full readout (tempo│meters│analyser) bottom-right, transients only at bottom-left, waveform everywhere else. Deck-area map catch-up — deferred here from deck-row-condense — is now due: Deck, Overview Waveform, Spectrum Analyser, Renaming, and the Messages node's surfaces sentence.
