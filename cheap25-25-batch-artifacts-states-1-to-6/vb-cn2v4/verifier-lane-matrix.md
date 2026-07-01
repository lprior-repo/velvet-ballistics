# Verifier Lane Matrix: vb-cn2v4 — Keys reject zero RunId (P1 bug)

This matrix is the narrative companion to `verifier-lane-decisions.jsonl`.
It records, for every `(requirement_id, contract_clause, proof_seed_id)`
tuple, the verifier lanes the planner activates or rejects and the
rationale for each decision.

## Lane Profile Summary

- Required lanes: `verus`, `kani`, `proptest` (rust-local + kani + verus
  per femdation directive; proptest is the default-profile companion for
  property/rejection claims).
- Not-applicable lanes: `flux-rs`, `loom`, `miri`, `cargo-fuzz`.

## Decision Matrix

| Req | CC | Seed | Verifier | Applicability | Risk tags driving decision | Primary obligation ID(s) |
|---|---|---|---|---|---|---|
| C1 (encoder-rejects-zero-run) | C-001 | `vb-cn2v4-seed-001` | `verus` | required | `rust_local`, `parser_codec` | `PO-001-VERUS-MIRROR` |
| C1 (encoder-rejects-zero-run) | C-001 | `vb-cn2v4-seed-001` | `kani` | required | `bounded_state`, `rejection` | `PO-003-KANI-SPLIT-HARNESS` |
| C1 (encoder-rejects-zero-run) | C-001 | `vb-cn2v4-seed-001` | `proptest` | required | `property`, `rejection` | `PO-005-PROPTEST-PER-PREFIX` |
| C1 (encoder-rejects-zero-run) | C-001 | `vb-cn2v4-seed-001` | `flux-rs` | not_applicable | (none) | (waiver: WVR-001) |
| C1 (encoder-rejects-zero-run) | C-001 | `vb-cn2v4-seed-001` | `loom` | not_applicable | (none) | (waiver: WVR-002) |
| C1 (encoder-rejects-zero-run) | C-001 | `vb-cn2v4-seed-001` | `miri` | not_applicable | (none) | (waiver: WVR-003) |
| C1 (encoder-rejects-zero-run) | C-001 | `vb-cn2v4-seed-001` | `cargo-fuzz` | not_applicable | (none) | (waiver: WVR-004) |
| C2 (shared-guard-helper) | C-002 | `vb-cn2v4-seed-002` | `proptest` | required (via C1) | `rust_local`, `refactor` | `PO-005-PROPTEST-PER-PREFIX`, `PO-006-PROPTEST-MUTATION` |
| C3 (error-code-mapping-stable) | C-003 | `vb-cn2v4-seed-006` | `proptest` | not_applicable | (none — covered by `tests.rs::symbolic_code_table`) | (no obligation) |
| C6 (kani-harness-split) | C-006 | `vb-cn2v4-seed-004` | `kani` | required | `kani`, `proof-harness`, `refactor` | `PO-003-KANI-SPLIT-HARNESS`, `PO-004-KANI-ORDER-OF-CHECKS` |
| C7 (verus-mirror-invalid-run) | C-007 | `vb-cn2v4-seed-005` | `verus` | required | `verus`, `production_binding`, `parser_codec` | `PO-001-VERUS-MIRROR`, `PO-002-VERUS-DECODER-SYMMETRY` |
| C8 (encoder-decoder-symmetry) | C-008 | `vb-cn2v4-seed-003` | `verus` | required (companion to C1 verus) | `round-trip`, `invariant`, `parser_codec` | `PO-002-VERUS-DECODER-SYMMETRY` |
| C8 (encoder-decoder-symmetry) | C-008 | `vb-cn2v4-seed-003` | `kani` | required (companion to C6 kani) | `bounded_state`, `round-trip` | `PO-003-KANI-SPLIT-HARNESS` |
| C8 (encoder-decoder-symmetry) | C-008 | `vb-cn2v4-seed-003` | `proptest` | required (companion to C5) | `property`, `round-trip` | `PO-005-PROPTEST-PER-PREFIX` |
| workflow-model-error-shift (call-site shift) | (workflow) | `vb-cn2v4-seed-008` | `proptest` | required (via C1/C5) | `behaviour_shift`, `typed-error` | `PO-005-PROPTEST-PER-PREFIX` |

## Per-Lane Rationale

### `verus` (required)

**Required for:** `vb-cn2v4-seed-001`, `vb-cn2v4-seed-005`,
`vb-cn2v4-seed-003`.

**Why required:** The dominant risk shape is `rust_local` +
`parser_codec` (per `references/risk-taxonomy.md` cheat sheet:
`pure_core ⇒ verus + kani + flux-rs (if refinement) + proptest`,
`parser ⇒ cargo-fuzz + kani (rejection) + proptest + verus
(parse spec)`). Verus is the only lane that proves the rejection
for **all** `run` values (unbounded), not just the bounded domain
Kani explores. The Verus mirror at
`verification/verus/extern_vb_storage_keys.rs:199-204` already
exists and is the project's established `extern_*.rs` companion
module pattern; the production-binding gate
`scripts/check-verus-production-binding.sh` exempts
`extern_*.rs` and `production_inner/*` files.

**Production binding:** Mechanism `WEAK_EXTERN`. The mirror
`SpecKeyEncodeError` is a hand-written shadow enum bound to
`crates/vb_storage/src/error/mod.rs::JournalError::InvalidRunId`
via the file's doc-comment header (which cites the production
path/line). The `assume_specification` clauses on the run-bearing
mirror fns are the verified surface.

### `kani` (required)

**Required for:** `vb-cn2v4-seed-001`, `vb-cn2v4-seed-004`,
`vb-cn2v4-seed-003`.

**Why required:** Per `references/risk-taxonomy.md` cheat sheet,
`rejection ⇒ kani` is mandatory for rejection claims. The current
`kani_typed_partitioned_ids.rs::assert_key_contracts` already
exercises the four typed-partitioned-id encoders with
`kani::Arbitrary` symbolic inputs (GOD RULE 1 compliant). The
patch updates the `match` arms to distinguish the rejection path
from the happy path. Kani's CBMC backend provides bounded
symbolic proof with `kani::cover` reachability evidence.

**Companion role:** Companion to `verus` (unbounded) and
`proptest` (property pressure). Kani owns the bounded symbolic
exploration of `SymbolicKeyInputs` (run_value in `[0, 2^32-1]`,
seq_value in `[0, 2^32-1]`, etc.).

### `proptest` (required)

**Required for:** `vb-cn2v4-seed-001`, `vb-cn2v4-seed-002`,
`vb-cn2v4-seed-003`, `vb-cn2v4-seed-006`, `vb-cn2v4-seed-007`,
`vb-cn2v4-seed-008`.

**Why required:** Per `references/risk-taxonomy.md` cheat sheet,
`property ⇒ proptest (+ companion verus/kani for invariants)`.
proptest is the default-profile companion for
`rejection`/`property` claims. The per-prefix
`encoder_rejects_zero_run_id_for_every_prefix` covers all six
public encoder entry points; the mutation-resistance proptest
confirms the guard is not removable.

**Companion role:** Companion to `verus` (unbounded) and
`kani` (bounded symbolic). proptest owns the randomized property
pressure across the 10k case budget.

### `flux-rs` (not_applicable)

**Limitation kind:** `surface_absent`.

**Reason:** The seed `risk_tags` do not include `refinement`,
`ownership`, or `index`. The validation is a single `== 0`
integer compare inside a private helper; encoding it as a Flux
refinement adds the same predicate the helper already enforces.
The contract is a `rejection` claim (not a refinement on a
type-level index/length relationship). Flux is not selected
because the seed's risk shape is purely
`rejection + rust_local + parser_codec` and Flux's decidable
fragment would only encode a trivial refinement that the
helper already enforces structurally.

**Evidence ref:** `domain-model.md#RunId@sha256` and
`workflow-model.md#encoder-decision-workflow@sha256`. The RunId
newtype (`crates/vb_core/src/ids/mod.rs:24-55`) is a structural
newtype around `u64` with no refinement predicate; the
validation lives in the encoder (C2 contract). Flux cannot
add value here.

### `loom` (not_applicable)

**Limitation kind:** `surface_absent`.

**Reason:** Encoders are pure synchronous; no `Send`/`Sync`
boundary; no async; no threads. The encoders are called from
`FjallJournal` methods (`headers.rs`, `snapshots.rs`,
`indexes.rs`, etc.) but those calls are synchronous; the
encoder fns themselves are `pub fn` with no concurrency
surface. Loom's schedule exploration would find no
interleavings to explore.

**Evidence ref:** `boundary-map.md@sha256` enumerates the 19
call sites that invoke the encoders; none of them is async,
none of them crosses a thread boundary. The encoder fns are
declarative byte construction with no shared state.

### `miri` (not_applicable)

**Limitation kind:** `surface_absent`.

**Reason:** `vb_storage` sets `#![forbid(unsafe_code)]` at the
crate root; no FFI, no raw pointers, no `unsafe` block in any
encoder path. The validation cannot introduce undefined
behaviour that Miri would catch.

**Evidence ref:** `boundary-map.md#production-code-requires-edit-encoder-fix-surface@sha256`
and `rg -n "unsafe" crates/vb_storage/src/keys.rs` returns
empty.

### `cargo-fuzz` (not_applicable)

**Limitation kind:** `risk_out_of_scope`.

**Reason:** Encoders are typed-input (typed `RunId`,
`EventSeq`, `StepIdx`, etc.), not byte-stream parsers. The
contract explicitly notes fuzz is "optional/friendly-evidence"
but not required. The rejection claim is per-typed-input, not
per-byte-stream; proptest covers the rejection space
efficiently at 10k cases; Kani provides bounded symbolic
coverage.

The contract's C9 lists workspace tests
(`restate_doctor_key_decode_tests.rs:353-396`) that exercise
the decoder side with hostile byte inputs. If the planner
were to add a fuzz target, it would be a friendly-evidence
surface (per contract C8 lane profile) but is explicitly
optional. The proptest `PO-005-PROPTEST-PER-PREFIX` is
sufficient and is the chosen rust-local lane.

**Evidence ref:** `contract.md#C8-lane-profile-hint-for-proof-planner@sha256`
which states: "No fuzz required by contract; a fuzz target is a
natural friendly-evidence surface for hostile input but is
optional."

## Cross-Lane Discipline

The plan distinguishes lane roles per the
`references/lane-decision-guide.md` cross-lane discipline
section:

- **Verus (primary)** owns the unbounded `SpecKeyEncodeError`
  invariant and the production-binding gate. The mirror body is
  the trusted-base projection recorded in the binding ledger.
- **Kani (companion)** owns the bounded symbolic exploration
  via `kani_typed_partitioned_ids.rs::assert_key_contracts`.
  Kani's `kani::cover` provides reachability evidence for the
  `run_value == 0` rejection arm.
- **proptest (companion)** owns the randomized property
  pressure across all six public encoder entry points.

Each lane has a unique target or distinct evidence:

- Verus target: `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError`
  + `assume_specification` clauses on `run_event_key`,
  `journal_key`, `encode_key`.
- Kani target: `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts`
  (existing, reorganised for split).
- proptest target: `crates/vb_storage/src/keys/tests.rs::encoder_rejects_zero_run_id_for_every_prefix`
  (new, added by test-writer per the contract's C5 18-test flip).

## Self-Audit

- [x] Every `(req, cc, seed, verifier)` tuple in the default
      profile has exactly one lane decision.
- [x] No default-profile verifier has `not_applicable` without
      `non_applicability_evidence_refs` containing at least one
      hash.
- [x] Every `required` lane decision has at least one paired
      `proof-obligation/v1` ID, and the obligation exists in
      `proof-obligations.planned.jsonl`.
- [x] All `decision_reason` strings cite concrete `risk_tags`
      and avoid the weak vocabulary ("not needed", "too hard",
      "covered by other lane", "low risk", "we'll add this
      later").
- [x] All `not_applicable` rows have a typed `limitation_kind`
      (`surface_absent` or `risk_out_of_scope`).
- [x] No two rows duplicate `(req, cc, seed, verifier)` with
      conflicting `applicability`.
- [x] Verus obligations include `production_binding` with
      mechanism `WEAK_EXTERN` and the required fields
      (`production_path`, `production_lines`,
      `assume_specification_targets`).
- [x] Kani obligations include `model_bounds` with
      `unwind`/`input_size`/`mem_high`/`mem_max`.
- [x] proptest obligations include `model_bounds` with `cases`
      and `input_size`.
- [x] All obligations have non-empty `expected_evidence` and
      `command`.
- [x] All obligations have `tool_metadata.version_pin` set.
- [x] Six obligations total (within the 5-7 femdation envelope).
