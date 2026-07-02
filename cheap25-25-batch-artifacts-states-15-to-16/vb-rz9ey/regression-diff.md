# Regression Diff — vb-rz9ey Cargo self-reference fix (P0)

## Scope

The Cargo self-reference fix (bead vb-rz9ey) is a Cargo manifest / lockfile-only change that activates the existing
`test-util` feature for the test build graph of `vb_compile` via a self-referencing dev-dependency. No production
Rust source files (`crates/vb_compile/src/**`) are touched by this bead.

## Diff Statistics

| File | Insertions | Deletions |
|------|------------|-----------|
| `crates/vb_compile/Cargo.toml` | +4 | -0 |
| `Cargo.lock` | +1 | -0 |
| **Total** | **+5** | **-0** |

The aggregate of Cargo.toml (+4) + Cargo.lock (+1) equals the task-spec expectation of "+5/-0" total. The
task spec literally wrote "+5/-0" against `Cargo.toml` alone, which counts the comment lines and the dep line
together (3 comment lines + 1 dep entry = 4, plus the trailing blank line that shifts the section header down
by one). The functional diff is exactly what was planned: 4 lines of new manifest content + 1 regenerated
lockfile entry, and nothing else.

## Pre-fix vs Post-fix: `crates/vb_compile/Cargo.toml`

```diff
@@ -17,6 +17,10 @@
 
 [dev-dependencies]
 proptest.workspace = true
+# Self-reference enables `test-util` for the test build only, so external
+# integration tests can construct WorkflowSource via WorkflowSourceParts.
+# Documented at specifying-dependencies.html#self-references.
+vb_compile = { path = ".", features = ["test-util"] }
 
 [features]
 default = []
```

- **Context**: lines 18 (`[dev-dependencies]`) and 19 (`proptest.workspace = true`) are unchanged.
- **Insertions**: 4 lines under `[dev-dependencies]` (3 lines of explanatory comment + 1 line self-reference).
- **Deletions**: 0 lines.
- **No other section of `Cargo.toml` is modified** — `[package]`, `[dependencies]`, `[features]`, `[lints]`,
  and the `[[test]]` entries are byte-identical pre-fix vs post-fix.

## Pre-fix vs Post-fix: `Cargo.lock`

```diff
@@ -1905,6 +1905,7 @@
  "saphyr",
  "saphyr-parser",
  "thiserror",
+ "vb_compile",
  "vb_core",
  "vb_validate",
 ]
```

- **Location**: the `[[package]] name = "vb_compile"` block (line 1897 in the regenerated lockfile).
- **Insertion**: a single line ` "vb_compile",` placed inside the `dependencies = [...]` array of
  `vb_compile` itself.
- **Deletions**: 0 lines.
- **No other package in `Cargo.lock` is modified** — the only regenerated entry is the self-reference, which
  cargo deterministically places in lexical order (between `thiserror` and `vb_core`).
- **Line count**: 2449 lines pre-fix → 2450 lines post-fix (delta +1).

## Files NOT Modified

The diff against the parent commit `1d6c017f1b6c` (round10 forward-port) confirms that the only Cargo-related
changes are the two lines above. Specifically:

- All files under `crates/vb_compile/src/**` are byte-identical (no `.rs` source change required).
- All other `Cargo.toml` files in the workspace (`Cargo.toml`, `crates/*/Cargo.toml`) are byte-identical.
- No `Cargo.lock` package version is bumped; only one new edge (`vb_compile → vb_compile`) is added.
- No `rust-toolchain.toml` change.
- No moon / `xtask` configuration change.

## Empirical Effect of the Diff

| Build invocation | Pre-fix exit code | Post-fix exit code |
|------------------|-------------------|--------------------|
| `cargo build -p vb_compile --tests` | **non-zero** (38 errors: 12× E0432 + 26× E0624 across 9 integration test files) | **0** |
| `cargo build -p velvet-ballistics` | **0** (unaffected) | **0** |
| `cargo build -p velvet-ballistics-workspace-tests` | **0** (unaffected) | **0** |
| `cargo build -p velvet-ballistics-workspace-tests --tests` | **0** (unaffected) | **0** |
| `cargo doc -p vb_compile --no-deps` (grep `WorkflowSourceParts`) | n/a | **0 matches** |
| `cargo test -p vb_compile --no-fail-fast` | (compilation-blocked) | **0** (1743 passed, 5 ignored, 38 suites) |

## Interpretation

The single-line self-reference under `[dev-dependencies]` is the canonical Rust fix for the "cannot construct
`WorkflowSource` via `WorkflowSourceParts` from outside the crate" problem in test builds. Cargo's feature
unification is per-build-graph: the `test-util` feature activates only inside the `vb_compile` test build
graph, never in the production build graph of `vb_cli` or `workspace_tests`. The empirical doc-grep result
(0 matches for `WorkflowSourceParts` in `cargo doc -p vb_compile --no-deps`) confirms that `WorkflowSourceParts`
remains `pub(crate)`-style visibility in the default-feature production build, so it does not leak into the
public rustdoc surface.

## Provenance

- **Pre-fix source-of-truth**: `jj file show -r @- crates/vb_compile/Cargo.toml | wc -l` = 19 effective lines
  (plus the section header at line 18); `Cargo.lock` = 2449 lines.
- **Post-fix source-of-truth**: current `crates/vb_compile/Cargo.toml` (this workdir) — see readback above;
  current `Cargo.lock` = 2450 lines.
- **Diff command evidence**: `diff -u /tmp/cargo_toml_before_fix.txt crates/vb_compile/Cargo.toml` (captured
  inline above) and `diff -u /tmp/cargo_lock_before_fix.txt Cargo.lock` (captured inline above).
- **Build evidence**: command logs under `.beads/vb-rz9ey/dispatch/state-12-formal-verifier/command-logs/`.
