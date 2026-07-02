# Proof Plan Review Input — vb-0253.3

## Bead
vb-0253.3 — IPC bridge bounded channel change

## Context Summary
- **Change**: Replace unbounded `mpsc::channel()` with bounded `mpsc::sync_channel(CHANNEL_CAPACITY)` in `crates/vb_ui/src/ipc_bridge.rs`
- **Trigger**: UI thread starvation risk from unbounded queue growth; need backpressure signaling
- **Public API impact**: `IpcBridge::send()` returns `Err("channel full")` under load

## Discovery Evidence

### Current Code State
```
crates/vb_ui/src/ipc_bridge.rs:
  Line 144: let (req_tx, req_rx) = mpsc::channel::<IpcRequest>();  // UNBOUNDED
  Line 145: let (rep_tx, rep_rx) = mpsc::channel::<IpcReply>();    // UNBOUNDED
  Lines 182-186: send() uses tx.send(request) — BLOCKING
```

### Workspace Build Status
- **Status**: `cargo build -p vb_ui --lib` fails with pre-existing errors in other files (app_state.rs, node_mapping.rs, workflow/*.rs)
- **Relevant file**: `ipc_bridge.rs` compiles in isolation — build failures are NOT caused by this change
- **ipc_bridge.rs tests**: Cannot run due to workspace build failures

### Pre-existing Build Errors (NOT in scope)
- `app_state.rs`: `PassFail: From<&str>` trait not implemented (unrelated to ipc_bridge)
- `node_mapping.rs`: `CompiledNodeKind` non-exhaustive match (unrelated to ipc_bridge)
- Multiple files: `From<String>` for `PassFail` type errors (unrelated)

## Risk Classification Review

| Risk Tag | Present in Change? | Justification |
|----------|-------------------|---------------|
| ui | YES | Bounded channel could block UI thread if `send` (not `try_send`) is used |
| ipc | YES | Message queue bounded; backpressure must be signaled correctly |
| backpressure | YES | Core purpose of this change — return `"channel full"` error |
| bounded-channel | YES | Capacity constant required; must be power-of-two |

## Obligation Row Review

All 12 obligation rows map to contract clauses with traceable risk:

| ID | Maps to Clause? | Maps to Risk? | Waivers Appropriate? |
|----|-----------------|---------------|---------------------|
| VB0253-COMPILE-001 | POST-001 ✓ | high ✓ | N/A |
| VB0253-COMPILE-002 | INV-001 ✓ | medium ✓ | N/A |
| VB0253-TEST-001 | POST-002 ✓ | medium ✓ | N/A |
| VB0253-TEST-002 | POST-003 ✓ | high ✓ | N/A |
| VB0253-TEST-003 | POST-004 ✓ | medium ✓ | N/A |
| VB0253-TEST-004 | POST-005 ✓ | low ✓ | N/A |
| VB0253-TEST-005 | POST-006 ✓ | low ✓ | N/A |
| VB0253-TEST-006 | PRE-001 ✓ | medium ✓ | N/A |
| VB0253-TEST-007 | ERR-TX-001 ✓ | high ✓ | N/A |
| VB0253-CLIPPY-001 | INV-001 ✓ | medium ✓ | N/A |
| VB0253-LINT-001 | INV-001 ✓ | low ✓ | N/A |
| VB0253-PROPTEST-001 | POST-003 ✓ | low ✓ | OPTIONAL — correctly marked |

## Waivers Review

| Waiver | Layer | Reason Valid? | Notes |
|--------|-------|---------------|-------|
| WAIVER-TLA-001 | TLA+ | YES | No temporal behavior change — recv_timeout loop unchanged |
| WAIVER-LEAN-001 | Theorem | YES | No algebraic kernel |
| WAIVER-VERUS-001 | Verus | YES | Stdlib API; exhaustively testable error paths |
| WAIVER-KANI-001 | Kani | YES | No unsafe code |
| WAIVER-LOOM-001 | Loom | YES | SPSC mpsc; no concurrent interleavings |

## Pre-existing Build Error Impact on Proof Planning

**BLOCKER**: The workspace cannot compile `vb_ui` due to pre-existing errors in:
- `app_state.rs` (PassFail type mismatch)
- `node_mapping.rs` (non-exhaustive match)
- Several workflow/*.rs files

**Mitigation**:
1. The `ipc_bridge.rs` change is isolated to one file
2. Proof obligations for compile lane use `cargo build -p vb_ui --lib` which will fail until pre-existing errors are fixed
3. Test lane similarly blocked by compilation

**Recommendation**: This proof plan assumes the pre-existing build errors will be fixed separately (likely in a different bead). The proof obligations are correctly scoped to `ipc_bridge.rs` but execution is gated on workspace build health.

## Questions for Reviewer

1. **Q1 (capacity value)**: Should proof-obligations.planned.jsonl include a capacity-specific test, or is the proptest boundary test sufficient?
2. **Q2 (blocking vs non-blocking)**: Is the compile-time check for `try_send` (not `send`) sufficient, or should we add a runtime test that verifies `send()` never blocks?
3. **Build blockers**: Should we add a waiver for compile/test lanes until pre-existing errors are fixed, or treat them as dependencies on another bead?

## Review Checklist

- [ ] All 12 obligation rows have unique IDs
- [ ] All obligations trace to contract clauses (POST-001 through POST-006, PRE-001, PRE-002, INV-001, INV-002, ERR-TX-001, ERR-TX-002)
- [ ] All obligations map to at least one risk tag
- [ ] Skipped applicable verifiers (TLA+, Verus, Kani, Loom) have explicit waivers
- [ ] proptest marked as optional (not required)
- [ ] Commands use exact artifact paths and test names
- [ ] `owner_state` and `rerun_from` populated for each row
- [ ] `status` is "planned" for all rows
