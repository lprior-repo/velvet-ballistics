# Coordinator evidence applicability

The five reviewer manifest hashes were independently recomputed and match candidate3eb8b0c5. Actual recorded stdout contains13 marker tests passed; other listed integration binaries were filtered and do not count as behavior coverage. The reviewer used timeout+tee instead of the requested immutable gate runner; the pipeline exit is not independent command-exit proof. Root canonical run and immutable producer runs remain required.

Two factual report corrections: source asserts exact injected message `injected batch commit failure (test hook)`, not the shortened quoted message in the review; the unchanged-history assertion checks count1 and exact first event, which together imply complete vector identity for this one-event scenario, not a literal Vec equality assertion. Deterministic hook executes before batch commit; it is not physical fsync/power-failure evidence.

Scope is this storage marker only. No current Runtime dispatch, grant lifecycle or full-v1 approval follows. Source/test hashes remain the acceptance applicability boundary.
