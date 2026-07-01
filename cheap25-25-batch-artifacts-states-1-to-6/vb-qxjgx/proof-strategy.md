# Proof Strategy — vb-qxjgx

Bead: **vb-qxjgx** — `JournalEvent::StepSucceeded` vs `JournalEvent::SlotWrittenEvent` Record-Kind split (P1 bug).

State: **4** (proof-planner). Owner: `proof-planner`. Handoff target: `proof-plan-reviewer` (State 4b), then `proof-writer` (State 5), then `proof-to-implementation` (State 7), then `formal-verifier` (State 12).

## 1. Scope and Forbidden Surfaces

In scope:
- `RecordKind::StepSucceeded = 33` (new arm; first free journal-family id; see
  `crates/vb_storage/src/records.rs:139` and `records.rs:207-242`).
- `JournalEvent::record_kind()` (events.rs:401-429) routes
  `StepSucceeded { .. } => RecordKind::StepSucceeded` and
  `SlotWrittenEvent { .. } => RecordKind::SlotWritten` (the OR collapse at
  `events.rs:406` is removed).
- `is_known_record_kind` and `validate_kind_family`
  (codec/validation.rs:23-60) include id 33 in the journal family.
- `EnforceKindParity for JournalEvent` (codec/kind_parity.rs:50-64) and
  `validate_journal_event_record_kind` (codec/mod.rs:97-111) accept envelope
  ids `{12, 33}` for `StepSucceeded`; accept envelope id 12 only for
  `SlotWrittenEvent`; reject envelope id 33 for `SlotWrittenEvent` with
  `RecordKindPayloadMismatch { envelope_kind: 33, payload_kind: 12 }`.
- `decode_journal_event` (codec/mod.rs:126-151) round-trips the canonical id
  33 emission and the legacy id 12 + `StepSucceeded` payload.
- `vb_runtime/src/durability_matrix.rs` rows for `set`, `do`, `choose`,
  `for_each`, `parallel`, `collect`, `aggregate`, `repeat`, `wait`, `ask`
  list `RecordKind::StepSucceeded` where the row closes a step
  (8 entry substitutions, see §6).
- `kani_record_kind.rs:252-289` (the `check_journal_family_exhaustive` and
  `check_all_existing_kinds_known` harnesses) include id 33.
- `proptests.rs:62,148` and `proptest_storage.rs:115,126` id→`RecordKind`
  generators map id 33 to `RecordKind::StepSucceeded` and the selection array
  includes 33.
- `flux_validation.rs:14,33` literal sets include 33 (DISABLED module;
  literal-sync only per vb-b8i8f).

Forbidden (per contract clauses PRE-005, POST-006, INV-005, INV-007, INV-010,
NON-GOALS):
- **No schema version bump.** `CURRENT_SCHEMA_VERSION = 1` is pinned at
  `crates/vb_storage/src/constants.rs:58` and asserted by `tests.rs:3925` and
  `tests.rs:4223`. The bead must not raise it.
- **No new `JournalError` variant.** The parity gate's only mismatch error
  is `RecordKindPayloadMismatch { envelope_kind, payload_kind }`; id 33 adds
  new envelope/payload pairs but no new variant.
- **No runtime config flag / compat-mode toggle.** The legacy envelope-12
  tolerance is encoded in the type system via
  `LegacyEnvelopeBinding::Legacy { accepted_ids: &[12, 33] }`; the parity
  gate honors both ids without a runtime check.
- **No id-keyed recovery counters.** `steps_succeeded` and `slots_written`
  at `recovery/replay/summary/apply.rs:32-52` are variant-keyed; the wire-id
  split does not alter the count.
- **No postcard enum wire-byte regression for `RecordKind` as a newtype.**
  `restate_postcard_newtype_compat_tests.rs` goldens are baseline-checked
  during implementation; if the new variant shifts serde enum encoding, the
  test must be re-baselined with reviewer approval (out of scope for this
  plan, tracked in `OPEN-Q-A3`).

## 2. Risk Classification

Per `references/risk-taxonomy.md` and the proof seeds' `risk_tags`:

| Seed(s) | Risk class | Default profile |
|---|---|---|
| PS-002, PS-013, PS-014, PS-015, PS-006, PS-017, PS-008, PS-009, PS-020, PS-024 | `field_sensitivity` | `proptest` + `kani` |
| PS-003, PS-004, PS-005, PS-007, PS-010, PS-016, PS-018, PS-022, PS-023 | `rejection` | `kani` + `proptest` |
| PS-011 (flux literals; module DISABLED) | `refinement` (not in scope) | `flux-rs` (blocked_tooling) |
| PS-019, PS-021 (static scans; non-behavior) | not in default profile | waived via static-scan |

The contract chose the **rust-local + kani + proptest + flux** lane profile
(see `delivery-scope.jsonl` and `codebase-map.md`); Verus is intentionally
out of scope for this bead. The deviation is documented per
requirement-contract-clause pair via `not_applicable` lane decisions in
`verifier-lane-decisions.jsonl` (see §5).

## 3. Lane Selection

Primary lanes (per `references/verifier-trigger-matrix.md`):

| Risk class | Primary | Companion | Rationale |
|---|---|---|---|
| `field_sensitivity` (bijection, sensitivity of the codec to id choice) | `kani` | `proptest` | Kani proves the bijection across all `kani::any()` u16 ids; proptest pressure-tests the same property across strategy-generated inputs. |
| `rejection` (parity gate admits/rejects envelope-payload pairs) | `kani` | `proptest` | Kani covers the full envelope-id grid via `kani::any()`; proptest enumerates the same grid with random sequences. |
| `refinement` (flux literals) | `flux-rs` | none | Module is `DISABLED` per vb-b8i8f; the literal-sync is a `proptest` obligation on the static source. `flux-rs` lane emits `blocked_tooling` with `tooling_acquisition_ref: BEAD-TOOL-FLUX-RS-INSTALL`. |

Secondary lanes (consulted when default profile is missing the primary):

- **proptest on the durability matrix** (POST-008, POST-024) — the matrix
  is a `&[DurabilityRow]` constant; the proptest generator enumerates each
  row's `journal_events` slice and asserts the post-fix `StepSucceeded`
  ordering. Kani would be redundant because the data is a compile-time
  constant.

- **proptest on recovery counters** (POST-009, INV-008) — a synthetic
  replay sequence mixing envelope-id 12 and envelope-id 33 `StepSucceeded`
  events yields the same `steps_succeeded` and `slots_written` totals as
  the pre-fix sequence.

## 4. Obligation Plan (7 rows)

The 7 obligations satisfy the contract's 24 proof seeds at the
`requirement_id` and `contract_clause` granularity. Each obligation binds
to a production symbol (no model modules, no `verification/` shadow types
disconnected from production).

| ID | Verifier | Risk | Target | Contract clause(s) | Seeds covered |
|---|---|---|---|---|---|
| PO-QXJGX-001 | kani | field_sensitivity | `crate::records::RecordKind::id` | POST-001, INV-002 | PS-001, PS-014 |
| PO-QXJGX-002 | kani | field_sensitivity | `crate::events::JournalEvent::record_kind` | POST-002, INV-001 | PS-002, PS-013 |
| PO-QXJGX-003 | kani | rejection | `crate::codec::validation::is_known_record_kind` and `crate::codec::validation::validate_kind_family` | POST-003, POST-004, POST-010, INV-003 | PS-003, PS-004, PS-010, PS-015 |
| PO-QXJGX-004 | kani | rejection | `crate::codec::EnforceKindParity::enforce_kind_parity` (JournalEvent impl) and `crate::codec::validate_journal_event_record_kind` | POST-005, POST-007, INV-004, ERR-006 | PS-005, PS-007, PS-016, PS-023 |
| PO-QXJGX-005 | kani | rejection | `crate::codec::decode_journal_event` | POST-006, POST-013, INV-005 | PS-006, PS-017, PS-022 |
| PO-QXJGX-006 | proptest | field_sensitivity | `crate::records::RecordKind::id` and `crate::events::JournalEvent::record_kind` (bijection + variant-keyed recovery) | POST-002, POST-009, INV-001, INV-008 | PS-002, PS-009, PS-013, PS-020 |
| PO-QXJGX-007 | proptest | field_sensitivity | `crate::runtime::durability_matrix::DURABILITY_MATRIX` and `crate::codec::validation::validate_schema_version` | POST-008, POST-012, PRE-007, INV-006, POST-011 (literal-sync) | PS-008, PS-011, PS-012, PS-018, PS-024 |

The flux-rs lane is `blocked_tooling` (one row in
`verifier-lane-decisions.jsonl`) and cites vb-b8i8f. The static-scan lanes
(PS-019, PS-021) are static-only obligations owned by `holzman-rust` and
`black-hat-reviewer`; they are NOT carried as proof-obligation rows because
they are not behavior-affecting and have no formal-verifier execution
boundary.

## 5. Verus Deviation and `not_applicable` Lane Decisions

The contract does not require Verus. Two `not_applicable` lane decisions
document the deviation:

- `VLD-QXJGX-VERUS-001` — `verus`, `not_applicable`, `limitation_kind:
  risk_out_of_scope` (the contract's risk profile is
  `rust-local + kani + proptest + flux`; no Verus spec is in scope). Evidence
  refs: contract.md SHA-256 and delivery-scope.jsonl SHA-256 (see
  `verifier-lane-decisions.jsonl`).
- `VLD-QXJGX-FLUX-001` — `flux-rs`, `blocked_tooling`, `limitation_kind:
  external_dependency_unavoidable`, `tooling_acquisition_ref:
  BEAD-TOOL-FLUX-RS-INSTALL`. Evidence refs:
  `codec/mod.rs:184-186` (DISABLED module) and vb-b8i8f closure. The
  literal-sync is enforced by PO-QXJGX-007 (proptest on the flux_validation
  source).

## 6. Durability Matrix Substitution (8 entry sites)

The 8 line sites in `vb_runtime/src/durability_matrix.rs` that currently
encode `RecordKind::SlotWritten` for a step-closing emission are:

| Line | Primitive | Pre-fix entry | Post-fix entry |
|---|---|---|---|
| 75 | set | `StepStarted, SlotWritten` | `StepStarted, StepSucceeded` |
| 89 | do | `StepStarted, ActionScheduled, ActionCompleted, SlotWritten` | `StepStarted, ActionScheduled, ActionCompleted, StepSucceeded` |
| 100 | choose | `StepStarted, SlotWritten` | `StepStarted, StepSucceeded` |
| 110 | for_each | `StepStarted, SlotWritten` | `StepStarted, StepSucceeded` |
| 120 | parallel | `StepStarted, SlotWritten` | `StepStarted, StepSucceeded` |
| 132, 133 | collect | `StepStarted, SlotWritten, SlotWritten` | `StepStarted, StepSucceeded, StepSucceeded` |
| 146, 147 | aggregate | `StepStarted, SlotWritten, SlotWritten` | `StepStarted, StepSucceeded, StepSucceeded` |
| 158 | repeat | `StepStarted, SlotWritten` | `StepStarted, StepSucceeded` |
| 171 | wait | `StepStarted, WaitScheduled, SlotWritten` | `StepStarted, WaitScheduled, StepSucceeded` |
| 186, 187 | ask | `StepStarted, AskScheduled, AskAnswered, SlotWritten, SlotWritten` | `StepStarted, AskScheduled, AskAnswered, StepSucceeded, StepSucceeded` |

(Line numbers refer to the pre-fix file; the post-fix file is line-shifted
by +1 per new variant declaration.) The `finish` primitive (line 198)
already uses `RunFinished` and is unaffected. The
`durability_matrix/tests.rs:50-51,63,73,84,94` assertions must update in
lockstep — `durability_matrix/tests.rs` is the canonical evidence site for
PO-QXJGX-007's proptest artifact.

## 7. Resource Governance and Tooling Pins

| Tool | Pin | Resource budget |
|---|---|---|
| `cargo-kani` | `0.67.0` (CBMC) | `-j 1`, `mem_high=20G`, `mem_max=24G`, `unwind=8` |
| `proptest` | `1.5` | `PROPTEST_CASES=10000`, `input_size=1024` |
| `cargo-flux` | `flux@nightly-2026-02-15` (per `references/resource-governance.md`; not used in this bead) | n/a (`blocked_tooling`) |
| `verus` | not used (per §5) | n/a (`not_applicable`) |

All Kani commands use `kani::any()` / `kani::any_where()` for symbolic
input and `kani::assert` for property assertions (no `cover!` as the sole
evidence). All proptest commands use `PROPTEST_CASES=10000` and an
explicit `prop_assume!` anti-invariant. The Kani harnesses extend the
existing harness groups at `kani_record_kind.rs:265-289` and add a new
parity-gate harness at `codec/kind_parity.rs` (post-fix).

## 8. Backward Compatibility Decision (per contract)

The contract chose **LEGACY ENVELOPE-12 TOLERANCE** (PRE-005, POST-005,
POST-006, INV-005, INV-007):

- The writer ALWAYS emits envelope id 33 for `StepSucceeded`
  (`JournalEvent::record_kind().id() == 33` is the canonical projection).
- The reader accepts BOTH envelope id 33 and envelope id 12 for a
  `StepSucceeded` payload via the typed
  `LegacyEnvelopeBinding::Legacy { accepted_ids: &[12, 33] }` accessor.
- `CURRENT_SCHEMA_VERSION` remains 1; no migration is added.
- `validate_journal_event_record_kind` (codec/mod.rs:97-111) and
  `EnforceKindParity for JournalEvent` (codec/kind_parity.rs:50-64) MUST
  both implement the same acceptance set; the parity gate is the only
  enforcement point.

The open questions A1, A2, A3 (see contract.md) are tracked but are
out of scope for this proof plan. They are tested in the
`OPEN-Q-A1/A2/A3` rows of the traceability matrix
(`traceability-matrix.jsonl`); their `behavior_affecting` is false and
they are owned by `manual-qa` and `holzman-rust`.

## 9. Handoff to State 4b

Artifacts emitted under `.beads/vb-qxjgx/`:

- `proof-strategy.md` (this file)
- `verifier-lane-decisions.jsonl` (proof-obligation/v1 schema; 12 rows)
- `verifier-lane-matrix.md` (human-readable summary of the decisions)
- `proof-coverage-matrix.md` (requirement-clause ↔ verifier ↔ obligation
  cross-table)
- `proof-obligations.planned.jsonl` (proof-obligation/v1 schema; 7 rows)
- `trusted-base-plan.md` (model-bound and trust-marker ledger)
- `waiver-candidates.jsonl` (zero rows; this bead has no behavior-affecting
  waiver)

`proof-plan-reviewer` (State 4b) will disposition each lane decision; the
planner does not self-approve. The reviewer writes
`verifier-lane-review/v1` and `proof-plan-review.md`.

The planner never claims proof success. `PASS` is the formal-verifier's
exclusive write authority; `status: planned` is the only value this plan
emits for `proof-obligation/v1.status`.
