# Proof Plan Review — vb-8mdp.1

**STATUS: REJECTED**

**Bead**: vb-8mdp.1 — Add IPC fragmented-frame and oversize-message tests
**Reviewer**: proof-plan-reviewer
**Reviewer Skill**: proof-plan-reviewer
**Reviewer Invocation**: proof-plan-reviewer:vb-8mdp.1:20260525
**Reviewed Artifacts**: proof-strategy.md, verifier-lane-decisions.jsonl, verifier-lane-review.jsonl, proof-coverage-matrix.md, proof-obligations.planned.jsonl, proof-to-implementation-input.md, trusted-base-plan.md, waiver-candidates.jsonl, proof-seeds.jsonl, traceability-matrix.jsonl, contract.md

---

## Review Result

**STATUS: REJECTED**

The proof plan has 3 critical blocking issues that must be resolved before approval.

---

## Critical Findings

### Finding 1: VB-IPC-FRAME-003 Has No Verifier Lane Decision (E_LANE_DECISION_MISSING)

**Severity**: BLOCKER

**Code**: `E_LANE_DECISION_MISSING`

**Details**: VB-IPC-FRAME-003 ("validate_frame_magic checks magic before any allocation") is a proof seed in proof-seeds.jsonl with `suggested_verifiers: ["kani", "code-review"]`. It appears in proof-coverage-matrix.md and verifier-lane-matrix.md with no verifier assigned (all cells show "—"). It is completely absent from verifier-lane-decisions.jsonl (0 rows reference it).

The coverage matrix marks it as "—" in every column. The verification-lane-policy requires that every proof seed be classified across the core verifier set with no silent omissions.

**Impact**: The proof seed has no lane decision and no proof obligation. The zero-allocation magic check claim is unverified.

**Repair Instructions**:
1. Add a lane decision row for VB-IPC-FRAME-003 in verifier-lane-decisions.jsonl
2. The seed suggests Kani or code-review. If Kani: create a harness proving `validate_frame_magic` returns HeaderDecodeFailed/InvalidMagic without any Vec allocation for any byte sequence < 4 or wrong magic. If code-review: explain why Kani is not applicable for this specific claim and document the code review location.
3. If the seed is out of scope for vb-8mdp.1 (because it's already covered by existing VB-IPC-FRAME-003 proofs), explicitly document which existing artifact covers it.

---

### Finding 2: VB-IPC-RESPONSE-001 Has No Verifier Lane Decision (E_LANE_DECISION_MISSING)

**Severity**: BLOCKER

**Code**: `E_LANE_DECISION_MISSING`

**Details**: VB-IPC-RESPONSE-001 ("error response uses Health command in header") is a proof seed in proof-seeds.jsonl with `suggested_verifiers: ["code-review"]`. It appears in proof-coverage-matrix.md and verifier-lane-matrix.md with no verifier assigned (all cells show "—"). It is completely absent from verifier-lane-decisions.jsonl.

**Impact**: The proof seed has no lane decision and no proof obligation. The claim that error responses use Health command is unverified.

**Repair Instructions**:
1. Add a lane decision row for VB-IPC-RESPONSE-001 in verifier-lane-decisions.jsonl
2. Document whether code-review is sufficient for this behavioral claim (vs. needing a test or formal proof)
3. If the seed is out of scope, explicitly state which existing artifact covers it.

---

### Finding 3: New Obligations Cover Only 8 of 28 Proof Seeds — Scope Ambiguity

**Severity**: MAJOR

**Code**: `E_SCOPE_MISCLASSIFIED_BEHAVIOR`

**Details**: proof-obligations.planned.jsonl contains 15 obligations addressing 8 unique proof seeds:
- VB-IPC-DECODE-001 (3 obligations: KANI, VERUS, PROPTEST)
- VB-IPC-DECODE-003 (2 obligations: KANI, VERUS)
- VB-IPC-DECODE-004 (2 obligations: KANI, VERUS)
- VB-IPC-SERVER-002 (1 obligation: TLA+)
- VB-IPC-SERVER-003 (2 obligations: KANI, TLA+)
- VB-IPC-FRAGMENT-001 (2 obligations: TLA+, PROPTEST)
- VB-IPC-FRAGMENT-002 (2 obligations: TLA+, PROPTEST)
- VB-IPC-SERVER-004 (1 obligation: TLA+)

The remaining 20 proof seeds from proof-seeds.jsonl are not addressed by the new obligations. The plan relies on "existing coverage" for:
- VB-IPC-DECODE-002, 005, 006, 007
- VB-IPC-POSTCARD-001, 002
- VB-IPC-BOUNDED-001, 002
- VB-IPC-MAGIC-001, 002, 003
- VB-IPC-VERSION-001
- VB-IPC-COMMAND-001, 002
- VB-IPC-FRAME-001, 002, 003
- VB-IPC-PAYLOAD-001
- VB-IPC-RESPONSE-001
- VB-IPC-SERVER-001

The plan does not explicitly identify which existing artifacts cover which seeds, nor does it explain why VB-IPC-FRAME-003 and VB-IPC-RESPONSE-001 have zero verifier assignment while other existing seeds are implicitly covered.

**Repair Instructions**:
1. Add a coverage table in proof-plan-review.md explicitly listing all 28 seeds, their status (existing-covered, new-planned, or out-of-scope), and the artifact that provides coverage for existing seeds.
2. Or: narrow the scope to only the 8 seeds that are actively addressed, and move the other 20 seeds to a separate tracking item.

---

## Validated Strengths

The following aspects of the plan are sound and do not require changes:

- **Kani lane (decode order)**: New harness `kani_harness_decode_order_total_fn` correctly targets all 2^192 inputs via `kani::any()` on `[u8; 24]`. Existing harnesses cover decode order for magic/version/reserved/payload_len ordering. Command→reserved gap identified by reviewer and addressed in lane.
- **Verus lane**: 3 obligations planned for DECODE-001/003/004 spec-fn proofs. The spec is required to bind mathematically to actual decode implementation in `frame_types.rs`, not a vacuum spec. Reviewer's note correctly flags this.
- **TLA+ lane (server behavior)**: 8 obligations covering READ_CHUNK_BYTES bound (SERVER-001), no pre-allocation (SERVER-002), oversize disconnect (SERVER-003), dispatch ordering (SERVER-004), partial header wait (FRAGMENT-001), partial payload wait (FRAGMENT-002). State machine modeling is appropriate for sequential I/O behavior.
- **Proptest lane**: 3 obligations covering decode totality runtime check (DECODE-001), partial header/propayload server behavior (FRAGMENT-001/002). Complement to Kani exhaustive check.
- **Non-applicable lanes (loom, miri, flux, cargo-fuzz)**: All four non-applicable lanes are justified with concrete evidence, not weak reasoning like "not needed" or "too hard".
- **Non-vacuity**: Proof-obligations.planned.jsonl has exact commands, bounded assumptions (2^192, 4096 chunk size, 1 MiB max), and expected evidence that is not vacuously true.
- **Trusted base**: trusted-base-plan.md correctly identifies Rust stdlib, byteorder reads, type invariants, and compile-time constants as trusted surfaces. Kani symbolic execution bounds are justified.
- **Bridge planning**: proof-to-implementation-input.md correctly maps proof claims to Rust source refs in `frame_types.rs`, `server/*.rs`, `commands.rs`, `bounded.rs`, `error.rs`.
- **No self-approval**: proof-plan-review.md shows STATUS: PENDING_APPROVAL — reviewer has not stamped their own work.
- **Waivers**: No behavior-affecting waivers requested. Non-applicable lane "waivers" are properly evidenced.

---

## Non-Blocking Observations

These do not block approval but should be addressed:

1. **VB-IPC-SERVER-002 (no pre-allocation) relies solely on TLA+**: Reviewer note correctly identifies this. The TLA+ invariant `allocation_size = 0 in WaitingHeader/WaitingPayload` is the right approach, but the TLA+ spec must explicitly model an `allocation_size` variable and only enable allocation in Dispatching state.

2. **VB-IPC-SERVER-003 (oversize disconnect) split correctly**: Kani proves `decode()` returns `PayloadTooLarge` without I/O (pure function property). TLA+ proves server disconnects without reading payload bytes (state machine property). This is appropriate defense-in-depth.

3. **Proptest for FRAGMENT-001/002 requires server test harness**: The obligations correctly note "server test harness with mock socket" is required. The plan must ensure this harness exists or is created.

---

## Required Repair Actions

| # | Finding Code | Repair Action | State to Rerun From |
|---|-------------|---------------|-------------------|
| 1 | E_LANE_DECISION_MISSING | Add VB-IPC-FRAME-003 lane decision to verifier-lane-decisions.jsonl; identify existing artifact or create new Kani obligation | verifier-lane-decisions.jsonl |
| 2 | E_LANE_DECISION_MISSING | Add VB-IPC-RESPONSE-001 lane decision to verifier-lane-decisions.jsonl; document code-review sufficiency justification | verifier-lane-decisions.jsonl |
| 3 | E_SCOPE_MISCLASSIFIED_BEHAVIOR | Add explicit coverage table for all 28 proof seeds; identify which existing artifacts cover out-of-scope seeds | proof-plan-review.md |

---

## Verifier Lane Review Summary

| Verifier | Obligations | Status |
|----------|-----------|--------|
| Kani | 4 new + existing | accepted (with VB-IPC-SERVER-003-KANI-001 new harness required) |
| Verus | 3 new | accepted (spec must bind to actual decode implementation) |
| TLA+ | 8 new | accepted (SERVER-002 allocation model must be explicit) |
| Proptest | 3 new | accepted (server test harness must exist) |
| Loom | not_applicable | accepted (single-threaded I/O justification sufficient) |
| Miri | not_applicable | accepted (#![forbid(unsafe_code)] evidence sufficient) |
| Flux | not_applicable | accepted (not in project scope, Kani+Verus sufficient) |
| Cargo-fuzz | not_applicable | accepted (Kani exhausts 2^192) |

---

**Reviewer**: proof-plan-reviewer
**Invocation ID**: proof-plan-reviewer:vb-8mdp.1:20260525
**Timestamp**: 2026-05-25
**STATUS: REJECTED**
