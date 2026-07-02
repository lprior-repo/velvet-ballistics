---
bead_id: vb-rz9ey
title: Fix vb_compile test compilation: WorkflowSourceParts private (Cargo self-reference, P0)
state: 7 (proof-to-implementation)
skill: proof-to-implementation
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
contract_sha256: e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66
proof_obligation_count: 2 (PO-001, PO-002)
behavior_affecting: false
scope_class: cargo-manifest-metadata-only
rust_refinement_obligation_count: 0
refinement_obligations_emitted: false
refinement_obligation_rationale: |
  Per `proof-schemas.md` (`rust-refinement-obligation/v1` rules): "Every
  behavior-affecting proof obligation needs a matching Rust refinement
  obligation with concrete source refs, independent behavior tests, separate
  refinement harness refs, and executed command evidence by State 12."

  This bead is `behavior_affecting: false` and `scope_class: cargo-manifest-metadata-only`.
  No Rust behavior is being added, removed, or modified. The visibility
  invariant is statically enforced by rustc and the cargo feature-unification
  is statically enforced by cargo. There are no behavior tests, refinement
  harnesses, or production source modifications that a `rust-refinement-obligation/v1`
  row could bind to without producing a vacuous obligation.

  The two `proof-obligation/v1` rows (PO-001, PO-002) are themselves
  cargo-build / cargo-doc invocations against existing production Rust —
  they ARE the implementation evidence. The bridge therefore emits zero
  `rust-refinement-obligation/v1` rows and zero refinement-harness refs.

  This is consistent with the State-5 proof-writer NO_PROOF_WORK disposition
  (proof-writer-report.md §"Why no proof artifacts materialize") and the
  State-6 proof-reviewer approval (proof-review.md §"NO_PROOF_WORK
  Disposition Validation").
disposition: NO_RUST_REFINEMENT
authored_by: proof-to-implementation (direct child of femdation; no sub-agents)
authored_at: 2026-07-01T19:00:00Z
---

# Proof-to-Rust Map — vb-rz9ey

**Bead**: `vb-rz9ey` — Fix `vb_compile` test compilation: `WorkflowSourceParts` private (Cargo self-reference, P0)
**State**: 7 (proof-to-implementation)
**Scope class**: `cargo-manifest-metadata-only`
**Behavior-affecting**: `false`

---

## 1. Bridge Disposition: NO_RUST_REFINEMENT

This is a metadata-only bead. There is **no production Rust source change** —
the entire fix is one line in `crates/vb_compile/Cargo.toml
[dev-dependencies]` and a one-line `Cargo.lock` regeneration. Because no Rust
behavior is added, removed, or modified, there are no `rust-refinement-obligation/v1`
rows to emit.

The State-5 proof-writer already documented this in
`proof-writer-report.md §"NO PROOF WORK — empty artifact bundle"` (verified and
approved by State-6 proof-reviewer in `proof-review.md §3`):

> "Both obligations are **cargo-build / cargo-doc invocations against existing
> production Rust**, not executable proof code. There is: no Verus `proof fn`
> to write ... no Kani `#[kani::proof]` harness to write ... no Flux
> refinement to write ... no Loom model to write ... no Miri harness to
> write ... no `cargo-fuzz` harness to write ..."

State 7 mirrors the same shape: no executable Rust refinement code, no
behavior tests, no refinement harness — the cargo build/doc invocations
themselves are the evidence surface.

---

## 2. Per-Obligation Bridge Rows

| PO  | requirement_id | verifier | source_refs | behavior_test_refs | refinement_harness_refs | evidence_command | mapping_status | notes |
|-----|----------------|----------|-------------|--------------------|------------------------|------------------|----------------|-------|
| PO-001 | REQ-RZ9EY-TESTBUILD-COMPILE | proptest | `crates/vb_compile/Cargo.toml:18-19` (`[dev-dependencies]`, post-fix self-reference entry) | cargo build cargo invocations of 9 existing integration test files: `crates/vb_compile/tests/common/mod.rs`, `digest_structural_fields.rs`, `proptest_digest_foreach.rs`, `digest_set_finish_regression.rs`, `digest_ask_explicit_arm.rs`, `proptest_digest_determinism.rs`, `proptest_digest_ask_timeout_sensitivity.rs`, `proptest_digest_ask_prompt_sensitivity.rs`, `proptest_digest_ask_ordering.rs` | none (no Flux refinement; N/A per `proof-to-implementation-input.md §1 PO-001`) | `cargo build -p vb_compile --tests --message-format=human` | `planned` (deferred to State-12) | The cargo build invocation is the static-visibility-gate proof (rustc enforces `cfg(any(test, feature="test-util"))`); pre-fix baseline shows real `E0432`/`E0624` errors across the 9 test files. |
| PO-002 | REQ-RZ9EY-DOWNSTREAM-PRESERVE | proptest | `crates/vb_cli/Cargo.toml:8` (`vb_compile = { path = "../vb_compile" }`); `crates/workspace_tests/Cargo.toml:39` (`vb_compile = { path = "../vb_compile" }`); `crates/vb_compile/src/yaml_ast/types/workflow.rs:107-127` (`pub(crate)` arm of `WorkflowSourceParts` in default-features build) | none new (existing cargo builds of `vb_cli` and `workspace_tests` are the validation surface; no new behavior tests added) | none (no Flux refinement; N/A per `proof-to-implementation-input.md §1 PO-002`) | `(cargo build -p vb_cli --message-format=human && cargo build -p workspace_tests --message-format=human && cargo doc -p vb_compile --no-deps --message-format=human 2>&1 \| grep -c WorkflowSourceParts)` | `planned` (deferred to State-12) | Cargo's per-build-graph feature unification enforces isolation; `cargo doc --no-deps` proves the cfg-gate remains closed in default-features production builds. |

### 2.1 Source-Ref Rationale

The two `source_refs` columns name **production manifest files and production
source paths** that the cargo invocations target. For PO-001 the source
refers to the manifest file that the State-6 holzman-rust agent will edit;
for PO-002 the source refs name the downstream consumer manifests and the
cfg-gated `pub(crate)` arm of `WorkflowSourceParts`. These are not file-only
refs — each is a concrete `path::line` cite per `bridge-review-rubric.md`.

### 2.2 Behavior-Test-Ref Rationale

For PO-001 the "behavior tests" are the 9 existing integration test files
that `cargo build -p vb_compile --tests` compiles. These tests are the
validation surface; they would fail to compile if the fix were reverted
(verified by the pre-fix baseline of 38 `E0432`/`E0624` errors documented
in `proof-evidence.md §3`). No new behavior tests are added by this bead
(per `contract.md §5` and `proof-to-implementation-input.md §2`).

For PO-002 the "behavior tests" are the existing `cargo build -p vb_cli` and
`cargo build -p workspace_tests` invocations. These are downstream compile
checks that would fail if the dev-dep self-reference leaked `test-util` into
the production build graph. No new behavior tests are added.

### 2.3 Refinement-Harness-Ref Rationale

No Flux refinement is required for this bead. The visibility invariant is
statically enforced by rustc and the feature isolation is statically
enforced by cargo. There is no dynamic behavior to refine, no executable
proof code to maintain, and no separate refinement harness that would not
collapse into a duplicate of the cargo invocation already cited as the
evidence command.

### 2.4 Why `mapping_status: planned` (not `materialized` or `verified`)

State 7 maps proof claims to Rust behavior obligations. Per
`proof-schemas.md` (`rust-refinement-obligation/v1`):
"`planned` is allowed at State 7 and rejected at State 12 closure."

Since this bead emits zero `rust-refinement-obligation/v1` rows, the
`mapping_status: planned` token is informational only (carried in the table
above to show the deferred execution is the cargo invocation itself, not a
separate refinement harness). The actual `rust-refinement-obligation/v1`
file (`rust-refinement-obligations.jsonl`) is **empty** — see §3.

---

## 3. Output Artifact Set

| artifact | exists | sha256 | purpose |
|----------|--------|--------|---------|
| `proof-to-rust-map.md` | YES (this file) | (sha256 in ledger entry) | per-obligation bridge rows; declares NO_RUST_REFINEMENT disposition |
| `rust-refinement-obligations.jsonl` | YES, **empty** (0 bytes, 0 rows) | (sha256 in ledger entry) | empty JSONL; no behavior change means no `rust-refinement-obligation/v1` rows to emit |

The empty `rust-refinement-obligations.jsonl` is the authoritative statement
that zero `rust-refinement-obligation/v1` rows are required for this bead.
Per `proof-schemas.md`: "`planned` is allowed at State 7 and rejected at
State 12 closure." Since the bridge emits zero rows, the State-12 closure
gate does not apply to refinement obligations; it applies only to the two
`proof-obligation/v1` rows (PO-001, PO-002) which are themselves the
cargo invocations.

---

## 4. Cross-Reference

- `proof-to-implementation-input.md` (State 4) — the planner handoff already
  contains the per-PO source refs, behavior test refs, and evidence commands.
  This State-7 bridge is a re-statement confirming zero `rust-refinement-obligation/v1`
  rows are required for a metadata-only patch.
- `proof-writer-report.md` (State 5) — declares zero proof/model/harness
  artifacts; State 7 declares zero rust refinement artifacts.
- `proof-review.md` (State 6) — approves the State-5 NO_PROOF_WORK
  disposition; State 7's NO_RUST_REFINEMENT disposition is the mirror at
  the bridge level.
- `proof-obligations.planned.jsonl` — the 2 `proof-obligation/v1` rows
  (PO-001, PO-002) that the bridge maps.
- `contract.md §5` — "This bead does not write tests. Existing tests are
  the validation."
- `contract.md §9` — "`behavior_affecting: false` — no waiver needed."

---

## 5. Handoff to Bridge Review (State 7-bridge)

The `proof-reviewer` agent reviews this bridge independently. The expected
disposition is `APPROVED` because:

1. `behavior_affecting: false` (verified by `contract.md §1` and
   `proof-review.md §1`).
2. `scope_class: cargo-manifest-metadata-only` (verified by
   `contract.md §1`).
3. The two `proof-obligation/v1` rows are themselves cargo invocations
   against existing production Rust — there is no Rust behavior to
   refine.
4. `rust-refinement-obligations.jsonl` is empty (zero rows), consistent
   with the no-behavior-change scope.
5. `mapping_status: planned` is allowed at State 7 (per
   `proof-schemas.md`); the table rows are informational only.
6. No file-only refs, no prose refs, no missing harness refs, no
   missing evidence paths, no behavior-affecting waivers (per
   `bridge-review-rubric.md`).
