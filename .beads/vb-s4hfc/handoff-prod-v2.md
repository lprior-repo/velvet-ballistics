# VB-S4HFC Production Handoff

## Completed Production Changes

Three semantic corrections implemented in vb_storage production code:

### 1. Run Binding Validation at Encode Boundary
**File:** `crates/vb_storage/src/events/wire_serialize_policy.rs`

PolicyDecisionEvent serialization now validates `event.run == decision.ticket.run` before encoding. Mismatched events return a typed serde error with both run identifiers.

### 2. Run Binding Validation at Decode Boundary
**File:** `crates/vb_storage/src/events/wire_deserialize_policy.rs`

PolicyDecisionEvent deserialization validates `event.run == decision.ticket.run` after decoding. Malformed mismatched events are rejected with typed deserialization error.

### 3. Neutral Policy Decision Lifecycle Preservation
**File:** `crates/vb_storage/src/journal/incident.rs`

`derive_lifecycle_state_from_events` now scans events in reverse and skips neutral `PolicyDecisionEvent` variants. A policy decision event no longer overwrites preceding terminal lifecycle states (Failed/Cancelled/Completed). Added helper `is_lifecycle_affecting` to classify neutral vs affecting events.

## Gate Results

| Check | Exit Code | Command |
|-------|-----------|---------|
| Production compile | 0 | `cargo check -p vb_storage --lib` |
| Strict Clippy | 0 | `cargo clippy -p vb_storage --lib -- -D warnings` |
| Rustfmt | Applied | Changed files formatted with pinned nightly |

## Files Modified

- `crates/vb_storage/src/events/wire_serialize_policy.rs` — run binding validation on encode
- `crates/vb_storage/src/events/wire_deserialize_policy.rs` — run binding validation on decode
- `crates/vb_storage/src/journal/incident.rs` — neutral event skipping in lifecycle derivation

## What Was NOT Changed

- No test modifications (independent 3090 workspace handles tests)
- No new production files
- No unrelated source deletions
- No shell/helper script edits
- Tag 38 (PolicyDecision) and tag 39 (reserved for admission) unchanged
- Old stable tags unchanged

## Remaining Work (Other Agent Workspaces)

- Test coverage for run binding validation (independent3090)
- Runtime dispatch integration (separate admission worker)
- End-to-end behavioral tests
- Full moon ci gate across all packages

## Design Notes

- Validation returns typed serde errors, not panics
- Outer event run is authoritative; decision.ticket.run is cross-checked
- Policy decision events are neutral facts that journal the evaluation but do not alter lifecycle derivation
- No new accidental authority minted; JournalEventSeq remains independent of ticket SeqNo
