# Process feedback

## 2026-08-11 — Leading with consequence rather than mechanism

Context: planning `resolution-screening`. The agent explained a defect and a proposed reversal twice — first in implementation terms, then, on request, in end-user terms. The second version was substantially clearer to the user, who asked for the difference to be captured as reusable guidance.

The two versions contained the same decisions. What changed was the ordering and the vocabulary:

- **Subject of the sentence.** First version made code the actor: "`recompute_status` probes the library before resolving any entry". Second made the product or the user the actor: "opening a playlist makes Deck walk your whole music library for nothing".

- **Consequence before mechanism.** First version described what the code does and left the user to derive why it matters. Second stated the cost — everyday actions got slower — and offered the mechanism as support.

- **Identifiers replaced by their purpose.** `within_tolerance`, `LibrarySnapshot::probe`, `candidates()` became "the check", "looking through your library". The names carried no information the user needed to decide.

- **Trade-offs made concrete.** "Size is not invariant under retagging" became "adding cover art changes a file's size but not how long it plays". The abstract property was true but the concrete instance is what makes the decision obvious.

- **Judgement stated plainly.** "I think the Intent has it backwards" rather than a hedge assembled from technical qualifiers.

- **Detail offered, not omitted.** The second version ended by offering the code-level reasoning. Plain language replaced the *lead*, not the availability of depth.

What the second version did **not** do: simplify the decision, hide the defect, or soften the fact that the agent had introduced it. Plainness is about which vocabulary leads, not about saying less.

Possible general guidance for agents: *state what the user would experience before what the code does; use identifiers only after the consequence is established, or when asked. If a design property is abstract, give the everyday action that demonstrates it.*

A related failure surfaced in the same exchange and may be worth guidance of its own: the agent wrote a test asserting the fix it had built (the library is screened once) rather than the requirement (the library is screened only when something needs finding). Tests written from the implementation confirm the implementation.
