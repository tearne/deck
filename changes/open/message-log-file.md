# Message Log File

**Mode:** Formal

*(Split from the original message-log change. Builds on [message-stream](../archive/2026-08-11-message-stream.md), landed at v0.16.1.)*

## Intent

Past messages should survive the session. Append every message from the message stream to a log file in the state dir, alongside the panic log and error reports, so issues can be diagnosed after the fact — e.g. hash corruptions, whose reports currently sit in `error_reports/` with no surrounding narrative. Error-report writes emit a message naming the report, making the log the chronological index into `error_reports/`.

## Carried from the split

Settled with the user during the original change's planning:

- Human-readable lines, not JSONL.
- Everything logged regardless of severity, at least to start with.
- Rotation with a generous default retention, configurable in `config.toml`.
- The message-history overlay's header should name the log file's location (requested during message-display; deferred here because the file must exist first).
