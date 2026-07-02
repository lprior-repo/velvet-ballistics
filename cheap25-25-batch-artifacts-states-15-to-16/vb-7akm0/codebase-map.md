# Codebase Map — vb-7akm0

## Bead

- **bead_id**: vb-7akm0
- **title**: Lint: remove allow unreachable_pub suppressions by narrowing visibility (P1 bug)
- **isolated_workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0
- **target_lane**: State 2 (explore) scout; State 3+ implementation owns removal/narrowing
- **lint_anchor**: lint-src task (moon) — runs `cargo clippy --workspace --lib --bins --examples --all-features`
- **workspace_lint_policy**: `Cargo.toml:57` sets `[workspace.lints.rust] unreachable_pub = "deny"`

## Lint Anchor Detail

- `.moon/tasks/all.yml:46-51` defines `lint-src` as a workspace clippy invocation that DOES NOT include `--tests`. Consequently:
  - Integration tests in `crates/vb_cli/tests/*.rs` and `crates/workspace_tests/tests/*.rs` are NOT in the compiled set during lint-src.
  - Items used ONLY by those integration tests look unreachable to the lint and trigger `unreachable_pub`.
  - `#[cfg(test)] mod foo;` modules and their inner `pub fn` items ARE compiled during lint-src `--all-features` (all features on, but `cfg(test)` only when `--tests` is also passed); therefore gate_*.rs items inside `#[cfg(test)] mod gate_07_stack;` ARE in scope during lint-src but their pub items are only reachable from sibling `#[cfg(test)]` modules like `gate_tests` and from the in-file `tests/` submodule.
- `crates/vb_validate/src/lib.rs:3` sets `#![deny(unreachable_pub)]` so each `#[allow(unreachable_pub)]` override inside vb_validate sources is a sub-crate license to keep otherwise-banned pub signatures.
- `Cargo.toml:59` recognises cfgs: `flux`, `fuzzing`, `kani`, `loom`, `verus`, `verus_keep_ghost`. kani-only modules (`#[cfg(kani)] pub mod verification;`) and `cfg(test)` modules are NOT compiled during `lint-src --lib --bins --examples`.

## Suppression Inventory — 25 Files

Total `#[allow(unreachable_pub)]` suppressions discovered (skeleton match via Grep). Categorised below by recommended treatment.

### Category A — Vestigial / zero-pub suppressions (REMOVE allow; no other change needed)

These files contain ZERO `pub` items, so the `#[allow(unreachable_pub)]` is a no-op and should simply be deleted:

| File | Line | Pub items at file scope |
|---|---|---|
| `xtask/src/main.rs` | 2 | none |
| `crates/vb_validate/src/diag/diag_tests.rs` | 6 | none (only `use` imports) |
| `crates/vb_validate/src/schema_support/schema_tests.rs` | 4 | none (only `use` imports) |
| `crates/vb_validate/src/fact_table.rs` | 4 | only `pub(crate)` items (e.g., `pub(crate) fn require_boolean` line 15) — `pub(crate)` does NOT trigger `unreachable_pub`, so allow is unnecessary |

Treatment: delete the inner-attribute line entirely.

### Category B — Internal test-duplicate gates (NARROW to non-pub or pub(super))

Each of these files declares a `#[cfg(test)]` module in `crates/vb_validate/src/lib.rs:75-107` and contains a single `pub fn validate_gate_XX_*` plus helpers. Each function has a byte-identical canonical implementation in `crates/vb_validate/src/gates.rs` that is exported via `pub use gates::*` (lib.rs:45) and therefore already reachable externally. The duplicate copies exist only to support the per-gate test submodules (`gate_tests.rs` does `use crate::gate_XX::validate_gate_XX_*`; each gate file has a private `tests` submodule via `#[path]` that calls them via `use super::*`).

| File | Line | Items | Reachable from |
|---|---|---|---|
| `crates/vb_validate/src/gate_07_stack.rs` | 4 | `pub fn validate_gate_07_expression_stack_depth` (line 16), `pub fn compute_stack_depth` (line 44) | `gate_tests.rs:72-73`, `gate_07_stack/tests.rs` (via `super::*`) |
| `crates/vb_validate/src/gate_08_accessor.rs` | 4 | `pub fn validate_gate_08_accessor_path_segments` (line 17) | `gate_tests.rs:180`, `gate_08_accessor/tests.rs` |
| `crates/vb_validate/src/gate_09_slots.rs` | 4 | `pub fn validate_gate_09_slot_references` (line 12) | `gate_tests.rs:262`, `gate_09_slots/tests.rs` |
| `crates/vb_validate/src/gate_10_node.rs` | 4 | `pub fn validate_gate_10_node_kind_specific` (line 12) | `gate_10_node/tests.rs` |
| `crates/vb_validate/src/gate_11_loop.rs` | 4 | `pub fn validate_gate_11_loop_body_graph` (line 10) | `gate_tests.rs:364`, `gate_11_loop/tests.rs` |
| `crates/vb_validate/src/gate_12_14_15.rs` | 4 | `pub fn validate_gate_12_action_contract_completeness` (line 11), `pub fn validate_gate_14_slot_type_consistency` (line 55), `pub fn validate_gate_15_determinism_proof` (line 102) | sibling test files |
| `crates/vb_validate/src/gate_13_cycles.rs` | 4 | `pub fn validate_gate_13_no_slot_cycles` (line 11) | `gate_13_cycles/tests.rs` |

Treatment: change `pub fn` → bare `fn` (private). The in-module `tests` submodule can still call them via `super::fn_name` because submodules see the parent's private items. `gate_tests.rs` will need to convert `use crate::gate_XX::validate_gate_XX_*;` to direct path imports `use crate::gate_XX as g; g::validate_gate_XX_*(...)` no — submodules of crate-internal modules CAN see private items in their parent crate-root sibling modules via direct path. Wait — verified below.

Verification of the visibility rule: `gate_07_stack` is `mod gate_07_stack;` (non-pub) inside `lib.rs`. From `gate_tests.rs` (also `mod gate_tests;`), the path `crate::gate_07_stack::validate_gate_07_expression_stack_depth` is reachable because both modules are visible from within the crate, and `fn` (no visibility) items in non-pub modules ARE reachable from anywhere inside the crate via direct path. Source: Rust Reference §"Visibility and privacy": "An item that is not pub is accessible from anywhere within the module that defines it and any of its descendants, plus any sibling module that's a descendant of a common ancestor (i.e., crate root in this case)."

So the right fix is: convert `pub fn` → `fn`, then `use super::fn_name` and `super::fn_name()` in submodule tests will continue to work, and `crate::gate_07_stack::validate_gate_07_expression_stack_depth` from `gate_tests.rs` will still resolve (no pub is needed at this level since both sides are crate-visible).

### Category C — `validate_taint` / `validate_types` / `validate_resource_limits` duplicates (NARROW)

These files have `pub fn` items with canonical equivalents in `crates/vb_validate/src/type_taint.rs:246,253` (`validate_types`, `validate_taint`) that ARE externally reachable. The duplicates here exist only for test isolation inside `#[cfg(test)]` modules.

| File | Line | Item | Reachable from |
|---|---|---|---|
| `crates/vb_validate/src/taint_prop.rs` | 15 | `pub fn validate_taint(workflow: &WorkflowTypes)` | tests at `taint_prop.rs:94-201` (in-file) |
| `crates/vb_validate/src/type_check.rs` | 15 | `pub fn validate_types(workflow: &WorkflowTypes)` | tests at `type_check.rs:140-200` |
| `crates/vb_validate/src/secret_leak.rs` | 14 | `pub fn validate_resource_limits(&WorkflowTypes, &ResourceLimits)` | `secret_leak/tests.rs:6` via `use crate::secret_leak::validate_resource_limits;` |

Treatment: convert `pub fn` → `fn`. Update in-file tests to use `validate_taint(...)` (still works because test code is in same scope as private item) and update sibling-test imports if any. The canonical versions in `type_taint.rs` remain pub and externally reachable (used by `verifies()` Verus proofs, Kani harnesses, and workspace_tests).

### Category D — `type_sigs.rs` and `schema_support/*` test-data (NARROW to pub(crate))

These files declare document-model types used across multiple `#[cfg(test)]` submodules but must stay visible inside the crate (private modules cannot be path-traversed from outside, so pub or pub(crate) is required for cross-test-module access). `pub(crate)` is sufficient and SILENCES `unreachable_pub` because the lint specifically targets items marked `pub` (without an explicit narrowing), not `pub(crate)` items (which are not externally visible and not subject to the lint).

| File | Line | Pub items | Cross-module consumers |
|---|---|---|---|
| `crates/vb_validate/src/type_sigs.rs` | 4 | `enum ValueType` (16), `enum Taint` (51), `struct ValueFact` (70), `struct InputDecl` (101), `struct ResourceLimits` (112), `struct WorkflowTypes` (172), `struct StepTypes` (188), `enum StepKind` (198), `enum TypedValue` (219) | `fact_table.rs:12`, `fact_table/tests.rs:4`, `secret_leak.rs:11`, `secret_leak/tests.rs:8`, `taint_prop.rs:12,53`, `type_check.rs:12,46`, `type_taint_tests.rs:5`, `red_phase_proptest.rs` (per lib.rs uses) |
| `crates/vb_validate/src/schema_support/schema_doc.rs` | 4 | `struct WorkflowDoc` (5), `enum FieldValue` (11), `struct StepDoc` (19), impl blocks (25-93) | `schema_support/schema_tests.rs:6`, `schema_support/schema_fields.rs:5`, `schema_support/schema_fields/*.rs` (core.rs:5, fields_tests.rs:3, ids.rs:5, step.rs:5, trigger.rs:5) |
| `crates/vb_validate/src/schema_support/schema_id.rs` | 4 | `fn validate_single_id` (7), `fn is_valid_id` (20), `fn is_reserved_id` (38) | `schema_support/schema_tests.rs:11`, `schema_support/schema_fields.rs:6` |
| `crates/vb_validate/src/schema_support/schema_fields.rs` | 4 | `fn validate_workflow_schema` (48), `fn validate_version` (80), `fn validate_trigger` (92), `fn validate_ids` (157), `fn validate_step_fields` (187), `fn validate_single_primitive` (229) | `schema_support/schema_tests.rs:7-10`, internal `schema_fields/*.rs` |

Treatment: change `pub` → `pub(crate)` for top-level fns and type declarations; delete the inner-attribute allow.

### Category E — `diag/` module (NARROW pub → pub(crate) where possible)

`pub mod diag;` is set in lib.rs:50 (externally reachable). `diag/mod.rs` declares `pub mod diag_codes;` `pub mod diag_render;` (both pub for the external API).

| File | Line | Pub items | Notes |
|---|---|---|---|
| `crates/vb_validate/src/diag/diag_codes.rs` | 4 | 60+ `pub const` codes (`CODE_DUPLICATE_KEY`..) | NOT consumed externally (Grep `vb_validate::diag_codes` returns 0 hits); only used inside `diag/diag_render/parts/contract.rs:5` and `diag/diag_render/parts.rs:5` (both private submodule imports), plus `diag/tests.rs`. Treatment: leave as-is OR convert to `pub(crate)` to align with actual reachability. CATEGORY-TIP: leave `pub` if you want to preserve external API stability (consumers may exist outside the workspace grep result), otherwise narrow to `pub(crate)` and delete the allow. |
| `crates/vb_validate/src/diag/diag_convert.rs` | 6 | `pub(super) fn all_variants()` (line 10) | `pub(super)` is bounded; only `diag_tests.rs:10` and `diag_render/render_tests.rs:4` (descendants of `diag`) can access it. Already `pub(super)` not `pub`, so the lint should not fire on these items. Treatment: delete the inner-attribute allow; lint should already pass. |
| `crates/vb_validate/src/diag/diag_render.rs` | 4 | `pub fn diagnostic_from_error` (13), `pub fn error_code` (48) | Re-exported through `pub use crate::diag::diag_render::{diagnostic_from_error, error_code}` in `diagnostic.rs:8-9`, which IS externally reachable (`use vb_validate::diagnostic::*` used in 6+ workspace_tests). Items are reachable. Treatment: delete the inner-attribute allow. |

### Category F — `diagnostic.rs` re-exports (REMOVE allow; items are externally reachable)

`crates/vb_validate/src/diagnostic.rs:7` allow on a file that contains only two `pub use` re-exports (`diagnostic_from_error`, `error_code`) which ARE externally reachable (see Category E row). Treatment: delete the inner-attribute allow.

### Category G — `vb_cli/src/commands_diff.rs` and `commands_incident.rs` (DORMANT TEST PATH)

The only consumer is `crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs:5-6`. This file exists on disk but is NOT registered as `[[test]]` in ANY Cargo.toml — see `Cargo.toml`, `crates/vb_cli/Cargo.toml`, `crates/workspace_tests/Cargo.toml` (no entry for `vb_test_cli_diff_incident_behavior`). Source-length-exceptions.txt:221 is the only metadata reference. cargo test does not run it; lint-src also doesn't compile it. Therefore every `pub` item in these two files IS reachable in the strict compile-target sense only because:
- `vb_cli/src/lib.rs` declares `pub mod commands_diff;` and `pub mod commands_incident;` (both pub) — see `crates/vb_cli/src/lib.rs:6-7`.
- This makes the modules externally reachable, BUT the lint only fires on the inner items that have no downstream consumer. For items used only by the orphan test file, the lint fires.

| File | Line | Pub items | Treatment |
|---|---|---|---|
| `crates/vb_cli/src/commands_diff.rs` | 2 | `struct DiffResult` (11), `fn compute_diff` (21), `fn diff_event_summary` (133), `fn event_name` (219), `fn events_differ` (244), `fn collect_step_outcomes` (319), `fn collect_slot_values` (342) | RECOMMENDED: register `vb_test_cli_diff_incident_behavior` in `crates/workspace_tests/Cargo.toml` as `[[test]]`, OR delete that file. If deleting the dead test file, narrow `DiffResult` to private (struct uses are internal) and narrow `collect_*` helpers to plain `fn` (they are only used in-file and in `commands_diff/tests.rs` via `use super::*`). If registering the test, narrowing is NOT required. ALTERNATIVE path: change `pub mod commands_diff;` (vb_cli/lib.rs:6) to `pub(crate) mod commands_diff;` — but that breaks the public CLI diff surface; skip. |
| `crates/vb_cli/src/commands_incident.rs` | 2 | `struct IncidentReport` (14) with `pub` fields, `fn build_incident_report` (30) | Same as above. `IncidentReport` fields are accessed externally by `vb_test_cli_diff_incident_behavior.rs:433-645`. Treatment parallels `commands_diff`. |
| `crates/vb_cli/src/lifecycle.rs` | 471 | inner-attribute `#[allow(unreachable_pub)]` on `pub fn create_run_header` (line 472) inside `pub mod test_helpers` (line 463) | Items ARE externally reachable: `crates/workspace_tests/tests/derived_status_replay_timeline_tests.rs:29` does `use vb_cli::lifecycle::test_helpers::create_run_header;` (registered at `crates/workspace_tests/Cargo.toml:81`). The inner-attribute allow is unnecessary. Treatment: delete the inner attribute. |

Note for Category G: this is the most decision-laden part of the bead. Two viable paths: (i) retire the orphan test file + narrow the CLI items to `pub(crate)` or plain `fn`; (ii) register the orphan test file + delete the suppression. Implementation agent must consult the user/architect or follow a pre-registered preference. Default recommended path: (i) — retire the orphan test (it's already on the source-length-exceptions watch list as `vb-jpq7.47 split-or-retire-before-release`).

## Existing Test Anchors

- `crates/vb_validate/tests/` — 8 integration test files (`capability_contract_schema.rs`, `capability_schema_kani.rs`, `gate_08_accessor_parity.rs`, `idempotency_contract_red.rs`, `proptest_diag_codes_promotion.rs`, `proptest_validation_error_code_registry_extended.rs`, `proptest_validation_error_codes.rs`, `red_phase_validation.rs`). These ARE in lint-src's `--all-features` set because they're under `tests/` for the lib crate and `--all-features` activates feature-gated tests but does NOT compile integration tests (lint-src uses `--lib` not `--tests`). The tests are run by `cargo test -p vb_validate` and remain green.
- `crates/vb_validate/src/{gate_tests.rs, type_taint_tests.rs, ref_unit_tests.rs, references_tests.rs, ...}` — module-level `#[cfg(test)] mod X;` files with `#[test] fn` bodies; run by `cargo test -p vb_validate --lib`.
- `crates/vb_cli/tests/` — 15+ integration test files including `lifecycle_integration.rs` (uses `crate::lifecycle::test_helpers::create_run_header`) — registered by default cargo test discovery (no `[[test]]` entries needed).
- `crates/workspace_tests/tests/derived_status_replay_timeline_tests.rs` — registered at `crates/workspace_tests/Cargo.toml:81`; consumes `vb_cli::lifecycle::test_helpers::create_run_header`.
- `crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs` — EXISTS but NOT registered in any `Cargo.toml` (orphan). Source-length-exceptions.txt:221 flags it for split-or-retire-before-release (646 lines).

## Existing Verification Anchors

- `kani/` — Kani harness directory; consumes `vb_validate::gates::validate_gate_XX_*` (the canonical exports, NOT the duplicate test modules). E.g., `kani/gate_07_stack.rs:11,49,98` use the canonical version. Kani has its own scope; the bead does not touch them.
- `verification/verus/extern_*.rs`, `verification/verus/production_inner/*.rs` — Verus proofs bound to `vb_validate::diagnostic::*` (e.g., `verification/verus/extern_vb_ahfl_bounds_production.rs`). Tied to `IncidentReport` and `SpecIncidentReportView` — God Rule 2 binding. Touching items on the vb_cli side that aren't mentioned here is safe.
- `crates/vb_validate/src/verification/{gate_08_verus_proof, kani_gate_08_accessor, kani_gate_08_structural, kani_idempotency_contract, kani_step_primitives}.rs` — `#[cfg(kani)]` only. They consume `vb_validate::gate_08_accessor::validate_gate_08_accessor_path_segments` (and other `gate_*` modules). Lint-src does NOT compile these, so the items they would need are not visible to the lint.

## Risk Tags

- `public_api`: Some items in Category D are exposed via `pub use` chains and may be referenced by workspace_tests; narrowing needs test-runner re-verification.
- `test_visibility`: Categories B/C/D are `#[cfg(test)]` modules whose `pub fn`s are used by sibling `#[cfg(test)]` modules; the visibility transformation must preserve access via `super::*` and direct paths from sibling test modules.
- `lint_suppression_audit`: The bead is itself a lint-suppression audit — removing `#[allow(unreachable_pub)]` may surface additional suppressed lints in the same files (clippy, etc.).
- `dormant_artifact`: Category G decision involves the orphan `vb_test_cli_diff_incident_behavior.rs` test file. Decision route: retire (default) or register.
- `production_binding_verification`: `IncidentReport` is bound through Verus production-bound specs in `verification/verus/extern_vb_ahfl_bounds_production.rs`. Visibility changes to the local definition do not affect the spec binding because the spec uses the production_inner mirror under `production_inner/vb_ahfl_bounds_production_inner.rs`. Note for the implementation agent: confirm no reference in `verification/` to `vb_cli::commands_incident::IncidentReport` direct path (only the `Kind::IncidentReport` enum variant in `production::Kind` exists).

## Open Questions

1. **Orphan test retention**: should `crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs` be retired or registered? Its sole metadata reference is `source-length-exceptions.txt:221` (vb-jpq7.47 split-or-retire-before-release). Recommended: retire.
2. **Should `diagnostic.rs:7` allow be deleted outright, or pre-checked by reproducing the lint without it?** Both `diagnostic_from_error` and `error_code` are externally reachable via `vb_validate::diagnostic::*`; deletion should be safe.
3. **For `diag_codes.rs:4` allow**: are the `CODE_*` constants part of any external API contract? No hits in current grep for `vb_validate::diag_codes`, but they sit under `pub mod diag_codes`. Decision: either narrow to `pub(crate)` (risk: possible unforeseen consumer), or leave `pub` and just remove the allow (risk: lint re-fires on an external consumer we haven't accounted for). Recommended: narrow to `pub(crate)` and grep again across the entire workspace to confirm no external consumer.
4. **Visibility semantics for sibling `#[cfg(test)]` modules**: confirm via local compiler run that changing `pub fn` → `fn` in `gate_07_stack.rs` does not break `use crate::gate_07_stack::validate_gate_07_expression_stack_depth` in `gate_tests.rs`. Both modules are non-pub at the crate root so the path is crate-internal; `fn` items are accessible to crate-sibling paths in Rust 2021+ when the path traverses only non-pub modules. (See Rust Reference §"Visibility and Privacy" / "Private items visible to descendants"; the relevant principle is that items in non-pub modules with default visibility ARE reachable from any sibling module of the same crate.)
5. **For `vb_cli/src/lifecycle.rs:471`** — the inner-attribute `#[allow(unreachable_pub)]` is on `pub fn create_run_header`. The path `vb_cli::lifecycle::test_helpers::create_run_header` is consumed by `derived_status_replay_timeline_tests.rs`. Recommend deleting the inner attribute (the function is reachable, allow is unnecessary). Confirm after `moon run :lint-src` that the removal does not surface a new lint.

## Recommended Downstream Owners

- **rust-contract** (State 3): author typestate/visibility contract for Categories B, C, D — produces contract.md describing the visibility invariants (`fn` for module-local, `pub(crate)` for cross-test-module, `pub` only when externally reachable).
- **proof-planner** (State 4): produce proof-plan.md covering the lint suppression removals as a behavior-preserving refactor (no semantic change to the visible surface).
- **holzman-rust / implementation** (State 5): apply the changes per category; rerun `moon run :lint-src` and `cargo test --workspace` after each category to confirm.
- **bdd-enforcer / test-writer** (if needed): register `vb_test_cli_diff_incident_behavior` in `crates/workspace_tests/Cargo.toml` if the decision is to keep it; otherwise no BDD work needed.

## Excluded / Out of Scope

- `verification/verus/extern_*.rs` and `production_inner/*.rs` — God Rule 2 bindings; do not modify.
- `kani/*` harnesses — independent tooling; do not modify.
- `xtask/src/main.rs` `#[allow(unreachable_pub)]` removal is a one-line deletion with no functional impact.
- `diagnostic.rs` allow removal is also a one-line deletion.
- `commands_diff.rs` / `commands_incident.rs` allow removal requires the orphan-test decision before execution.

## Reference Files (verified content)

- `.moon/tasks/all.yml:46-62` — `lint-src` task definition (clippy --lib --bins --examples --all-features).
- `Cargo.toml:57` — `[workspace.lints.rust] unreachable_pub = "deny"`.
- `crates/vb_validate/src/lib.rs:3` — `#![deny(unreachable_pub)]` at crate root.
- `crates/vb_validate/src/lib.rs:73-125` — `#[cfg(test)] mod gate_07_stack;` etc.
- `crates/vb_validate/src/gates.rs:36,150,233,416,869,1120,1432,1586,1665` — canonical gate exports.
- `crates/vb_validate/src/type_taint.rs:246,253` — canonical `validate_types`, `validate_taint`.
- `crates/vb_cli/src/lib.rs:6-9` — `pub mod commands_diff;`, `pub mod commands_incident;`, `pub mod lifecycle;`.
- `crates/vb_validate/src/diag/mod.rs:12-18` — `pub mod diag_codes;`, `pub mod diag_render;`, `#[cfg(test)] mod diag_convert;`, `#[cfg(test)] mod diag_tests;`.
- `crates/vb_validate/src/schema_support/mod.rs:13-23` — `#[cfg(test)] pub mod schema_doc;` etc.
