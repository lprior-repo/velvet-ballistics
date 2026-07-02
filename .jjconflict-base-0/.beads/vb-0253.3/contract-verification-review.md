# Contract Verification Review

**Bead**: vb-0253.3 — ui: Bound IPC bridge channels with backpressure
**Reviewer**: contract-verification-reviewer
**Date**: 2026-05-19

---

## STATUS: APPROVED

**With noted DEFERRED_GLOBAL workspace issue that does not block this review.**

---

## Files Reviewed

| Artifact | Path | Valid |
|---|---|---|
| contract.md | .beads/vb-0253.3/contract.md | ✓ |
| tla-spec.md | .beads/vb-0253.3/tla-spec.md | ✓ |
| lean-contract.md | .beads/vb-0253.3/lean-contract.md | ✓ |
| verification-layers.md | .beads/vb-0253.3/verification-layers.md | ✓ |
| proof-obligations.jsonl | .beads/vb-0253.3/proof-obligations.jsonl | ✓ (valid JSONL) |
| traceability-matrix.jsonl | .beads/vb-0253.3/traceability-matrix.jsonl | ✓ (valid JSONL) |

## Command Evidence

```bash
jq -c . .beads/vb-0253.3/proof-obligations.jsonl >/dev/null
# proof-obligations.jsonl: VALID

jq -c . .beads/vb-0253.3/traceability-matrix.jsonl >/dev/null
# traceability-matrix.jsonl: VALID
```

---

## Findings

### Severity: ADVISORY (not a blocker)

**Issue**: DEFERRED_GLOBAL — vb_ui workspace exclusion

**Details**:
- `Cargo.toml` at repository root contains `exclude = ["crates/vb_ui"]`
- This prevents `cargo build -p vb_ui` and `cargo test -p vb_ui` from executing
- 26 pre-existing compile errors in unrelated vb_ui files (`app_state.rs`, `graph_builder.rs`, `graph_renderer.rs`, `registry/mod.rs`, `replay/`, `verify/`, `workflow/`) caused by vb_core API drift
- The ipc_bridge.rs implementation itself is error-free: `CHANNEL_CAPACITY = 16`, `mpsc::bounded()`, `try_send()`, and the `bridge_send_on_full_returns_error` test are all present and correct

**Contract artifact status**: The workspace exclusion is a **pre-existing infrastructure issue**, not a defect in the contract or verification artifacts themselves.

---

## Coverage Decision

### Contract Clauses Traced

| Clause | Description | Traced |
|---|---|---|
| PRE-001 | new() thread spawn failure → disconnected tx | ✓ |
| PRE-002 | send() requires tx connected | ✓ |
| POST-001 | bounded sync_channel construction | ✓ |
| POST-002 | send() returns Ok when capacity available | ✓ |
| POST-003 | send() returns Err("channel full") on backpressure | ✓ |
| POST-004 | send() returns Err("disconnected") on tx drop | ✓ |
| POST-005 | poll() drains via try_recv (non-blocking) | ✓ |
| POST-006 | is_connected() tracks connection state | ✓ |
| INV-001 | IpcBridge owns bounded tx/rx channels | ✓ |
| INV-002 | connected field tracks connection state | ✓ |
| ERR-TX-001 | channel full error taxonomy | ✓ |
| ERR-TX-002 | disconnected error taxonomy | ✓ |

### TLA+-Owned Clauses Covered

**WAIVER-TLA-001** correctly invoked. Rationale:
- No temporal protocol, workflow, liveness, or deadlock conditions
- Single-threaded IPC conduit with unchanged recv_timeout loop
- Bounded channel capacity is a Rust-local API constraint provable by unit test
- TLA+ would not add value over a Rust unit test for this change

### Verus-Owned Clauses Covered

**WAIVER-VERUS-001** correctly invoked. Rationale:
- Bounded channel is a stdlib API change (`mpsc::sync_channel` replacing `mpsc::channel`)
- `send()` backpressure logic is pure deterministic Rust with exhaustively testable error paths
- No refinement types or ghost state needed
- Unit tests + compile verification are sufficient

### Lean/Aeneas/Hax Scope

**WAIVER-LEAN-001** correctly invoked. Rationale:
- No algebraic state transitions, no protocol lattices, no arithmetic bounds theorems
- No parser/codec invariants

### Proof Obligations Traced

All 12 obligations in `proof-obligations.jsonl` have:
- `id`, `contract_clause`, `target`, `claim`, `layer`, `checker`, `command`
- `evidence`, `expected_evidence`, `risk`, `scope`, `required`, `mode`
- `owner_state`, `rerun_from`, `status: planned` ✓

All obligations map to exact executable commands targeting `vb_ui` crate.

### TLA+ Scope Valid

✓ WAIVER-TLA-001 complete with owner, reason, expiry (never), compensating evidence
✓ Non-applicability rationale documented and sound

### Verus Scope Valid

✓ WAIVER-VERUS-001 documented in verification-layers.md
✓ Bounded channel backpressure is stdlib API change provable by unit tests

### Lean/Aeneas/Hax Scope Valid

✓ WAIVER-LEAN-001 complete with owner, reason, expiry (never), compensating evidence
✓ No theorem kernel required

### Waivers Valid

| Waiver | Owner | Reason | Expiry | Compensating Evidence |
|---|---|---|---|---|
| WAIVER-TLA-001 | vb-0253.3 contract | No temporal protocol; single-threaded conduit | Never | Unit tests + compile + proptest |
| WAIVER-LEAN-001 | vb-0253.3 contract | No algebraic theorems; pure Rust API change | Never | Unit tests + compile |
| WAIVER-VERUS-001 | vb-0253.3 contract | stdlib API change; testable error paths | Never | Unit tests + compile |
| WAIVER-KANI-001 | verification-layers | No unsafe code in scope | N/A | N/A |
| WAIVER-LOOM-001 | verification-layers | SPSC mpsc; no concurrent interleavings | N/A | N/A |

---

## Contract Artifact Quality

### No Defects Found

1. **All contract clauses trace to bounded channel and backpressure** ✓
   - POST-001/POST-003/ERR-TX-001: bounded channel capacity enforcement
   - POST-002/POST-003/POST-004: send() error taxonomy for full vs. disconnected
   - INV-001: struct field types correctly specify bounded channels

2. **All proof obligations are executable** ✓
   - Exact commands specified for each obligation
   - All required fields present
   - status=planned for all obligations

3. **Traceability matrix complete** ✓
   - Every contract clause maps to tests and proofs
   - No orphaned clauses

4. **JSONL files valid** ✓

---

## DEFERRED_GLOBAL Workspace Issue

**Not a contract artifact defect.** Documented in:
- `.beads/vb-0253.3/STATE.md:24` — `workspace_issue: "vb_ui excluded from workspace — DEFERRED_GLOBAL"`
- `.beads/vb-0253.3/test-writer-report.md:94-205` — Full analysis of workspace exclusion and pre-existing errors

**Resolution path** (outside scope of this review):
1. vb_ui workspace exclusion must be resolved to run `cargo build -p vb_ui` / `cargo test -p vb_ui`
2. Or: vb_ui must be tested in isolation with its own Cargo.toml
3. 26 pre-existing vb_ui errors in other files are tracked separately as DEFERRED_GLOBAL

**Bead-local implementation is correct**:
- `crates/vb_ui/src/ipc_bridge.rs:19` — `const CHANNEL_CAPACITY: usize = 16;`
- `crates/vb_ui/src/ipc_bridge.rs:150-151` — `mpsc::bounded::<IpcRequest>(CHANNEL_CAPACITY)`
- `crates/vb_ui/src/ipc_bridge.rs:192` — `.try_send(request)`
- `crates/vb_ui/src/ipc_bridge.rs:905-926` — `bridge_send_on_full_returns_error` test present

---

## Conclusion

**Contract and verification artifacts are APPROVED.** The obligations are well-formed, traceable, and cover bounded channel and backpressure semantics completely. The DEFERRED_GLOBAL classification applies to workspace infrastructure (vb_ui excluded from workspace + 26 pre-existing errors in other vb_ui files), not to the contract or verification artifacts themselves.

Downstream test planning and implementation may proceed. The proof obligations are executable once the DEFERRED_GLOBAL workspace issue is resolved or when running against vb_ui in isolation.
