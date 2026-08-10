# Fallback Duration Only

## Intent

*(Proposed by [[resolution-complexity-review]].)*

The descriptive fallback only offers a candidate if at least one description field matches exactly (case-insensitively). A track that was re-encoded *and* retagged — the ordinary "cleaned up my library" case — scores zero and is reported unavailable with nothing offered, even when a file of near-identical duration is sitting in the library.

Duration proximity is a strong signal on its own, and the operator confirms every re-link anyway. Offer candidates on duration alone, ranking by description similarity rather than gating on it.
