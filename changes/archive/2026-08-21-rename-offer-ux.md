# Rename Offer UX

**Mode:** Formal

## Intent

Two rough edges on the load-time rename offer. Pressing Esc doesn't dismiss it — Esc falls through to its normal job, which is quit, so trying to wave the offer away can exit the application. And accepting the offer opens a centred modal tag editor, while the same editing via the browser uses the RHS panel — inconsistent presentations of one tool.

## Approach

- Esc while an offer is live (countdown or lingering `⚠`) dismisses it and is consumed — Esc's step-up-one-level convention.
- One editor presentation for both routes: the RHS panel position (right 30% of the spacer, over the art when the browser is closed), carrying the modal's blue theme — the look worth keeping — into the panel geometry. The centred-modal renderer retires.

## Plan

- [x] Esc dismisses the offer
- [x] Offer-path editor renders in the RHS panel position
- [x] Panel editor restyled with the modal's blue theme; modal renderer removed

## Log

- The modal's body was adopted wholesale into the panel renderer (dividers, field carets, filename preview, hint line), so both routes are now one function; the modal, its popup positioning helper, and the width constants are gone.

- Restoring a neighbouring private function sliced out during surgery cost one round-trip through git history.

- Second round, per the user's fuller reading of consistency: accepting the offer now opens the whole browser screen — browser at the file's directory with compliance markers on and the track highlighted, editor in the panel, browser dimmed while the editor owns input. Saving resumes the compliance cleanup flow among the file's neighbours. The bare-RHS render path from round one remains only as a fallback if the browser can't open (unreadable directory).

## Conclusion

Completed at v0.28.2; patch bump confirmed. Consistency landed one step beyond the filed Intent: rather than restyling the offer's editor in place, the offer now opens the browser route's whole screen (user-directed, round two). The editor has one renderer, one theme; the modal is gone. Map: Renaming and Metadata Editor updated at wrap-up.
