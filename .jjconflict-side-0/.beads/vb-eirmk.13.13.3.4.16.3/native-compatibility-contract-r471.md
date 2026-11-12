# Native compatibility contract r471

Preserve the recovery-owned arena contract and all frozen candidates; fix the exact two strict-lib diagnostics from runtime-native-r471.log without suppressions, public API changes, checkpoint loss or altered timer policy.

The private RecoveredStateSource carrier is obsolete: its only remaining caller passes no retry checkpoint and always provides a restored store; real retry checkpoints are independently validated and restored in lifecycle/chunk_001/recover.rs. Remove the unused carrier and unreachable selector only, reusing the existing full-store validation calculation directly after unchanged root-bundle and retry/store incompatibility checks; preserve original typed errors, prefix checks and complete checkpoint bytes.

The private drive_one_ready_run helper has no callers; preserve the active tick_inner idle path through drive_one_due_timer and pending-run driving through the existing explicit Resume path, without introducing another scheduling path or changing timer authority. Remove only the obsolete helper after a complete caller search.

The legacy symbol-prefix fixture adds only source_metadata: None in its existing WorkflowParts literal, preserving all other source bytes and assertions. Resolve the three explicitly assigned legacy test files against both frozen parents, preserving every test identity and complete module registration while recording any authoritative current-semantics conflict rather than silently accepting contradictory old oracles. No NestedTogether guard or production behavior change is authorized for fixture compatibility.

The conflicted author workspace does not establish native acceptance. Provide immutable per-batch source hashes, exact diffs and format receipts; parent integration runs strict-lib and scoped native tests before independent CODE and TEST reviews.
