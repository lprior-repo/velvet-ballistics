# Domain Model — vb-7akm0 Lint-Suppression Audit

| Field | Value |
|---|---|
| bead_id | vb-7akm0 |
| state | 3 (rust-contract) |
| skill | rust-contract |
| source_checkout | /home/lewis/src/velvet-ballistics |
| isolated_workspace | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0 |
| captured_at | 2026-07-01T16:04Z |
| upstream_artifacts | STATE.md, codebase-map.md, delivery-scope.jsonl (States 1 + 2) |
| downstream_owners | proof-planner (State 4), holzman-rust (State 11) |

## 0. Read-only Confirmation

- No production code, test, proof, config, or dependency file was modified while authoring this artifact.
- The coord checkout `/home/lewis/src/velvet-ballistics` remains untouched.
- All file/line references verified against the tree at `HEAD = git rev-parse origin/main` (workspace-level JJ parent commit `rsvywymk 1d6c017f`).

## 1. Ubiquitous Language

The bead closes a single P1 lint bug: 25 files in the workspace carry `#[allow(unreachable_pub)]` suppressions that hide a violation of Rust's visibility invariant. The contract domain vocabulary is the audit of those suppressions, not their introduction. The following terms MUST be used consistently across all State 3 artifacts.

| Term | Definition | Source anchor |
|---|---|---|
| Visibility Invariant | A `pub` item is only allowed at a particular syntactic location when there exists an external path (a downstream crate, a `pub use` re-export, or a sibling `pub` ancestor module) that can reach it. | `Cargo.toml:57`, `crates/vb_validate/src/lib.rs:3` |
| Lint Suppression | An inner-attribute `#[allow(unreachable_pub)]` (or equivalent lint attribute) that locally overrides a `deny` at the workspace or crate root. | codebase-map.md §21 |
| Vestigial Suppression | A suppression whose file has no `pub` items at file scope. The lint cannot fire; the suppression is a no-op. | delivery-scope.jsonl rows 1–4 |
| Internal Duplicate | A `pub fn` re-declared inside a `#[cfg(test)]` module that already has a byte-identical canonical export in a non-test sibling module (e.g., `vb_validate::gates::*`). The duplicate is used only by sibling test modules and is reachable through the crate-internal path. | codebase-map.md §37 (Category B) |
| External Duplicate | A `pub fn` that mirrors a canonical export but is consumed only by in-file `#[cfg(test)] mod tests` submodules via `use super::*` or by sibling `#[cfg(test)]` modules via `crate::path::name` direct paths. | codebase-map.md §58 (Category C) |
| Cross-Test Type | A type declared `pub` because multiple `#[cfg(test)]` submodules consume it via `crate::type_name`. `pub(crate)` is sufficient (and silences the lint). | codebase-map.md §70 (Category D) |
| Orphan Test Path | A pub-mod boundary (`pub mod commands_diff;`) whose inner `pub` items are consumed only by an integration test file NOT registered in any `Cargo.toml` `[[test]]` entry. The lint fires because the test is not in the lint-src compile set. | codebase-map.md §97 (Category G) |
| Dormant Artifact | A file on disk with no `[[test]]` registration and no other in-tree consumer. Identified by absence in `Cargo.toml` test globs. | `crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs`, `.config/source-length-exceptions.txt:221` |
| Lint Anchor | The moon task that surfaces the lint as an error: `lint-src` runs `cargo clippy --workspace --lib --bins --examples --all-features`. | `.moon/tasks/all.yml:46-62` |
| Lint Surface | The set of files compiled by `lint-src`. Integration tests in `crates/*/tests/` are NOT in the surface; `#[cfg(test)] mod foo;` modules inside the lib target ARE (because `cfg(test)` activates during `cargo clippy --lib`). | codebase-map.md §14-19 |
| Production-Bound Spec | A Verus proof that uses `#[path = ".../crates/...rs"]` (STRONG binding) or `production_inner/*_inner.rs` mirror (WEAK binding) to read production Rust. Touching visibility of an externally-reachable type that is referenced through such a binding is forbidden. | `verification/verus/extern_vb_ahfl_bounds_production.rs` |
| Closure Workflow | The downstream flow `Reported → Triaged → Reproduced → FixedInSource → GatePassed → Closed` per the closure substrate (NOT this bead). This bead bypasses the closure workflow because the change is purely a lint-compliance refactor with no semantic effect. | go-skill reference set |

## 2. Entities, Value Objects, Aggregates

### 2.1 `Suppression` (aggregate root)

A `Suppression` is the unit of lint-cleanup work. Its identity is the `(file_path, suppression_line, kind)` triple, where `kind` is the categorical treatment.

| Field | Type | Cardinality | Notes |
|---|---|---|---|
| `file_path` | `RepoPath` (string, see §2.2) | exactly one | Relative to source checkout root |
| `suppression_line` | `u32` | exactly one | 1-indexed line of the inner attribute |
| `kind` | `SuppressionKind` (enum, see §2.3) | exactly one | One of the six categorical treatments |
| `category` | `Category` (enum, see §2.4) | exactly one | Codebase-map category letter A..G |
| `pub_items_at_file_scope` | `Vec<PubItemRef>` | zero-or-more | What `pub` items live at the file scope |
| `consumers` | `Vec<ConsumerRef>` | zero-or-more | Downstream in-tree consumers |
| `externally_reachable_items` | `Vec<PubItemRef>` | zero-or-more | Items genuinely reachable across the crate boundary |
| `recommended_treatment` | `Treatment` (enum, see §2.5) | exactly one | One of: `delete-allow`, `pub-to-pub-crate`, `pub-fn-to-fn`, `decision-required` |
| `risk_tags` | `Vec<RiskTag>` | zero-or-more | Tags from delivery-scope.jsonl row `risk_tags` |
| `behavior_affecting` | `bool` | exactly one | `false` for all rows in this bead (visibility-only change) |
| `production_bound` | `bool` | exactly one | `true` iff a Verus/Flux spec binding references the file |

Per-bead mapping: 25 rows, each row in `delivery-scope.jsonl` is one `Suppression`.

### 2.2 `RepoPath` (newtype)

```rust
/// Repository-relative path. Reject absolute paths and `..` segments.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RepoPath(SmolStr);

impl RepoPath {
    /// Smart constructor. Rejects:
    /// - empty string
    /// - leading `/`
    /// - any `..` segment
    /// - segments containing `\0`
    pub fn new(s: impl AsRef<str>) -> Result<Self, RepoPathError>;
}
```

### 2.3 `SuppressionKind` (enum)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SuppressionKind {
    /// File has zero `pub` items; the allow is a no-op.
    VestigialSuppression,
    /// File has duplicate canonical export; pub fns are reachable only via crate-internal paths.
    InternalDuplicate,
    /// File mirrors a canonical export; reachable from sibling test submodules via direct paths.
    ExternalDuplicate,
    /// File declares types used across multiple #[cfg(test)] submodules; pub(crate) suffices.
    CrossTestType,
    /// File is the diag module whose constants/functions may be externally reachable via re-export.
    DiagModule,
    /// File is the diagnostic.rs re-export of diag_render items.
    DiagnosticReexport,
    /// File is in vb_cli with a dormant-test decision requirement.
    OrphanTestDecision,
}
```

### 2.4 `Category` (enum)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Category {
    VestigialSuppression = 0,    // A: delete allow
    GateInternalDuplicate = 1,   // B: pub fn -> fn
    TaintTypeDuplicate = 2,      // C: pub fn -> fn
    SchemaSupportNarrow = 3,     // D: pub -> pub(crate)
    DiagModuleMixed = 4,         // E: delete allow OR narrow to pub(crate)
    DiagnosticReexport = 5,      // F: delete allow
    OrphanTestDecision = 6,      // G: decision required (delete allow vs register)
}
```

### 2.5 `Treatment` (enum)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Treatment {
    /// Delete the inner-attribute line; the lint does not fire on the remaining items.
    DeleteAllow,
    /// Change `pub` to `pub(crate)`; the lint does not fire on `pub(crate)` items.
    PubToPubCrate,
    /// Change `pub fn` to bare `fn`; reachability via crate-internal direct paths is preserved.
    PubFnToFn,
    /// Defer to a human/architect decision: retire the orphan test or register it.
    DecisionRequired { recommendation: DecisionRecommendation },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DecisionRecommendation {
    RetireOrphanTest,
    RegisterOrphanTest,
    NarrowModuleVisibility,
}
```

### 2.6 `PubItemRef` (struct)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PubItemRef {
    pub item_name: SmolStr,        // e.g. "validate_gate_07_expression_stack_depth"
    pub item_kind: ItemKind,       // enum: Fn | Struct | Enum | Const | Use | Mod
    pub line: u32,                 // 1-indexed
    pub current_visibility: Visibility, // pub | pub(crate) | pub(super) | private
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ItemKind {
    Fn,
    Struct,
    Enum,
    Const,
    Use,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Visibility {
    /// Bare `pub`.
    Pub,
    /// `pub(crate)`.
    PubCrate,
    /// `pub(super)`.
    PubSuper,
    /// No `pub` keyword; default private visibility.
    Private,
}
```

### 2.7 `ConsumerRef` (struct)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsumerRef {
    pub consumer_path: RepoPath,           // e.g. "crates/vb_validate/src/gate_tests.rs"
    pub consumer_line: u32,                // 1-indexed; the line of the import/use
    pub import_style: ImportStyle,         // enum: CratePath | SuperPath | Glob
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ImportStyle {
    /// `use crate::path::name;` from a sibling module.
    CratePath,
    /// `use super::*;` from a child `#[cfg(test)] mod tests`.
    SuperPath,
    /// `use super::name;` from a child `#[cfg(test)] mod tests`.
    SuperExplicit,
}
```

### 2.8 `RiskTag` (enum)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RiskTag {
    LintSuppressionAudit,
    TestVisibility,
    PublicApi,
    DormantArtifact,
    DecisionRequired,
    ProductionBindingVerification,
    TestSuiteReverify,
}
```

### 2.9 Per-Bead Mapping

| file_path | category | treatment | pub_items | consumers | externally_reachable |
|---|---|---|---|---|---|
| `xtask/src/main.rs` | A | DeleteAllow | 0 | — | none |
| `crates/vb_validate/src/diag/diag_tests.rs` | A | DeleteAllow | 0 | — | none |
| `crates/vb_validate/src/schema_support/schema_tests.rs` | A | DeleteAllow | 0 | — | none |
| `crates/vb_validate/src/fact_table.rs` | A | DeleteAllow | 0 (only `pub(crate)` items) | — | none |
| `crates/vb_validate/src/gate_07_stack.rs` | B | PubFnToFn | 2 | 2 (gate_tests, gate_07_stack/tests.rs) | none |
| `crates/vb_validate/src/gate_08_accessor.rs` | B | PubFnToFn | 1 | 2 | none |
| `crates/vb_validate/src/gate_09_slots.rs` | B | PubFnToFn | 1 | 2 | none |
| `crates/vb_validate/src/gate_10_node.rs` | B | PubFnToFn | 1 | 1 | none |
| `crates/vb_validate/src/gate_11_loop.rs` | B | PubFnToFn | 1 | 2 | none |
| `crates/vb_validate/src/gate_12_14_15.rs` | B | PubFnToFn | 3 | sibling tests | none |
| `crates/vb_validate/src/gate_13_cycles.rs` | B | PubFnToFn | 1 | 1 | none |
| `crates/vb_validate/src/taint_prop.rs` | B (functionally C) | PubFnToFn | 1 | in-file tests | none |
| `crates/vb_validate/src/type_check.rs` | B (functionally C) | PubFnToFn | 1 | in-file tests | none |
| `crates/vb_validate/src/secret_leak.rs` | C | PubFnToFn | 1 | secret_leak/tests.rs:6 | none |
| `crates/vb_validate/src/type_sigs.rs` | D | PubToPubCrate | 9 types | 7 consumers | none |
| `crates/vb_validate/src/schema_support/schema_doc.rs` | D | PubToPubCrate | 12 items | 7 consumers | none |
| `crates/vb_validate/src/schema_support/schema_id.rs` | D | PubToPubCrate | 3 fns | 2 consumers | none |
| `crates/vb_validate/src/schema_support/schema_fields.rs` | D | PubToPubCrate | 6 fns | 4 consumers | none |
| `crates/vb_validate/src/diag/diag_codes.rs` | E | DeleteAllow or PubToPubCrate | 60+ consts | 3 (all internal) | TBD (decision) |
| `crates/vb_validate/src/diag/diag_convert.rs` | E | DeleteAllow | 1 (`pub(super)`) | 3 (descendants of `diag`) | none |
| `crates/vb_validate/src/diag/diag_render.rs` | E | DeleteAllow | 2 fns | 4 (re-exported via diagnostic.rs) | 2 (diagnostic_from_error, error_code) |
| `crates/vb_validate/src/diagnostic.rs` | F | DeleteAllow | 2 re-exports | 6+ workspace_tests | 2 (same as above) |
| `crates/vb_cli/src/commands_diff.rs` | G | DecisionRequired | 7 items | 1 orphan | 7 (if orphan test is registered) |
| `crates/vb_cli/src/commands_incident.rs` | G | DecisionRequired | 2 items | 1 orphan | 2 (if orphan test is registered) |
| `crates/vb_cli/src/lifecycle.rs` | G | DeleteAllow | 1 fn | 2 (registered tests) | 1 (create_run_header) |

## 3. Commands, Events, Policies

### 3.1 Commands (issued by the implementation owner)

| Command | Producer | Pre-condition | Post-condition |
|---|---|---|---|
| `DeleteAllow(suppression)` | holzman-rust (State 11) | `treatment == DeleteAllow` | inner-attribute removed; lint-src exit 0 |
| `NarrowVisibility(suppression, new_visibility)` | holzman-rust (State 11) | `treatment ∈ {PubToPubCrate, PubFnToFn}` | visibility changed; consumers still resolve |
| `RegisterOrphanTest(test_path)` | holzman-rust (State 11) | `treatment == DecisionRequired` and recommendation is register | `[[test]]` entry added to `crates/workspace_tests/Cargo.toml`; allow removed |
| `RetireOrphanTest(test_path)` | holzman-rust (State 11) | `treatment == DecisionRequired` and recommendation is retire | file deleted; CLI items narrowed to private/pub(crate); allow removed |

### 3.2 Events (emitted by the lint-anchor gate)

| Event | Emitted on | Payload |
|---|---|---|
| `LintCleared { file_path, suppression_line, gate_run_id, at }` | `lint-src` exits 0 for a particular file after suppression removal | file:line + run id + UTC timestamp |
| `LintSurfaced { file_path, line, lint_label, at }` | `lint-src` exits non-zero (failure mode) | lint label + UTC timestamp |
| `TestCompiles { test_path, gate_run_id, exit_status, at }` | `cargo test -p <crate>` exits 0 against the post-change source | gate run + status |
| `TestBreaks { test_path, expected_path, exit_status, stderr_excerpt, at }` | a previously-passing test now fails because of a visibility narrowing | the consumer path that no longer resolves + stderr excerpt |

### 3.3 Policies (invariants the implementation owner MUST satisfy)

| Policy | Scope | Enforcement |
|---|---|---|
| Rust-Visibility Invariant | All `pub` items in the workspace | After all changes, every remaining `pub` item is either (a) reachable from a `pub use` chain that exits the crate, (b) reachable from a registered integration test, or (c) reachable from a `#[cfg(test)] mod` that is in the `lint-src` compile set. |
| No-New-Pub | All narrowed items | The narrowing MUST reduce visibility, not raise it. `private` is allowed; `pub` from `pub(crate)` is forbidden. |
| Canonical-Version Reachable | Categories B and C | After `pub fn` → `fn`, the canonical `vb_validate::gates::*` and `vb_validate::type_taint::*` exports MUST remain reachable; Kani and Verus proofs that consume the canonical versions MUST still compile. |
| Production-Bound Spec Preservation | Category G (commands_incident) | The Verus production-bound spec `verification/verus/extern_vb_ahfl_bounds_production.rs` MUST NOT be affected by visibility changes to `vb_cli::commands_incident::IncidentReport` because the spec uses the `production_inner/*_inner.rs` mirror (WEAK binding) — verify mirror is unchanged. |
| Orphan-Test Default Retire | Category G | The default recommendation for `commands_diff.rs` and `commands_incident.rs` is to retire the orphan test (`vb_test_cli_diff_incident_behavior.rs`), because it is already on `source-length-exceptions.txt:221` as `vb-jpq7.47 split-or-retire-before-release`. |
| Test-Suite Re-verify | All narrowed files | After narrowing, `cargo test --workspace` MUST exit 0; specifically `cargo test -p vb_validate --lib` and `cargo test -p vb_cli --lib` and `cargo test --workspace --tests`. |
| No-Behaviour-Affecting Change | All rows | Visibility changes MUST NOT alter program semantics; only the lint-compliance state changes. |
| Toolchain-Pin Compliance | All rows | Visibility changes MUST compile under the pinned nightly per `docs/rust-governance.md`; no use of unstable features beyond those already approved (e.g., `try_blocks`, `portable_simd`). |

## 4. Forbidden States (Made Unrepresentable)

The domain model refuses the following illegal states by construction.

| Illegal state | Why it's illegal | Defended by |
|---|---|---|
| `Suppression { treatment: PubFnToFn, externally_reachable_items: [item] }` | A function genuinely consumed by an integration test MUST stay `pub`; narrowing it to `fn` breaks the consumer. | Canonical-Version Reachable policy |
| `Suppression { file_path: "vb_validate/src/diagnostic.rs", treatment: PubFnToFn }` | diagnostic.rs contains re-exports that ARE externally reachable; the only legal treatment is `DeleteAllow`. | Diagnostic Re-export policy (encoded in §2.3 `DiagnosticReexport` variant mapping) |
| `Suppression { category: OrphanTestDecision, treatment: DeleteAllow, decision_recorded: false }` | Category G requires a decision before deletion; a default delete without a recorded decision is illegal. | Orphan-Test Default Retire policy |
| `Suppression { category: GateInternalDuplicate, externally_reachable_items: [duplicate_name] }` | The duplicate is by definition NOT externally reachable; its existence as `pub` is the lint violation. | Internal-Duplicate semantics |
| `Suppression { kind: DiagModule, treatment: PubFnToFn }` | diag module items are `const`s and `fn`s in a non-`mod`-level pub boundary; the treatment is `DeleteAllow` (when re-exported) or `PubToPubCrate` (when internal). PubFnToFn does not apply because consts are not fns. | Treatment/Category mismatch rule |
| `Suppression { behavior_affecting: true, lane: rust-local }` | This bead is behavior-preserving; all visibility changes must be `behavior_affecting: false`. Lanes are not behavior-affecting because the verification is a structural lint pass. | No-Behaviour-Affecting Change policy |
| `RepoPath { ... leading_slash: true }` | Absolute paths are not repo-relative. | RepoPath smart constructor |
| `Suppression { file_path: ".../vb_test_cli_diff_incident_behavior.rs" }` | The orphan test file is NOT a suppression site; only the CLI files inside vb_cli/src/ are. | file_path scope |

## 5. Open Domain Questions

These are surfaced for downstream owners; they are NOT blockers for State 3.

1. **Orphan-test retention**: should `crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs` be retired (default) or registered? — **OWNER: holzman-rust (State 11) + user/architect**. Recommendation: retire, because `source-length-exceptions.txt:221` already flags it as `vb-jpq7.47 split-or-retire-before-release`.
2. **diag_codes narrow-vs-keep**: should the 60+ `CODE_*` constants in `crates/vb_validate/src/diag/diag_codes.rs` be narrowed to `pub(crate)` (full audit before deletion) or kept `pub` (delete-allow only)? — **OWNER: holzman-rust (State 11)**. Recommendation: narrow to `pub(crate)` after a final fresh grep confirms zero external consumer.
3. **Visibility semantics for sibling `#[cfg(test)]` modules**: confirm via `cargo check -p vb_validate --lib --all-features` that changing `pub fn` → `fn` in `gate_07_stack.rs` does not break `use crate::gate_07_stack::compute_stack_depth` in `gate_tests.rs`. Rust 2021+ allows non-pub module items to be referenced via crate-internal direct paths when both modules are siblings of the crate root. — **OWNER: holzman-rust (State 11)**. Verification: re-run `cargo test -p vb_validate --lib` after each Category B narrowing.
4. **Verus production-binding audit**: confirm that the production_inner mirror for `vb_cli::commands_incident::IncidentReport` (in `verification/verus/production_inner/vb_ahfl_bounds_production_inner.rs`) does not import `vb_cli::commands_incident::IncidentReport` directly. If it does, narrowing `IncidentReport` to private/pub(crate) breaks the binding. — **OWNER: proof-writer (State 5) + formal-verifier (State 12)**. Verification: `grep IncidentReport verification/verus/production_inner/`.
5. **`lifecycle.rs:471` inner-attribute**: confirm that removing the inner attribute does not surface a new lint (e.g., clippy lint not already covered). `create_run_header` IS externally reachable; deletion should be safe. — **OWNER: holzman-rust (State 11)**. Verification: re-run `moon run :lint-src` after deletion.
6. **`diag_render.rs:4` allow deletion**: confirm that the two pub fns (`diagnostic_from_error`, `error_code`) remain reachable via `diagnostic.rs:8-9` re-export after the allow is removed. — **OWNER: holzman-rust (State 11)**. Verification: `grep -R 'vb_validate::diagnostic::' crates/` to confirm cross-crate consumers.
7. **Public API stability**: for `diag_codes.rs`, narrowing `pub const CODE_*: u16` to `pub(crate) const CODE_*: u16` could break out-of-tree consumers (downstream crates or examples). Confirm via `grep -R 'vb_validate::diag::diag_codes::CODE_' .` before narrowing. — **OWNER: holzman-rust (State 11)**.

## 6. Cross-Cutting Constraints

| Constraint | Source | Applies to |
|---|---|---|
| `unreachable_pub = "deny"` at workspace lints | `Cargo.toml:57` | All rows |
| `#![deny(unreachable_pub)]` at `vb_validate` crate root | `crates/vb_validate/src/lib.rs:3` | All `vb_validate` rows |
| `lint-src` task surface excludes integration tests | `.moon/tasks/all.yml:46-62` | All rows (explains why orphan-test pub items fire) |
| `lint-src` task surface INCLUDES `#[cfg(test)] mod` items | `cargo clippy --lib --all-features` semantics | Categories B/C/D (explains why sibling-test items must remain crate-internal) |
| No use of `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg` in any production code | AGENTS.md Engineering Rules | All rows |
| No new unstable Rust features outside existing allowlist | AGENTS.md Engineering Rules; `docs/rust-governance.md` | All rows (visibility changes use stable syntax only) |
| Verus production-binding discipline | `scripts/check-verus-production-binding.sh`, AGENTS.md | Category G commands_incident row only |

## 7. Glossary Cross-Reference

- `Visibility Invariant` — see §1. The single Rust-core invariant this bead defends.
- `Lint Anchor` — `lint-src` moon task. The exit-0 gate that this bead must clear.
- `Lint Surface` — see §1. The set of files compiled by `lint-src`.
- `Dormant Artifact` — `vb_test_cli_diff_incident_behavior.rs`. The orphan test file whose retention decides Category G.
- `Production-Bound Spec` — see §1. The Verus/Flux binding that must remain intact.

End of domain-model.md.