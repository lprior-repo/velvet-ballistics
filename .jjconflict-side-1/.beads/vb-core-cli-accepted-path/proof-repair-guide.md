# Proof Repair Guide: vb-core-cli-accepted-path

## State 6 Proof Review Retry After State 5 PO-007 Partial Pass

**Current state:** State 6 proof-review, attempt 7-of-7 (final), REJECTED.
**Blocking defect:** 1 LETHAL (LETHAL-2) remains open; 1 HIGH remains open.

### What Was Resolved

- **LETHAL-1 (digest mismatch):** RESOLVED. State 10 added `ArtifactDigestMismatch` error and digest equality check in `admit_artifact_run`. Fresh Kani re-run: 0 of 611 failed, VERIFICATION:- SUCCESSFUL.

### What Remains Open

- **LETHAL-2 (strict presence-only bypass):** OPEN. `strict_legacy_presence_only_bypass_rejects_required_blocker` fails at `crates/vb_runtime/src/kani_capability_harnesses.rs:217`. The harness tests `admit_run` which uses presence-only `compiled_ir_exists()` check with `AlwaysPresentArtifactStore::shared()`. **State 10 only fixed `admit_artifact_run`, NOT `admit_run`** - these are separate code paths. Fresh re-run: 1 of 120 failed, VERIFICATION:- FAILED.

- **HIGH (aggregate gauntlet partial):** OPEN. `moon run :verify-proof` exit 0 does not include the digest-mismatch or strict-bypass blocker harnesses. These pass/fail separately.

### Exact Required Route

1. **State 10 (implementation owner):** Fix `admit_run` to reject strict/journaled bypass via presence-only `compiled_ir_exists()` check. Options:
   - Gate `admit_run` for strict/journaled policies to require storage-backed artifact loading
   - Remove or wall off the `AlwaysPresentArtifactStore::shared()` path for strict/journaled shards
   - Or: obtain explicit reviewer-approved PO-007 waiver for the `admit_run` bypass path with compensating evidence (TLA-ACCEPT-001, VERUS-DIGEST-001, VERUS-POLICY-001, VERUS-ADMISSION-001, FUZZ-ARTIFACT-001)

2. **State 5 (proof-writer):** After production fix, rerun `strict_legacy_presence_only_bypass_rejects_required_blocker` to confirm PASS. Update gauntlet to include blocker harnesses or record focused PASS as compensating waiver evidence.

3. **State 6 (proof-review):** Retry after State 5 fresh PASS evidence.

### What Not To Touch

Do not repair verification artifacts (TLA models, Verus models, Kani harnesses) — they correctly encode required behavior. The defect is in production code (`admit_run` bypass path).
