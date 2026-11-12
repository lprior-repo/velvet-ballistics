# vb-ny068 Event-to-JSON Policy Pin Tests — Added Tests and Unchanged Boundaries

## Added Tests

Four new focused tests added in `crates/velvet_ballistics/src/replay/event_to_json_tests.rs`
declared in `crates/velvet_ballistics/src/replay/mod.rs` as `#[cfg(test)] mod event_to_json_tests;`.

All four tests construct actual `JournalEvent::RunAdmission` events with valid typed fields
(`RunId::new(1)`, `EventSeq::new(1)`, distinct `WorkflowDigest` instances for artifact/input/policy)
and call the real `super::event_to_json` serializer, asserting full JSON value equality.

### 1. `policy_pin_pinned_full_json_equality`
- **Purpose:** Verify full JSON output for `RunPolicyPin::Pinned(distinct policy digest)`.
- **Setup:** Build `RunAdmission` with `RunPolicyPin::Pinned(policy_digest)` where `policy_digest`
  is a distinct `WorkflowDigest` (bytes `[0x03; 32]`) from the artifact (`[0x01; 32]`) and
  input (`[0x02; 32]`) digests.
- **Assertion:** `event_to_json(&event)` produces JSON with exact field-by-field equality,
  including `policy_pin` serialized as `Pinned(<workflow-digest-bytes>)` via Debug formatting.
  All other fields (`seq`, `type`, `run`, `artifact_digest`, `input_digest`,
  `granted_capabilities`, `policy`) are also asserted equal.

### 2. `policy_pin_unverified_full_json_equality`
- **Purpose:** Verify full JSON output for legacy/explicit `RunPolicyPin::Unverified`.
- **Setup:** Build `RunAdmission` with `RunPolicyPin::Unverified` and the same distinct
  artifact/input digests.
- **Assertion:** `event_to_json(&event)` produces JSON with exact field-by-field equality,
  including `policy_pin` serialized as the string `"Unverified"`. All other fields
  asserted equal.

### 3. `pinned_policy_digest_distinct_from_artifact_and_input`
- **Purpose:** Verify the pinned policy digest cannot confuse artifact identity.
- **Setup:** Build `RunAdmission` with `RunPolicyPin::Pinned(policy_digest)`.
- **Assertion:** The serialized `policy_pin` string is distinct from both `artifact_digest`
  and `input_digest` strings. The `policy_pin` string must contain the `Pinned(` wrapper,
  confirming the Debug representation distinguishes it structurally from bare digests.

### 4. `unverified_policy_distinct_from_pinned_representation`
- **Purpose:** Verify `Unverified` and `Pinned` produce provably distinct JSON values.
- **Setup:** Build two separate `RunAdmission` events, one with `RunPolicyPin::Unverified`
  and one with `RunPolicyPin::Pinned(policy_digest)`. Serialize both via `event_to_json`.
- **Assertion:** The `policy_pin` values are not equal. The `Unverified` value is exactly
  the string `"Unverified"`.

## Unchanged Boundaries

- No changes to `crates/velvet_ballistics/src/replay/event_to_json.rs` (production serializer).
- No changes to `crates/velvet_ballistics/src/replay/mod.rs` beyond adding the `#[cfg(test)]`
  module declaration.
- No changes to any other files in `crates/velvet_ballistics/src/` or elsewhere.
- No test clippy/lint changes; existing test conventions followed.
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg`.
- No unchecked indexing, slicing, casts, or arithmetic.
- All test error propagation uses `Result` with `?` and descriptive strings.

## Test File Size

- `event_to_json_tests.rs`: 187 physical lines, 4 test functions, 3 helper functions.
