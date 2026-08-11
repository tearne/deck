# Message Display

**Mode:** Formal

*(Split from the original message-log change. Builds on [message-stream](../archive/2026-08-11-message-stream.md) (landed, v0.16.1); independent of the log file.)*

## Intent

Messages appear in surfaces attached to different UI elements, and the eye is often elsewhere — a message can expire unseen, and there is no way to look back. Give messages a unified live display and an on-demand history view, so a missed message is a glance away rather than gone.

## Approach

### Live display stays on the global bar

Since message-stream landed, the bar already shows the stream's latest message and is visible in every state — browser open or closed, any terminal height. Making it *the* live surface (the user's instinct) means no second live surface over the art, and the art-area fallback question dissolves. The art area is used for history only.

### History overlay

A keyboard-toggled overlay over the album-art area, reusing the help overlay's mechanism (same space, same Clear-then-dark-box rendering). Mutually exclusive with the browser, like help. It opens showing the log's tail, newest at the bottom — one line per message: clock time, severity-coloured text, source-prefixed the same way the bar renders it — and scrolls with j/k through the full session history. Lines truncate to width; messages are concise by design. Esc or the toggle key closes it.

### A configurable action, default `N`

The toggle is a new config action alongside the existing help toggle, default `N` (Shift+n).

### No deck-row echoes

The echo idea (deck messages also flashing on their deck row) is dropped rather than built: it reintroduces the split attention this work exists to remove, and the deck-row-condense change will review that row's future anyway.

## Plan

- [x] Message log read access on the stream — ordered messages with time, severity, source, text
- [x] `message_history` config action, default `N`
- [x] History overlay renderer over the art area — newest at bottom, clock time, severity colours, source prefixes, width truncation
- [x] Key handling — toggle opens/closes, j/k scroll, Esc closes, unavailable while the browser is open
- [x] New action documented in keybindings.md

Added after first hand-back:

- [x] History lines wrap with a hanging indent; scrolling becomes line-based
- [x] `N` shown in the help overlay

## Log

- Clock time uses a `date +%z` query at startup for the zone offset — std exposes only UTC, and a timezone-database dependency wasn't warranted. Falls back to UTC display if the query fails.

- While the history is open only Esc, j/k, and the arrow keys are intercepted; every other key (transport, mixer) acts normally, matching the help overlay's behaviour.

- Help and history overlays are mutually exclusive — opening one closes the other.

- The header row shows scroll position as "(N older, M newer)" counts.

- Terminal-native copy needs no code: mouse capture swallows click-drag, but Shift+drag bypasses it — confirmed working, so selection/copy was dropped from scope.

- Wrapping reuses the candidate picker's `wrap_words`, upgraded to hard-split tokens longer than the line (paths); the picker gains the same fix. Scroll counts display lines, and the renderer returns its clamped value so overscroll can't accumulate.

- The history header naming the log file's location is deferred to message-log-file (no file exists yet); noted there.

## Conclusion

Completed at v0.17.2; minor bump confirmed. Two tasks were added after the first hand-back (wrapped history lines, `N` in the help overlay); selection/copy needed no code (terminal Shift+drag bypasses mouse capture). keybindings.md was updated alongside. The history header naming the log file's location rides with message-log-file, and the Messages map node remains outstanding from message-stream.
