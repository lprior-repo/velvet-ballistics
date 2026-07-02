# Trusted Base Plan — vb-5bqmr

## Bead

`vb-5bqmr` — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)

## Purpose

This plan enumerates every trust marker, model reduction, or trusted
abstraction that the planned proof obligations introduce or rely on. Each
row is keyed by an ID that appears in `proof-obligations.planned.jsonl`
under `trusted_base_refs`. The proof-writer will materialise these into
`trusted-base-ledger/v1` rows; the formal-verifier closes them at State 12.

The bead is small (7 obligations across 4 lanes). Trust markers are minimal:

1. **No `assume(...)` / `axiom` / `admit` / `external_body`** in the planned
   Verus proof code. The `proof_decode_three_arms_partition` body uses only
   standard Verus idioms (`assert(...)`, `assert by (...)`, `reveal`,
   `use_type_invariant`). The STRONG production binding via
   `#[path = "crates/vb_storage/src/slot_extra.rs"]` + `assume_specification`
   ensures the exec fn body is the production body.
2. **No `#[trusted]` / `#[ignore]` / `extern_spec` / `opaque`** broadening by
   the Flux-RS obligation. The Flux refinement annotations live on the
   existing public constants and the existing `decode_slot_written_extra`
   signature; no new trusted primitives are introduced.
3. **Kani `kani::cover!` is paired with `kani::assert` or function-contract
   postconditions.** Reachability is non-vacuity evidence, not property
   satisfaction. See `references/implementation-binding.md` Rule 3.
4. **Proptest strategies are non-vacuous** and each harness has an explicit
   anti-invariant (`result != Ok(LegacyFrameExtra(_))`,
   `assert!(matches!(...))` instead of `is_ok()` / `is_err()`).

## Trust marker ledger (planned)

### TB-KANI-001-cover-reachability

- **Source obligation**: `PO-KANI-001` (C-DEC-002 + C-NEG-004/005)
- **Trust marker**: `kani::cover!(bytes[4] == 0x02)` and
  `kani::cover!(bytes[4] == 0xFF)` paired with `kani::assert` on the
  `VersionMismatch { found }` exact output.
- **Reason**: cover! is non-vacuity evidence that the unknown-version
  branch is reachable through the symbolic input. Required because the
  harness uses `kani::assume(bytes[4] != 0x01)` to constrain the input
  space; without paired cover!, the harness would be vacuously satisfied
  on the empty set.
- **Compensating evidence**: the harness also asserts the **exact**
  VersionMismatch payload via `assert_eq!(found, bytes[4])`, which is the
  satisfaction evidence.
- **Scope**: PO-KANI-001 harness only.
- **Behavior-affecting**: false (this is a model-reduction row, not a
  behavior waiver).
- **Owner**: proof-writer (State 5) materialises into
  `trusted-base-ledger.jsonl`.
- **Expiry**: 2026-12-31 (re-justification on toolchain or harness change).

### TB-KANI-002-alloc-counter

- **Source obligation**: `PO-KANI-002` (C-DEC-004 + C-NEG-006)
- **Trust marker**: a manually-incremented `u32 allocations_count` counter
  inside the Kani harness, asserted to be zero on the legacy arm.
- **Reason**: Kani's `--mem-predicates` flag enables memory-safety checks
  but does NOT count Vec/Box allocations. The C-NEG-006 zero-allocation
  invariant requires a custom counter; the counter itself is harness
  instrumentation, not production code.
- **Compensating evidence**: the legacy arm is asserted via
  `assert!(allocations_count == 0)`, AND the classification is asserted
  to be `LegacyFrameExtra`; together these imply the production legacy arm
  (which uses only `split_at_checked` and `split_first`) allocated zero
  bytes. A second layer of evidence is the proptest round-trip in
  PO-PROP-002 which exercises the legacy arm under non-mock conditions.
- **Scope**: PO-KANI-002 harness only.
- **Behavior-affecting**: false (instrumentation row).
- **Owner**: proof-writer.
- **Expiry**: 2026-12-31.

### TB-KANI-002-cover-reachability

- **Source obligation**: `PO-KANI-002` (C-DEC-004)
- **Trust marker**: three paired `kani::cover!` entries — one each for the
  v1-decode, unknown-version, and legacy classification — proving every
  arm is reachable through the symbolic input space.
- **Reason**: the partition invariant (C-DEC-004) requires
  exhaustion; reachability evidence demonstrates the partition is not
  vacuously satisfied on an empty input set.
- **Compensating evidence**: the harness asserts the partition itself
  (`count_ones() == 1` on the four-arm indicator vector) via `kani::assert`.
- **Scope**: PO-KANI-002 harness only.
- **Behavior-affecting**: false.
- **Owner**: proof-writer.
- **Expiry**: 2026-12-31.

### TB-PROP-003-tracing-capture

- **Source obligation**: `PO-PROP-003` (C-REC-002 + C-RUN-002)
- **Trust marker**: a `tracing_subscriber::fmt::layer().with_writer(buf)`
  capture sink in test scope, replacing the default stdout writer.
- **Reason**: the C-REC-002 / C-RUN-002 claim requires asserting that the
  `tracing::warn!` event is emitted with the right fields (`slot`,
  `found` for hydrate; `slot`, `seq`, `found` for collect). The capture
  sink is test-scope-only harness instrumentation; production tracing is
  unaffected.
- **Compensating evidence**: the harness asserts both the returned error
  variant (exact-match) AND the captured warn event content (string
  contains `"slot="`, `"found=0x02"`). If the tracing init fails the test
  fails loudly.
- **Scope**: PO-PROP-003 harness only.
- **Behavior-affecting**: false (test instrumentation).
- **Owner**: proof-writer.
- **Expiry**: 2026-12-31.

### TB-PROP-003-compile-time-exhaustiveness

- **Source obligation**: `PO-PROP-003` (C-REC-001 + C-RUN-001 + C-FOR-001
  + C-FOR-002 + C-REC-004 + C-RUN-004)
- **Trust marker**: the existing `recovery_unit_tests.rs:1149-1172`
  `_exhaustive_match` test which must remain green unchanged.
- **Reason**: the C-REC-004 (RecoveryError not widened) and C-RUN-004
  (CollectExtraHydrationFailureKind gains exactly one arm) invariants
  are enforced by a compile-time match-exhaustiveness test that the
  planner does not modify. If the bead accidentally widens
  `RecoveryError`, this test breaks at `cargo build`.
- **Compensating evidence**: the test is already a pre-existing assertion;
  the planner does not introduce new trust, it relies on an existing
  source of truth.
- **Scope**: `recovery_unit_tests.rs:1149-1172` and the new exhaustive
  match arms in `hydrate.rs:209-235` + `collect.rs:256-273`.
- **Behavior-affecting**: false (compile-time check, not a runtime waiver).
- **Owner**: holzman-rust (State 11) verifies the test remains green;
  proof-writer records this in the ledger.
- **Expiry**: 2026-12-31.

## Production-binding audit (GOD RULE 2)

The Verus obligation (`PO-VERUS-001`) binds via the **STRONG** mechanism:

```yaml
production_binding:
  mechanism: STRONG
  production_path: crates/vb_storage/src/slot_extra.rs
  production_lines: 60-69 (NEW body)
  assume_specification_targets:
    - production::decode_slot_written_extra
  exec_wrapper_required: true
  drift_detection: build-time
  drift_gate_script: scripts/check-verus-production-binding.sh
```

The spec file is `verification/verus/vb_storage/slot_extra_decode_partition.rs`.
It MUST contain:

1. `#[path = "crates/vb_storage/src/slot_extra.rs"] mod production;`
   (direct production source inclusion).
2. `assume_specification[ production::decode_slot_written_extra ](...)`
   bridge attaching a spec contract to the production exec fn.
3. NO `#[verifier::external_body]`, `assume(...)`, `axiom`, or `admit`
   in the proof body.
4. NO `WEAK_MIRROR` fallback; the STRONG bind is required because the
   decoder body is the production body.

The drift-gate is `bash scripts/check-verus-production-binding.sh` from the
repo root, run by `moon ci` before any proof evidence is accepted. A
failure of the drift gate re-emits the spec with the new production body.

## Anti-laundering notes

- The Verus `proof_decode_three_arms_partition` proof body uses only
  standard Verus idioms: `assert(...)`, `assert by (...)`, `use_type_invariant`,
  and `reveal(...)`. No `assume(...)` short-circuits the obligation.
- The Kani harnesses use `kani::any()` and `kani::any_where()` for symbolic
  bytes; no hardcoded structural inputs (GOD RULE 1).
- The Flux refinement annotations live on existing public symbols; no
  `#[trusted]` / `#[ignore]` / `extern_spec` / `opaque` broadening.
- The proptest strategies are non-vacuous (`prop::collection::vec(any::<u8>(),
  0..=256)`) and each harness has an explicit anti-invariant.
- No behavior-affecting waiver is emitted. The waiver-candidates.jsonl file
  contains only a single non-behavior row (see below).

## Cross-reference

- `references/implementation-binding.md` — Rule 3 (Kani cover! discipline),
  Rule 9 (proptest strategy + anti-invariant discipline).
- `references/plan-quality-gates.md` — Gate 4 (implementation binding),
  Gate 7 (waiver discipline), Gate 8 (trust marker ledger).
- `references/tooling-availability-gate.md` — Verus / Kani / Flux /
  proptest detection commands.

## Owner summary

| Trust marker | Owner (State 5+ → State 12) | Status |
|---|---|---|
| TB-KANI-001-cover-reachability | proof-writer → formal-verifier | planned |
| TB-KANI-002-alloc-counter | proof-writer → formal-verifier | planned |
| TB-KANI-002-cover-reachability | proof-writer → formal-verifier | planned |
| TB-PROP-003-tracing-capture | proof-writer → formal-verifier | planned |
| TB-PROP-003-compile-time-exhaustiveness | holzman-rust → formal-verifier | planned (re-uses existing test) |

No new unsafe / unproved axiom / disabled check is introduced. The bead is
cleanly inside the `forbid(unsafe_code)` boundary and the four-lane proof
surface.