# Fallback Ranking

**Mode:** Formal

## Intent

*(Proposed by [[resolution-complexity-review]].)*

When a track has been re-encoded its audio is entirely new, so the identity stored in the playlist can never match it again. Deck's recovery is to offer similar library files to confirm.

Two things stop that working. A file is only offered when artist, title, album or year matches exactly, and re-encoding usually comes with retagging — so nothing is offered. Even when tags survive, matching is literal enough that *The Beatles* and *Beatles* are strangers.

Offer the plausible files, ranked by how alike they look, and let the operator decide.

## Approach

### Token-set overlap, not edit distance

Compare fields as sets of normalised tokens, scoring the proportion shared. The real failures are extra, missing and reordered words — `feat.` suffixes, track-number prefixes — not the typos edit distance buys.

### Normalise before comparing

Lowercase, strip punctuation, collapse whitespace, drop a leading "the". This removes the differences that carry no meaning.

### Title and artist score, album lightly, year not at all

Title and artist identify the track. Album is release-level, so it counts but weakly. Year is excluded: every track of the same year matches it perfectly, diluting the fields that actually discriminate.

### Duration admits, similarity orders

The ±2 s window stays the only admission rule; similarity ranks what passes, with duration delta as the tiebreak. A candidate scoring zero is still offered — that is the point of the change.

### Ten offers

Cap after ranking. If neither signal puts the right file in the top ten, a longer list will not help.

## Plan

- [x] Normalise description fields for comparison — lowercase, strip punctuation, collapse whitespace, drop a leading "the"
- [x] Score a field as the proportion of tokens shared
- [x] Combine field scores with title and artist weighted above album, excluding year
- [x] Offer candidates that score zero, ranked below every scored candidate
- [x] Cap the offered list at ten after ranking
- [x] Cover the retagged re-encode and the near-miss artist forms in tests

## Conclusion

Normalisation gained accent folding beyond the approved list; without it a routine tagging difference scored zero, reproducing the bug being fixed.

No documentation impact: the format spec leaves ranking implementation-defined, and Deck's own map has no playlist node to catch up.

Ranking is proven only against constructed cases. Whether ten offers and these weights suit a real library is untested.

## Log

- Normalisation also folds common accented characters to their ASCII base, which the plan's list did not name. Without it `Beyoncé` and `Beyonce` tokenise differently and score zero — a routine difference in music tags, not an edge case.
- Field scores are Jaccard overlap (shared ÷ combined), so extra tokens cost something rather than nothing: `01 - Come Together` against `Come Together` scores 0.67, well above unrelated text but below an exact match.
- Weights are 0.4 title, 0.4 artist, 0.2 album.
- An empty field on either side scores zero rather than matching another empty one, so two untagged files don't rank as similar to each other.
- Version 0.15.23 → 0.15.24.
