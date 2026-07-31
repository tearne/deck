# Browser Tag Compliance

**Mode:** Formal

## Intent

A browser workflow for cleaning up badly-named tracks: flag files whose filename doesn't match their tags (non-compliant with the `Title - Artist` policy), highlight them in the listing, and let the operator fix them one after another. Sit in a folder and the loop is: scan flags the offenders → jump to the next → edit it → its marker clears → jump to the next.

Compliance is compared against tags, which means opening and probing each file (via `propose_rename_stem`), so the check runs as a **background scan** — streaming results into the listing as they arrive, cancelling and restarting when the operator navigates, and caching so a revisit is instant. Non-compliant entries get a marker (and a count); a command jumps to the next/previous flagged entry, and an edit re-checks the file so a fix clears its flag.

Scope: the current directory's listing (a bounded set), not a whole-workspace scan.

Depends on [[browser-file-operations]] — fixing a flagged file in place is the in-browser edit operation that change introduces.
