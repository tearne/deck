# Error Capture Rethink

**Mode:** Formal

*(Spun out during message-log-file planning, superseding its error-report cross-reference question.)*

## Intent

Faults are currently preserved as dated files under `error_reports/` (identity-mismatch folders, identity-unhashable text files), with the panic log alongside. Now that a persistent message log exists, revisit the optimum way to capture errors: which faults still need standalone report files (preserved artefacts like original/edited file pairs plausibly do), which collapse into log lines, and how messages and reports should reference each other.
