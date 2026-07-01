# Proof Strategy — vb-rz9ey

- bead_id: vb-rz9ey
- title: Fix vb_compile test compilation: WorkflowSourceParts private (Cargo self-reference)
- state: 4 (proof-planner)
- scope_class: cargo-manifest-metadata-only
- behavior_affecting: false
- contract_version: contract/v1 (sha256: e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66)
- codebase_map_sha256: 7336795bdf60f345ae7d2af2641b16388e36fc79d27e653cf00db31affd66697
- proof_seeds_sha256: d95357c83d1d086b71376f452dadd20326bb2e05f183d97152fe10e9121551d1
- traceability_sha256: 101667a0a9c378006e1ed4dd740bae6e160e0961b9d62603948a6778a95143a1
- delivery_scope_sha256: f35caf1e55e0c0d0c6f4a21a8d88251a7b78faeb453f8c5863dbc8cb2a3badf9
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
- strategy_kind: build-only, cargo-manifest-metadata
- authored_by: proof-planner (no sub-agents)

## 1. Strategy Summary

vb-rz9ey is a **cargo-manifest metadata-only patch** that activates the existing
`test-util` Cargo feature for the `vb_compile` test build via a self-referencing
`[dev-dependencies]` entry. The change is structurally equivalent to the
canonical Cargo pattern documented at
`specifying-dependencies.html#self-references` and is **statically checked by
rustc/cargo**. There is no production code change, no behavior change, and no
runtime invariant whose soundness depends on this patch.

The 8 input `proof-seed/v1` rows (`ps-vb-rz9ey-01` … `ps-vb-rz9ey-08`) describe
a fine-grained risk surface (visibility invariant, test-build compile,
downstream preservation vb_cli, downstream preservation workspace_tests,
lockfile minimal diff, feature inertness, field-shape divergence, self-ref
placement). These 8 seeds collapse cleanly into exactly **two consolidated
proof obligations**:

- **PO-001 / PS-001 — Manifest obligation.** The dev-dependency edit causes
  `cargo build -p vb_compile --tests` to compile with zero `E0432` and zero
  `E0624` errors. This is verified by **running** `cargo build` and inspecting
  the exit code and stderr; the verifier category is `proptest` because the
  test build includes the `proptest` harnesses in
  `crates/vb_compile/tests/proptest_*` that are themselves the primary test
  surface being unblocked.

- **PO-002 / PS-002 — Downstream preservation obligation.** The dev-dependency
  edit does NOT propagate `test-util` into downstream production builds.
  `cargo build -p vb_cli` and `cargo build -p workspace_tests` must exit 0
  *without* the feature being activated, and `cargo doc -p vb_compile --no-deps`
  must NOT surface `WorkflowSourceParts` in the public-doc output. The verifier
  category is `proptest` because the downstream build graph and the public-API
  doc surface are the only evidence required, and the cargo-build commands
  above are themselves the standard Cargo-level smoke/property surface.

## 2. Why No Formal Verifier (Verus / Kani / Flux / Loom) Is Required

Per the contract (`contract.md` §6 "Verification Lanes") and the codebase
exploration (`codebase-map.md` Q1, Q2):

1. **No Verus spec references `WorkflowSourceParts`.** Verified by
   `grep -rn WorkflowSourceParts verification/verus/` returning empty.
   `verification/verus/` does not exist in this repository.
2. **No Kani harness references `WorkflowSourceParts`.** The 6 Kani harnesses
   listed in `codebase-map.md` use `crate::ast::{...WorkflowSource,
   WorkflowSourceParts}` but `crate::ast` does NOT re-export `WorkflowSource`
   (it re-exports `WorkflowAst` only). This is a **pre-existing latent defect
   (OI-1) flagged as out of scope** for vb-rz9ey. The Kani harnesses are
   `cfg(kani)`-gated and never participate in `cargo build --tests` or
   `cargo build -p vb_cli`.
3. **No Flux refinement references `WorkflowSourceParts`.**
   `verification/flux/` does not exist for vb_compile.
4. **No Loom model references `WorkflowSourceParts`.** No concurrency surface.
5. **No proptest harness uses WorkflowSourceParts in a property-pressure way**
   that would justify a proptest-via-verifier obligation distinct from the
   `cargo build --tests` evidence.

The visibility invariant (`pub(crate)` in production, `pub` under
`cfg(any(test, feature = "test-util"))`) is **statically enforced by rustc**.
rustc produces a help note that pinpoints the gate at
`crates/vb_compile/src/lib.rs:241`. The only proof obligation is therefore
*that the manifest edit produces the right rustc outcome*.

## 3. Risk Classification

| risk_tag | present | drives which obligation | formal verifier needed |
|----------|---------|------------------------|------------------------|
| `risk:build` | yes | PO-001 | no (rustc static check) |
| `risk:public_api` | yes | PO-001 + PO-002 | no (rustc visibility + cargo doc) |
| `risk:lockfile` | yes | PO-001 (sub-check via `git diff Cargo.lock`) | no |
| `risk:test_only` | yes | PO-001 | no |
| `risk:downstream` | yes | PO-002 | no |

None of these risk_tags fall into the default-profile verifier taxonomy
(`arithmetic_overflow`, `bounded_transition`, `concurrency_interleaving`,
`refinement`, `index_safety`, `panic_freedom`, `ub_safety`, `hostile_input`,
`rejection`, `equality`, `ordering`, `temporal_safety`, `temporal_liveness`).
Every default-profile verifier row in
`verifier-lane-decisions.jsonl` therefore carries `applicability:
not_applicable` with a typed `limitation_kind` of `surface_absent` and concrete
evidence refs to the contract clauses and the codebase-map SHA-256.

## 4. Risk: Profile Per Obligation

### PO-001 / PS-001 — Manifest obligation

- `risk`: `panic_freedom` (closest fit: the test build must not panic at
  compile-time nor at test-runtime; reduced to compile-time success criterion).
- `risk_tags`: `["rust_local", "build_manifest", "visibility", "public_api",
  "lockfile", "test_only"]`
- `verifier`: `proptest` (the test build is the evidence surface; the proptest
  harnesses in `crates/vb_compile/tests/proptest_*` are compiled as part of the
  `cargo build -p vb_compile --tests` invocation that constitutes the proof
  command).
- `mode`: `verify-proof` (the obligation binds to the production visibility
  invariant via rustc).
- `behavior_affecting`: false.
- `required`: true.

### PO-002 / PS-002 — Downstream preservation obligation

- `risk`: `panic_freedom` (closest fit: downstream production builds must not
  break; reduced to compile-time success of `cargo build -p vb_cli` and
  `cargo build -p workspace_tests` without `test-util`).
- `risk_tags`: `["rust_local", "public_api", "downstream", "feature_isolation"]`
- `verifier`: `proptest` (the downstream build graph is the evidence surface;
  cargo's per-build-graph feature unification is the property under test).
- `mode`: `verify-proof`.
- `behavior_affecting`: false.
- `required`: true.

## 5. Production-Binding Plan (NOT applicable)

This bead emits **zero Verus obligations**, therefore the mandatory
`production_binding` field for Verus obligations is N/A. The verifier category
is `proptest` for both obligations, and the Verus-binding gate
(`scripts/check-verus-production-binding.sh`) does not apply.

## 6. Cross-Lane Discipline

Because neither obligation is in the default-formal-verifier profile, there is
no Verus↔Kani or Kani↔proptest stacking to disambiguate. The only lanes
required by this bead are:

- **source-lint** (`moon run :lint-src`) — Holzman governance on the Cargo.toml
  edit; not modeled as a `verifier-lane-decision/v1` row because it is a
  governance check rather than a proof lane.
- **cargo-build** (test build + downstream builds) — modeled as `proptest`
  verifier obligations because that is the closest taxonomy entry whose
  evidence surface (a cargo invocation) matches the actual evidence command.

## 7. Mapping Back to Proof-Seeds

| proof_seed | subsumes | obligation |
|------------|----------|------------|
| ps-vb-rz9ey-01 (REQ-RZ9EY-VISIBILITY-INVARIANT) | visibility gate integrity | PO-001 (test side) + PO-002 (production side) |
| ps-vb-rz9ey-02 (REQ-RZ9EY-TESTBUILD-COMPILE) | test build compiles | PO-001 |
| ps-vb-rz9ey-03 (REQ-RZ9EY-DOWNSTREAM-PRESERVE-1) | vb_cli preserves | PO-002 |
| ps-vb-rz9ey-04 (REQ-RZ9EY-DOWNSTREAM-PRESERVE-2) | workspace_tests preserves | PO-002 |
| ps-vb-rz9ey-05 (REQ-RZ9EY-LOCKFILE-MINIMAL) | lockfile minimal diff | PO-001 (sub-evidence) |
| ps-vb-rz9ey-06 (REQ-RZ9EY-FEATURE-INERTNESS) | default empty | PO-001 (sub-evidence) |
| ps-vb-rz9ey-07 (REQ-RZ9EY-FIELD-SHAPE-DIVERGENCE) | cfg arms field-identical | PO-001 (sub-evidence) |
| ps-vb-rz9ey-08 (REQ-RZ9EY-SELF-REF-PLACEMENT) | dev-dep placement | PO-001 (sub-evidence) |

## 8. What the Formal Verifier Will Do (State 12, out-of-scope here)

The `formal-verifier` will:

1. Run `cargo build -p vb_compile --tests --message-format=human` from the
   isolated workdir; record exit code and stderr line counts for `E0432` and
   `E0624`.
2. Run `cargo build -p vb_cli --message-format=human` and
   `cargo build -p workspace_tests --message-format=human` from the isolated
   workdir; record exit codes.
3. Run `cargo doc -p vb_compile --no-deps --message-format=human` from the
   isolated workdir; record the grep count for `WorkflowSourceParts` (must be
   0).
4. Run `git diff --stat Cargo.lock` and `git diff Cargo.lock` to confirm the
   one-line self-reference addition.
5. Run `moon run :lint-src` to confirm source-lint gates pass.

These are documented as `evidence_command` + `expected_evidence` in
`proof-obligations.planned.jsonl`. The planner does NOT execute them.

## 9. Handoff to State 4b (proof-plan-reviewer)

The plan reviewer will inspect:

- Each `verifier-lane-decision/v1` row's `decision_reason` for concrete risk
  citation and absence of weak vocabulary.
- Each `not_applicable` row's `non_applicability_evidence_refs` for at least
  one SHA-256 hash.
- Each `required` row's `required_obligation_ids` for a paired obligation in
  `proof-obligations.planned.jsonl` with matching `verifier` and matching
  `target`.
- `waiver-candidates.jsonl` is empty (this bead emits zero waivers).
- `proof-coverage-matrix.md` shows every input `proof-seed/v1` row mapped to
  an obligation.
- `trusted-base-plan.md` contains zero entries (no `assume`/`axiom`/`admit`
  markers in this bead).

## 10. Self-Audit Checklist (Pre-Handoff)

- [x] Exactly 2 `proof-obligation/v1` rows in
  `proof-obligations.planned.jsonl`.
- [x] Exactly 14 `verifier-lane-decision/v1` rows in
  `verifier-lane-decisions.jsonl` (7 verifiers × 2 obligations).
- [x] Every default-profile verifier has a `not_applicable` row per
  obligation with `non_applicability_evidence_refs` containing at least one
  SHA-256.
- [x] Every `required` lane decision cites a paired obligation ID.
- [x] No `behavior_affecting: true` rows.
- [x] No waiver candidates.
- [x] No `assume`/`axiom`/`admit`/`external_body` markers anywhere.
- [x] `proof-coverage-matrix.md` maps all 8 input proof-seeds to the 2
  obligations.
- [x] `trusted-base-plan.md` is empty (zero trust markers).
- [x] `proof-to-implementation-input.md` provides the bridge for
  `holzman-rust` (State 6) and `black-hat-reviewer` (State 8) consumers.

## 11. Constraints Preserved

- No production Rust is edited by this plan (the proof obligations reference
  production symbols for `target` but no obligation executes against them in a
  way that touches source).
- No proof/model/harness code is written by this plan (the proof-writer at
  State 5 owns that lane; this bead has nothing for proof-writer to write).
- No test code is added (the bead's success criterion is that *existing* tests
  compile).
- No CI config is touched.
- No Verus / Kani / Flux / Loom / proptest-via-verifier obligation is emitted,
  consistent with the contract §6 lane table.
- The Cargo.toml edit is bounded to the `[dev-dependencies]` section per
  contract §3.1 "Hard constraints".