bead_id: vb-zrop
phase: 7

# Test Plan

Behavior: verify-standard rejects ignored fallible results and accepts explicit handling.
Given: the repository source contains the scoped violations from baseline.
When: holzman-rust repairs the call sites without changing scanner policy.
Then: `bash scripts/check-ignored-fallible-results.sh` exits 0 and `moon run :verify-standard` exits 0.

No new product behavior tests are required because no public runtime behavior or API changes are intended. Existing tests compile/run through the canonical verification lane.
