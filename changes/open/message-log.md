# Message Log

**Mode:** Formal

## Intent

*(Parked — captured as an aside.)*

Notifications shown in the deck/browser notification areas are transient — they expire and are gone. Capture them as a persistent **message log**: every message (with its timestamp) that surfaces in a notification area is appended to a log **file**, and optionally viewable in-app as a **scrollback box** opened by a keyboard shortcut.

Rough shape to design later:

- A single sink that every notification passes through, recording text + time + severity.
- A log file (state dir alongside the panic log / error reports), append-only.
- An in-UI overlay/box with scrollback over recent messages, toggled by a key.
