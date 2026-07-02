# Proof-to-Implementation Input — vb-rz9ey

- bead_id: vb-rz9ey
- state: 4 (proof-planner)
- authored_by: proof-planner
- contract_sha256: e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
- target_state: 6 (holzman-rust), 7 (proof-to-implementation bridge), 8 (black-hat-reviewer), 12 (formal-verifier)

This document is the handoff from `proof-planner` (State 4) to the
downstream agents. It maps every `proof-obligation/v1` row to its production
source references, its independent behavior-test references (where applicable),
its refinement-harness references (where applicable), and the exact evidence
commands the downstream agents must execute or cite.

## 1. Obligation → Implementation Bridge

### PO-001 — Manifest obligation

- **Production source ref**: `crates/vb_compile/Cargo.toml` `[dev-dependencies]`
  (line 18-19 in pre-fix state; the self-reference entry must be added here
  per contract §3.1).
- **Production source ref (visibility gate)**: `crates/vb_compile/src/yaml_ast/types/workflow.rs:107-149`
  (the two cfg arms of `WorkflowSourceParts` whose visibility the manifest
  edit activates) and `crates/vb_compile/src/lib.rs:241` (the root-level
  re-export).
- **Independent behavior tests**: `cargo build -p vb_compile --tests`
  compiles the 9 affected integration test files (verified by baseline:
  38 errors before fix → 0 errors after fix). The test files are:
  - `crates/vb_compile/tests/common/mod.rs`
  - `crates/vb_compile/tests/digest_structural_fields.rs`
  - `crates/vb_compile/tests/proptest_digest_foreach.rs`
  - `crates/vb_compile/tests/digest_set_finish_regression.rs`
  - `crates/vb_compile/tests/digest_ask_explicit_arm.rs`
  - `crates/vb_compile/tests/proptest_digest_determinism.rs`
  - `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs`
  - `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs`
  - `crates/vb_compile/tests/proptest_digest_ask_ordering.rs`
- **Refinement harness ref**: N/A (no Flux refinement in this bead).
- **Evidence command (formal-verifier at State 12)**:
  ```
  cargo build -p vb_compile --tests --message-format=human
  ```
  Expected exit code: 0. Expected stderr: zero lines matching `E0432`, zero
  lines matching `E0624`.
- **Sub-evidence commands**:
  ```
  git diff --stat Cargo.lock              # expect: 1 file changed, 1 insertion(+), 0 deletions(-)
  git diff Cargo.lock                     # expect: single-line addition in vb_compile's own closure
  awk '/^\[dependencies\]/,/^\[/' crates/vb_compile/Cargo.toml | grep -c 'features = \["test-util"\]'  # expect: 0
  awk '/^\[dev-dependencies\]/,/^\[/' crates/vb_compile/Cargo.toml | grep -A2 'vb_compile = '  # expect: vb_compile = { path = ".", features = ["test-util"] }
  moon run :lint-src                      # expect: exit 0
  ```
- **Owner**: `holzman-rust` (State 6) for the manifest edit; `black-hat-reviewer`
  (State 8) for the sub-evidence cross-check; `formal-verifier` (State 12) for
  the primary evidence command.

### PO-002 — Downstream preservation obligation

- **Production source ref**: `crates/vb_cli/Cargo.toml` line 8
  (`vb_compile = { path = "../vb_compile" }`) and
  `crates/workspace_tests/Cargo.toml` line 39
  (`vb_compile = { path = "../vb_compile" }`). Both consumers must NOT
  activate `test-util`; cargo's per-build-graph feature unification enforces
  the isolation.
- **Production source ref (visibility gate preservation)**:
  `crates/vb_compile/src/yaml_ast/types/workflow.rs:107-127` (the
  `pub(crate)` arm must remain `pub(crate)` in default-features builds).
- **Independent behavior tests**: `cargo build -p vb_cli` and
  `cargo build -p workspace_tests` must both exit 0. The downstream
  `vb_cli` binary's public-API surface does not include `WorkflowSourceParts`
  (verified by `cargo doc -p vb_compile --no-deps | grep -c WorkflowSourceParts`
  returning 0).
- **Refinement harness ref**: N/A (no Flux refinement in this bead).
- **Evidence command (formal-verifier at State 12)**:
  ```
  cargo build -p vb_cli --message-format=human
  cargo build -p workspace_tests --message-format=human
  cargo doc -p vb_compile --no-deps --message-format=human 2>&1 | grep -c WorkflowSourceParts
  ```
  Expected: both `cargo build` invocations exit 0; the `grep -c` returns 0.
- **Sub-evidence commands**:
  ```
  awk '/^\[dependencies\]/,/^\[/' crates/vb_compile/Cargo.toml | grep -c 'features = \["test-util"\]'  # expect: 0
  grep -nE '^default = ' crates/vb_compile/Cargo.toml     # expect: default = []
  grep -nE '^test-util = ' crates/vb_compile/Cargo.toml   # expect: test-util = []
  diff <(sed -n '107,127p' crates/vb_compile/src/yaml_ast/types/workflow.rs | sed 's/pub(crate)/pub/g') <(sed -n '129,149p' crates/vb_compile/src/yaml_ast/types/workflow.rs)  # expect: empty diff (visibility normalization only)
  ```
- **Owner**: `black-hat-reviewer` (State 8) for the negative-check evidence
  commands; `formal-verifier` (State 12) for the primary evidence command.

## 2. Behavior Test Plan Cross-Reference

This bead does NOT add new behavior tests. The 9 existing integration test
files listed in PO-001 above are the validation surface; their compilation
under `cargo build -p vb_compile --tests` is the primary success metric.

For runtime behavior (not compilation), the `cargo test -p vb_compile`
invocation runs the proptest harnesses in:

- `tests/proptest_digest_foreach.rs`
- `tests/proptest_digest_determinism.rs`
- `tests/proptest_digest_ask_timeout_sensitivity.rs`
- `tests/proptest_digest_ask_prompt_sensitivity.rs`
- `tests/proptest_digest_ask_ordering.rs`

These are not new obligations; they are sub-evidence within PO-001
(`expected_evidence` paragraph).

## 3. Forbidden Mutations (from contract.md §3.3)

The `holzman-rust` agent MUST NOT edit any file outside
`crates/vb_compile/Cargo.toml [dev-dependencies]`. Specifically forbidden:

- `crates/vb_compile/src/yaml_ast/types/workflow.rs`
- `crates/vb_compile/src/yaml_ast/types.rs`
- `crates/vb_compile/src/yaml_ast/mod.rs`
- `crates/vb_compile/src/lib.rs`
- `crates/vb_compile/Cargo.toml [features]` block
- `crates/vb_compile/Cargo.toml [dependencies]` block
- `crates/vb_compile/tests/**/*.rs`
- `Cargo.toml` (workspace root)
- `Cargo.lock` (hand-edit; regenerate only)

## 4. Required Mutation (from contract.md §3.1)

The `holzman-rust` agent MUST add exactly these lines to
`crates/vb_compile/Cargo.toml [dev-dependencies]`:

```toml
[dev-dependencies]
proptest.workspace = true
# Self-reference enables `test-util` for the test build only, so external
# integration tests can construct WorkflowSource via WorkflowSourceParts.
# Documented at specifying-dependencies.html#self-references.
vb_compile = { path = ".", features = ["test-util"] }
```

Hard constraints (per contract §3.1):

- Line MUST live in `[dev-dependencies]`, NOT `[dependencies]`.
- `path = "."` exactly.
- `features = ["test-util"]` exactly (no other features).
- No line changes outside `[dev-dependencies]`.

## 5. Lockfile Regeneration

After the manifest edit, `holzman-rust` MUST regenerate `Cargo.lock` by
running `cargo build -p vb_compile --tests` (which auto-updates the lockfile
as a side effect) OR by running `cargo metadata` followed by a normal
`cargo build`. The expected `Cargo.lock` diff is **exactly +1 line**
referencing `vb_compile` in `vb_compile`'s own test-binary closure. No
hand-edits.

The `landing-skill` agent MUST verify `git diff --stat Cargo.lock` shows
exactly `1 file changed, 1 insertion(+), 0 deletions(-)` before landing.

## 6. Source Lint

The `holzman-rust` agent MUST run `moon run :lint-src` after the manifest
edit and confirm exit 0 before handing off to `black-hat-reviewer`. This is
governance, not formal verification, but it is a hard gate.

## 7. Handoff Sequence

```
proof-planner (State 4)
   ↓ [this document + 7 artifacts under .beads/vb-rz9ey/]
proof-plan-reviewer (State 4b) — dispositions every verifier-lane-decision
   ↓ [verifier-lane-review.jsonl + proof-plan-review.md]
proof-writer (State 5) — SKIPPED: zero proof artifacts to write
   ↓
proof-to-implementation (State 7) — bridge map (this document IS the bridge)
   ↓
holzman-rust (State 6, parallel) — Cargo.toml edit + Cargo.lock regen + moon run :lint-src
   ↓
black-hat-reviewer (State 8) — verifies PO-001 and PO-002 sub-evidence
   ↓
formal-verifier (State 12) — runs PO-001 and PO-002 evidence_command, records verification-ledger/v1
   ↓
landing-skill — jj land with the lockfile-drift guard
```

## 8. Cross-Reference

- See `proof-strategy.md` for the overall strategy and risk classification.
- See `verifier-lane-decisions.jsonl` for the 14 lane decisions (2 required +
  12 not_applicable).
- See `proof-obligations.planned.jsonl` for the 2 obligation rows.
- See `proof-coverage-matrix.md` for the (seed, obligation, lane) mapping.
- See `trusted-base-plan.md` for the (empty) trusted-base inventory.
- See `waiver-candidates.jsonl` for the (single, ledger-anchor) waiver row.
- See `contract.md` for the source-of-truth contract clauses and invariants.
- See `codebase-map.md` for the production source layout, baseline error
  counts, and downstream-crate analysis.