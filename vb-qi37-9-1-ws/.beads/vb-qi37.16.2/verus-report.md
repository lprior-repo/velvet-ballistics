# Verus Report

STATUS: APPROVED

## Tool Availability

- `command -v verus`: `/home/lewis/.local/bin/verus`
- `verus --version`: `0.2026.05.05.d03e906`, toolchain `1.95.0-x86_64-unknown-linux-gnu`
- `verusfmt --check .beads/vb-qi37.16.2/verus_resume_harness.rs`: `VERUSFMT_MISSING` (not used as proof evidence)

## Command Repair

The State 3 commands named standalone production Rust modules, which Verus rejected before proof checking because they either passed multiple input files or lacked crate/dependency context. Contract review approved Verus scope for Rust-local pure/core behavior; State 12 repaired the executable command by adding a dedicated harness with minimal pure models for the five approved obligations.

## Exact Obligation Command

```bash
verus .beads/vb-qi37.16.2/verus_resume_harness.rs
```

Outcome: exit 0.

```text
verification results:: 6 verified, 0 errors
```

## Obligation Results

- `VERUS-INV-001` — PASS; `proof_handle_resume_preserves_invariants` proves successful resume requires `Resumable`, transitions to `Running`, appends `Resumed`, and returns matching `ResumeResult` fields.
- `VERUS-PRE-002` — PASS; `proof_is_resumable_exhaustive` proves `is_resumable(state) <==> state == RuntimeState::Resumable`.
- `VERUS-PRE-003` — PASS; `proof_hydration_completeness` proves the hydration predicate entails reconstructable matching events.
- `VERUS-POST-004` — PASS; `proof_append_immutable` proves pure Seq append length, last element, and prior-index preservation.
- `VERUS-INV-003` — PASS; `proof_resume_result_fields_present` proves non-optional field presence through the harness typestate model.

## Trusted Boundaries

- No `assume`, `#[verifier::external_body]`, `#[verifier::external]`, or `axiom` was introduced in the harness.
- Trusted boundary remains refinement from production `RuntimeState`, `RuntimeJournal`, `JournalEvent`, `ResumeResult`, and `Shard::handle_resume` code to the minimal harness model.
- Storage durability, async scheduling, I/O, wall-clock, and CLI formatting remain outside Verus and covered by TLA+/replay/integration evidence.

## Trust Scan

`grep`/content scan for `assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom` under `.beads/vb-qi37.16.2/*.rs`: no matches.
