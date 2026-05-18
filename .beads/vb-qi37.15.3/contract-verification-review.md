# Contract Verification Review — vb-qi37.15.3

**Bead:** vb-qi37.15.3 — cli: Add trace command
**Phase:** State 6 (contract-verification-reviewer)
**Reviewer:** contract-verification-reviewer
**Generated:** 2026-05-18

---

## Review Verdict

STATUS: APPROVED

---

## Contract Clauses Under Review

| Clause | Type | Formal Coverage | Adequacy |
|---|---|---|---|
| INV-001: build_trace determinism | invariant | TRACE-VERUS-001 (Verus forall proof) | ✓ Adequate |
| INV-002: read-only trace | invariant | TLA+ waived (shell layer, no writes) | ✓ Adequate |
| INV-003: output determinism | invariant | TRACE-VERUS-001 + serde_json determinism assumption | ✓ Adequate |
| PRE-001: valid run_id | precondition | TRACE-ERR-001 (clippy gate) | ✓ Adequate |
| ERR-001: invalid run_id → ParseError | error | TRACE-ERR-001 (clippy gate) | ✓ Adequate |
| POST-001–007 | postconditions | Deferred to State 8 (integration tests) | ✓ Pending implementation |
| ERR-002, ERR-004 | error taxonomy | Deferred to State 8 (integration tests) | ✓ Pending implementation |

---

## Formal Adequacy Assessment

### INV-001 Determinism — Formal Proof Sufficiency

**Claim:** `build_trace` is pure: same `&[JournalEvent]` always produces identical `Vec<TraceEntry>` in same order.

**Formal evidence:** `proof_trace_one_applied_globally_deterministic` (Verus) proves:
```
forall events1, events2:
  events1.len() == events2.len()
  && forall i: 0 <= i < events1.len() ==> events1[i] == events2[i]
  ==> forall i: 0 <= i < events1.len()
      ==> spec_trace_one(i, &events1[i]) == spec_trace_one(i, &events2[i])
```

**Binding to production:** `spec_trace_one` mirrors `trace_one` exactly — same 18-variant match, same event_type strings, same field extractions, same hardcoded `seq: 0` for RunResumed/RunRetried/RunAnswered.

**Adequacy:** The proof establishes the mathematical property at the spec level. The production `trace_one` is a direct transcription of the spec logic into Rust. Since both use identical match arms and field accesses, the proof result transfers to the implementation.

**No vacuum:** Four distinct non-vacuous proofs cover variant totality (18 arms), reflexivity, per-event determinism lemma, and the global forall property.

### TLA+ Waiver Adequacy

Trace performs no state machine transitions, no concurrency, no retry/lease logic. It is a pure function mapping an immutable input sequence to an output sequence. No TLA+ temporal properties are at risk. Compensating evidence (Verus INV-001 proofs) is formally adequate.

### Clippy Gate Adequacy (ERR-001 / PRE-001)

`cargo clippy -p vb_cli -- -D warnings` enforces zero `unwrap`/`expect`/`panic` in the parse path. This guarantees `parse_run_id` returns `ParseError` explicitly rather than panicking. Adequate for ERR-001 and PRE-001.

---

## Waived Lanes — Permanent Waiver Assessment

| Lane | Waiver | Compensating Evidence | Verdict |
|---|---|---|---|
| TLA+ | Permanent | Read-only pure function; Verus INV-001 | ✓ Justified |
| Kani | Permanent | Verus exhaustive match (18 variants) | ✓ Justified |
| Flux | Permanent | No refinement-type properties | ✓ Justified |
| Loom | Permanent | No concurrency primitives | ✓ Justified |
| Miri | Permanent | `#![forbid(unsafe_code)]` | ✓ Justified |
| Fuzz | Permanent | run_id validated by parse_run_id | ✓ Justified |

---

## Deferred Obligations

| Obligation | Owner State | Reason | Status |
|---|---|---|---|
| TRACE-CLI-001 through CLI-007 | 8 | Require integration tests + moon ci | Pending |
| TRACE-ERR-002, ERR-004 | 8 | Require integration test execution | Pending |
| TRACE-PROP-001 | 8 | Optional proptest | Pending |

All deferred obligations are correctly routed to State 8 (test-writer) with `rerun_from: 8`. No premature deferral.

---

## Artifact Path Observations

- `proof-obligations.planned.jsonl` (State 4) records wrong artifact path (`vb_cli/commands_journal.rs` vs actual `vb_cli_commands_journal_trace.rs`) and wrong variant count (16 vs 18). These are State 4 documentation issues, not proof defects. Proof uses correct paths.
- `proof-obligations.jsonl` (State 3) records wrong crate path (`velvet_ballastics` vs actual `vb_cli`). Production code confirmed at `crates/vb_cli/src/commands_journal.rs`.

These path errors do not affect proof validity or contract adequacy.

---

## Recommendation

**Contract and proof obligations are adequate.** All formal obligations that can be verified at this stage are verified. Deferred obligations are correctly routed. No contract-parity issues found.

**Advance to State 7 (test-planner).** No proof-repair-guide.md needed.
