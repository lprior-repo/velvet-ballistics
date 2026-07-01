# Contract — vb-7akm0 Lint-Suppression Audit

| Field | Value |
|---|
| bead_id | vb-7akm0 |
| state | 3 (rust-contract) |
| skill | rust-contract |
| source_checkout | /home/lewis/src/velvet-ballistics |
| isolated_workspace | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0 |
| captured_at | 2026-07-01T16:04Z |
| upstream_artifacts | domain-model.md, type-contracts.md, workflow-model.md, error-taxonomy.md, boundary-map.md, hazard-analysis.md |

## 0. Scope

This file is the top-level contract for the lint-suppression audit bead. It binds the requirement IDs, contract clauses, and downstream-proof anchors that the rest of the State 3 artifacts (and the State 4+ obligations) will reference. It is NOT a proof artifact, a test plan, or an implementation contract; it is the cross-reference that lets a downstream reviewer verify that every domain concern has a corresponding type contract, error variant, boundary map entry, hazard, and proof seed.

The contract clauses are partitioned by category:

| Clause prefix | Category |
|---|---|
| `LS-VESTIGIAL` | A (vestigial suppressions, delete-allow only) |
| `LS-INTERNAL` | B (gate internal duplicates, pub fn → fn) |
| `LS-TAINT` | C (taint/type/secret-leak duplicates, pub fn → fn) |
| `LS-SCHEMA` | D (schema support narrow, pub → pub(crate)) |
| `LS-DIAG` | E (diag module mixed treatments) |
| `LS-REEXPORT` | F (diagnostic.rs re-export, delete-allow) |
| `LS-ORPHAN` | G (orphan-test decision) |
| `LS-LIFECYCLE` | G (lifecycle.rs delete-allow) |
| `LS-INVARIANT` | bead-wide (Rust visibility invariant) |
| `LS-VERIFY` | bead-wide (lint-src + cargo test gates) |

## 1. Cross-Reference Matrix

| Req ID | Category | Clause | Domain claim | Type anchor | Error variant | Boundary | Hazard | Behavior-affecting |
|---|---|---|---|---|---|---|---|---|
| `R-vb-7akm0-001` | A | `LS-VESTIGIAL.1` | `xtask/src/main.rs` has zero `pub` items; removing the inner-attribute allow is safe. | `Suppression { kind: VestigialSuppression }` | n/a (delete-allow only) | boundary-map §2.1 | H1, H9 | No |
| `R-vb-7akm0-002` | A | `LS-VESTIGIAL.2` | `crates/vb_validate/src/diag/diag_tests.rs` has zero `pub` items; removing the inner-attribute allow is safe. | same as 001 | n/a | boundary-map §2.1 | H1, H9 | No |
| `R-vb-7akm0-003` | A | `LS-VESTIGIAL.3` | `crates/vb_validate/src/schema_support/schema_tests.rs` has zero `pub` items; removing the inner-attribute allow is safe. | same as 001 | n/a | boundary-map §2.1 | H1, H9 | No |
| `R-vb-7akm0-004` | A | `LS-VESTIGIAL.4` | `crates/vb_validate/src/fact_table.rs` has only `pub(crate)` items; the allow is a no-op (the lint does not fire on `pub(crate)` items). | same as 001 | n/a | boundary-map §2.1 | H1, H9 | No |
| `R-vb-7akm0-005` | B | `LS-INTERNAL.1` | `gate_07_stack::{validate_gate_07_expression_stack_depth, compute_stack_depth}` are reachable from sibling `#[cfg(test)] mod` via `crate::gate_07_stack::name` and from in-file `mod tests` via `super::*`. Narrowing to `fn` preserves both paths. | `ConsumerRef { import_style: CratePath \| SuperPath }` | `CargoTestNonZeroExit` | boundary-map §2.2 | H2, H9 | No |
| `R-vb-7akm0-006` | B | `LS-INTERNAL.2` | `gate_08_accessor::validate_gate_08_accessor_path_segments` is reachable from sibling `#[cfg(test)] mod gate_tests` (line 180) and from `gate_08_accessor/tests.rs`. Narrowing to `fn` preserves both paths. | same as 005 | same as 005 | boundary-map §2.2 | H2, H9 | No |
| `R-vb-7akm0-007` | B | `LS-INTERNAL.3` | `gate_09_slots::validate_gate_09_slot_references` is reachable from `gate_tests.rs:262` and `gate_09_slots/tests.rs`. Narrowing to `fn` preserves both paths. | same as 005 | same as 005 | boundary-map §2.2 | H2, H9 | No |
| `R-vb-7akm0-008` | B | `LS-INTERNAL.4` | `gate_10_node::validate_gate_10_node_kind_specific` is reachable from `gate_10_node/tests.rs` only. Narrowing to `fn` preserves the path. | same as 005 | same as 005 | boundary-map §2.2 | H2, H9 | No |
| `R-vb-7akm0-009` | B | `LS-INTERNAL.5` | `gate_11_loop::validate_gate_11_loop_body_graph` is reachable from `gate_tests.rs:364` and `gate_11_loop/tests.rs`. Narrowing to `fn` preserves both paths. | same as 005 | same as 005 | boundary-map §2.2 | H2, H9 | No |
| `R-vb-7akm0-010` | B | `LS-INTERNAL.6` | `gate_12_14_15::{validate_gate_12_action_contract_completeness, validate_gate_14_slot_type_consistency, validate_gate_15_determinism_proof}` are reachable from sibling test submodules. Narrowing all three to `fn` preserves the paths. | same as 005 | same as 005 | boundary-map §2.2 | H2, H9 | No |
| `R-vb-7akm0-011` | B | `LS-INTERNAL.7` | `gate_13_cycles::validate_gate_13_no_slot_cycles` is reachable from `gate_13_cycles/tests.rs`. Narrowing to `fn` preserves the path. | same as 005 | same as 005 | boundary-map §2.2 | H2, H9 | No |
| `R-vb-7akm0-012` | C | `LS-TAINT.1` | `taint_prop::validate_taint` is reachable from in-file tests (lines 94-201) and from `type_taint.rs:253` (canonical export). Narrowing to `fn` preserves the in-file path; canonical remains `pub`. | `ConsumerRef { import_style: SuperExplicit }` | `CargoTestNonZeroExit` | boundary-map §2.3 | H2, H9 | No |
| `R-vb-7akm0-013` | C | `LS-TAINT.2` | `type_check::validate_types` is reachable from in-file tests (lines 140-200) and from `type_taint.rs:246` (canonical export). Narrowing to `fn` preserves the in-file path; canonical remains `pub`. | same as 012 | same as 012 | boundary-map §2.3 | H2, H9 | No |
| `R-vb-7akm0-014` | C | `LS-TAINT.3` | `secret_leak::validate_resource_limits` is reachable from `secret_leak/tests.rs:6` via `use crate::secret_leak::validate_resource_limits;`. Narrowing to `fn` preserves the crate-internal direct path. | `ConsumerRef { import_style: CratePath }` | `CargoTestNonZeroExit` | boundary-map §2.3 | H2, H9 | No |
| `R-vb-7akm0-015` | D | `LS-SCHEMA.1` | `type_sigs::{ValueType, Taint, ValueFact, InputDecl, ResourceLimits, WorkflowTypes, StepTypes, StepKind, TypedValue}` are used by `fact_table.rs`, `secret_leak.rs`, `taint_prop.rs`, `type_check.rs`, `type_taint_tests.rs`, etc. Narrowing all nine to `pub(crate)` preserves cross-`#[cfg(test)]`-module access. | `Visibility::PubCrate` | `CargoTestNonZeroExit` | boundary-map §2.4 | H3, H9 | No |
| `R-vb-7akm0-016` | D | `LS-SCHEMA.2` | `schema_support/schema_doc.rs` items (WorkflowDoc, FieldValue, StepDoc, plus 9 methods) are used by `schema_tests.rs`, `schema_fields.rs`, and `schema_fields/*.rs`. Narrowing all 12 to `pub(crate)` preserves cross-test access. | same as 015 | same as 015 | boundary-map §2.4 | H3, H9 | No |
| `R-vb-7akm0-017` | D | `LS-SCHEMA.3` | `schema_support/schema_id.rs` items (`validate_single_id`, `is_valid_id`, `is_reserved_id`) are used by `schema_tests.rs:11` and `schema_fields.rs:6`. Narrowing to `pub(crate)` preserves access. | same as 015 | same as 015 | boundary-map §2.4 | H3, H9 | No |
| `R-vb-7akm0-018` | D | `LS-SCHEMA.4` | `schema_support/schema_fields.rs` items (6 fns) are used by `schema_tests.rs:7-10`, `schema_fields/core.rs:6`, `schema_fields/ids.rs:6`, `schema_fields/step.rs:6`. Narrowing to `pub(crate)` preserves access. | same as 015 | same as 015 | boundary-map §2.4 | H3, H9 | No |
| `R-vb-7akm0-019` | E | `LS-DIAG.1` | `diag_codes.rs` has 60+ `CODE_*` constants not consumed externally. The implementation MUST decide between `DeleteAllow` (keep `pub`, assume external API stability) and `PubToPubCrate` (narrow + grep first). | `Treatment::DeleteAllow \| PubToPubCrate` | `LintSrcNonZeroExit` if decision is wrong | boundary-map §2.5 | H4, H9 | No |
| `R-vb-7akm0-020` | E | `LS-DIAG.2` | `diag_convert::all_variants` is `pub(super)`, NOT `pub`. The lint does NOT fire on `pub(super)` items. The inner-attribute allow is unnecessary; deleting it is safe. | `Visibility::PubSuper` | n/a | boundary-map §2.5 | H9 | No |
| `R-vb-7akm0-021` | E | `LS-DIAG.3` | `diag_render::{diagnostic_from_error, error_code}` are re-exported via `diagnostic.rs:8-9` and ARE externally reachable. Deleting the inner-attribute allow is safe. | `Visibility::Pub` + `reachable_via_external_path: true` | n/a | boundary-map §2.5 | H5, H9 | No |
| `R-vb-7akm0-022` | F | `LS-REEXPORT.1` | `diagnostic.rs` re-exports `diagnostic_from_error` and `error_code`. Both are externally reachable via `vb_validate::diagnostic::*` (used by 6+ workspace_tests). Deleting the inner-attribute allow is safe. | same as 021 | n/a | boundary-map §2.6 | H5, H9 | No |
| `R-vb-7akm0-023` | G | `LS-ORPHAN.1` | `commands_diff.rs` has 7 `pub` items consumed only by the orphan test `vb_test_cli_diff_incident_behavior.rs` (not registered in any Cargo.toml). The default recommendation is to retire the orphan test and narrow the items to `pub(crate)` or private. The implementation MUST record a decision before executing. | `Treatment::DecisionRequired { recommendation: RetireOrphanTest \| RegisterOrphanTest }` | `DecisionMissing`, `DecisionConflictsDefault` | boundary-map §2.7 | H6, H9 | No |
| `R-vb-7akm0-024` | G | `LS-ORPHAN.2` | `commands_incident::{IncidentReport, build_incident_report}` are consumed only by the orphan test. The default recommendation is to retire. Pre-condition: `grep IncidentReport verification/verus/production_inner/` MUST return no results before narrowing. | same as 023 + `ProductionBindingVerification` risk tag | `DecisionMissing`, `NewUnreachablePubLabel` if production_inner mirror breaks | boundary-map §2.7 | H6, H7, H9 | No |
| `R-vb-7akm0-025` | G | `LS-LIFECYCLE.1` | `lifecycle::test_helpers::create_run_header` IS externally reachable (via `vb_cli::lifecycle::test_helpers::create_run_header`, used by `derived_status_replay_timeline_tests.rs:29` and `lifecycle_integration.rs`). Deleting the inner-attribute allow is safe. | `Visibility::Pub` + `reachable_via_external_path: true` | n/a | boundary-map §2.7 | H8, H9 | No |
| `R-vb-7akm0-026` | bead-wide | `LS-INVARIANT.1` | After all 25 suppressions are cleared, every remaining `pub` item in the workspace is reachable from an external path (downstream crate, registered integration test, or `#[cfg(test)] mod` in the lint-src compile set). | `VisibilityInvariant::is_satisfied()` | n/a | boundary-map §1, §3 | H9, H10 | No |
| `R-vb-7akm0-027` | bead-wide | `LS-INVARIANT.2` | The change is behavior-preserving: no production-code symbol changes its semantics; only the visibility or attribute metadata changes. | `Suppression::is_behavior_preserving() == true` for all 25 rows | `AuditTrailInvariant` | boundary-map §4 | H9 | No |
| `R-vb-7akm0-028` | bead-wide | `LS-VERIFY.1` | `moon run :lint-src` exits 0 after all 25 changes are applied. | `LintSrcNonZeroExit` is never raised | n/a | boundary-map §2.1-§2.7 | H9, H10 | No |
| `R-vb-7akm0-029` | bead-wide | `LS-VERIFY.2` | `cargo test --workspace` exits 0 after all 25 changes are applied. | `CargoTestNonZeroExit` is never raised | n/a | boundary-map §2.1-§2.7 | H2, H3, H8, H9 | No |
| `R-vb-7akm0-030` | bead-wide | `LS-VERIFY.3` | `bash scripts/check-verus-production-binding.sh` exits 0 after Category G changes (defends H7). | `AuditTrailInvariant` | n/a | boundary-map §2.7 | H7 | No |

## 2. Contract Clauses

### 2.1 `LS-VESTIGIAL` (Category A, vestigial suppressions)

```text
LS-VESTIGIAL.1
  xtask/src/main.rs has no `pub` items at file scope; the inner-attribute
  `#[allow(unreachable_pub)]` is a no-op. Removing the attribute MUST NOT
  cause `lint-src` to exit non-zero.

LS-VESTIGIAL.2
  crates/vb_validate/src/diag/diag_tests.rs has no `pub` items at file scope;
  the allow is a no-op. Removing the attribute MUST NOT cause `lint-src` to
  exit non-zero.

LS-VESTIGIAL.3
  crates/vb_validate/src/schema_support/schema_tests.rs has no `pub` items
  at file scope; the allow is a no-op. Removing the attribute MUST NOT
  cause `lint-src` to exit non-zero.

LS-VESTIGIAL.4
  crates/vb_validate/src/fact_table.rs has only `pub(crate)` items at file
  scope (e.g., `pub(crate) fn require_boolean`); the lint does NOT fire on
  `pub(crate)` items. Removing the allow MUST NOT cause `lint-src` to exit
  non-zero.
```

### 2.2 `LS-INTERNAL` (Category B, gate internal duplicates)

```text
LS-INTERNAL.1..7
  For each `gate_XX.rs` file in {07_stack, 08_accessor, 09_slots, 10_node,
  11_loop, 12_14_15, 13_cycles}, every `pub fn` listed in
  `pub_items_at_file_scope` is reachable from:
    (a) a sibling `#[cfg(test)] mod` via `crate::gate_XX::name` direct path, AND
    (b) an in-file `mod tests` submodule via `use super::*` or `super::name()`.
  Narrowing `pub fn` → `fn` MUST preserve both reachability paths, AND
  `cargo test -p vb_validate --lib` MUST exit 0 after the narrowing.
```

### 2.3 `LS-TAINT` (Category C, taint/type/secret-leak duplicates)

```text
LS-TAINT.1
  taint_prop.rs:15 `pub fn validate_taint(workflow: &WorkflowTypes)` is
  reachable from in-file tests at lines 94-201 via name resolution. The
  canonical export at type_taint.rs:253 remains `pub`. Narrowing the
  duplicate to `fn` MUST NOT cause in-file tests to fail.

LS-TAINT.2
  type_check.rs:15 `pub fn validate_types(workflow: &WorkflowTypes)` is
  reachable from in-file tests at lines 140-200 via name resolution. The
  canonical export at type_taint.rs:246 remains `pub`. Narrowing the
  duplicate to `fn` MUST NOT cause in-file tests to fail.

LS-TAINT.3
  secret_leak.rs:14 `pub fn validate_resource_limits(&WorkflowTypes, &ResourceLimits)`
  is reachable from secret_leak/tests.rs:6 via
  `use crate::secret_leak::validate_resource_limits;`. Narrowing to `fn`
  MUST preserve the crate-internal direct path.
```

### 2.4 `LS-SCHEMA` (Category D, schema support narrow)

```text
LS-SCHEMA.1
  type_sigs.rs items ValueType, Taint, ValueFact, InputDecl, ResourceLimits,
  WorkflowTypes, StepTypes, StepKind, TypedValue are used by fact_table.rs,
  secret_leak.rs, taint_prop.rs, type_check.rs, type_taint_tests.rs, etc.
  Narrowing each from `pub` to `pub(crate)` MUST preserve cross-`#[cfg(test)]`-
  module access.

LS-SCHEMA.2
  schema_support/schema_doc.rs items (WorkflowDoc, FieldValue, StepDoc,
  plus 9 methods) are used by schema_tests.rs, schema_fields.rs, and
  schema_fields/*.rs. Narrowing all 12 to `pub(crate)` MUST preserve access.

LS-SCHEMA.3
  schema_support/schema_id.rs items (`validate_single_id`, `is_valid_id`,
  `is_reserved_id`) are used by schema_tests.rs:11 and schema_fields.rs:6.
  Narrowing all 3 to `pub(crate)` MUST preserve access.

LS-SCHEMA.4
  schema_support/schema_fields.rs items (validate_workflow_schema,
  validate_version, validate_trigger, validate_ids, validate_step_fields,
  validate_single_primitive) are used by schema_tests.rs:7-10,
  schema_fields/core.rs:6, schema_fields/ids.rs:6, schema_fields/step.rs:6.
  Narrowing all 6 to `pub(crate)` MUST preserve access.
```

### 2.5 `LS-DIAG` (Category E, diag module)

```text
LS-DIAG.1
  diag_codes.rs has 60+ `pub const CODE_*: u16` declarations not consumed
  externally (per fresh grep). The implementation MUST decide between:
    (a) DeleteAllow: keep `pub`, remove the inner-attribute.
    (b) PubToPubCrate: narrow all 60+ constants to `pub(crate)`.
  If option (b) is chosen, `grep -R 'vb_validate::diag::diag_codes::CODE_' .`
  MUST return no external consumer. Both options MUST satisfy
  `moon run :lint-src` exit 0.

LS-DIAG.2
  diag_convert.rs:10 `pub(super) fn all_variants()` is NOT subject to the
  `unreachable_pub` lint (which targets `pub`, not `pub(super)`). Removing
  the inner-attribute allow is safe and MUST NOT cause `lint-src` to exit
  non-zero.

LS-DIAG.3
  diag_render.rs items `diagnostic_from_error` (line 13) and `error_code`
  (line 48) are re-exported via diagnostic.rs:8-9 and ARE externally
  reachable. Removing the inner-attribute allow is safe.
```

### 2.6 `LS-REEXPORT` (Category F, diagnostic.rs)

```text
LS-REEXPORT.1
  diagnostic.rs re-exports `diagnostic_from_error` and `error_code` via
  `pub use`. Both are externally reachable via `vb_validate::diagnostic::*`
  (consumed by 6+ workspace_tests). Removing the inner-attribute allow is
  safe and MUST NOT cause `lint-src` to exit non-zero.
```

### 2.7 `LS-ORPHAN` (Category G, orphan-test decision)

```text
LS-ORPHAN.1
  commands_diff.rs has 7 `pub` items (DiffResult, compute_diff,
  diff_event_summary, event_name, events_differ, collect_step_outcomes,
  collect_slot_values) consumed only by the orphan test
  `vb_test_cli_diff_incident_behavior.rs` (NOT registered in any
  Cargo.toml). The implementation MUST record a decision (RetireOrphanTest
  or RegisterOrphanTest) before executing the change. The default
  recommendation is RetireOrphanTest.

LS-ORPHAN.2
  commands_incident.rs has 2 `pub` items (IncidentReport, build_incident_report)
  consumed only by the orphan test. Pre-condition for narrowing IncidentReport:
  `grep IncidentReport verification/verus/production_inner/` MUST return no
  results (verifies the production_inner mirror is independent).
```

### 2.8 `LS-LIFECYCLE` (Category G, lifecycle.rs)

```text
LS-LIFECYCLE.1
  lifecycle.rs:472 `pub fn create_run_header` inside `pub mod test_helpers`
  (line 463) IS externally reachable via
  `vb_cli::lifecycle::test_helpers::create_run_header`, used by
  derived_status_replay_timeline_tests.rs:29 and lifecycle_integration.rs.
  Removing the inner-attribute allow is safe.
```

### 2.9 `LS-INVARIANT` (bead-wide)

```text
LS-INVARIANT.1
  After all 25 suppressions are cleared, every remaining `pub` item in the
  workspace satisfies `reachable_via_external_path == true` (i.e., is
  reachable from a downstream crate, a registered integration test, or a
  `#[cfg(test)] mod` in the lint-src compile set).

LS-INVARIANT.2
  The change is behavior-preserving: no production-code symbol changes its
  semantics. The `behavior_affecting` field is `false` for every
  `Suppression` row.
```

### 2.10 `LS-VERIFY` (bead-wide)

```text
LS-VERIFY.1
  `moon run :lint-src` exits 0 against the post-change source.

LS-VERIFY.2
  `cargo test --workspace` exits 0 against the post-change source,
  specifically:
    cargo test -p vb_validate --lib
    cargo test -p vb_cli --lib
    cargo test --workspace --tests

LS-VERIFY.3
  `bash scripts/check-verus-production-binding.sh` exits 0 after Category G
  changes (defends H7 production-binding drift).
```

## 3. Behavior-Affecting Flagging

The contract clauses tagged `No` in §1's `Behavior-affecting` column are behavior-preserving; they do not change production behavior. They may still produce `behavior_affecting: false` proof seeds (e.g., `verus` postconditions, `flux` refinements on the parse function).

No clause in this bead is `Yes`. This is the defining property of a lint-compliance refactor.

## 4. Risk Coverage Table

| Risk tag (from delivery-scope.jsonl) | Clause(s) | Proof seed IDs |
|---|---|---|
| `lint_suppression_audit` | All clauses | PS-vb-7akm0-001..PS-vb-7akm0-030 |
| `test_visibility` | LS-INTERNAL.1..7, LS-TAINT.1..3 | PS-vb-7akm0-005..PS-vb-7akm0-014 |
| `public_api` | LS-DIAG.1, LS-REEXPORT.1, LS-LIFECYCLE.1 | PS-vb-7akm0-019, PS-vb-7akm0-022, PS-vb-7akm0-025 |
| `dormant_artifact` | LS-ORPHAN.1, LS-ORPHAN.2 | PS-vb-7akm0-023, PS-vb-7akm0-024 |
| `decision_required` | LS-ORPHAN.1, LS-ORPHAN.2 | PS-vb-7akm0-023, PS-vb-7akm0-024 |
| `production_binding_verification` | LS-ORPHAN.2 | PS-vb-7akm0-024 |
| `test_suite_reverify` | LS-VERIFY.1, LS-VERIFY.2 | PS-vb-7akm0-028, PS-vb-7akm0-029 |

## 5. Open Contract Questions

1. Whether `LS-DIAG.1` should be split into two separate clauses (DeleteAllow-only vs PubToPubCrate) or kept as a single clause with two options. Recommendation: keep as one clause; the decision is implementation-time.
2. Whether `LS-INVARIANT.2` should be enforced by a `cargo clippy --workspace --all-features` lint group (e.g., `clippy::restriction`) that flags `behavior_affecting: true` rows. Recommendation: yes; defer to a future bead.
3. Whether `LS-VERIFY.3` should be a hard pre-condition or a soft check. Recommendation: hard pre-condition for Category G rows.
4. Whether `LS-VERIFY.2` should be split into per-crate invocations to localize failures. Recommendation: yes; the implementation owner runs each per-crate invocation separately after each category.

End of contract.md.