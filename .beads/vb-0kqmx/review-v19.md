# Independent Adversarial Test Review: vb-h7rex (v19)

**Reviewer:** LOCAL3090 (independent agent)
**Workspace:** /home/lewis/src/isoloated/vb-policy-tests-v19
**Frozen commit:** 6e958d14ecc4c0900fa319b8182f8f11faf2bfc3
**Baseline:** main 58a1f9580ca8beb32f80118ce08df521688a249a
**Scope:** Static review of storage/admission policy pin tests only. No Runtime dispatch acceptance.

## Status: APPROVED (one noted gap, non-blocking)

## Durable Recovery Tests (5 public)

**File:** `crates/vb_runtime/tests/policy_pin_durable_recovery.rs`

### Positive Control (strict + journaled)
Both `strict_admission_policy_pin_survives_durable_recovery` and `journaled_admission_policy_pin_survives_durable_recovery` delegate to shared scenario `policy_pin_survives_recovery_scenario`. Scenario:
- Computes real `compute_policy_digest` from workflow
- Submits artifact with contracts
- Produces real workflow failure + checkpoint via actual Runtime
- Verifies checkpoint admission contains `RunPolicyPin::Pinned(policy_digest)` matching computed digest
- Verifies artifact digest differs from policy pin (distinct identities)
- Drops runtime AND journal handles → forces cold reopen
- Reopens journal, verifies pin survives in both RunAdmission event AND RetryCheckpoint
- Recovers via `recover_retry` with original checkpoint
- Verifies tick succeeds, retry ticket has correct step/attempt/run_attempt (attempt 1, run_attempt 2)
- Completes retried action, verifies run completes
- Post-recovery: new checkpoint must still contain same policy pin
- Final: original RunAdmission event (seq 1) still present in journal

**Assessment:** Strong positive test. Real runtime, real journal, real failure boundary, cold reopen, full lifecycle.

### Legacy Version Rejection
`legacy_version_1_checkpoint_is_explicitly_rejected` constructs v1 header `[1, 0, 0, 0, 0]` and verifies `decode_checkpoint_payload` returns `RunCheckpointV1Error::UnsupportedVersion`. Not a historical migration test — explicit version boundary.

### Tampered Pin Rejection
`tampered_policy_pin_rejected_by_recover_retry`:
- Produces real durable failure + checkpoint with correct pin (Scope A)
- Uses test-local `ForgedRunAdmission` DTO to construct hostile admission with forged pin `[0xFF; 32]` (Scope B)
- Forged checkpoint attached to clone of real checkpoint
- Records journal state before recovery
- Calls `recover_retry` with forged checkpoint → expects `RuntimeError::Recovery` containing "retry checkpoint does not match durable RetryCheckpoint evidence"
- Verifies journal NOT mutated (event count unchanged)
- Verifies original checkpoint still has correct pin

**Assessment:** Proper hostile checkpoint test. Uses same DTO path as mismatch test for consistency.

### Admission Policy Pin Mismatch (vb-h7rex core)
`admission_policy_pin_mismatch_rejected_by_recover_retry`:
- Produces real durable failure + checkpoint with correct pin (Scope A)
- **Positive control:** Reconstructs admission with original pin via `forge_admission_with_pin`, verifies FULL-Eq with original — proves serialization path is faithful before testing mismatch
- Forges checkpoint with wrong pin `[0xEE; 32]`, incremented run_attempt, new RunFailed event, updated prior_seq (Scope B)
- **Persists forged checkpoint as durable checkpoint** (Scope C) — critical: checkpoint validation validates value equality, so forged checkpoint must BE the durable checkpoint
- Drops all journal handles, reopens to force durability
- Verifies forged checkpoint is now the durable one
- Calls `recover_retry` → enqueues command
- **Tick processing** must reject with `RuntimeError::RecoveryCannotResume { reason: "admission_policy_pin_mismatch" }` — exact error type and reason validated
- Verifies journal NOT mutated (full vec equality of events before/after)
- Verifies original journal prefix complete and unchanged

**Assessment:** This is the core vb-h7rex test. Correctly handles the subtlety that checkpoint validation validates value equality, so the shard must additionally validate the admission policy pin. The positive control (same DTO, original pin → FULL-Eq) is a strong correctness guarantee.

## Binary Wire Contract Tests (6 tests)

**File:** `crates/vb_storage/tests/policy_pin_wire_contract.rs`

All four admission variants tested with real non-empty capabilities and distinct digests:

| Test | Tag | Binding | Pin | Policy |
|------|-----|---------|-----|--------|
| legacy_unbound_unverified_roundtrips_as_tag_24 | 24 | LegacyUnbound | Unverified | Relaxed |
| digest_bound_unverified_roundtrips_as_tag_36 | 36 | Digest | Unverified | Journaled |
| unbound_pinned_roundtrips_as_tag_39 | 39 | LegacyUnbound | Pinned | Strict |
| bound_pinned_roundtrips_as_tag_41 | 41 | Digest | Pinned | Strict |

- `roundtrip_event` helper: encodes via `postcard::to_allocvec`, verifies tag byte, encodes full record, decodes, verifies envelope record_kind matches, verifies FULL-Eq of original vs decoded
- Pinned variants additionally verify decoded event retains pinned policy identity via match assertion
- `all_admission_record_kinds_have_stable_ids` asserts stable tag IDs (24, 36, 39, 41)
- `wire_envelope_kind_matches_event_record_kind` verifies tag byte matches record_kind id

**Assessment:** Complete wire contract coverage. Uses distinct run/digest/capability fixtures. Not just roundtrip — verifies tag, envelope, and identity retention.

## Checkpoint Version Cases

**File:** `crates/vb_runtime/src/run_checkpoint_tests.rs`

- `versioned_decoder_rejects_version_1_header_before_layout_decode` — v1 header rejected with `UnsupportedVersion`
- `versioned_decoder_rejects_unrecognized_future_version` — v99 rejected with `UnsupportedVersion`
- `versioned_decoder_rejects_truncated_valid_checkpoint` — truncates valid v2 checkpoint, expects `DecodeError`
- `versioned_decoder_rejects_malformed_payload` — v2 header + garbage body → `DecodeError`
- `versioned_decoder_rejects_empty_payload` — empty → `UnsupportedVersion`
- `versioned_decoder_rejects_truncated_varint_version` — `[255]` → `UnsupportedVersion`

Full pin capture/restore tested in `policy_pin_survives_checkpoint_capture_restore_roundtrip`.

## Legacy Fixture Verification

Wire contract tests label fixtures explicitly:
- `admission_event_unbound_unverified()` — "Legacy unbound + unverified (tag 24)"
- `admission_event_bound_unverified()` — "Digest-bound + unverified (tag 36)"

The tag IDs (24, 36) are asserted stable in `all_admission_record_kinds_have_stable_ids`. These are genuinely legacy variants — tag 24 predates the policy pin feature.

## CLI Submit Path

**File:** `crates/vb_storage/src/inputs/submission.rs:128`

```rust
policy_pin: crate::RunPolicyPin::Pinned(self.artifact.policy_digest),
```

Production `RunSubmission::admission_event` sets `RunPolicyPin::Pinned(artifact.policy_digest)`. No fallback to artifact identity.

**File:** `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:172`

```rust
.with_policy_pin(RunPolicyPin::Pinned(artifact.policy_digest))
```

Production admission path also sets pinned policy pin correctly.

## Shared Test Namespace (crates/vb_storage/src/tests.rs)

`journal_event_variants_for_run_and_seq` (line 2994) constructs 25 event variants including one `RunAdmission`:

```rust
JournalEvent::RunAdmission {
    input_digest: crate::RunInputBinding::LegacyUnbound,
    run,
    seq,
    artifact_digest: test_digest(2),
    granted_capabilities: CapabilitySet::empty(),
    policy: RuntimePolicy::Strict,
    policy_pin: crate::RunPolicyPin::Unverified,
}
```

**Finding (non-blocking):** Shared fixture only includes the legacy `RunPolicyPin::Unverified` variant. Does NOT include the new `RunPolicyPin::Pinned` variants (tags 39, 41) or the bound unverified variant (tag 36). However, the dedicated `policy_pin_wire_contract.rs` tests cover these variants thoroughly. No assertions were removed from the shared namespace.

## CLI event_to_json Test Coverage

**File:** `crates/velvet_ballistics/src/replay/event_to_json_tests.rs`

Two full JSON equality tests:
- `policy_pin_pinned_full_json_equality` — `RunPolicyPin::Pinned` with distinct policy digest `[0x03; 32]`
- `policy_pin_unverified_full_json_equality` — `RunPolicyPin::Unverified`

Both construct expected JSON by hand (not roundtrip) and compare field-by-field. Use distinct identity digests (artifact `[0x01; 32]`, input `[0x02; 32]`, policy `[0x03; 32]`).

**Assessment:** Complete coverage of new exporter path.

## RED Evidence

**File:** `.beads/vb-h7rex/evidence/ROOT_RED_V16.json`
- Exit code: 101 (test failure)
- Test: `admission_policy_pin_mismatch_rejected_by_recover_retry`
- Duration: 5.507s
- Environment: proper RUSTFLAGS, CARGO_TARGET_DIR, nightly-2026-04-28

**File:** `.beads/vb-h7rex/evidence/ROOT_RED_V16.stdout`
```
test admission_policy_pin_mismatch_rejected_by_recover_retry ... FAILED
Error: "forged policy pin must be rejected during tick processing"
```

Behavior failure (not compile error). Correct test name. Evidence boundaries respected.

## Remaining Gaps (Non-Blocking)

1. **Shared fixture missing new variants:** `crates/vb_storage/src/tests.rs:2994` only includes legacy `RunPolicyPin::Unverified` in `journal_event_variants_for_run_and_seq`. New pinned variants (tags 39, 41) not in shared fixture. Dedicated wire contract tests cover these.

2. **No Runtime dispatch acceptance tests:** This is a storage/admission slice. Runtime dispatch tests (acceptance-level) are explicitly out of scope per brief.

## Conclusion

All five public durable recovery tests are correct and well-structured. All six binary wire contract tests properly cover all four admission variants. Checkpoint version cases are complete. Production submit paths correctly use `RunPolicyPin::Pinned`. CLI exporter tests cover both pinned and unverified variants. RED evidence shows correct behavior failure on unfixed implementation.

One noted gap (shared fixture missing new variants) is non-blocking given dedicated wire contract test coverage.

**Status: APPROVED**
