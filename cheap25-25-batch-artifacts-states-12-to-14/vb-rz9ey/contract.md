# Contract — vb-rz9ey

- bead_id: `vb-rz9ey`
- title: Fix `vb_compile` test compilation — `WorkflowSourceParts` private (P0)
- skill_state: 3 (rust-contract)
- version: `contract/v1`
- scope_class: `cargo-manifest-metadata-only`
- behavior_affecting: `false`
- companions: `domain-model.md`, `type-contracts.md`, `workflow-model.md`, `error-taxonomy.md`, `boundary-map.md`, `hazard-analysis.md`

## 1. Purpose

Enable `cargo test -p vb_compile` (and `cargo build -p vb_compile --tests`) to compile cleanly by activating the existing `test-util` Cargo feature for the test build only, while preserving `WorkflowSourceParts` as `pub(crate)` in every production build.

## 2. Domain Claim

> The visibility invariant "**`WorkflowSourceParts` is `pub(crate)` in production and `pub` only under `cfg(any(test, feature = "test-util"))`**" is encoded in `crates/vb_compile/src/yaml_ast/types/workflow.rs:107-149` and `crates/vb_compile/src/lib.rs:241`. This bead claims that activating `test-util` for the test build — via a single Cargo self-referencing dev-dependency — is the *only* required change to satisfy the invariant across both production and test builds simultaneously, without any source-code edit.

## 3. Required Changes

### 3.1 Source-of-Edit (Cargo Manifest)

**File**: `crates/vb_compile/Cargo.toml`
**Section**: `[dev-dependencies]` (line 18-19)

Insert a self-referencing entry after the existing `proptest.workspace = true` line:

```toml
[dev-dependencies]
proptest.workspace = true
# Self-reference enables `test-util` for the test build only, so external
# integration tests can construct WorkflowSource via WorkflowSourceParts.
# Documented at specifying-dependencies.html#self-references.
vb_compile = { path = ".", features = ["test-util"] }
```

**Hard constraints**:

- The line MUST live in `[dev-dependencies]`, NOT `[dependencies]`.
- `path = "."` exactly (no quotes, no sub-path).
- `features = ["test-util"]` exactly (no other features).
- No line changes outside `[dev-dependencies]`. The `[features]` block, `[dependencies]` block, and `[[test]]` table MUST be untouched.
- Optional but recommended: a leading comment explaining intent (verbatim wording above).

### 3.2 Source-of-Regeneration (Lockfile)

**File**: `Cargo.lock`

After the manifest edit, regenerate via `cargo metadata` or `cargo build -p vb_compile --tests`. The expected diff is **exactly +1 line** referencing `vb_compile` in `vb_compile`'s own test-binary closure. No hand-edits.

### 3.3 Off-Limits

The following files MUST NOT be modified by this bead:

| Path | Reason |
|------|--------|
| `crates/vb_compile/src/yaml_ast/types/workflow.rs` | Visibility logic is already correct. |
| `crates/vb_compile/src/yaml_ast/types.rs` | Re-exports are already correct. |
| `crates/vb_compile/src/yaml_ast/mod.rs` | Module-level re-exports are already correct. |
| `crates/vb_compile/src/lib.rs` | Root re-exports are already correct. |
| `crates/vb_compile/Cargo.toml [features]` | Feature declaration is already correct. |
| `crates/vb_compile/tests/**/*.rs` | Tests are correctly written against the public surface. |
| `Cargo.toml` (workspace root) | Workspace member list is unchanged. |

## 4. Invariants

| ID | Invariant | Lane |
|----|-----------|------|
| INV-1 | `WorkflowSourceParts` visibility in production: `pub(crate)`. `cargo doc -p vb_compile --no-deps 2>&1 | grep -c WorkflowSourceParts` returns 0. | `black-hat-reviewer` |
| INV-2 | `WorkflowSourceParts` visibility in test build: `pub`. `cargo build -p vb_compile --tests` exits 0 with 0 `E0432` and 0 `E0624` errors. | `holzman-rust`, CI |
| INV-3 | `cargo build -p vb_cli --message-format=human` exits 0. | `black-hat-reviewer` |
| INV-4 | `cargo build -p workspace_tests --message-format=human` exits 0. | `black-hat-reviewer` |
| INV-5 | `git diff --stat Cargo.lock` shows exactly one line added, no other changes. | `black-hat-reviewer`, `landing-skill` |
| INV-6 | `default = []` in `crates/vb_compile/Cargo.toml` is preserved. | `black-hat-reviewer` |
| INV-7 | The two cfg arms of `WorkflowSourceParts` (`workflow.rs:107-127` and `:129-149`) remain field-identical. | `black-hat-reviewer` |
| INV-8 | The self-reference entry sits in `[dev-dependencies]`, NOT `[dependencies]`. | `black-hat-reviewer` |

## 5. Required Test Plan Coverage

This bead does not write tests. Existing tests are the validation:

| Test file | Behavior verified (after fix) |
|-----------|-------------------------------|
| `crates/vb_compile/tests/common/mod.rs` | Test helpers compile; 10 `WorkflowSource::new(WorkflowSourceParts{...})` constructions. |
| `crates/vb_compile/tests/digest_structural_fields.rs` | B15-B19 step/digest field sensitivity runs. |
| `crates/vb_compile/tests/proptest_digest_foreach.rs` | proptest foreach parity runs. |
| `crates/vb_compile/tests/digest_set_finish_regression.rs` | Set+finish regression runs. |
| `crates/vb_compile/tests/digest_ask_explicit_arm.rs` | Ask explicit arm runs. |
| `crates/vb_compile/tests/proptest_digest_determinism.rs` | proptest determinism runs. |
| `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs` | Ask timeout sensitivity runs. |
| `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs` | Ask prompt sensitivity runs. |
| `crates/vb_compile/tests/proptest_digest_ask_ordering.rs` | Ask ordering runs. |
| (and 5 tests that depend on `common/mod.rs`) | All transitively unblocked. |

## 6. Verification Lanes

| Lane | Tool | Required? | Reason |
|------|------|-----------|--------|
| Behavior test | `cargo test -p vb_compile` | YES | Primary success metric. |
| Negative compile (production) | `cargo build -p vb_cli` | YES | Prevents API leak. |
| Negative compile (dev) | `cargo build -p workspace_tests` | YES | Prevents API leak. |
| Lockfile review | `git diff Cargo.lock` | YES | Drift guard. |
| Source lint | `moon run :source-lint` | YES | Holzman governance. |
| Verus | n/a | NO | No Verus spec references `WorkflowSourceParts` (verified in `codebase-map.md` Q2). |
| Kani | n/a | NO | This bead is `cargo test` only. The pre-existing Kani latent defect (Q1) is out of scope. |
| Flux | n/a | NO | No Flux refinement references `WorkflowSourceParts`. |
| Loom | n/a | NO | No concurrency surface in this bead. |
| proptest | n/a | NO | proptest harnesses themselves are the test surface; no proptest-via-verifier obligation. |
| cargo-fuzz / fuzzcheck | n/a | NO | No parser/codec surface in this bead. |
| TLA+ | n/a | NO | No temporal workflow. |

## 7. Downstream Owners

| Lane | Owner | Action |
|------|-------|--------|
| Implementation | `holzman-rust` | Edit `Cargo.toml` and regenerate `Cargo.lock`. |
| Black-hat review | `black-hat-reviewer` | Run all four compile/build checks; verify INV-1 through INV-8. |
| Landing | `landing-skill` | Standard jj land. |

## 8. Proof Obligations Emitted

See `proof-seeds.jsonl` for `proof-seed/v1` rows. No Verus/Kani/Flux/Loom/proptest obligations are emitted because this bead is build-only and the visibility invariant is statically enforced by rustc.

## 9. Behavior Waiver Status

`behavior_affecting: false` — no waiver needed. The visibility contract is statically verified by `rustc`/`cargo`.

## 10. Open Items Deferred

| ID | Item | Why deferred |
|----|------|--------------|
| OI-1 | Kani harnesses at `src/kani_digest_ask_*.rs` import `WorkflowSource` from `crate::ast` (not re-exported there) | Pre-existing latent defect; out of scope for vb-rz9ey; needs separate bead. |
| OI-2 | `WorkflowSourceParts` field-shape divergence risk between cfg arms | Pre-existing structural risk; needs invariant-enforcement (e.g. macro) in a separate bead. |
| OI-3 | Downstream crates (`vb_cli`, `workspace_tests`) could import `WorkflowSourceParts` directly in future | Latent; not currently exercised. |

## 11. Contract Versioning

This is `contract/v1`. Any scope expansion (e.g. addressing OI-1) requires a new contract version and a new bead.
