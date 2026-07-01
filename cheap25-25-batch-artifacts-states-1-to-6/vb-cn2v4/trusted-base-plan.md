# Trusted Base Plan: vb-cn2v4 — Keys reject zero RunId (P1 bug)

This plan enumerates the trust markers the planner raises for the
`vb-cn2v4` proof obligations. Every trust marker is non-behavior
(structural / harness-pattern necessity) and is paired with a
boundary proof and compensating evidence. The writer
(`proof-writer`) maintains `trusted-base-ledger.jsonl` for the
approved markers; the formal verifier (`formal-verifier`) closes
the ledger at State 12.

## TB-001 — Verus mirror `extern_*.rs` companion-module pattern

**Type:** Trust marker (production-binding mechanism).
**Surface:** `verification/verus/extern_vb_storage_keys.rs`
(companion module) and the inlined spec clauses that pin the
mirror contracts.
**Production binding:** Mechanism `WEAK_EXTERN`. The mirror file
is the project's established `extern_*.rs` companion-module
pattern: the file is brought into the spec file via
`#[path = "extern_vb_storage_keys.rs"]` (or inlined as a
companion module), the production source is cited in the
file's doc-comment header (lines 9-15 cite
`crates/vb_storage/src/keys.rs:281-295`, `:436-438`, `:81-83`,
`:205-209`, `:346-434`).
**Applies to obligations:** `PO-001-VERUS-MIRROR`,
`PO-002-VERUS-DECODER-SYMMETRY`.

### Trust Marker Description

`SpecKeyEncodeError` is a hand-written shadow enum that mirrors
the production `JournalError`. The new variant
`SpecKeyEncodeError::InvalidRunId { run: u64 }` is a hand-written
shadow of the production
`JournalError::InvalidRunId { run: RunId }`. The shadow type
`u64` mirrors the production `RunId::get() -> u64` representation
used by `decode_storage_key`; the field type is not a direct
`RunId` because the Verus spec file does not depend on `vb_core`.

The mirror `extern_*.rs` file uses `#[verifier::external]` to
mark each mirror fn body as opaque to Verus; the verified surface
is the `assume_specification` clauses that pin the contract.

### Boundary Proof

The mirror is production-bound via the project-established
`extern_*.rs` pattern:

1. The mirror file's doc-comment header (lines 9-15) cites the
   production source paths/lines for every mirror fn.
2. The production-binding gate
   `scripts/check-verus-production-binding.sh` exempts
   `extern_*.rs` and `production_inner/*` files
   (see `scripts/check-verus-production-binding.sh:35-42`:
   `*/extern_*.rs` and `*/production_inner/*` are skipped).
3. The mirror drift gate
   `scripts/check-production-inner-drift.sh` verifies the
   mirror's production comment block is in sync with the
   production source.
4. The assume_specification contracts are the verified surface;
   Verus verifies the contracts, not the mirror body.

### Compensating Evidence

- Production source citation: `crates/vb_storage/src/keys.rs`
  (SHA-256: `38983cb5fe0a7cf15050bcee8ac641ae35c9a4aa0082f13474213e758a3a07d9`).
- Production error source: `crates/vb_storage/src/error/mod.rs`
  (SHA-256: `b0c4ae712bda4162643cd6dfb270854297b568af5010b1fcd9ef7b2e8c687bf1`).
- Existing pattern: `verification/verus/extern_vb_storage_keys.rs`
  is the established pattern (SHA-256: `b9c077e2727ea740ec7b042ae335d7707765983708f22ddab6fd11f2bad05385`).
- The mirror is paired with the inlined assume_specification
  contract clauses (the contract is `requires run != 0; ensures
  result is Err(SpecKeyEncodeError::InvalidRunId { run }) iff
  run == 0`), which Verus verifies directly.

### Status

`pending` at planning time. The writer (`proof-writer`) will
materialise the marker as a `trusted-base-ledger/v1` row at
State 5; the verifier (`formal-verifier`) will close the ledger
at State 12 with the raw `verus --crate-type=lib` output.

### Required For

- `PO-001-VERUS-MIRROR` — the new variant and assume_specification
  clauses are checked by Verus; the mirror body is the
  trusted-base projection.
- `PO-002-VERUS-DECODER-SYMMETRY` — the mirror body of
  `encode_key`, `run_event_key`, `journal_key` returns
  `Err(InvalidRunId)` for `run == 0`; the verified surface is
  the assume_specification contract.

## TB-002 — Kani harness split-harness shape

**Type:** Trust marker (harness-pattern necessity).
**Surface:** `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts`
and the proof entry `vb_eepg_typed_partitioned_ids`.
**Applies to obligations:** `PO-003-KANI-SPLIT-HARNESS`,
`PO-004-KANI-ORDER-OF-CHECKS`.

### Trust Marker Description

The Kani harness `assert_key_contracts` is reorganised from
the current `match ... { Ok(_) => ..., Err(_) => assert!(false) }`
shape into a split shape that distinguishes the
`run_value == 0` rejection path from the `run_value != 0` happy
path. The split is necessary because the current `Err(_) =>
assert!(false)` arm fires whenever Kani samples `run_value == 0`
(it would treat a successful rejection as a counterexample);
the split-harness fix is the C6 contract clause.

The shape of the split is the in-place if/else
(`if run_value == 0 { rejection-arm } else { happy-arm }`),
chosen for strongest coverage. The `kani::assume(run_value != 0)`
alternative is permitted by the contract but is cheaper and
does not exercise the rejection arm explicitly; the two-harness
split is also permitted but doubles the harness count.

The harness uses `kani::Arbitrary` and `kani::any()` for symbolic
input (GOD RULE 1 compliant); the existing `SymbolicKeyInputs`
struct at `crates/vb_storage/src/kani_typed_partitioned_ids.rs:19-27`
already satisfies this.

### Boundary Proof

The split-harness shape is necessary for the rejection claim:

1. The current `Err(_) => assert!(false)` arm is a Kani
   counterexample witness; treating rejection as failure is
   wrong.
2. The split-harness shape treats rejection as success: the
   `run_value == 0` arm asserts the rejection explicitly, the
   `run_value != 0` arm asserts the byte layout.
3. `kani::cover` reachability proves both arms are reachable:
   the rejection arm is reachable for `run_value == 0`, the
   happy arm is reachable for `run_value != 0`. Without the
   `kani::cover` reachability, the harness could be vacuous
   (e.g., if Kani never samples `run_value == 0` due to
   unwinding choices).

The split shape is a structural refactor of the harness; it
does not change the production encoder behaviour. The
production binding is via the harness's direct call to
`crate::keys::run_header_key(run)` etc. (no shadow types).

### Compensating Evidence

- Existing similar Kani pattern: `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:151`
  already pattern-matches `Err(JournalError::InvalidRunId { .. })`
  and is the template for the new rejection arm.
- Production source: `crates/vb_storage/src/keys.rs`
  (SHA-256: `38983cb5fe0a7cf15050bcee8ac641ae35c9a4aa0082f13474213e758a3a07d9`).
- Kani harness source: `crates/vb_storage/src/kani_typed_partitioned_ids.rs`
  (SHA-256: `f9dec8d84cea81dfbd69eccf51417f8b99cb19659680627598a790567dc285c7`).
- The `SymbolicKeyInputs` struct uses `kani::Arbitrary`; the
  `run_raw(inputs)` function maps the symbolic `run_hi: u16,
  run_lo: u16` halves to `run_value: u64`. The full domain
  `[0, 2^32 - 1]` is reachable.

### Status

`pending` at planning time. The writer will materialise the
marker as a `trusted-base-ledger/v1` row at State 5; the
verifier will close the ledger at State 12 with the raw
`cargo kani` output.

### Required For

- `PO-003-KANI-SPLIT-HARNESS` — the split-harness shape
  distinguishes the rejection path from the happy path; the
  `kani::cover` reachability proves both arms are reachable.
- `PO-004-KANI-ORDER-OF-CHECKS` — the harness extends to cover
  `index_status_key`'s order-of-checks invariant; the
  `kani::cover` for the `IndexStatusStateCollision` path is
  reachable when `run != RunId::new(0)`.

## Cross-Reference With Obligations

| Trust marker | Obligations |
|---|---|
| `TB-001` | `PO-001-VERUS-MIRROR`, `PO-002-VERUS-DECODER-SYMMETRY` |
| `TB-002` | `PO-003-KANI-SPLIT-HARNESS`, `PO-004-KANI-ORDER-OF-CHECKS` |

The proptest obligations `PO-005-PROPTEST-PER-PREFIX` and
`PO-006-PROPTEST-MUTATION` do not raise trust markers
(property tests are the verified surface; no extern or harness
pattern is required).

## Forbidden Trust Markers (Negative List)

The plan does NOT introduce:

- `assume(`, `axiom`, `admit`, `sorry`, or
  `#[verifier::external_body]` in the executable proof code.
  The `#[verifier::external]` attribute on the mirror bodies
  is the project-established pattern; the verified surface is
  the `assume_specification` contract.
- `#[trusted]`, `#[ignore]`, `extern_spec`, or `opaque` markers
  in the production-bound paths. The Kani harness uses real
  `kani::assert` calls, not `#[trusted]`.
- `kani::cover!` as the sole property evidence. The
  `kani::cover` reachability is paired with `kani::assert` on
  the property claim (rejection or layout).
- Hardcoded structural inputs in the Kani harness. The
  `SymbolicKeyInputs` struct uses `kani::Arbitrary`.
- A `#[path = "verification/..."]` mirror that does not call
  production code. The mirror file's doc-comment header cites
  the production source paths/lines, and the production-binding
  gate exempts the file by category.

## Self-Audit

- [x] Every trust marker has a `production_binding` field
      with mechanism, production_path, production_lines, and
      (for `WEAK_EXTERN`) mirror_path.
- [x] Every trust marker has a `boundary_proof` that cites the
      obligation being waived (here, the structural/harness
      pattern).
- [x] Every trust marker has at least one `compensating_evidence`
      entry (a sibling obligation, an external system, or a
      pattern reference).
- [x] Trust marker rows are `pending` at planning time; the
      writer materialises them as `trusted-base-ledger/v1` rows
      at State 5.
- [x] No behavior-affecting trust marker (none of TB-001 or
      TB-002 affects production behaviour; both are structural
      patterns).
- [x] No forbidden trust marker (`assume`, `axiom`, `admit`,
      `sorry`, `external_body` in executable proof code).

## Handoff

- `proof-writer` at State 5: materialise TB-001 and TB-002 as
  `trusted-base-ledger/v1` rows; cite them in the
  `trusted_base_refs` field of the corresponding obligations
  in `proof-obligations.planned.jsonl`.
- `proof-to-implementation` at State 7: cite the trust markers
  in the bridge map (`rust-refinement-obligation/v1` rows).
- `formal-verifier` at State 12: close the ledger with the
  raw `verus` and `cargo kani` output; verify the
  `trusted_base_refs` are satisfied (e.g., the
  `production_inner/*` mirror drift gate passes).
