# Active Deck Indication

**Mode:** Formal

## Intent

The selected deck is marked only by a tiny yellow highlight on its number — too easy to lose. Make the active deck unmistakable at a glance, likely via its header, without disturbing the existing colour scheme.


## Approach

### Left accent bar down the active deck's regions

A vertical bar on the left edge marks the active deck. Because the layout groups the three detail waveforms at the top and each deck's header/info/overview strip at the bottom, the bar appears as two disjoint segments per deck: one against its detail waveform, one against its header strip. Disconnected but unambiguous, and it ties the big waveform to its controls.

### Reserve a one-column gutter rather than overlay

A single-column gutter is split off the left of the whole UI before the existing vertical row split, and the bar is drawn in it. Content shifts right by one column; the detail area narrows, and the waveform buffer width already derives from that area so it adapts on its own. Reserving space avoids drawing over the deck number or the waveform's leftmost column, at the cost of one column of width — a deliberate minimum since waveform columns are time resolution.

### Reuse the selection yellow

The bar uses the same yellow that already marks the selected number, so no new colour enters the scheme. The number stays yellow too — the bar reinforces it rather than replacing it.


## Plan

- [x] Reserve a one-column left gutter (horizontal split) and route every row through the content area.
- [x] Draw the accent bar in the gutter against the active deck's detail-waveform row and its notif/info/overview rows.
- [x] Bump Cargo patch (0.11.19 → 0.11.20).


## Conclusion

Completed at 0.11.20. The active deck is marked by a yellow `┃` in a one-column left gutter, in two segments — beside its detail waveform and beside its header/info/overview strip — following the selection. The whole UI shifted right one column; the waveform buffer width derives from the (now one-narrower) detail area, so it adapted without extra work. Map catch-up candidate: the Deck Selection node could note how selection is shown.
