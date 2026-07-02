# Boundary Map — vb-7akm0 Lint-Suppression Audit

| Field | Value |
|---|
| bead_id | vb-7akm0 |
| state | 3 (rust-contract) |
| skill | rust-contract |
| captured_at | 2026-07-01T16:04Z |
| upstream_artifacts | domain-model.md, type-contracts.md, workflow-model.md, error-taxonomy.md |

## 0. Scope

This file draws the boundaries between the audit pure core, the imperative shell (moon ci gate runner), the file-shell (the 25 source files being modified), and the parser boundary (the JSONL row → `Suppression` parse). Every boundary is a place where the audit's `forbid(unsafe_code)` discipline and zero-unwrap rule must be visibly enforced.

The boundary map is the answer to the question: "Which code is responsible for what change?"

## 1. Layer Topology

```text
                 ┌──────────────────────────────────────┐
                 │  External: user/architect decision   │
                 │  (for Category G only)               │
                 └────────────────┬─────────────────────┘
                                  │ Decide(recommendation)
                                  ▼
┌────────────────────────────────────────────────────────┐
│  PARSER BOUNDARY  (smart constructors, single entry)   │
│  - parse_suppression_from_delivery_scope               │
└────────────────┬───────────────────────────────────────┘
                 │ Vec<Suppression>
                 ▼
┌────────────────────────────────────────────────────────┐
│  PURE AUDIT CORE  (functional, no I/O, no time)        │
│  - Treatment validation                                │
│  - Visibility invariant checking                       │
│  - Forbidden-combination detection                     │
└────────────────┬───────────────────────────────────────┘
                 │ Vec<Suppression> + Treatment
                 ▼
┌────────────────────────────────────────────────────────┐
│  FILE SHELL  (the 25 source files)                     │
│  - Each file is one Suppression's target               │
│  - Inner-attribute removal / visibility narrowing       │
└────────────────┬───────────────────────────────────────┘
                 │ Modified source
                 ▼
┌────────────────────────────────────────────────────────┐
│  IMPERATIVE SHELL  (moon ci gate runner)               │
│  - moon run :lint-src                                  │
│  - cargo test --workspace                              │
│  - cargo check -p vb_validate --lib --all-features     │
└────────────────┬───────────────────────────────────────┘
                 │ Exit status, raw log
                 ▼
┌────────────────────────────────────────────────────────┐
│  STORAGE / OBSERVABILITY SHELL                        │
│  - .beads/vb-7akm0/audit-log.jsonl (append-only)       │
│  - .evidence/lint-src/<run_id>/exit-code.txt           │
│  - .evidence/cargo-test/<run_id>/exit-code.txt         │
└────────────────────────────────────────────────────────┘
```

The audit core (top two layers above the file shell) is `forbid(unsafe_code)`, contains no `std::time`, no filesystem, no network, no `std::process`, no `rand`. All crossings of the file-shell boundary go through the implementation owner (holzman-rust).

## 2. Boundary Inventory (Per-Category)

For each category in the bead, the boundary map names the responsible layer for the change site.

### 2.1 Category A — Vestigial Suppressions (4 files)

- **Files**: `xtask/src/main.rs`, `crates/vb_validate/src/diag/diag_tests.rs`, `crates/vb_validate/src/schema_support/schema_tests.rs`, `crates/vb_validate/src/fact_table.rs`
- **Responsible layers**:
  - Parser boundary: each `Suppression` is parsed from a `delivery-scope.jsonl` row.
  - Pure core: `Suppression.is_delete_allow_only()` returns true for all four.
  - File shell: the inner-attribute line is removed. No visibility change.
  - Imperative shell: `moon run :lint-src` exits 0 after the removal.
- **Fix location**: the 4 files listed above. No production symbols modified.
- **Boundary discipline**: each removal MUST NOT delete any other attribute or import; the change is exactly one line per file.

### 2.2 Category B — Gate Internal Duplicates (6 files + 2 functional C)

- **Files**: `crates/vb_validate/src/gate_07_stack.rs`, `gate_08_accessor.rs`, `gate_09_slots.rs`, `gate_10_node.rs`, `gate_11_loop.rs`, `gate_12_14_15.rs`, `gate_13_cycles.rs`, `taint_prop.rs`, `type_check.rs`
- **Responsible layers**:
  - Pure core: `Suppression.treatment == PubFnToFn`; `ConsumerRef.import_style.is_crate_internal() == true` for every consumer.
  - File shell: `pub fn` → `fn` for the items in `pub_items_at_file_scope`.
  - Imperative shell: `cargo test -p vb_validate --lib` MUST exit 0 after each narrowing.
- **Fix location**: the 8 files listed above. Canonical exports in `gates.rs` and `type_taint.rs` remain `pub`.
- **Boundary discipline**: each narrowing MUST preserve the function signature (parameters, return type, generics); only the visibility changes. Submodule `tests` continues to use `use super::*` and `super::fn_name()`.

### 2.3 Category C — Taint/Type/Secret-Leak Duplicates (3 files)

- **Files**: `taint_prop.rs`, `type_check.rs`, `secret_leak.rs`
- **Responsible layers**: identical to Category B for `taint_prop.rs` and `type_check.rs`. `secret_leak.rs` is a single helper function (`validate_resource_limits`) consumed by `secret_leak/tests.rs:6` via `use crate::secret_leak::validate_resource_limits;`.
- **Boundary discipline**: after `pub fn` → `fn`, the in-file tests in `taint_prop.rs:94-201` and `type_check.rs:140-200` continue to use `validate_taint(...)` and `validate_types(...)` via name resolution (same module = same scope).

### 2.4 Category D — Schema Support Narrow to `pub(crate)` (4 files)

- **Files**: `crates/vb_validate/src/type_sigs.rs`, `schema_support/schema_doc.rs`, `schema_support/schema_id.rs`, `schema_support/schema_fields.rs`
- **Responsible layers**:
  - Pure core: `Suppression.treatment == PubToPubCrate`; `externally_reachable_items.is_empty()`.
  - File shell: `pub` → `pub(crate)` for the items in `pub_items_at_file_scope`.
  - Imperative shell: `cargo test -p vb_validate --lib` exits 0.
- **Fix location**: the 4 files listed above. The `#[cfg(test)] pub mod schema_*` declarations in `schema_support/mod.rs` remain unchanged (the modules themselves stay pub to the test surface; the inner items get narrowed).
- **Boundary discipline**: `pub(crate)` is sufficient because all consumers in `pub_items_at_file_scope[*].consumers` are sibling `#[cfg(test)]` modules.

### 2.5 Category E — Diag Module (3 files)

- **Files**: `crates/vb_validate/src/diag/diag_codes.rs`, `diag/diag_convert.rs`, `diag/diag_render.rs`
- **Responsible layers**:
  - Parser boundary: each row's `kind` is `DiagModule`.
  - Pure core: `diag_convert.rs` is `DeleteAllow` (the only pub item is `pub(super)` and is not subject to the lint). `diag_render.rs` is `DeleteAllow` (items are re-exported via `diagnostic.rs:8-9`). `diag_codes.rs` is `DeleteAllow` OR `PubToPubCrate` — pending the decision in `domain-model.md §5.2`.
  - File shell: inner-attribute removal (and possibly `pub` → `pub(crate)` for `diag_codes.rs` if the narrowing decision is taken).
  - Imperative shell: `moon run :lint-src` exits 0.
- **Boundary discipline**: if `diag_codes.rs` is narrowed, run `grep -R 'vb_validate::diag::diag_codes::CODE_' .` first to confirm zero external consumer.

### 2.6 Category F — Diagnostic Re-Export (1 file)

- **Files**: `crates/vb_validate/src/diagnostic.rs`
- **Responsible layers**: identical to Category A (DeleteAllow). The two `pub use` items (`diagnostic_from_error`, `error_code`) are externally reachable via `vb_validate::diagnostic::*`.

### 2.7 Category G — Orphan Test Decision (3 files)

- **Files**: `crates/vb_cli/src/commands_diff.rs`, `commands_incident.rs`, `lifecycle.rs`
- **Responsible layers**:
  - External (user/architect): decides the recommendation (retire vs register).
  - Pure core: `Suppression.treatment == DecisionRequired`; recommendation is set.
  - File shell: per the recommendation.
    - `lifecycle.rs`: DeleteAllow only (create_run_header IS externally reachable).
    - `commands_diff.rs` / `commands_incident.rs`: retire orphan test, then narrow items to `pub(crate)` (per default recommendation).
  - Imperative shell: `moon run :lint-src` exits 0; `cargo test -p vb_cli --lib` exits 0.
- **Fix location**: the 3 files listed above; optionally `crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs` (retire).
- **Boundary discipline**: the Verus production-binding audit (`grep IncidentReport verification/verus/production_inner/`) MUST be performed before narrowing `commands_incident::IncidentReport`. The mirror file MUST not import it directly.

## 3. Boundary Crossings (Per Layer Pair)

| From → To | Crossing mechanism | Failure mode |
|---|---|---|
| External → Parser | `delivery-scope.jsonl` row | Smart-constructor rejection; `LintAuditError::RepoPathParse`, `Suppression` |
| Parser → Pure Core | `Vec<Suppression>` (already validated) | None |
| Pure Core → File Shell | `Suppression.treatment` + `pub_items_at_file_scope` | None (the file shell is a controlled environment) |
| File Shell → Imperative Shell | source change lands via `git commit` | `git apply` may fail if hunks don't match; surfaced as `LintAuditError::AuditTrailInvariant` |
| Imperative Shell → External | `moon run :lint-src` exit code | `LintAuditError::LintSrcNonZeroExit`, `NewUnreachablePubLabel` |
| Imperative Shell → Storage | raw-log persistence to `.evidence/...` | IO errors handled by the gate runner, not the audit |
| Storage → Observability | `DiagnosticCode` emission | None |

## 4. Boundary Discipline Rules (Holzman Rust Lifted)

The audit domain inherits the following rules from AGENTS.md and `docs/rust-governance.md`, applied to each boundary:

1. **No `unsafe`** in any of the 25 modified files. `forbid(unsafe_code)` already declared at every crate root.
2. **No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`** introduced by the refactor. Existing uses (in pre-existing code) are out of scope.
3. **No unchecked indexing, slicing, casts, or arithmetic** introduced. Visibility changes use only stable syntax.
4. **No YAML, JSON, or HTTP parsers** in the runtime core. The parser boundary is the ONLY place `serde_json` is imported (for `parse_suppression_from_delivery_scope`).
5. **Generated Rust mode is mandatory for maxperf execution**. Visibility changes do not touch perf-gated modules.
6. **Speed claims require real benchmark evidence**. This bead makes no speed claims.

## 5. Storage / Time / FFI / Unsafe / Parser Boundaries (Recap)

| Boundary | Audit decision |
|---|---|
| Storage | All audit state lives under `.beads/vb-7akm0/` and `.evidence/`; the audit core holds no `std::fs` imports. |
| Time | `Iso8601` is parsed once at the parser boundary and threaded through; the audit core never calls `std::time::SystemTime::now()`. |
| FFI | None in scope for this bead. |
| Unsafe | None. `forbid(unsafe_code)` at every crate root already enforces this; the bead does not add new `unsafe` uses. |
| Parser | `parse_suppression_from_delivery_scope` is the ONLY parse function. Every other parse site is rejected by code review. |

## 6. Open Boundary Questions

1. Whether the audit-log appender should live in a separate crate (`vb_audit_log`) or inside the bead's `.beads/` directory. Recommendation: stay in `.beads/vb-7akm0/audit-log.jsonl` for this bead; defer the crate decision.
2. Whether the per-file change should be a single `git commit` per file or per category. Recommendation: per category (6 commits total); makes black-hat review tractable.
3. Whether the orphan test (`vb_test_cli_diff_incident_behavior.rs`) should be retired before or after the CLI item narrowing. Recommendation: before — retire the test first, then narrow the items, then re-run `moon run :lint-src`.

End of boundary-map.md.