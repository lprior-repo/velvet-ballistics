# Black Hat Code Review: v19 Candidate (vb-d4bde)

**Candidate:** 6e958d14ecc4c0900fa319b8182f8f11faf2bfc3
**Baseline:** main 58a1f9580ca8beb32f80118ce08df521688a249a
**Scope:** Admission policy pin retention, checkpoint v2, storage durable deny
**Reviewer:** LOCAL5090
**Date:** 2026-10-22

## Status: APPROVED

Zero BLOCKING findings. See findings below with severity classification.

---

## Policy Pin Retention (vb-h7rex)

### Architecture Assessment

The policy pin design is sound. `RunPolicyPin` is a sealed two-variant enum (`Unverified` | `Pinned(WorkflowDigest)`) that carries the actual policy identity from the accepted artifact. The distinction between "legacy/relaxed admission" (Unverified) and "verified Strict/Journaled admission" (Pinned) is explicit and typed, not implicit in absence of a field.

### Wire Contract Verification

**Byteorder and tag stability:** Tags derive from `RecordKind.id()` constants (24, 36, 39, 41), not hand-assigned magic numbers. The enum discriminants are `u8` variants with explicit `serde` serialization — stable by design.

**Serde arity preserved:**
- Tag 24 (`RunAdmission`): 5 fields — unchanged from prior layout
- Tag 36 (`RunAdmissionBound`): 6 fields — unchanged from prior layout
- Tag 39 (`RunAdmissionPinned`): new, 6 fields (adds policy digest)
- Tag 41 (`RunAdmissionBoundPinned`): new, 7 fields (adds policy digest)

Tag 38 (`PolicyDecision`) and tag 40 (`Blob`) are unaffected.

### Admission Constructor Behavior

The compatibility constructors (`new`, `with_idempotency_evidence`, `with_budget`) explicitly produce `RunPolicyPin::Unverified`. This is correct: these are used by callers without an accepted artifact's policy identity. The actual pin is set via `with_policy_pin(RunPolicyPin::Pinned(policy_digest))` after artifact validation.

**Risk observed:** The compatibility constructors default to `Unverified`. If a caller forgets to call `with_policy_pin` after strict/journaled admission, the runtime will silently record `Unverified` instead of the actual policy digest. This is mitigated by the journal emission path (`crates/vb_runtime/src/journal/chunk_002.rs:95`) which passes `admission.policy_pin()` directly — if the runtime sets the pin correctly, the journal is correct. The durable recovery tests (`policy_pin_durable_recovery.rs`) verify the full call path end-to-end.

### Checkpoint V2 Migration

**Version-aware decoding:** `decode_checkpoint_payload` reads the VARINT-encoded version field first using `postcard::Deserializer`, then dispatches. Version 1 is explicitly rejected; only version 2 is accepted. This prevents misdecoding of legacy checkpoints that lack the `policy_pin` field in `RunAdmission`.

**Checkpoint capture:** `RunCheckpointV1::capture` captures `state.admission` verbatim, including the `policy_pin` field. The checkpoint struct has `version: 2`.

**Roundtrip fidelity:** Postcard serialization/deserialization of `RunCheckpointV1` preserves the entire admission record including the policy pin. The runtime's `recover_retry` path validates the durable checkpoint value against the supplied checkpoint, then restores the full validated checkpoint.

### Recovery Validation Callpath

Full recovery (`validate_recovered_admission_evidence` in `crates/vb_runtime/src/recovery/full/admission.rs`) calls `validate_recovered_policy_pin` as part of the evidence chain. This validates:
- `Unverified` → accepted (legacy/historical record)
- `Pinned(pin)` → validated against `artifact.policy_digest`; mismatch produces `RuntimeError::RecoveryCannotResume { reason: "admission_policy_pin_mismatch" }`

**Retry checkpoint recovery** (`crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:401-408`) validates the checkpoint's admission policy pin against the loaded artifact *before* restoring run state. This catches `admission_policy_pin_mismatch` that the separate full recovery flow also validates.

**Cross-child integration:** The storage crate's `recover_run_admission_from_events` (in `derive.rs`) correctly propagates `policy_pin` through `RecoveredRunAdmission`. The runtime's `hydrate_run_admission_from_events` calls `with_policy_pin` to attach it to the reconstructed `RunAdmission`. This chain is verified by the durable recovery tests.

---

## Emergency Deny Storage (vb-ct31x)

### Design Assessment

The emergency deny overlay uses a dedicated Fjall keyspace (`emergency_deny`) with a single fixed key (`active`) and a versioned one-byte value (`[1]`). This is correct: constant marker, no counter, no sequence, no race condition.

### Parser Boundedness

`emergency_deny_state` performs exact byte-length checking:
- Length 0 → `EmergencyDenyMarkerMalformed`
- Length 1 with correct version byte → `Active`
- Length 1 with wrong version → `EmergencyDenyMarkerUnsupportedVersion`
- Length > 1 → `EmergencyDenyMarkerMalformed`

No unchecked indexing or slicing. The `first()` call is safe after the length check.

### Durability Semantics

`activate_emergency_deny` uses `batch.strict().commit()` which maps to `PersistMode::SyncAll`. This is durable: the marker is persisted to stable storage before the function returns. The function always performs a strict batch commit even if the marker is already active, because a prior failed persistence could have left visible but not durable data.

### Corruption Handling

The function reads and validates the existing marker before writing. If the current state is corrupt or has an unsupported version, it returns an error *without* overwriting the corrupt data. This prevents clobbering corruption with a fresh marker.

### No Release/Reset

The marker is monotonic: once active, it stays active. There is no `deactivate_emergency_deny` or reset path. This is correct for an emergency overlay: activation is a deliberate, permanent signal.

---

## Engineering Rules Compliance

**No forbidden patterns:** No `unsafe`, `panic!`, `unwrap()`, `expect()`, `todo!`, `unimplemented!`, `dbg!()`, or unchecked indexing/slicing/casts/arithmetic in new production code. All new files have `#![forbid(unsafe_code)]`.

**File size limits:** All production files are well under the 300 physical line limit. New functions are small and focused.

**New RecordKinds:** IDs 39 and 41 are stable constants, not magic numbers. They are distinct from existing kinds (38 = PolicyDecision, 40 = Blob). The codec tests verify kind family routing.

**Error codes:** New error variants (`EmergencyDenyMarkerUnsupportedVersion`, `EmergencyDenyMarkerMalformed`) have distinct diagnostic codes (0x4042, 0x4043) and typed names.

---

## Findings

### MINOR: Redundant Read in activate_emergency_deny

`crates/vb_storage/src/journal/emergency_deny.rs:80`: `activate_emergency_deny` reads the current state and discards it (`let _ = current;`) after validation. The read is necessary to detect corrupt/unsupported state, but the result is not used for any branching. This is correct but slightly confusing — a comment or explicit early return for the corrupt case would improve clarity. Not a correctness issue.

### MINOR: Unverified Default is Silent

The compatibility constructors default to `Unverified` with no compiler warning or explicit opt-in flag. A future caller constructing an admission with `RunAdmission::new(...)` for Strict/Journaled policy might forget to call `with_policy_pin`. The journal emission path propagates whatever pin is set, so the journal would contain `Unverified` instead of the actual policy digest. Mitigated by durable recovery tests and the fact that `with_policy_pin` is the documented way to set the pin after artifact validation.

### MINOR: No Runtime Policy Dispatch

The implementation correctly defers runtime policy dispatch acceptance to a later bead. The current implementation validates policy pins and persists them but does not implement runtime enforcement based on the overlay state. This matches the explicit scope statement.

### OBSERVATION: Test Coverage is Comprehensive

The test suite covers:
- Wire roundtrip for all 4 admission variants (tags 24, 36, 39, 41)
- Checkpoint capture with policy pin
- Checkpoint version-aware decode (rejects v1, accepts v2)
- Durable recovery across journal close/reopen for both Strict and Journaled
- Tampered policy pin rejection by recover_retry
- Tampered policy pin persistence via new RunFailed + new checkpoint (not just in-memory forging)
- Emergency deny: new/inactive, activate/active, idempotent repeat, persistence across close, corruption detection (wrong length, wrong version, empty, trailing bytes), batch commit failure, retry after failure, strict persistence on already-active
- CLI JSON export with policy_pin field

---

## Conclusion

The v19 candidate is well-structured and correctly implements admission policy pin retention through the full durable recovery callpath. The wire contract is stable, the checkpoint versioning is robust, the emergency deny storage seam is correctly bounded and durable, and recovery validation is comprehensive.

No BLOCKING findings. The candidate is approved for landing.
