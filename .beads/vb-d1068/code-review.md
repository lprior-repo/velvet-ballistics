# vb-d1068 Code Review

Reviewer: LOCAL3090 (independent)
Beacon: vb-3v011
Commit range: be65d9191527..a36705032ef5
Date: 2026-10-21

## Scope

Four CLI internal files in `crates/velvet_ballistics/src/`:
- `run.rs` (24 lines changed)
- `run_compiled.rs` (7 lines changed)
- `run_compiled_runtime.rs` (5 lines changed)
- `submit.rs` (1 line changed — visibility only)

## Verdict: PASS

The implementation correctly delegates artifact submission from the legacy hand-built
`VerificationProof::new(zero, REQUIRED_GATE_COUNT, true)` envelope to the typed schema-v3
`vb_storage::submit_artifact` producer path. Policy threading is consistent across all
durability modes.

## Findings

### Correctness: Artifact Producer Path (run.rs)

`submit_cli_compiled_artifact` previously:
- Hand-built `AcceptedArtifact` with `digest: zero` then recomputed
- Set `verification = VerificationProof::new(zero, REQUIRED_GATE_COUNT, true)`
- This produced `ArtifactEvidence::Unverified` (schema-v1) with no evidence envelope
- Would fail `checked_evidence_schema` under any Strict/Journaled admission

`submit_cli_compiled_artifact` now:
- Delegates to `vb_storage::submit_artifact(journal, compiled, policy)`
- Which calls `prepare_compiled_artifact` → `prepare_with_profile` → `build_evidence`
- Produces `ArtifactEvidence::Verified(evidence)` with `schema_version == 3`
- Includes `admission_policy` binding, canonical digest verification, checksum validation

**Finding 1 (correctness, resolved):** Producer path now matches typed evidence contract.
No hand-built envelopes. No `VerificationProof::new` in producer path. ✓

### Correctness: Policy Threading (run.rs, run_compiled.rs, run_compiled_runtime.rs, submit.rs)

Policy derivation is now single-sourced through `submit::submission_policy`:

```
DurabilityMode::Strict   → RuntimePolicy::Strict
DurabilityMode::Journaled → RuntimePolicy::Journaled
DurabilityMode::None     → RuntimePolicy::Relaxed
```

**Finding 2 (correctness, resolved):** Previously `runtime_config_for_durability` mapped
`Journaled → Strict` (via `ShardConfig::default().policy`), causing artifacts prepared
under `Journaled` to be admitted under `Strict`. The mismatch would trigger
`verify_accepted_artifact_for_policy` → `ArtifactMalformed`. Now `Journaled → Journaled`
via the shared `submission_policy` helper. ✓

**Finding 3 (correctness):** `submit::submission_policy` visibility changed `fn` →
`pub(crate)`. No public API change; `velvet-ballistics` is not depended on by other crates. ✓

### Evidence Chain Integrity

The fix preserves all evidence chain invariants:
- `source_digest` = `compiled.digest()` (BLAKE3 of canonical source)
- `digest` = `accepted_artifact_digest(artifact)` (BLAKE3 of serialized envelope)
- `policy_digest` = `compute_policy_digest(compiled)`
- Evidence envelope carries `admission_policy` binding
- `checked_parts` recomputes canonical-IR digest
- `bind_artifact_digest` ensures envelope digest matches verification digest

No bypass of evidence/admission. No weakened assertions. No `allow`/`ignore`. ✓

### Risk Analysis

**Risk 1 (low):** `submit_artifact` returns `Result<AcceptedArtifact, JournalError>`.
Caller (`store_workflow_artifacts`) maps to `CliExitCode::StorageError`. Error propagation
is correct — admission failures surface as CLI exit codes, not silently swallowed.

**Risk 2 (low):** `runtime_config_for_durability` now uses `ShardConfig { policy: ...,
..default() }` struct update syntax. All other `ShardConfig` fields remain at default.
No behavioral change beyond policy.

**Risk 3 (low):** `store_compiled_artifact` now takes `policy` argument. All callers
pass `config.policy` derived from the same durability mode. No caller omission possible
(type system enforces).

## Verification

- Clippy strict: `cargo clippy -p velvet-ballistics -- -Dwarnings` → exit 0, 8.0s
- No unsafe/unwrap/expect/panic/todo/dbg in changed lines
- No unchecked indexing/slicing/casts/arithmetic
- No new dependencies

## Remaining Limitations

- Full Kani/Flux/Verus fleet not run (per instructions: targeted review only)
- Cross-crate integration tests (`accepted_artifact_red_phase`, `core_yaml`, etc.)
  not in scope; integration note confirms no API change affects them

## Conclusion

Production fix is correct, minimal, and consistent with typed evidence contract.
Policy threading eliminates Journaled→Strict admission mismatch. No evidence chain
bypass. Ready for test review.
