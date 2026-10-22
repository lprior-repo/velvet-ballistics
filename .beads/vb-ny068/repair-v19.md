# PRECHECK_V18 Repair — vb-ny068

## Workspace Verification
- `git rev-parse --show-toplevel`: `/home/lewis/src/isoloated/vb-policy-integration-v18` ✓
- `jj root`: `/home/lewis/src/isoloated/vb-policy-integration-v18` ✓

## Source Changes

### 1. crates/vb_storage/src/events/wire_serialize_run.rs
- **Problem:** 330 physical lines exceeded 300-line hard limit.
- **Fix:** Split admission serialization into sibling module `wire_serialize_admission.rs`.
- **Result:** `wire_serialize_run.rs` now 193 lines; `wire_serialize_admission.rs` is 146 lines.
- **Behavior:** Byte-for-byte identical tag24/36/39/41 payload contracts preserved. All 6 admission serialization functions moved unchanged. Main `serialize()` now routes `RunAdmission` to `super::admission::serialize_admission()`.

### 2. crates/vb_storage/src/events/wire_serialize_admission.rs (NEW)
- Contains: `serialize_admission`, `serialize_admission_payload`, `serialize_pinned_admission_payload`, `serialize_pinned_unbound_admission`, `serialize_pinned_bound_admission`, `serialize_bound_admission_payload`.
- Module declared in `wire_serialize.rs` via `#[path = "wire_serialize_admission.rs"] mod admission;`.

### 3. crates/velvet_ballistics/src/replay/event_to_json.rs
- **Problem:** RunAdmission pattern didn't bind `policy_pin`, compiler error E0027.
- **Fix:** Bind `policy_pin` in match pattern; add `"policy_pin"` field to JSON output using same debug-format pattern as existing fields.

### 4. crates/workspace_tests/tests/vb_s25h2_recover_entry_points.rs
- **Problem:** Legacy fixture `run_admission()` missing `policy_pin` field (E0063).
- **Fix:** Add `policy_pin: vb_storage::RunPolicyPin::Unverified` — truly historical legacy fixture, consistent with other test fixtures in repository.

## Verification
- `cargo fmt --all`: exit 0
- `cargo fmt --all --check`: exit 0 (via `/tmp/vb-run-gate.py`)
- Both production files under 300 physical lines.
- No Go, no proof planning, no broad Moon gates.

## Acceptance
All 3 PRECHECK_V18 failures repaired:
1. E0063 fixture missing field → Unverified pin added.
2. E0027 pattern missing field → bound and exported.
3. File over 300 lines → split into 2 focused modules.