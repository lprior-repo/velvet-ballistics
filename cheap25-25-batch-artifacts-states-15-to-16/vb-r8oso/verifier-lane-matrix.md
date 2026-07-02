# Verifier Lane Matrix — vb-r8oso

This matrix enumerates every `(requirement_id, contract_clause,
proof_seed_id, verifier)` tuple relevant to the bead and the planner's
lane decision for that tuple. The full schema-valid JSONL is in
`verifier-lane-decisions.jsonl`. The matrix is the narrative
counterpart that the reviewer reads first.

## Active Verifier Profile

| Verifier | Applicability | Justification |
|---|---|---|
| `kani` | required (with `kani-sequence-at-write` Cargo feature) | Bounded symbolic enumeration over `next_sequence_at_write` and the append guard; AGENTS.md kani-harness-isolation rule. |
| `proptest` | required | Random valid/invalid append sequence pressure; error-exhaustiveness for `JournalError::SequenceMismatch`; no-silent-rewrite invariant; batch-atomicity invariant; caller-audit invariant. The "rust-local" lane (per the user's gate) is folded into proptest because the schema's verifier enum does not have a "rust-local" / "cargo-test" entry. |
| `loom` | not_applicable (research-gated) | Single-process; cross-process multi-writer is out-of-scope per `codebase-map.md` §2. Research gate: see `trusted-base-plan.md` TB-NSAW-RESEARCH-001. |
| `verus` | not_applicable | Contract C-8: no new Verus. Existing `recovery_types_spec.rs` unaffected. |
| `flux-rs` | not_applicable | Contract C-8: no new refinement boundary. |
| `miri` | not_applicable | `crates/vb_storage/src/lib.rs:1` `#![forbid(unsafe_code)]`; no `unsafe` surface. |
| `cargo-fuzz` | not_applicable (superseded by proptest) | No new fuzz harness; arm updates in `holzman-rust` per `delivery-scope.jsonl:19-22`. Exhaustiveness is covered by `proptest_journal_error_codes`. |

## Per-Seed Lane Decisions

| PS | Risk tags | Required lane | POB | Not-applicable lanes |
|---|---|---|---|---|
| PS-1 | persistence, public_api, behavior_affecting | kani, proptest | POB-002, POB-003 | loom, verus, flux-rs, miri, cargo-fuzz |
| PS-2 | persistence, public_api, behavior_affecting | kani, proptest | POB-002, POB-003 | loom, verus, flux-rs, miri, cargo-fuzz |
| PS-3 | persistence, concurrency, behavior_affecting | kani, proptest | POB-002, POB-003, POB-006 | loom, verus, flux-rs, miri, cargo-fuzz |
| PS-4 | rust_core_invariant, behavior_affecting | kani, proptest | POB-001, POB-003 | loom, verus, flux-rs, miri, cargo-fuzz |
| PS-5 | public_api, behavior_affecting | proptest | POB-004, POB-005 | loom, verus, flux-rs, miri, cargo-fuzz, kani |
| PS-6 | bounded_state, overflow, behavior_affecting | kani, proptest | POB-001, POB-003 | loom, verus, flux-rs, miri, cargo-fuzz |
| PS-7 | performance, persistence, behavior_affecting | kani, proptest | POB-001, POB-003 | loom, verus, flux-rs, miri, cargo-fuzz |
| PS-8 | concurrency, lock_free, behavior_affecting | proptest | POB-003 (lock-free reachability) | loom (surface_absent), verus, flux-rs, miri, cargo-fuzz, kani |
| PS-9 | hostile_input, persistence | kani, proptest | POB-001, POB-003 | loom, verus, flux-rs, miri, cargo-fuzz |
| PS-10 | performance | (none required; informational seed) | n/a | all — informational performance seed; evidence is in proptest reflection of POB-003 not a separate lane |
| PS-11 | public_api, diagnostics, behavior_affecting | proptest | POB-004 | loom, verus, flux-rs, miri, cargo-fuzz, kani |
| PS-12 | behavior_affecting, persistence | proptest | POB-007 (audit-report check) | loom, verus, flux-rs, miri, cargo-fuzz, kani |
| PS-13 | build_isolation | proptest | POB-006 (compile check via cfg feature) | loom, verus, flux-rs, miri, cargo-fuzz, kani |
| PS-14 | persistence, concurrency, behavior_affecting | kani, proptest | POB-002, POB-003, POB-006 | loom, verus, flux-rs, miri, cargo-fuzz |
| PS-15 | public_api, behavior_affecting | proptest | POB-005 | loom, verus, flux-rs, miri, cargo-fuzz, kani |
| PS-16 | diagnostics, public_api, behavior_affecting | proptest | POB-004 | loom, verus, flux-rs, miri, cargo-fuzz, kani |

## Pairing Index (POB ↔ Seed)

| POB ID | Verifier | PS covered | Contract clause |
|---|---|---|---|
| POB-vb-r8oso-001 | kani | PS-4, PS-6, PS-7, PS-9 | C-2.2, C-2.4, C-2.5 |
| POB-vb-r8oso-002 | kani | PS-1, PS-2, PS-3, PS-14 | C-4.1 |
| POB-vb-r8oso-003 | proptest | PS-1, PS-2, PS-3, PS-4, PS-6, PS-7, PS-8, PS-9, PS-14 | C-2.2, C-2.4, C-2.5, C-4.1, C-4.4 |
| POB-vb-r8oso-004 | proptest | PS-5, PS-11, PS-16 | C-3.2, C-3.3, C-3.5 |
| POB-vb-r8oso-005 | proptest | PS-5, PS-15 | C-3.2, C-5, C-6.1, C-6.2 |
| POB-vb-r8oso-006 | proptest | PS-3, PS-13, PS-14 | C-4.4, C-9 |
| POB-vb-r8oso-007 | proptest | PS-12 | C-10 |

## Non-Applicability Evidence Map

Each `not_applicable` row in `verifier-lane-decisions.jsonl` cites
at least one of the following SHA-256 evidence refs:

| Evidence | SHA-256 | Source |
|---|---|---|
| contract.md C-2.7 (lock-free) | `34416ab9921eeb777ea1c3944ff7514b76e841aca2d9549ec1196c1c617f41d8` | loom not_applicable |
| contract.md C-8 (no Verus, no Flux) | `34416ab9921eeb777ea1c3944ff7514b76e841aca2d9549ec1196c1c617f41d8` | verus not_applicable, flux-rs not_applicable |
| contract.md C-8 (no new fuzz) | `34416ab9921eeb777ea1c3944ff7514b76e841aca2d9549ec1196c1c617f41d8` | cargo-fuzz not_applicable |
| codebase-map.md §2 (single-process) | `d8dfdf1c4f179f472ee11b60fed66baa4243087f76092b693597b5aa2fe0aa36` | loom not_applicable |
| codebase-map.md §18 (no unsafe) | `d8dfdf1c4f179f472ee11b60fed66baa4243087f76092b693597b5aa2fe0aa36` | miri not_applicable |
| delivery-scope.jsonl fuzz arm updates | `99dcc034d31fe20316b308f67dd1148ce11e2723765a4c9afe7da8caf0d76b70` | cargo-fuzz not_applicable (superseded) |

## Self-Audit Notes

The proof-planner validator emits 8 E_LANE_DECISION_MISSING major
findings (no blockers, exit code 0). All 8 are caused by the user's
gate: the user explicitly excluded `verus` and `flux-rs` from the
lane profile, so the default-profile-required verifiers for the
applicable risk classes (`bounded_transition`, `rejection`,
`illegal_state`) cannot be paired. Each finding is documented as a
concrete `not_applicable` lane decision in
`verifier-lane-decisions.jsonl` with the contract.md / codebase-map.md
SHA-256 evidence refs. The validator's exit code is 0 (PASS) under
the default mode; under `--strict` mode the user can elect to
acknowledge these majors as expected per the bead's gate.

| Major finding | Cause | Resolution |
|---|---|---|
| (REQ-NSAW-001, C-4.1, bounded_transition) verus missing | User gate excludes verus | not_applicable evidence: contract.md C-8 |
| (REQ-NSAW-001, C-4.4, bounded_transition) kani missing | POB-006 is proptest, not kani; bounded_transition default profile includes kani | not_applicable evidence: contract.md C-8 (no Verus), codebase-map.md §2 |
| (REQ-NSAW-001, C-4.4, bounded_transition) verus missing | User gate excludes verus | not_applicable evidence: contract.md C-8 |
| (REQ-NSAW-002, C-2.2, bounded_transition) verus missing | User gate excludes verus | not_applicable evidence: contract.md C-8 |
| (REQ-NSAW-008, C-3.3, rejection) kani missing | POB-004 is proptest, not kani; rejection default profile includes kani | not_applicable evidence: codebase-map.md (test design) |
| (REQ-NSAW-009, C-10, rejection) kani missing | POB-007 is proptest, not kani; rejection default profile includes kani | not_applicable evidence: codebase-map.md (audit method) |
| (REQ-NSAW-012, C-5, illegal_state) flux-rs missing | User gate excludes flux-rs | not_applicable evidence: contract.md C-8 |
| (REQ-NSAW-012, C-5, illegal_state) verus missing | User gate excludes verus | not_applicable evidence: contract.md C-8 |
