# Verifier Lane Matrix — vb-qxjgx

Human-readable summary of `verifier-lane-decisions.jsonl`. Each row
displays the lane decision, the bound obligation(s), the risk profile, and
the disposition. The full machine-checkable schema is in
`verifier-lane-decisions.jsonl`; this file is for review-time consumption.

## 1. Required Lanes (10 rows)

| Lane ID | Verifier | Risk tags | Applicability | Reason (truncated) | Bound obligation(s) | Risk class | Status |
|---|---|---|---|---|---|---|---|
| VLD-QXJGX-001 | kani | wire-format, persistence, public-api | required | POST-001: RecordKind::StepSucceeded.id() == 33 with closed-set bijection. Kani is the primary lane for field_sensitivity. | PO-QXJGX-001 | field_sensitivity | planned |
| VLD-QXJGX-002 | kani | persistence, wire-format, user-visible-behavior | required | POST-002: events.rs:406 OR-collapse removed; StepSucceeded projects to StepSucceeded, SlotWrittenEvent to SlotWritten. | PO-QXJGX-002 | field_sensitivity | planned |
| VLD-QXJGX-003 | kani | parser-codec, wire-format, persistence | required | POST-003: is_known_record_kind(33) == true. Kani extends check_journal_family_exhaustive. | PO-QXJGX-003 | rejection | planned |
| VLD-QXJGX-004 | kani | parser-codec, wire-format, persistence | required | POST-004: validate_kind_family(MAGIC_JOURNAL_EVENT, 33) == Ok. Kani sweep over u16 magic x u16 kind. | PO-QXJGX-003 | rejection | planned |
| VLD-QXJGX-005 | kani | parser-codec, wire-format, migration, persistence | required | POST-005: parity gate accepts {12, 33} for StepSucceeded. Kani grid over envelope_id x payload_variant. | PO-QXJGX-004 | rejection | planned |
| VLD-QXJGX-006 | kani | parser-codec, wire-format, persistence | required | POST-007: SlotWrittenEvent + id 33 returns Err(RecordKindPayloadMismatch { envelope_kind: 33, payload_kind: 12 }). | PO-QXJGX-004 | rejection | planned |
| VLD-QXJGX-007 | kani | parser-codec, wire-format, migration, persistence | required | POST-006: decode_journal_event round-trips canonical id-33 and legacy id-12 + StepSucceeded. | PO-QXJGX-005 | rejection | planned |
| VLD-QXJGX-008 | proptest | wire-format, public-api, persistence | required | POST-002 + INV-001: one-to-one projection. Proptest generator reaches every variant. | PO-QXJGX-006 | field_sensitivity | planned |
| VLD-QXJGX-009 | proptest | user-visible-behavior, public-api, persistence | required | POST-008: durability matrix step-closing rows list StepSucceeded. Proptest enumerates each row. | PO-QXJGX-007 | field_sensitivity | planned |
| VLD-QXJGX-010 | proptest | user-visible-behavior, public-api | required | POST-009 + INV-008: recovery counters variant-keyed. Proptest mixes legacy id-12 and canonical id-33 envelopes. | PO-QXJGX-006 | field_sensitivity | planned |
| VLD-QXJGX-011 | proptest | migration, persistence, public-api | required | INV-006: validate_schema_version pins CURRENT_SCHEMA_VERSION=1. Proptest covers 0, 1, 2, and the full u16 sweep. | PO-QXJGX-007 | field_sensitivity | planned |

## 2. Blocked-Tooling Lanes (1 row)

| Lane ID | Verifier | Risk tags | Applicability | Limitation kind | Tooling acquisition | Reason (truncated) |
|---|---|---|---|---|---|---|
| VLD-QXJGX-012 | flux-rs | parser-codec, wire-format | blocked_tooling | external_dependency_unavoidable | BEAD-TOOL-FLUX-RS-INSTALL | flux-rs not in workspace (vb-b8i8f); `codec/mod.rs:184-186` comments out `pub mod flux_validation`. Literal-sync is enforced by PO-QXJGX-007 third proptest. |

## 3. Risk-Class Coverage Summary

| Risk class | Required lanes | Required-lane decisions | Companion-lane decisions | Profile gaps (Major) |
|---|---|---|---|---|
| `field_sensitivity` | kani + proptest | VLD-QXJGX-001, VLD-QXJGX-002, VLD-QXJGX-008, VLD-QXJGX-009, VLD-QXJGX-010 | (same) | 4: REQ-001/002 missing proptest; REQ-008/009 missing kani |
| `rejection` | kani + proptest | VLD-QXJGX-003, VLD-QXJGX-004, VLD-QXJGX-005, VLD-QXJGX-006, VLD-QXJGX-007, VLD-QXJGX-011 | (none for pure kani obligations) | 3: REQ-003/005/006 missing proptest |
| `refinement` (flux-rs) | flux-rs | VLD-QXJGX-012 (blocked_tooling) | PO-QXJGX-007 (literal-sync via proptest) | None (compensating evidence is independent) |

The 7 Major findings are accepted deviations from the default profile; see
`proof-coverage-matrix.md` §3 and `trusted-base-plan.md` §5 for the
rationale.

## 4. Verifier-Trigger Cross-Reference

| Risk tag(s) | Triggered verifier | Lane decision | Reason |
|---|---|---|---|
| wire-format, persistence, public-api | kani | VLD-QXJGX-001 | Wire-id bijection requires bounded symbolic proof over the u16 id space. |
| wire-format, persistence, public-api | proptest | VLD-QXJGX-008 | Randomized pressure on the same property. |
| parser-codec, wire-format, migration, persistence | kani | VLD-QXJGX-005, VLD-QXJGX-006 | Parity gate acceptance grid is a bounded rejection property. |
| parser-codec, wire-format, persistence | kani | VLD-QXJGX-003, VLD-QXJGX-004 | Family/kind rejection is a bounded rejection property. |
| user-visible-behavior, public-api, persistence | proptest | VLD-QXJGX-009, VLD-QXJGX-010 | Durability matrix and recovery counters are compile-time constants; proptest enumerates the constant. |
| parser-codec, wire-format | flux-rs (blocked_tooling) | VLD-QXJGX-012 | Refinement literals require flux-rs; the module is DISABLED per vb-b8i8f. |

## 5. Self-Audit Checklist

- [x] Every `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple has exactly one lane decision.
- [x] No default-profile verifier has `not_applicable` without `non_applicability_evidence_refs` (only `blocked_tooling` is used, with `tooling_acquisition_ref`).
- [x] Every `required` lane decision has at least one paired `proof-obligation/v1` ID.
- [x] Every `proof-obligation/v1` row is referenced by a required lane decision.
- [x] No `blocked_tooling` row advances past State 4; the VLD-QXJGX-012 row documents the open tooling gap.
- [x] All `decision_reason` strings cite concrete `risk_tags` and avoid the weak vocabulary ("not needed", "too hard", "low risk", etc.).
- [x] All `not_applicable` rows (none in this plan) have a typed `limitation_kind`; the `blocked_tooling` row has `limitation_kind: external_dependency_unavoidable`.
- [x] No two rows duplicate `(requirement_id, contract_clause, proof_seed_id, verifier)`.

## 6. Validator Output (raw)

```
plan_dir: .beads/vb-qxjgx
obligations: 7
lane_decisions: 12
waivers: 0
findings_total: 7
findings_blocker: 0
findings_major: 7
status: PASS
```

The 7 Major findings are documented in `proof-coverage-matrix.md` §3 as
accepted deviations. No Blocker findings. The plan is ready for
`proof-plan-reviewer` (State 4b).
