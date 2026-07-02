# Proof Plan Review Input — vb-qi37.17.1

## Bead

`vb-qi37.17.1` — cli: Add incident command

## Proof Verdict

**NO FORMAL PROOF REQUIRED.** All formal verifier lanes are `not_applicable`.
Proof is via 16 test obligations + 3 structural (compile) obligations.

## Verifier Lane Decisions

| Lane | Decision | Justification |
|------|----------|---------------|
| TLA+ | `not_applicable` | Read-only query, no temporal behavior, no state machine |
| Verus | `not_applicable` | Pure functions fully exercised by unit tests; no ghost state or refinement |
| Kani | `not_applicable` | No `unsafe` code, no bounded-model-checking targets |
| Miri | `not_applicable` | No `unsafe` code, no UB sources |
| Proptest | `not_applicable` | Finite input domain; exhaustive fixed-input tests are sufficient |
| Loom | `not_applicable` | No concurrent code |
| Fuzz | `not_applicable` | Simple event-stream parser with finite event variants |

## Obligations Requiring Review

### Structural (compile) — 3 obligations

| ID | Clause | Risk | Status |
|----|--------|------|--------|
| COMPILE-001 | INV-005 | medium | `planned` — `cargo check --workspace` |
| COMPILE-002 | INV-005 | medium | `planned` — `cargo check --workspace` |
| UNWRAP-001 | INV-001 | high | `planned` — `cargo clippy --workspace --lib --bins -- -D warnings` |
| UNWRAP-002 | INV-001 | low | `waived` — `as_str().unwrap_or()` on Option is zero-panic |
| DEAD-001 | INV-006 | low | `planned` — rustc dead_code lint |

### Unit tests — 13 obligations

| ID | Clause | Risk | Tests covered |
|----|--------|------|---------------|
| T-001 | POST-001, PRE-003 | medium | Empty events |
| T-002 | POST-001 | medium | RunFailed detection |
| T-003 | POST-001 | medium | ActionCompleted → confirmed |
| T-004 | POST-001 | medium | ActionFailed → failed |
| T-005 | POST-001 | medium | Multiple actions |
| T-006 | POST-001 | medium | RunCancelled |
| T-007 | POST-001 | medium | Last step tracking |
| T-008 | INV-001, PRE-003 | medium | Unknown variants |
| T-009 | POST-002 | medium | RunFailed hints empty |
| T-010 | POST-002 | medium | RunFailed hints full |
| T-011 | POST-002 | medium | RunCancelled hints empty |
| T-012 | POST-002 | medium | RunCancelled hints full |
| T-013 | POST-002 | medium | Unknown code → 0 hints |

### Integration tests — 3 obligations

| ID | Clause | Risk | Tests covered |
|----|--------|------|---------------|
| T-014 | POST-003, INV-003 | medium | Failed run JSON output |
| T-015 | POST-003, INV-002 | high | Missing run error (no stack trace) |
| T-016 | POST-004 | medium | Non-failed run exit code |

### Manual QA — 1 obligation

| ID | Clause | Risk | Description |
|----|--------|------|-------------|
| QA-001 | INV-002 | high | No stack traces in any output path |

## Contract Coverage Matrix

| Contract Clause | Covered by | Status |
|-----------------|-----------|--------|
| PRE-001 | T-014 (parse_run_id validated upstream) | Covered |
| PRE-002 | T-015 (structured error on bad db) | Covered |
| PRE-003 | T-001 through T-008 | Covered |
| PRE-004 | T-009 through T-013 | Covered |
| POST-001 | T-001 through T-008 | Covered |
| POST-002 | T-009 through T-013 | Covered |
| POST-003 | T-014, T-015 | Covered |
| POST-004 | T-014, T-015, T-016 | Covered |
| INV-001 | UNWRAP-001, T-001 through T-013 | Covered |
| INV-002 | QA-001, T-015 | Covered |
| INV-003 | T-014 | Covered |
| INV-004 | T-014 | Covered |
| INV-005 | COMPILE-001, COMPILE-002 | Covered |
| INV-006 | DEAD-001 | Covered |

## Review Questions for proof-plan-reviewer

1. **UNWRAP-002 waiver** — Is `as_str().unwrap_or("unknown")` on `serde_json::Value`
   (returns `Option<&str>`) truly zero-panic? Yes — `Option::unwrap_or` never panics
   because it has a fallback. Waiver is sound.

2. **T-015 risk level** — Marked high because missing-run errors must never produce
   stack traces. This is the only obligation where a single failure means a
   contract violation (INV-002). Is that correct severity? Yes — the zero-unwrap
   rule explicitly forbids `unwrap()`/`expect()` on fallible operations.

3. **Mutation testing scope** — Contract specifies `cargo-mutants` on `vb_cli`
   crate targeting `commands_incident.rs` and `app_impl.rs`. Should this be an
   obligation row? Not for this plan — mutation testing is a quality gate, not a
   proof obligation. It's covered by the QA layer.

4. **No TLA+ despite event-sourced system** — The incident command does not
   mutate state; it reads a FjallJournal snapshot and produces output. TLA+
   would verify the journal's internal consistency (a separate bead's concern).
   Is this justification acceptable? Yes — temporal proof belongs to the
   storage/recovery layer, not the query command.

## Evidence Expectations

| Obligation type | Evidence | Success criteria |
|-----------------|----------|-----------------|
| COMPILE | `cargo check --workspace` stdout | 0 E0061 errors |
| UNWRAP/DEAD | `cargo clippy --workspace --lib --bins -- -D warnings` | 0 warnings |
| UNIT tests | `cargo test --package vb_cli --lib commands_incident::tests` | 13 PASS, 0 FAIL |
| INT tests | `cargo test --package vb_cli --test incident_integration` | 3 PASS, 0 FAIL |
| QA | Manual `velvet-ballistics incident` run | No stack traces in any output |
