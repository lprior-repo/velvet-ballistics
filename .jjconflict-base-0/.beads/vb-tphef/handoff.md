# vb-tphef Handoff: Typed Policy Evaluation Foundations

## Status: Foundation Complete

All listed corrective changes implemented and verified. Parent still requires wiring/storage/replay layers.

## Changes Made

### 1. DecisionRecord Constructor Removed
- Deleted `#[allow(clippy::too_many_arguments)]` attribute entirely
- Deleted `DecisionRecord::new` 9-argument constructor function
- All callers now use struct literal syntax
- Updated files: `crates/vb_core/src/policy.rs`, `crates/vb_core/src/policy/new_tests.rs`, `crates/vb_runtime/src/policy.rs`, `crates/vb_runtime/src/policy/tests.rs`

### 2. CurrentPolicyIdentity Added
- New `CurrentPolicyIdentity` enum: `Unconfigured` | `Configured { digest: PolicyDigest }`
- Stores only policy identity (digest), never the full rule
- `CurrentLivePolicy::identity()` conversion accessor provided
- `DecisionRecord.current_live` field type changed from `CurrentLivePolicy` to `CurrentPolicyIdentity`
- Original `CurrentLivePolicy` with full rule semantics preserved unchanged
- File: `crates/vb_core/src/policy.rs`

### 3. Taint Encoding Fixed
- Added `encode_taint_canonical(taint: Taint) -> Option<u8>` to `crates/vb_core/src/policy.rs`
- Explicit enum matching for all 5 variants (Clean=0, DerivedFromSecret=1, Secret=2, Random=3, TimeDependent=4)
- No unknown→255 collapse; compiler errors on future Taint variants not explicitly handled
- `compute_input_digest` returns `RuntimeResult<PolicyDigest>`; encoding failure propagates as typed `RuntimeError::Core`

### 4. Function Decomposition
- `evaluate` decomposed into: `evaluate` (27 lines) + `evaluate_live_layer` (19 lines)
- `compute_input_digest` decomposed into: `compute_input_digest` (22 lines) + `push_ticket_parts` (24 lines)
- All helpers ≤25 logical lines
- File: `crates/vb_runtime/src/policy.rs`

### 5. Comprehensive Tests Added
New test file: `crates/vb_runtime/src/policy/new_tests.rs` (23 tests)
- Each of 8 ticket fields varied separately (run, step, seq, action, run_attempt, attempt, idempotency_key, capacity)
- Different canonical value bytes produce distinct digests
- All 5 Taint variants produce valid distinct digests
- Per-run artifact pin remains exact
- Emergency overlay cannot be loosened by future config
- All tests return `RuntimeResult<()>` and use `?`

Updated test file: `crates/vb_runtime/src/policy/tests.rs` (12 tests updated to Result-returning)
- Original tests preserved verbatim: `crates/vb_core/src/policy/original_tests.rs`
- Core policy tests: `crates/vb_core/src/policy/new_tests.rs`

## Gate Results

### Cargo Test (Focused)
```
vb_core: 49 passed; 0 failed; 0 ignored; 0 measured; 2142 filtered out
vb_runtime: 92 passed; 0 failed; 0 ignored; 0 measured; 1876 filtered out
Command: cargo test -p vb_core -p vb_runtime --lib -- policy --test-threads=1
```

### Clippy
```
Finished `dev` profile [unoptimized + debuginfo] target(s)
Command: cargo clippy -p vb_core -p vb_runtime --lib -- -D warnings
```

### Formatter
```
All modified files formatted per workspace edition
Command: cargo fmt --check -- crates/vb_core/src/policy.rs crates/vb_core/src/policy/new_tests.rs crates/vb_runtime/src/policy.rs crates/vb_runtime/src/policy/tests.rs crates/vb_runtime/src/policy/new_tests.rs
```

## Source Files Modified

- `crates/vb_core/src/policy.rs` (+119 lines) — CurrentPolicyIdentity enum, encode_taint_canonical helper, DecisionRecord field type change, constructor removal
- `crates/vb_core/src/policy/new_tests.rs` (+10 lines) — identity method test, struct literal test update
- `crates/vb_runtime/src/policy.rs` (+58 lines, -64 lines) — evaluate reimplementation with struct literals, Result return, decomposed helpers
- `crates/vb_runtime/src/policy/tests.rs` (rewritten) — all tests updated to handle RuntimeResult return
- `crates/vb_runtime/src/policy/new_tests.rs` (new, 23 tests) — comprehensive field/taint variation tests

## Verification Commands

```bash
cd /home/lewis/src/isoloated/velvet-ballistics-vb-tphef-local5090-policy

# Run focused policy tests
cargo test -p vb_core -p vb_runtime --lib -- policy --test-threads=1

# Clippy on affected crates
cargo clippy -p vb_core -p vb_runtime --lib -- -D warnings

# Format check on modified files
cargo fmt --check -- crates/vb_core/src/policy.rs crates/vb_core/src/policy/new_tests.rs crates/vb_runtime/src/policy.rs crates/vb_runtime/src/policy/tests.rs crates/vb_runtime/src/policy/new_tests.rs
```

## Limitations (Parent Still Requires)

- No journal storage layer for decision records
- No durable replay of policy decisions
- No integration with the action dispatch loop
- Shard and runtime lifecycle structs unchanged per spec
- Original RuntimePolicy (Strict/Journaled/Relaxed) semantics untouched

## Review Scope

Black hat review should focus on:
1. Digest collision properties (input bytes + taint + all 8 ticket fields)
2. Non-collapsing Taint encoding (no future variant collapse to same byte)
3. Emergency overlay monotonicity (cannot be cleared)
4. Typed error propagation on unknown taint variants
5. Struct literal correctness in evaluate (field ordering, no omissions)