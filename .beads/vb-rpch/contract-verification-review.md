# Contract Verification Review — vb-rpch
## State: 6 contract-verification-reviewer
## Date: 2026-05-19

---

## STATUS: REJECTED

---

## Executive Summary

The TLA+ spec `RecoveryReplayFull.tla` does NOT implement the 6 required invariants (TLA-001 through TLA-006). The cfg declares invariants that don't exist in the spec. The formal verification evidence is insufficient to approve the contract. There is also a contract-implementation gap in POST-006.

---

## 1. Contract Correctness Assessment

### 1.1 Contract vs Implementation Gaps

| Clause | Finding | Severity |
|--------|---------|----------|
| POST-006 | `hydrate_run_frame` does NOT set `max_parallel_in_flight`. The contract requires: "`max_parallel_in_flight` reflects observed peak". Implementation (hydrate.rs:32-123) only calls `increment_executed()` based on tail event count — it never calls `set_max_parallel_in_flight`. Compare: `hydrate_run_frame_from_events` (hydrate.rs:223) DOES call `frame.set_max_parallel_in_flight(peak)`. | **BLOCKING** |
| POST-007 | `unsupported` field not populated by `hydrate_run_frame_from_events`. Contract requires: "unsupported field correctly marks any missing slot_values, slot_taint, action_payloads, or pending_actions". Implementation populates steps, slots, PC but never sets `unsupported`. | MEDIUM |

**GAP-1 (BLOCKING)**: `hydrate_run_frame` is missing `set_max_parallel_in_flight` call. This is a direct contract-implementation gap.

---

## 2. Spec vs Contract Gaps (TLA+ Owned Clauses)

### 2.1 TLA-001: ReplaySeqOrder — NOT IMPLEMENTED

**Contract requires**: Events replayed in ascending seq; steps monotonic increasing per attempt

**Spec has**: `StepOrderInvariant` (lines 51-55) — only checks step index non-decreasing. Does NOT enforce sequence number ordering.

**Gap**: The spec does not model `seq` ascending order per attempt. `ReplaySeqOrder` is not defined.

### 2.2 TLA-002: TailCausalAfterSnapshot — NOT IMPLEMENTED

**Contract requires**: All tail seq > snapshot seq (invariant: TailCausalAfterSnapshot)

**Spec has**: `snapshot_seq` variable exists (line 22) but is NEVER updated from -1. The `Next` action (lines 90-152) never modifies `snapshot_seq`. `TailCausalAfterSnapshot` is not defined in the spec.

**Gap**: No snapshot/tail causal consistency modeled.

### 2.3 TLA-003: OnlyIncompleteRuns — NOT IMPLEMENTED

**Contract requires**: Only runs without terminal event of max attempt are returned (invariant: OnlyIncompleteRuns)

**Spec has**: No `DiscoverIncomplete` action. No `recovered_runs` variable. No `OnlyIncompleteRuns` invariant defined.

**Gap**: Incomplete run discovery is not modeled.

### 2.4 TLA-004: NoResolvedReExecution — PARTIAL

**Contract requires**: Resolved action+step never appears in replay output; tracker blocks re-execution via `NonIdempotentActionBlocked`

**Spec has**: `NoDoubleScheduling` invariant (lines 60-67) checks no duplicate `ActionScheduled` for same (action, step). This does NOT model the tracker blocking behavior. The `Next` action (lines 98-100) allows `ActionScheduled` without checking if already in `tracker.completed` or `tracker.failed`.

**Gap**: Rust `core.rs:82-89` checks `tracker.is_resolved()` BEFORE allowing ActionScheduled and returns `NonIdempotentActionBlocked` error. TLA+ spec silently allows it.

### 2.5 TLA-005: RecoveryErrorExhaustive — NOT IMPLEMENTED

**Contract requires**: Every error variant reachable from defined inputs

**Spec has**: No `RecoveryError` variant modeling. No `last_error` variable. No error state machine.

**Gap**: Error exhaustiveness not verified.

### 2.6 TLA-006: DigestVerificationOrder — NOT IMPLEMENTED

**Contract requires**: Workflow digest verified before IR digest

**Spec has**: No `digest_level` variable. No `CheckWorkflowDigest` or `CheckIrDigest` actions. No `DigestVerificationOrder` invariant defined.

**Gap**: Digest verification ordering not modeled.

---

## 3. cfg vs Spec Mismatch (CRITICAL)

The `evidence/specs/RecoveryReplayFull.cfg` declares:

```
INVARIANT
    TypeOK
    TailCausalAfterSnapshot
    ReplaySeqOrder
    OnlyIncompleteRuns
    NoResolvedReExecution
    DigestVerificationOrder
```

But `RecoveryReplayFull.tla` only defines 4 invariants:
- `NoDivergenceInvariant` (line 57) — NOT in cfg
- `StepOrderInvariant` (line 51) — NOT in cfg  
- `NoDoubleScheduling` (line 60) — NOT in cfg
- `ActionSafety` (line 72) — NOT in cfg

**The cfg declares 6 invariants that don't exist as definitions in the TLA+ spec.**

The 4 invariants that DO exist in the spec are NOT declared in the cfg.

This is a complete breakdown of TLA+ artifact integrity.

---

## 4. Verification Evidence Assessment

### 4.1 TLC Execution — WAIVER_APPLIED (Simulation Only)

Proof-evidence.md claims 144,036+ states (line 47) but also shows 21,404 states in simulation mode (line 89-92). These are contradictory. TLC simulation mode does NOT provide exhaustive verification; it explores a subset of state space.

**WAIVER_APPLIED**: The proof-evidence.md (line 95) states a waiver was applied for PO-VB-008 through PO-VB-013 citing state space explosion. However, waivers do not make the contract clauses verified.

### 4.2 Verus — BLOCKED_TOOLING

No execution confirmed. Spec files exist but `cargo verus` returns a placeholder. No 0-error output recorded.

### 4.3 Kani — Present but Location Wrong

The harness is at `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/kani_recovery_hydrate.rs` — outside the isolated workdir.

---

## 5. GOD RULES Assessment

| Rule | Assessment |
|------|------------|
| No Hardcoded Kani Shapes | Kani harness uses `kani::any::<u8>() % 18` for JournalEvent discriminant — compliant |
| No Vacuum Verus Proofs | Verus specs exist but BLOCKED_TOOLING — no execution |
| No Unbounded TLA+ Math | Constants bound: MAXSEQ=10, MAX_EVENTS=5 — compliant |
| No Loop Oscillations | Spec uses Append/recursion, terminates — compliant |
| No Blind Verification Mutations | WAIVER applied for TLC simulation — acceptable |

---

## 6. Sound Waivers

| Waiver | Assessment |
|--------|------------|
| GAP-3 (ActionAbiMismatch, PolicyDigestMismatch) | SOUND — deferred to vb-ty9, not reachable via public API |
| DEFERRED_GLOBAL (TerminalStateMismatch) | SOUND — no public API parameter |
| TLC Simulation Mode | SOUND but insufficient for proof — waiver承认 coverage gap |

---

## 7. Required Remediation

1. **GAP-1 FIX**: Add `set_max_parallel_in_flight` call to `hydrate_run_frame`. The `apply_tail_events` function in `hydrate_support.rs` must return peak parallel in-flight count, or a separate pass must compute it.

2. **Rewrite TLA+ spec** to implement all 6 required invariants (TLA-001 through TLA-006). At minimum:
   - Define `ReplaySeqOrder` enforcing seq ascending order
   - Define `TailCausalAfterSnapshot` with proper snapshot_seq updates
   - Define `OnlyIncompleteRuns` with DiscoverIncomplete action
   - Define `NoResolvedReExecution` with tracker-based blocking
   - Add `last_error` variable and error state machine for TLA-005
   - Add `digest_level` and CheckWorkflowDigest/CheckIrDigest for TLA-006

3. **Fix cfg declarations** to match actual spec invariants

4. **Run exhaustive TLC** model checking (not simulation) and record actual invariant pass/fail per invariant

5. **Resolve POST-007 `unsupported` field** — either implement it or update contract to remove the requirement

---

## 8. Traceability Summary

| Clause | Status |
|--------|--------|
| PRE-001 | Test-only (Kani present but BLOCKED_TOOLING Verus) |
| PRE-002 | Test-only |
| PRE-003 | VERUS-POST-001-TODO (unresolved) |
| PRE-004 | VERUS-PRE-004-TODO (unresolved) |
| POST-001 | ❌ TLA-001 not implemented |
| POST-002 | ❌ TLA-001 not implemented |
| POST-003 | ❌ TLA-001/TLA-006 not implemented |
| POST-004 | ❌ TLA-001 not implemented |
| POST-005 | ❌ TLA-001 not implemented |
| POST-006 | ❌ GAP-1: missing set_max_parallel_in_flight |
| POST-007 | ❌ unsupported field not set |
| POST-008 | ❌ TLA-003 not implemented |
| POST-009 | ❌ TLA-004 not implemented |
| POST-010 | BLOCKED_TOOLING Verus |
| INV-001 | ✅ Adequate (enum exhaustiveness) |
| INV-002 | BLOCKED_TOOLING Verus |
| INV-003 | BLOCKED_TOOLING Verus |
| INV-004 | BLOCKED_TOOLING Verus |
| INV-005 | BLOCKED_TOOLING Verus |
| INV-006 | ❌ TLA-003 not implemented |

---

**Verdict: REJECTED**

The contract cannot be approved. GAP-1 (missing `set_max_parallel_in_flight`) is a direct contract-implementation gap. The TLA+ spec does not implement TLA-001 through TLA-006. The cfg declares invariants that don't exist in the spec. Formal verification evidence is insufficient.

---
