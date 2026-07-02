# Trusted Base Plan: vb-uwxct

## Plan Status

All trusted-base entries are planned for State 4 (proof-planner) and must be
reviewed at State 4b (proof-plan-reviewer) and State 6 (proof-reviewer). No
behavior-affecting waivers are included; all entries are **test-only** modeling
or reference debt that must be validated by raw evidence in the corresponding
proof obligation.

This is a **test-only repair**; production code at
`crates/vb_storage/src/keys.rs:480-496` and the `JournalError` enum are
reference-only. The trusted base is correspondingly small.

## Trusted Base Ledger (Planned)

| ID | Obligations | Marker | Kind | Reason | Compensating Evidence |
|----|-------------|--------|------|--------|------------------------|
| TBR-001 | PO-CARGO-LIB-001 (anchor) | `JournalError::SequenceOverflow` unit variant identity | `assume` (named) | The production contract returns this variant iff `seq.get() == u64::MAX`. The variant is the canonical typed-error contract; identity is verified by the existing canonical-positive unit test at `crates/vb_storage/src/keys/tests.rs:497-505`. | `PO-CARGO-LIB-001` raw cargo test pass; `crates/vb_storage/src/keys/tests.rs:497-505` is the canonical-positive reference. |
| TBR-002 | PO-CARGO-LIB-001 (anchor), PO-CARGO-TEST-001 (C1..C6) | Canonical proptest range `0u64..u64::MAX` (from `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs:129,131`) | `external_body` (reference) | The repair reuses an already-accepted proptest strategy from a sibling test file; the canonical-positive reference is on disk and PASSES today. | `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs:123-146` (`run_event_ordering` proptest); `PO-CARGO-LIB-001`; `PO-CARGO-TEST-001` after repair. |
| TBR-003 | PO-KANI-001 (C7) | `SymbolicKeyInputs` packing via `(hi << 16) | lo` for `run_raw` and `seq_raw` | `assume` (named) | Kani `kani::Arbitrary` derive generates arbitrary u16 pairs; the packing formula is the documented reconstruction of the u64 value. The pack/unpack symmetry is type-level integer arithmetic and is not a hidden assumption. | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:15-41` declares the struct and the helper functions; the kani-list probe in `PO-KANI-001` step (1) confirms the harness is registered; raw Kani PASS confirms the formula. |
| TBR-004 | PO-KANI-001 (C7) | Kani harness calls `keys::run_event_key` directly (production binding) | `external_body` (production binding) | The harness at `crates/vb_storage/src/kani_typed_partitioned_ids.rs:63-70` directly calls the production `pub fn run_event_key` symbol at `crates/vb_storage/src/keys.rs:81-83`; no mirror or shadow type is introduced. The harness is implementation-bound (STRONG binding per `proof-planner` skill). | `PO-KANI-001` raw Kani PASS; production `keys.rs:81-83` is read-only evidence; drift detection is compile-time (the harness re-resolves the symbol on every build). |

## Trusted Base Summary

- **Total entries**: 4
- **Assumes**: 2 (TBR-001, TBR-003) — both name a typed contract (variant identity, integer packing) that is verified by canonical-positive tests or type-level arithmetic
- **External bodies**: 2 (TBR-002, TBR-004) — both are reference/production-binding markers
- **Stubs**: 0
- **Extern specs**: 0
- **Behavior-affecting**: 0 (all are modeling/reference debt; no production semantics are waived)
- **Review state**: planned (owner_state 4)

## Forbidden Categories in Trusted Base

The following are **forbidden** in any trusted-base entry of this bead (per
`AGENTS.md` "Formal Verification Mandates" and `proof-planner` skill
"Anti-laundering"):

- `unsafe` blocks in executable proof code (GOD RULE 5 — production code does
  not change; the Kani harness file carries `#![forbid(unsafe_code)]`)
- `axiom`, `admit`, `external_body` in **executable** proof code (these
  markers appear only in the **trusted-base reference ledger**, not in
  produced proof artifacts)
- `cover!`-as-proof (Kani harness does not introduce a `cover!` claim; the
  repair uses an explicit `assert!(seq_value == u64::MAX)` instead)
- `kani::assume(seq_value != u64::MAX)` blanket constraint (explicitly
  forbidden by `contract.md` §4.2 and `codebase-map.md` "Open Questions" —
  masking the sentinel in the proof model would violate C7)

## Validation Plan

At State 4b (proof-plan-reviewer), each trusted-base entry must be:
1. Cited by the proof-writer in the actual test artifacts (the proptest
   range shrinks, the Kani harness match arms, the source-lint scan).
2. Reviewed for soundness: does the assumption hold in production?
   - TBR-001: `JournalError::SequenceOverflow` is the unit variant returned
     by `sequenced_run_key` at `keys.rs:485-487`. Verified by source read.
   - TBR-002: `0u64..u64::MAX` is a standard Rust range; the canonical
     proptest at `fjall_keyspace_manifest_tests.rs:129,131` uses the same
     shape. Verified by source read.
   - TBR-003: The packing formula `(u64::from(hi) << 16) | u64::from(lo)`
     reconstructs the u64 value when `hi` and `lo` are the high and low
     u16 halves. This is integer identity, not a model assumption.
   - TBR-004: The harness symbol resolution is compile-time. Any drift
     in `keys::run_event_key` will fail the harness compile.
3. Compensated by independent evidence: every trusted-base entry has at
   least one corresponding proof obligation that exercises the marker.
4. Marked `reviewer_disposition: accepted` or `rejected` with findings.

No trusted-base entry waives behavior-affecting requirements.

## Production Binding Audit (GOD RULE 2 / proof-planner mandate)

The proof-planner skill mandates a `production_binding` mechanism
(`STRONG` / `WEAK_MIRROR` / `WEAK_EXTERN`) for every Verus obligation.
This bead creates **zero Verus obligations** because no production code is
changed — adding a Verus proof obligation would be VACUUM and is forbidden
by GOD RULE 2.

The Kani obligation (`PO-KANI-001`) carries a `production_binding` field
in `proof-obligations.planned.jsonl`:

```yaml
production_binding:
  mechanism: STRONG
  production_path: crates/vb_storage/src/keys.rs
  production_lines: 81-83
  assume_specification_targets:
    - production::run_event_key
    - production::sequenced_run_key
  exec_wrapper_required: false
  drift_detection: compile-time
```

This is the only `production_binding` record in the bead, and it
documents that the Kani harness is implementation-bound to the production
`run_event_key` symbol without a mirror or wrapper.

## Risk Note

TBR-003 (the `SymbolicKeyInputs` packing formula) is the only trusted-base
entry that depends on Kani's symbolic-execution semantics rather than
production code. If Kani changes the `kani::Arbitrary` derive behavior
in a future release, this entry may need a refresh; the validator
(`scripts/kani-list.sh`) and raw Kani log will surface any drift.