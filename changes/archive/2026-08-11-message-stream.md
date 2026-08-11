# Message Stream

**Mode:** Explore

*(Split from the original message-log change, along with [message-log-file](message-log-file.md) and [message-display](message-display.md). This is the foundation the other two build on.)*

## Intent

Passive messages are created at ~30 sites, each choosing its display surface ad hoc, and every surface is a single slot — a newer message overwrites the current one, and expiry erases it. Introduce a single internal message stream: one structured message type that every site emits into, backed by an in-memory log so no message is ever lost. All messages then display in one place — the global bar — and the other surfaces give up their passive-message roles (their prompts stay). One small visible change: messages appear in one consistent place instead of three. This is the plumbing the log-file and display changes build on.

## Approach

### Current surfaces (inventory)

| Surface | Carries | Behaviour |
|---|---|---|
| Global bar (top row) | Browser, file-op, playlist, tag-editor outcomes; startup notes; warp refusal | One slot, 5 s countdown, Esc dismisses; idle it shows directory + version; also hosts the quit and load-into-playing-deck confirmations |
| Deck notification row ×3 | Per-deck warnings: unplayable playlist entries, unhashable identity | One slot each, 5 s; the row doubles as track title + `[BPM][Tick][Cue]`; also hosts the BPM confirmation and the rename offer |
| Browser header alert | Identity-mismatch critical alert | Takes over the browser header; falls back to the global bar when the browser is closed |
| On disk | Error reports (identity-mismatch, identity-unhashable), panic log | Chronologically named files/folders; no general message log |

### Prompts are not messages

The traffic splits in two. **Prompts** — BPM confirmation, rename offer, quit confirmation, load confirmation — await a keypress and stay attached to the thing they ask about; proximity is the point, and they stay out of the stream. **Messages** — everything passive — are what the stream carries.

### Message structure

A message is timestamp + severity + source context + text, where source context names the deck, playlist, or subsystem it concerns. Reason: in a unified stream, position no longer carries the context. Message text is kept as concise as possible — a display surface truncates rather than wraps, so anything past the terminal width is lost.

### The stream and its log

One sink every creation site emits into; messages append to an in-memory log, and the global bar renders the log's latest entry. Losslessness moves from the surfaces (where it's impossible — single slots) into the log. Routing messages back to their current surfaces was rejected: it would rebuild per-surface addressing that message-display removes, and source context already names what a message concerns.

The deck rows revert to pure track-title rows plus their prompts. The browser header alert retires — the global bar stays visible above an open browser, and per-message display durations are preserved, so the identity-mismatch alert keeps its 30 s lifetime against the usual 5.

### Map

No map node covers messaging — a coverage gap. A Messages node is proposed as post-build catch-up, negotiated per-node per MAP-GUIDANCE.

## Plan

**Topics**

- The message type (timestamp, severity, source context, text, display duration) and the single sink every site emits into, backed by the in-memory log.

- The global bar as the stream's renderer: shows the log's latest entry with the message's own duration; idle display, Esc-dismiss, and the prompts it hosts unchanged.

- Migration of every creation site to the sink — global-bar sites, the per-deck warnings (unplayable entries, identity unavailable), and the browser identity alert — tightening message text and adding source context as each moves.

- Retirement of the per-surface slots: deck rows revert to track title plus prompts, the browser header alert goes, and the global bar truncates rather than wraps.

**Done when** every passive message reaches the global bar through the stream with its source context, the in-memory log holds all of them in order, no other surface carries passive messages, and every prompt behaves exactly as before.

## Log

- The load-into-playing-deck prompt was displayed by writing into the global notification slot; it now renders directly from its pending-confirm state, so it stays out of the stream — and no longer vanishes after 5 s while the confirmation is still pending.

- Deck-sourced messages render with a `Deck N:` prefix on the bar; other sources render bare, with the source recorded in the message for the log-file and display changes.

- The deck row's `notif`-named identifiers were renamed to `title` — the row is a title row now, and the old names would have misled.

- The unhashable-identity warning moves from the deck row to the bar via `service_deck_frame`, which now takes the stream as a parameter.

## Conclusion

Completed at v0.16.1; the Log carries the deviations. Map catch-up deferred at the user's direction — a Messages node remains to be negotiated later. The open message-log-file and message-display changes build on this stream.
