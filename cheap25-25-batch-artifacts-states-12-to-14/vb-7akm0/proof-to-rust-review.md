---
bead_id: vb-7akm0
bead_title: "Lint: remove #[allow(unreachable_pub)] suppressions by narrowing visibility (P1 bug)"
phase: 7
updated_at: 2026-07-02T00:25:00Z
attempt: 1
---

# Proof-to-Rust Review — vb-7akm0

## Scope

Reviewed `proof-to-rust-map.md` and `rust-refinement-obligations.jsonl` against
`proof-obligations.planned.jsonl`, `proof-review.md`, `proof-findings.jsonl`,
`proof-to-implementation-input.md`, `delivery-scope.jsonl`, and `contract.md`.

## Bridge Adequacy Assessment

vb-7akm0 is a visibility-narrowing refactor. The upstream proof phase closed with
`APPROVED — NO_PROOF_WORK` (`proof-review.md` §11): no production Rust semantics change,
no formal-verifier artifact (Verus/Kani/Flux/Loom/proptest/fuzz/Miri/TLA+) was created,
the 8 formal lanes are `not_applicable`, and `trusted-base-ledger.jsonl` is empty. The
6 obligations are all `behavior_affecting=false` and resolve to gate-execution evidence
owned by State 11.

Consequently the proof-to-rust bridge carries **no refinement claims**, and
`rust-refinement-obligations.jsonl` is empty by construction. The map correctly records
this posture and provides a concrete Rust target plus an independent verification lane
for each of the 6 obligations.

## Obligation-by-Obligation Confirmation

| Obligation | Behavior-Affecting | Rust Target Present | Independent Gate | Refinement Needed | Adequacy |
|---|---|---|---|---|---|
| PO-LINT-001 | false | Yes (25 files) | `moon-lint-src` | No | ADEQUATE |
| PO-COMPILE-001 | false | Yes (18 files) | `cargo-check` | No | ADEQUATE |
| PO-TEST-001 | false | Yes (test consumers) | `cargo-test` | No | ADEQUATE |
| PO-EXTERN-001 | false | Yes (externality guards) | `grep-externality` + Verus binding/drift gates | No | ADEQUATE |
| PO-DECISION-001 | false | Yes (`decision-ack.md`) | `decision-ack` | No | ADEQUATE |
| PO-DECISION-GREP-001 | false | Yes (`production_inner/`) | `grep` | No | ADEQUATE |

## Behavior-Affecting Proof Claims Without Test Coverage

None. There are no behavior-affecting proof claims for this bead; every obligation is a
non-behavior-affecting gate-execution obligation with a concrete lane and evidence path.

## Verifier-Only Waivers

None. No behavior-affecting claim exists that would require a Rust-evidence waiver.
`waiver-candidates.jsonl` carries only the `W-NONE-001` sentinel (`behavior_affecting=false`).

## Consistency Checks

- Empty `rust-refinement-obligations.jsonl` is consistent with `proof-review.md` §11
  (`NO_PROOF_WORK`) and with all obligations being `behavior_affecting=false`. PASS.
- Every mapped Rust target matches a source ref in `proof-to-implementation-input.md`
  and the `proof-obligations.planned.jsonl` mapping. PASS.
- No trusted-base file (`extern_*.rs`, `production_inner/*.rs`, `kani/`,
  `xtask/src/main.rs` internals, moon `lint-src` task) is claimed as a refinement target.
  PASS.
- No self-approval: this review is distinct from the map author. PASS.

## Bridge Approval

**STATUS: APPROVED**

The proof-to-rust bridge is adequate. All 6 obligations are non-behavior-affecting and
map to concrete Rust targets with independent verification gates. No formal proof claims
exist, so no Rust refinement obligations are required; the empty
`rust-refinement-obligations.jsonl` is correct. No behavior-affecting claim lacks Rust
evidence, no repairs needed. Handoff proceeds to State 8 (test-planner) and State 11
(formal-verifier) for gate execution.
