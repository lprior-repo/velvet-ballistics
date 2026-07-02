---
bead_id: vb-rz9ey
title: Proof/Test/Source Alignment — Cargo self-reference fix (P0)
state: 12 (formal-verifier)
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
alignment_count: 2 (PTSA-001, PTSA-002)
mapping_status_summary: verified (2/2)
behavior_affecting: false
scope_class: cargo-manifest-metadata-only
authored_by: formal-verifier (direct child of femdation; no sub-agents)
authored_at: 2026-07-01T21:55:00Z
---

# Proof/Test/Source Alignment — vb-rz9ey

This artifact binds each `proof-obligation/v1` row to concrete production
Rust source paths, the existing behavior tests that exercise them, and the
executed cargo evidence commands with their raw logs.

## Alignment Summary

| PTSA ID | PO ID | Requirement | Source refs | Test refs | Mapping status |
|---------|-------|-------------|-------------|-----------|----------------|
| PTSA-001 | PO-001 | REQ-RZ9EY-TESTBUILD-COMPILE | `crates/vb_compile/Cargo.toml:18-19`, `crates/vb_compile/src/yaml_ast/types/workflow.rs:107,131`, `crates/vb_compile/Cargo.toml:25-27` | 9 integration test files under `crates/vb_compile/tests/` | `verified` |
| PTSA-002 | PO-002 | REQ-RZ9EY-DOWNSTREAM-PRESERVE | `crates/vb_compile/Cargo.toml:7-17`, `crates/vb_cli/Cargo.toml:7-8`, `crates/workspace_tests/Cargo.toml:39`, `crates/vb_compile/src/yaml_ast/types/workflow.rs:105-127` | `vb_cli` library+binary, `workspace_tests` crate | `verified` |

## PTSA-001 — PO-001 (REQ-RZ9EY-TESTBUILD-COMPILE)

**Source binding** (production paths that the obligation targets):

- `crates/vb_compile/Cargo.toml:18-19` — `[dev-dependencies]` section header
  and `proptest.workspace = true` line, which is the manifest context where
  the self-reference is inserted (post-fix lines 20-23).
- `crates/vb_compile/Cargo.toml:25-27` — `[features]` section declaring
  `test-util = []`. The dev-dep self-reference activates this feature.
- `crates/vb_compile/src/yaml_ast/types/workflow.rs:107` — `cfg(not(any(test,
  feature="test-util")))` arm of `WorkflowSourceParts` (`pub(crate)`).
- `crates/vb_compile/src/yaml_ast/types/workflow.rs:131` — `cfg(any(test,
  feature="test-util"))` arm of `WorkflowSourceParts` (`pub`). This is the
  arm selected when the dev-dep self-reference activates `test-util`.

**Test binding** (9 integration test files that exercise the obligation):

| Test file | Line cite for `WorkflowSourceParts` use |
|-----------|------------------------------------------|
| `crates/vb_compile/tests/common/mod.rs` | L12 (use), L20, L61, L88, L114, L140, L181, L196, L211, L226 (constructor calls) |
| `crates/vb_compile/tests/digest_structural_fields.rs` | (file imports `WorkflowSourceParts` from `vb_compile`) |
| `crates/vb_compile/tests/proptest_digest_foreach.rs` | proptest harness |
| `crates/vb_compile/tests/digest_set_finish_regression.rs` | regression test |
| `crates/vb_compile/tests/digest_ask_explicit_arm.rs` | digest Ask-path test |
| `crates/vb_compile/tests/proptest_digest_determinism.rs` | proptest digest determinism |
| `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs` | proptest Ask timeout |
| `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs` | proptest Ask prompt |
| `crates/vb_compile/tests/proptest_digest_ask_ordering.rs` | proptest Ask ordering |

**Evidence command** (executed in this workdir):

```
cargo build -p vb_compile --tests --message-format=human
```

**Result**: exit 0; zero lines matching `E0432` (unresolved import) in
stderr; zero lines matching `E0624` (private associated function) in
stderr. Raw log:
`.beads/vb-rz9ey/dispatch/state-12-formal-verifier/command-logs/cargo_build_vb_compile_tests.log`
(sha256 `6de3d7aa7d0a650ffc08fa55d738e78719ff7f7a08ac1eb702709c03e7706690`).

**Sub-evidence** (test execution):

```
cargo test -p vb_compile --no-fail-fast --message-format=human
```

**Result**: exit 0; "1743 passed, 5 ignored (38 suites, 8.11s)". Raw log:
`.beads/vb-rz9ey/dispatch/state-12-formal-verifier/command-logs/cargo_test_vb_compile.log`
(sha256 `ada3c3801f4bcf73a60b1c0a17ac26274e90ffe891ed11d496461bdc5a7f0a47`).

## PTSA-002 — PO-002 (REQ-RZ9EY-DOWNSTREAM-PRESERVE)

**Source binding** (production paths that the obligation targets):

- `crates/vb_compile/Cargo.toml:7-17` — `[dependencies]` section. The check
  is that this section does **not** contain a `vb_compile = ... features =
  ["test-util"]` entry (verified by `awk` returning 0 for `^vb_compile` under
  `[dependencies]`). The self-reference under `[dev-dependencies]` is what
  isolates the test-util activation.
- `crates/vb_cli/Cargo.toml:7-8` — downstream CLI consumer; line 8 is
  `vb_compile = { path = "../vb_compile" }` with no feature activation.
- `crates/workspace_tests/Cargo.toml:39` — cross-crate integration test
  consumer; `vb_compile = { path = "../vb_compile" }` with no feature
  activation.
- `crates/vb_compile/src/yaml_ast/types/workflow.rs:105-127` — the
  `pub(crate)` arm of `WorkflowSourceParts` selected by
  `cfg(not(any(test, feature="test-util")))` in the default-feature
  production build. This is what `cargo doc --no-deps` builds and what
  public-API consumers see.

**Test binding** (downstream consumers that exercise the obligation):

- `crates/vb_cli` (package `velvet-ballistics`) — library + binary crate.
  Built by `cargo build -p velvet-ballistics`. Compiles cleanly because
  `vb_compile` is requested without `test-util`.
- `crates/workspace_tests` (package `velvet-ballistics-workspace-tests`) —
  cross-crate integration tests crate. Built by
  `cargo build -p velvet-ballistics-workspace-tests` (and `--tests`). Same
  reasoning: `vb_compile` is requested without `test-util`.

**Evidence command** (executed in this workdir):

```
cargo build -p velvet-ballistics
cargo build -p velvet-ballistics-workspace-tests
cargo build -p velvet-ballistics-workspace-tests --tests
cargo doc -p vb_compile --no-deps
```

**Result**:
- `cargo build -p velvet-ballistics`: exit 0. Raw log
  `.beads/vb-rz9ey/dispatch/state-12-formal-verifier/command-logs/cargo_build_velvet_ballistics.log`
  (sha256 `c08c17eb3ac49089cf1e634eba4316bdb2b7c9b21c3c538fb63d6dc2c3a4f504`).
- `cargo build -p velvet-ballistics-workspace-tests`: exit 0. Raw log
  `.beads/vb-rz9ey/dispatch/state-12-formal-verifier/command-logs/cargo_build_workspace_tests.log`
  (sha256 `bb101a017ee14c88f3f9b74899818ab6e66b1b80bc251733b49238b92d30a6db`).
- `cargo build -p velvet-ballistics-workspace-tests --tests`: exit 0. Raw log
  `.beads/vb-rz9ey/dispatch/state-12-formal-verifier/command-logs/cargo_build_workspace_tests_tests.log`
  (sha256 `efbad186f221cb06fe536f89657b21e41ffa5e71d8b7ed7dcd294c4068626aad`).
- `cargo doc -p vb_compile --no-deps`: exit 0. `grep -c WorkflowSourceParts`
  on stdout/stderr returns 0; recursive grep of `target/doc/vb_compile/**`
  returns 0 matches. Raw log
  `.beads/vb-rz9ey/dispatch/state-12-formal-verifier/command-logs/cargo_doc_vb_compile_no_deps.log`
  (sha256 `7e6ec4cebcb4460e107899b84c70ae52fc3895037b13d789691611dd68054442`).

## Mapping Status

- All 2 obligations are `mapping_status: verified` (not `planned`).
- All source paths, test paths, harness paths, and evidence commands resolve
  to real artifacts in this workdir (no file-only or prose refs).
- Both `behavior_affecting: false` and `scope_class: cargo-manifest-metadata-only`
  are inherited from the State-4 contract and the State-7 bridge
  (`proof-to-rust-map.md §1`).
- Zero `rust-refinement-obligation/v1` rows exist (per
  `rust-refinement-obligations.jsonl`); zero refinement harnesses are
  required.
- No behavior-affecting waivers (per `formal-waivers.jsonl`, which is
  empty).

## Cross-Reference

- `proof-obligations.planned.jsonl` — source of the 2 `proof-obligation/v1`
  rows that this alignment binds.
- `verification-ledger.jsonl` — final PASS/FAIL disposition per obligation
  (both PASS, see `verification-ledger.jsonl` for evidence).
- `proof-to-rust-map.md` — State-7 bridge that pre-declared
  `NO_RUST_REFINEMENT` disposition.
- `regression-diff.md` — pre-fix vs post-fix diff of `Cargo.toml` and
  `Cargo.lock` confirming only the documented manifest/lockfile change.
