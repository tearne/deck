# Event Log

**Mode:** Formal

*(Spun out during message-log-file planning. Best taken after it lands.)*

## Intent

Messages today are notices, warnings and errors — things that interrupt. Expand the remit to routine events, such as which track was loaded on which deck, so the log reads as a session narrative rather than only its problems. Needs a design decision on visibility: routine events probably belong in the log and history view without occupying the global bar on every occurrence.

Rider agreed during map catch-up: rename the recorded kind from "message" to "event" in code (`Message` type, `messages` module naming) to match the map's prompts/events/hints vocabulary.
