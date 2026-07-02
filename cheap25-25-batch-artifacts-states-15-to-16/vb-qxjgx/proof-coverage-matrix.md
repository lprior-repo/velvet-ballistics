# Proof Coverage Matrix — vb-qxjgx

Cross-reference of every contract clause, proof seed, lane decision, and
proof obligation. Each row shows the verification chain end-to-end. Coverage
is judged at the `contract_clause` granularity.

## 1. Contract Clause Coverage

| Contract clause | Seed(s) | Lane decision(s) | Obligation(s) | Verifier | Risk | Coverage status |
|---|---|---|---|---|---|---|
| PRE-001 (RecordKind is `repr(u16)` non_exhaustive) | PS-001, PS-014 | VLD-QXJGX-001 | PO-QXJGX-001 | kani | field_sensitivity | Covered by PO-QXJGX-001; default-profile proptest missing (Major) |
| PRE-002 (JournalEvent OR-collapse removed) | PS-002, PS-013 | VLD-QXJGX-002 | PO-QXJGX-002 | kani | field_sensitivity | Covered by PO-QXJGX-002; default-profile proptest missing (Major) |
| PRE-003 (is_known/validate_kind_family pure const fn) | PS-003, PS-004, PS-015 | VLD-QXJGX-003, VLD-QXJGX-004 | PO-QXJGX-003 | kani | rejection | Covered by PO-QXJGX-003; default-profile proptest missing (Major) |
| PRE-004 (decode_journal_event is the only decode entry) | PS-022, PS-023 | (covered by VLD-QXJGX-005/006/007) | PO-QXJGX-004, PO-QXJGX-005 | kani | rejection | Covered by PO-QXJGX-004 and PO-QXJGX-005; default-profile proptest missing (Major) |
| PRE-005 (CURRENT_SCHEMA_VERSION=1 pinned) | PS-018 | VLD-QXJGX-011 | PO-QXJGX-007 | proptest | field_sensitivity | Covered by PO-QXJGX-007 second proptest (validate_schema_version) |
| PRE-006 (no forbidden constructs) | PS-021 | (static scan; holzman-rust) | n/a (out of proof scope) | static-scan | not in default profile | Out of scope for proof; holzman-rust owns the static-scan obligation |
| PRE-007 (durability matrix authoritative) | PS-008, PS-024 | VLD-QXJGX-009 | PO-QXJGX-007 | proptest | field_sensitivity | Covered by PO-QXJGX-007 first proptest (matrix rows) |
| POST-001 (StepSucceeded=33) | PS-001, PS-014 | VLD-QXJGX-001 | PO-QXJGX-001 | kani | field_sensitivity | Covered by PO-QXJGX-001 |
| POST-002 (record_kind projection split) | PS-002, PS-013 | VLD-QXJGX-002, VLD-QXJGX-008 | PO-QXJGX-002, PO-QXJGX-006 | kani + proptest | field_sensitivity | Covered by PO-QXJGX-002 (kani) and PO-QXJGX-006 (proptest on bijection) |
| POST-003 (is_known(33)=true) | PS-003, PS-015 | VLD-QXJGX-003 | PO-QXJGX-003 | kani | rejection | Covered by PO-QXJGX-003 (extends check_journal_family_exhaustive) |
| POST-004 (validate_kind_family(33)=Ok for journal) | PS-004, PS-015 | VLD-QXJGX-004 | PO-QXJGX-003 | kani | rejection | Covered by PO-QXJGX-003 |
| POST-005 (parity {12,33} for StepSucceeded) | PS-005, PS-016, PS-023 | VLD-QXJGX-005 | PO-QXJGX-004 | kani | rejection | Covered by PO-QXJGX-004 |
| POST-006 (round-trip id-33 and id-12) | PS-006, PS-016 | VLD-QXJGX-007 | PO-QXJGX-005 | kani | rejection | Covered by PO-QXJGX-005 |
| POST-007 (SlotWrittenEvent+id-33 rejected) | PS-007, PS-023 | VLD-QXJGX-006 | PO-QXJGX-004 | kani | rejection | Covered by PO-QXJGX-004 (cross-bind rejection grid) |
| POST-008 (durability matrix step-closing rows) | PS-008, PS-024 | VLD-QXJGX-009 | PO-QXJGX-007 | proptest | field_sensitivity | Covered by PO-QXJGX-007 |
| POST-009 (recovery counters variant-keyed) | PS-009, PS-020 | VLD-QXJGX-010 | PO-QXJGX-006 | proptest | field_sensitivity | Covered by PO-QXJGX-006 second proptest |
| POST-010 (kani family set includes 33) | PS-010, PS-015 | VLD-QXJGX-003 | PO-QXJGX-003 | kani | rejection | Covered by PO-QXJGX-003 |
| POST-011 (flux literals include 33) | PS-011 | VLD-QXJGX-012 | PO-QXJGX-007 | proptest (literal-sync) | field_sensitivity | Covered by PO-QXJGX-007 third proptest; flux-rs is blocked_tooling |
| POST-012 (proptest id→RecordKind generators include 33) | PS-012 | (covered by PO-QXJGX-006 and PO-QXJGX-007) | PO-QXJGX-006, PO-QXJGX-007 | proptest | field_sensitivity | Covered by both; generators are inside the proptest source files |
| POST-013 (sequence identity check) | PS-022 | VLD-QXJGX-007 | PO-QXJGX-005 | kani | rejection | Covered by PO-QXJGX-005 (sequence-identity branch) |
| INV-001 (one-to-one projection) | PS-013, PS-002 | VLD-QXJGX-002, VLD-QXJGX-008 | PO-QXJGX-002, PO-QXJGX-006 | kani + proptest | field_sensitivity | Covered |
| INV-002 (closed id set) | PS-014 | VLD-QXJGX-001 | PO-QXJGX-001 | kani | field_sensitivity | Covered by PO-QXJGX-001 |
| INV-003 (id/known/family agree) | PS-015 | VLD-QXJGX-003 | PO-QXJGX-003 | kani | rejection | Covered by PO-QXJGX-003 |
| INV-004 (parity acceptance set) | PS-016, PS-023 | VLD-QXJGX-005 | PO-QXJGX-004 | kani | rejection | Covered by PO-QXJGX-004 |
| INV-005 (writer emits canonical id) | PS-017 | VLD-QXJGX-007 | PO-QXJGX-005 | kani | rejection | Covered by PO-QXJGX-005 (canonical encode branch) |
| INV-006 (schema version pinned) | PS-018 | VLD-QXJGX-011 | PO-QXJGX-007 | proptest | field_sensitivity | Covered by PO-QXJGX-007 second proptest |
| INV-007 (no runtime config flag) | PS-019 | (static scan; out of proof scope) | n/a | static-scan | not in default profile | Out of scope for proof; holzman-rust + black-hat-reviewer own the static scan |
| INV-008 (counters variant-keyed) | PS-020 | VLD-QXJGX-010 | PO-QXJGX-006 | proptest | field_sensitivity | Covered by PO-QXJGX-006 |
| INV-009 (no forbidden constructs) | PS-021 | (static scan; out of proof scope) | n/a | static-scan | not in default profile | Out of scope for proof; holzman-rust owns the static scan |
| INV-010 (out-of-scope additions forbidden) | (compliance) | (compliance) | n/a | not_applicable | not in default profile | Compliance obligation, no formal-verifier execution boundary |
| ERR-006 (RecordKindPayloadMismatch literal) | PS-023 | VLD-QXJGX-005, VLD-QXJGX-006 | PO-QXJGX-004 | kani | rejection | Covered by PO-QXJGX-004 (literal envelope_kind/payload_kind asserted) |
| ERR-005 (RecordKindFamilyMismatch) | (covered) | VLD-QXJGX-004 | PO-QXJGX-003 | kani | rejection | Covered by PO-QXJGX-003 |
| ERR-004 (UnknownRecordKind for kind outside closed set) | (covered) | VLD-QXJGX-003 | PO-QXJGX-003 | kani | rejection | Covered by PO-QXJGX-003 (negative grid) |
| ERR-002 / ERR-003 (schema version) | PS-018 | VLD-QXJGX-011 | PO-QXJGX-007 | proptest | field_sensitivity | Covered by PO-QXJGX-007 second proptest |
| OPEN-Q-A1 (ephemeral journal audit) | PS-019, PS-020 | (manual-qa) | n/a | manual-qa | not in default profile | Out of scope; manual-qa-ephemeral-journal-audit.md is the evidence |
| OPEN-Q-A2 (id-33 reservation audit) | PS-019 | (manual-qa) | n/a | manual-qa | not in default profile | Out of scope; migration-id-reservation-audit.md is the evidence |
| OPEN-Q-A3 (postcard wire-byte audit) | PS-019 | (manual-qa) | n/a | manual-qa | not in default profile | Out of scope; restate_postcard_newtype_compat_tests.rs re-baseline is the evidence |

## 2. Verifier Coverage by Risk Class

| Risk class | Required lanes | Plan's lanes | Status |
|---|---|---|---|
| `field_sensitivity` | kani + proptest | kani (PO-QXJGX-001, PO-QXJGX-002, PO-QXJGX-007) and proptest (PO-QXJGX-006, PO-QXJGX-007) | Partial — proptest missing for REQ-001/POST-001 and REQ-002/POST-002 (Major, accepted); kani missing for REQ-008/POST-008 and REQ-009/POST-009 (Major, accepted) |
| `rejection` | kani + proptest | kani (PO-QXJGX-003, PO-QXJGX-004, PO-QXJGX-005) | Partial — proptest missing for REQ-003/POST-003, REQ-005/POST-005, REQ-006/POST-006 (Major, accepted) |
| `refinement` (flux) | flux-rs | flux-rs (VLD-QXJGX-012 blocked_tooling); proptest literal-sync (PO-QXJGX-007) | Covered as literal-sync; flux-rs blocked per vb-b8i8f |

The 7 Major findings are non-blocker default-profile coverage gaps. The
contract's risk profile is `rust-local + kani + proptest + flux`, not the
default `kani + proptest`. The plan documents the deviation via
`proof-coverage-matrix.md` §3 and `trusted-base-plan.md` §2.

## 3. Deviation from Default Profile (Non-Blocker)

| Deviation | Reason | Documented in |
|---|---|---|
| Verus obligations omitted | Contract's verifier profile is rust-local + kani + proptest + flux; no Verus spec is in scope (rust-contract.md NON-GOALS) | proof-strategy.md §5 |
| `rejection` proptest missing for POST-003/004/005/006/007 | These contract clauses are pure const-fn / match-expression properties; proptest would re-test the same property with redundant coverage. The kani harness already exercises the full u16 grid via `kani::any()`. | proof-coverage-matrix.md §1 (Major findings accepted) |
| `field_sensitivity` proptest missing for POST-001/002 | Same reasoning: kani `kani::any()` over u16 covers the full id space; proptest would shrink the same property without finding new counterexamples. | proof-coverage-matrix.md §1 |
| `field_sensitivity` kani missing for POST-008/009 | The durability matrix is a compile-time `&[DurabilityRow]` constant; Kani's symbolic execution provides no additional evidence beyond enumerating the constant. Proptest with a closed-shape strategy generator covers the same property. | proof-coverage-matrix.md §1 |
| flux-rs `blocked_tooling` (POST-011) | vb-b8i8f: flux_rs not in workspace; the module is commented out at `codec/mod.rs:184-186`. Literal-sync is enforced by proptest PO-QXJGX-007 third proptest. | trusted-base-plan.md §3, VLD-QXJGX-012 |

## 4. Production Binding Map

| Obligation | Production target | Production path | Binding mechanism |
|---|---|---|---|
| PO-QXJGX-001 | `crate::records::RecordKind::id` | `crates/vb_storage/src/records.rs:207-242` | STRONG (kani harness in `kani_record_kind.rs` calls the production function directly) |
| PO-QXJGX-002 | `crate::events::JournalEvent::record_kind` | `crates/vb_storage/src/events.rs:401-429` | STRONG (kani harness constructs each `JournalEvent` variant and calls `record_kind()`) |
| PO-QXJGX-003 | `crate::codec::validation::is_known_record_kind` and `validate_kind_family` | `crates/vb_storage/src/codec/validation.rs:23-60` | STRONG (kani harness calls the production functions) |
| PO-QXJGX-004 | `crate::codec::EnforceKindParity::enforce_kind_parity` and `validate_journal_event_record_kind` | `crates/vb_storage/src/codec/kind_parity.rs:50-64` and `crates/vb_storage/src/codec/mod.rs:97-111` | STRONG (kani harness exercises both impls) |
| PO-QXJGX-005 | `crate::codec::decode_journal_event` | `crates/vb_storage/src/codec/mod.rs:126-151` | STRONG (kani harness encodes then decodes the production function) |
| PO-QXJGX-006 | `crate::recovery::replay::summary::apply::apply` and `crate::events::JournalEvent::record_kind` | `crates/vb_storage/src/recovery/replay/summary/apply.rs:32-52` and `crates/vb_storage/src/events.rs:401-429` | STRONG (proptest generator uses production `JournalEvent` and the production replay entry point) |
| PO-QXJGX-007 | `crate::runtime::durability_matrix::DURABILITY_MATRIX` and `crate::codec::validation::validate_schema_version` and `crates/vb_storage/src/codec/flux_validation.rs:14,33` | `crates/vb_runtime/src/durability_matrix.rs:70-204` and `crates/vb_storage/src/codec/validation.rs:10-21` and `crates/vb_storage/src/codec/flux_validation.rs:14,33` | STRONG (proptest enumerates the production constant and reads the production source) |

## 5. Coverage Gaps (Out of Scope, Documented)

- `static-scan` obligations (PS-019, PS-021) are owned by `holzman-rust`
  and `black-hat-reviewer`; not part of `proof-obligations.planned.jsonl`.
  Rationale: they are not behavior-affecting in the sense of a
  formal-verifier executable obligation; they are source-lint checks that
  run in the CI moon gate.

- `manual-qa` obligations (PS-019 for OPEN-Q-A1/A2/A3) are owned by
  `hands-on-qa` and `manual-qa`; not part of this plan. The
  `runtime-skill-provenance.json` and `manual-qa-ephemeral-journal-audit.md`
  and `migration-id-reservation-audit.md` are the evidence references.

- `INV-010` (out-of-scope additions forbidden) is a compliance obligation;
  no formal-verifier execution boundary.
