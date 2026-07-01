# Hazard Analysis — vb-7akm0 Lint-Suppression Audit

| Field | Value |
|---|
| bead_id | vb-7akm0 |
| state | 3 (rust-contract) |
| skill | rust-contract |
| captured_at | 2026-07-01T16:04Z |
| upstream_artifacts | domain-model.md, type-contracts.md, workflow-model.md, error-taxonomy.md, boundary-map.md |

## 0. Scope

This file enumerates every hazard the lint-suppression audit domain must defend against, classified by category. Each hazard names the threat model, the affected suppression(s), the defense(s), and the residual risk. Hazards are *speculative* until proven by the corresponding proof seed and ledger evidence; this file lists the seeds, not the proof closure.

Per the rust-contract skill rules: hazards are emitted as `proof-seed/v1` rows in `proof-seeds.jsonl`. The `proof-planner` skill owns final lane decisions and the obligation set; this file does not commit to verifier commands.

## 1. Hazard Catalog

### H1 — Vestigial-Suppression Drift (Category A)

- **Category**: Rust-core invariant (lint discipline).
- **Threat model**: A future commit re-introduces an `#[allow(unreachable_pub)]` on a file with zero `pub` items. The suppression is a no-op but clutters the source and may mask a future legitimate lint.
- **Affected suppressions**: 4 (xtask/main.rs, diag_tests.rs, schema_tests.rs, fact_table.rs).
- **Defense**:
  - `Suppression.is_delete_allow_only()` is verified by the parser boundary.
  - `moon run :lint-src` after the deletion MUST exit 0 (confirms no new `pub` items surfaced).
  - The four files are listed in `delivery-scope.jsonl` with `category: vestigial-suppression`.
- **Residual risk**: A future commit could re-add the suppression. The audit-log records the original suppression line so a `git blame` reveals the regression.
- **Suggested lanes**: Verus (postcondition on `Suppression.is_delete_allow_only`); Flux-rs (refinement that deletion preserves the lint-clear state).

### H2 — Sibling-Test Visibility Drift (Category B/C)

- **Category**: Rust-core invariant + bounded state.
- **Threat model**: After `pub fn` → `fn` in `gate_07_stack.rs`, a sibling `#[cfg(test)]` module in `gate_tests.rs` does `use crate::gate_07_stack::compute_stack_depth;` and the compiler rejects the import because the path is no longer reachable. The Rust 2021 visibility rule (sibling modules of crate root can reach private items via direct paths) is subtle and may not be exercised by the lint-src compile set.
- **Affected suppressions**: 11 (Categories B and C: 6 gate files + 2 taint/type + 1 secret_leak).
- **Defense**:
  - `ConsumerRef.import_style.is_crate_internal()` is true for every consumer in B/C.
  - `cargo test -p vb_validate --lib` after each narrowing MUST exit 0.
  - In-file `#[cfg(test)] mod tests` submodules can use `use super::*` to bypass the visibility check (the submodule IS a descendant of the parent's module).
- **Residual risk**: A future Rust version changes the visibility rule. Unlikely; the rule has been stable since Rust 2018.
- **Suggested lanes**: Verus (refinement that `ConsumerRef.import_style.is_crate_internal()` is invariant); proptest (round-trip `cargo test -p vb_validate --lib` against the post-narrowing source); kani (string parsing of `use crate::gate_*::name` import statements).

### H3 — Cross-Test Type Drift (Category D)

- **Category**: Rust-core invariant + bounded state.
- **Threat model**: After `pub` → `pub(crate)` for `ValueType`, `Taint`, etc. in `type_sigs.rs`, a `#[cfg(test)]` submodule that uses these types via `use crate::type_sigs::ValueType;` fails to compile because `pub(crate)` items are not visible across test modules that are NOT descendants of the defining module.
- **Affected suppressions**: 4 (type_sigs.rs, schema_doc.rs, schema_id.rs, schema_fields.rs).
- **Defense**:
  - `pub(crate)` is the canonical visibility for crate-internal-but-cross-module items.
  - `cargo test -p vb_validate --lib` after each narrowing MUST exit 0; specifically tests in `fact_table/tests.rs`, `secret_leak/tests.rs`, `taint_prop.rs` internal tests, `type_check.rs` internal tests, `schema_support/schema_tests.rs`, `schema_fields/*.rs`.
- **Residual risk**: A future commit adds an OUT-OF-CRATE consumer (e.g., a workspace_test that uses `vb_validate::type_sigs::ValueType`); the narrowing breaks the consumer. Mitigation: grep before narrowing to confirm zero external consumer.
- **Suggested lanes**: Verus (postcondition that `Visibility::PubCrate.is_externally_visible() == false`); Flux-rs (refinement on cross-module visibility); proptest (compile-check after narrowing).

### H4 — Diag-Module Public-API Drift (Category E)

- **Category**: Rust-core invariant + API stability.
- **Threat model**: The 60+ `CODE_*` constants in `diag_codes.rs` are part of the externally-visible `vb_validate::diag::diag_codes` surface. An out-of-tree consumer (downstream crate, example binary) imports `vb_validate::diag::diag_codes::CODE_DUPLICATE_KEY`. Narrowing to `pub(crate)` breaks the consumer.
- **Affected suppressions**: 1 (diag_codes.rs).
- **Defense**:
  - Decision: keep `pub` and just `DeleteAllow` (option 1) OR narrow to `pub(crate)` after a fresh grep confirms zero external consumer (option 2).
  - If option 2 is taken, `grep -R 'vb_validate::diag::diag_codes::CODE_' .` MUST return no results outside `vb_validate/src/`.
- **Residual risk**: Out-of-tree consumers (not in the workspace tree) cannot be grepped. The audit logs the decision and the assumption.
- **Suggested lanes**: Verus (refinement that externally-visible items remain `pub`); proptest (string parsing of grep output).

### H5 — Diagnostic Re-Export Drift (Category F)

- **Category**: Rust-core invariant + API stability.
- **Threat model**: After removing the inner-attribute allow on `diagnostic.rs`, the lint fires on the two `pub use` re-exports if the items they re-export are not externally reachable. The re-exports point to `diag_render::diagnostic_from_error` and `diag_render::error_code`, which are also `pub`. The lint should NOT fire on these (they are reachable via `diagnostic::*`), but a future commit that breaks the re-export chain (e.g., renaming without updating both sides) triggers the lint.
- **Affected suppressions**: 1 (diagnostic.rs).
- **Defense**:
  - `DeleteAllow` is the only safe treatment for `diagnostic.rs`.
  - `moon run :lint-src` after the deletion MUST exit 0.
  - The two re-exports remain `pub use`, so `vb_validate::diagnostic::diagnostic_from_error` and `vb_validate::diagnostic::error_code` are still externally reachable.
- **Residual risk**: A future commit breaks the re-export chain. Mitigation: black-hat review includes a check that the re-export path resolves.
- **Suggested lanes**: Verus (postcondition on `vb_validate::diagnostic::*` reachability); Flux-rs (refinement that re-exports preserve reachability); proptest (cross-check via `cargo doc --no-deps -p vb_validate`).

### H6 — Orphan-Test Decision Drift (Category G, commands_diff/commands_incident)

- **Category**: Decision discipline + dormant artifact + API stability.
- **Threat model**: The default recommendation (retire orphan test) is executed without consulting the user/architect, and the retire breaks an out-of-tree consumer that relied on the orphan test path. Alternatively, the orphan test is registered but the registration adds 646 lines to the test surface (per `source-length-exceptions.txt:221`).
- **Affected suppressions**: 2 (commands_diff.rs, commands_incident.rs).
- **Defense**:
  - `DecisionRecommendation` is a mandatory field on `Treatment::DecisionRequired`.
  - The `workflow-model.md §5` decision path requires a human ack before `ApplyTreatment` fires on Category G rows.
  - The default recommendation is `RetireOrphan`, with the rationale documented in `domain-model.md §5.1`.
- **Residual risk**: An out-of-tree consumer exists for `IncidentReport` (or `DiffResult`) that the workspace cannot grep. Mitigation: black-hat review checks `verification/verus/production_inner/` for `IncidentReport` imports (WEAK binding) — if present, narrowing must preserve the production_inner mirror.
- **Suggested lanes**: Verus (postcondition that the production_inner mirror does not import `vb_cli::commands_incident::IncidentReport` directly); Flux-rs (refinement on the decision record); no formal lane for the orphan-test artifact itself.

### H7 — Production-Bound Spec Drift (Category G, commands_incident)

- **Category**: Rust-core invariant + refinement + production-binding.
- **Threat model**: The Verus proof `verification/verus/extern_vb_ahfl_bounds_production.rs` references `production::SpecIncidentReportProduction` via `assume_specification`. The production_inner mirror `verification/verus/production_inner/vb_ahfl_bounds_production_inner.rs` may import `vb_cli::commands_incident::IncidentReport` directly (WEAK binding). Narrowing `IncidentReport` to `pub(crate)` or private breaks the mirror.
- **Affected suppressions**: 1 (commands_incident.rs).
- **Defense**:
  - Pre-condition: `grep IncidentReport verification/verus/production_inner/` MUST return no results.
  - If the mirror does NOT import the type, narrowing is safe.
  - The WEAK binding is a `production_inner/*_inner.rs` file; the binding is enforced by `scripts/check-verus-production-binding.sh` with a drift gate.
- **Residual risk**: A future commit adds a direct import to the production_inner mirror. Mitigation: gate the audit on `bash scripts/check-production-inner-drift.sh` exit 0.
- **Suggested lanes**: Verus (refinement that the production_inner mirror is independent of the visibility of `vb_cli::commands_incident::IncidentReport`); Flux-rs (refinement on the mirror drift gate).

### H8 — Lifecycle Reachable Drift (Category G, lifecycle.rs)

- **Category**: Rust-core invariant + API stability.
- **Threat model**: After removing the inner-attribute allow on `lifecycle.rs:471`, the lint fires on `create_run_header` if it is not externally reachable. It IS externally reachable (via `vb_cli::lifecycle::test_helpers::create_run_header`, consumed by `derived_status_replay_timeline_tests.rs:29` and `lifecycle_integration.rs`). The lint should NOT fire.
- **Affected suppressions**: 1 (lifecycle.rs).
- **Defense**:
  - `DeleteAllow` is the only safe treatment for `lifecycle.rs`.
  - `pub mod test_helpers;` (line 463) makes the inner module reachable; `pub fn create_run_header` inside it is therefore externally reachable.
- **Residual risk**: A future commit breaks the consumer path. Mitigation: black-hat review includes a check that `derived_status_replay_timeline_tests.rs:29` and `lifecycle_integration.rs` still resolve.
- **Suggested lanes**: Verus (postcondition that `vb_cli::lifecycle::test_helpers::create_run_header` is reachable from the workspace_tests integration test surface); proptest (compile-check via `cargo test -p workspace_tests --tests`).

### H9 — Lint-Regression Drift (bead-wide)

- **Category**: Rust-core invariant + temporal.
- **Threat model**: A future commit re-introduces a `#[allow(unreachable_pub)]` on a file that has zero `pub` items OR re-introduces a legitimate lint violation. The `lint-src` gate should catch the latter.
- **Affected suppressions**: all 25 (after the audit, zero remain).
- **Defense**:
  - `lint-src` is the canonical regression gate; re-running it after every commit is the discipline.
  - The audit-log records every suppression removed, so `git blame` reveals reintroduction.
- **Residual risk**: A clippy lint rename in a future rustc nightly masks a violation. Mitigation: pin the toolchain per `docs/rust-governance.md`.
- **Suggested lanes**: Verus (postcondition on `VisibilityInvariant::is_satisfied()`); Flux-rs (refinement on the audit-log append-only invariant); kani (string parsing of `delivery-scope.jsonl` row count).

### H10 — Tooling Drift (bead-wide)

- **Category**: Performance/release + tooling.
- **Threat model**: `moon`, `cargo`, or `cargo clippy` is missing or upgraded past the version pinned in `docs/rust-governance.md`. The audit's evidence commands exit non-zero with `command-not-found` or a different error code.
- **Affected suppressions**: all 25 (the gate runner is shared).
- **Defense**:
  - `LintAuditError::ToolingMissing` and `ToolVersionMismatch` are explicit error variants with dedicated diagnostic codes.
  - `global-readiness-report.md` (State 1) enumerates the missing tools.
- **Residual risk**: A tool upgrade changes lint behavior. Mitigation: re-run the gate after every tool upgrade and compare the exit code to the recorded baseline.
- **Suggested lanes**: no formal lane; the substrate relies on `bash scripts/check-beads-server-mode.sh`-style script gates.

## 2. Hazard × Category Matrix

| Hazard \ Category | A | B | C | D | E | F | G |
|---|---|---|---|---|---|---|---|
| H1 Vestigial Suppression | ● |   |   |   |   |   |   |
| H2 Sibling-Test Visibility |   | ● | ● |   |   |   |   |
| H3 Cross-Test Type |   |   |   | ● |   |   |   |
| H4 Diag-Module Public-API |   |   |   |   | ● |   |   |
| H5 Diagnostic Re-Export |   |   |   |   |   | ● |   |
| H6 Orphan-Test Decision |   |   |   |   |   |   | ● |
| H7 Production-Bound Spec |   |   |   |   |   |   | ● |
| H8 Lifecycle Reachable |   |   |   |   |   |   | ● |
| H9 Lint-Regression Drift | ● | ● | ● | ● | ● | ● | ● |
| H10 Tooling Drift | ● | ● | ● | ● | ● | ● | ● |

A "●" indicates the hazard applies to that category. Each ● cell maps to at least one proof seed in `proof-seeds.jsonl`.

## 3. Pre-Existing Hazard Register (from codebase-map.md §3 + §6)

The following hazards were identified during State 2 (explore) and are inherited here:

- The exact visibility-rule semantics for sibling `#[cfg(test)]` modules in Rust 2021+ require local verification via `cargo check -p vb_validate --lib --all-features`.
- The orphan test file's source-length-exception reference (vb-jpq7.47) implies a longer-term plan to retire or split it.
- The Verus production-inner mirror file for `commands_incident::IncidentReport` exists and may import the type directly.

These are pre-existing risks; the audit MUST surface them in the audit-log as `DecisionRecorded` events with explicit `UNCONFIRMED` notes.

## 4. Hazard-Specific Residual Risks

After the canonical defenses are applied, the following residual risks remain and are deferred:

| Residual risk | Owner (proposed) |
|---|---|
| Out-of-tree consumer of `vb_validate::diag::diag_codes::CODE_*` (Category E) | external-coordination bead (out of scope) |
| Out-of-tree consumer of `vb_cli::commands_incident::IncidentReport` (Category G) | external-coordination bead (out of scope) |
| Tooling drift between rustc nightly releases | master plan rust-governance bead |
| Future `#[allow(unreachable_pub)]` reintroductions | black-hat-reviewer discipline + lint-clear audit log |

## 5. Open Hazard Questions

1. Whether H3 (Cross-Test Type Drift) should add a proof obligation requiring `pub(crate)` items to remain `pub(crate)` across crate-internal test boundaries. Recommendation: yes; defer to State 4.
2. Whether H6 (Orphan-Test Decision Drift) should require a separate human approval artifact in `.beads/vb-7akm0/decision-ack.md` before `ApplyTreatment` fires on Category G rows. Recommendation: yes; implement before State 5.
3. Whether the audit-log should be a typed JSONL with schema validation or a free-form Markdown log. Recommendation: typed JSONL with `audit-event/v1` schema; defer the schema design.
4. Whether H7 (Production-Bound Spec Drift) should require a `bash scripts/check-verus-production-binding.sh` exit-0 evidence artifact per Category G row. Recommendation: yes; required for State 12 closure.

End of hazard-analysis.md.