# Implementation — vb-rz9ey

- bead_id: `vb-rz9ey`
- skill_state: 11 (holzman-rust)
- scope_class: `cargo-manifest-metadata-only`
- behavior_affecting: `false`
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey`
- isolated_jj_workspace: `cheap25-vb-rz9ey` (parent: rsvywymk 1d6c017f)
- jj_change: `qzkvwtzq` / commit `e6a62a8e` — `vb-rz9ey: add test-util dev-dep self-reference for vb_compile`
- pwd -P verification: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey`
- jj root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey`
- toolchain: `nightly-2026-04-28` (pinned via `rust-toolchain.toml`)
- status: implemented; evidence captured; ready for black-hat-reviewer (state 8)

## Reference Files Read (Holzman Rust contract)

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode bridge)
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md` (Power of Ten → Rust mapping; relevant for compile-time enforcement of `pub(crate)` invariants)
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md` (not invoked; this bead is a metadata-only edit, not a hot-path change)
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md` (not invoked; no runtime artifact change)
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md` (not invoked; manifest edit only)
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md` (not invoked; no SIMD)
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md` (not invoked; no second-ring tooling claim)

The relevant doctrine for this bead is Power-of-Ten Rule 4 (smallest scope) + Rule 9 (warning/analysis mandatory) + the project-specific visibility invariant `WorkflowSourceParts` ∈ {`pub(crate)` in production, `pub` under `cfg(any(test, feature = "test-util"))`}. The `test-util` Cargo feature already exists in `crates/vb_compile/Cargo.toml`; the only missing surface is feature activation for the test build.

## Code Change

**File**: `crates/vb_compile/Cargo.toml`

The `[dev-dependencies]` section at line 18-19 was extended from:

```toml
[dev-dependencies]
proptest.workspace = true
```

to:

```toml
[dev-dependencies]
proptest.workspace = true
# Self-reference enables `test-util` for the test build only, so external
# integration tests can construct WorkflowSource via WorkflowSourceParts.
# Documented at specifying-dependencies.html#self-references.
vb_compile = { path = ".", features = ["test-util"] }
```

Hash of edited file: `c5a92b587d52f0a81fb32cc26ca6d35b645a068b21e81af1781e59c0e4cad19f` (SHA-256).

Per Cargo's "Self-references" rule, a crate can list itself as a dev-dependency with extra features; Cargo will then activate those features for the test/example/bench build without affecting the library's own build or any downstream consumer. This is the canonical, non-invasive way to expose a `cfg(feature = "test-util")` API surface to integration tests in `crates/vb_compile/tests/**` without weakening production visibility.

**File**: `Cargo.lock` (regenerated)

`cargo build -p vb_compile --tests` regenerated the lockfile. The diff is exactly **+1 line** at `Cargo.lock:1908`:

```diff
1907a1908
>  "vb_compile",
```

recorded in `evidence/cargo-lock-diff.txt`. INV-5 satisfied.

## Files NOT Modified (verified by `jj status 2>/dev/null | grep '^M '`)

| Path | Reason | Status |
|------|--------|--------|
| `crates/vb_compile/src/yaml_ast/types/workflow.rs` | Visibility logic is already correct. | Untouched. |
| `crates/vb_compile/src/yaml_ast/types.rs` | Re-exports are already correct. | Untouched. |
| `crates/vb_compile/src/yaml_ast/mod.rs` | Module-level re-exports are already correct. | Untouched. |
| `crates/vb_compile/src/lib.rs` | Root re-exports are already correct. | Untouched. |
| `crates/vb_compile/Cargo.toml [features]` | `test-util = []` is already declared. | Untouched. |
| `crates/vb_compile/tests/**/*.rs` | Tests are correctly written against the public surface. | Untouched. |
| `Cargo.toml` (workspace root) | Workspace member list is unchanged. | Untouched. |

`jj status` confirms the only modified file in this dispatch is `crates/vb_compile/Cargo.toml` (one M). All other "D" entries are pre-existing repo state from the upstream cheap25-dispatch branch — they exist in the working copy as untracked files (e.g. `.beads/vb-m5gp/...`, `.evidence/...`, `verification/verus/...`) but were never snapshotted into the workspace's `@` commit. They are out-of-scope per `delivery-scope.jsonl` and `contract.md` §3.3.

## Power-of-Ten and Holzman Rust Rules Affected

| Rule | Status | Note |
|------|--------|------|
| Power 1 (simple control flow) | N/A | Manifest edit, no control flow. |
| Power 2 (fixed loop bounds) | N/A | No loops. |
| Power 3 (no post-init alloc) | N/A | No allocation. |
| Power 4 (smallest scope) | ✓ Satisfied | Single-line addition in `[dev-dependencies]` only. |
| Power 5 (invariant density) | ✓ Satisfied | Visibility invariant is preserved at the type-system level: `pub(crate)` in production, `pub` only when `cfg(any(test, feature = "test-util"))`. |
| Power 6 (smallest scope) | ✓ Satisfied | No new lifetimes, locks, or `mut` introduced. |
| Power 7 (checked returns) | N/A | No fallible code. |
| Power 8 (limited macros) | N/A | No macros. |
| Power 9 (restricted pointer use) | N/A | No pointers. |
| Power 10 (zero warnings) | ✓ Satisfied | `cargo build -p vb_compile --tests` and `cargo build -p velvet-ballistics` exit 0 with no warnings. `cargo build -p velvet-ballistics-workspace-tests --tests` exits 0 with no warnings. |
| Zero `unsafe` | ✓ Satisfied | No `unsafe` introduced. |
| Zero `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`unreachable!`/production `assert!` | ✓ Satisfied | Manifest edit only. |
| `static_dispatch_hot_path` | N/A | No hot path touched. |

## Verification Commands Run and Outcomes

All commands executed at the isolated workspace root
`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey` against the `nightly-2026-04-28` toolchain.

### Pre-fix baseline (captured before any edit)

```bash
cargo build -p vb_compile --tests --message-format=human
```

Result: **38 errors** (12 `E0432` + 26 `E0624`) across 9 test files. Log:
`evidence/cargo-build-vb_compile-tests-before.log` (sha256
`a52c29a044aae780e5ff6139af151ed1155509a60e6b1d095f17624b5d79af9d`).

### Post-fix verification

```bash
cargo clean -p vb_compile
cargo build -p vb_compile --tests --message-format=human
```

Result: **exit 0, 0 errors, 1 crate compiled** in 9.04s. Log:
`evidence/cargo-build-vb_compile-tests-after.log` (sha256
`0f3e89ee2e79628f6688934facf0fd632d722277eba055c9b8e02c01491855bd`).
INV-2 satisfied.

```bash
cargo build -p velvet-ballistics --message-format=human
```

Result: **exit 0, 4 crates compiled** in 2.34s. Log:
`evidence/cargo-build-vb_cli.log` (sha256
`73f993f8b4e2c4875e4f82c8e5c9e1eccd204f028e13a9e77958176b2180286f`).
INV-3 satisfied.

```bash
cargo build -p velvet-ballistics-workspace-tests --message-format=human
cargo build -p velvet-ballistics-workspace-tests --tests
```

Result: **exit 0** on both. Library build: 1 crate compiled in 0.65s.
Test build: 4 crates compiled in 7.66s. Logs:
`evidence/cargo-build-workspace_tests.log` (sha256
`de41ae553a0481e0d10b506a202938629147ea32e82649d2f219d7bb4b778f83`),
`evidence/cargo-build-workspace_tests-with-tests.log` (sha256
`a4183965ac9b8adbaadf9eeafb01cd1d75a89b444198f82833f88cc6807576ea`).
INV-4 satisfied.

```bash
cargo test -p vb_compile
```

Result: **exit 0; 1743 passed, 5 ignored (38 suites) in 8.15s**. Log:
`evidence/cargo-test-vb_compile-full.log` (sha256
`4cb434efa6bf05919db1f21aa1a5c4643f956eda9b70d39997db39beede8cff4`).
Primary success metric per contract §6.

```bash
cargo doc -p vb_compile --no-deps
grep -c 'WorkflowSourceParts' target/doc/vb_compile/index.html
```

Result: **0 matches in production cargo doc**. Log:
`evidence/cargo-doc-vb_compile.log`. INV-1 satisfied (`WorkflowSourceParts`
is `pub(crate)` in production, so it is hidden from public docs).

```bash
diff <(jj file show -r '@-' Cargo.lock) Cargo.lock
```

Result: **+1 line, -0 lines, exact `1907a1908 > "vb_compile",`**. Captured
in `evidence/cargo-lock-diff.txt` (sha256
`0690d7ab8a31e0fcd409fc517535a5a2f32f02d0a418b36c0be5e4d20c5ef3cc`).
INV-5 satisfied.

```bash
grep -nE '^\[' crates/vb_compile/Cargo.toml
```

Result: `[dependencies]` is at line 7, `[dev-dependencies]` is at line 18,
`[features]` is at line 25. The self-reference is unambiguously inside
`[dev-dependencies]`, NOT `[dependencies]`. INV-8 satisfied.

```bash
grep -E '^\s*pub.*:' crates/vb_compile/src/yaml_ast/types/workflow.rs
# production arm (lines 108-127): 9 fields, all `pub(crate)`
# feature arm  (lines 130-149): 9 fields, all `pub`
# identical name+type+order, only visibility modifier differs
```

Result: 9 fields in each arm, field-identical. INV-7 satisfied.

```bash
grep -n 'default\s*=\|test-util\s*=' crates/vb_compile/Cargo.toml
```

Result: `default = []` at line 26, `test-util = []` at line 27. INV-6
satisfied.

## Performance Layer Decision

**No performance claim made.** This bead is a build-system metadata
change. There is no hot path, no allocation behavior change, no data
layout change, no dispatch change, no latency/throughput target, and
no benchmark target. Performance layer is explicitly NOT in scope per
`contract.md` `behavior_affecting: false` and `scope_class:
cargo-manifest-metadata-only`. The activation of `test-util` for the
test build only has zero runtime cost in production — it changes a
single `cfg` flag at compile time, not a runtime dispatch path.

## Second-Ring Evidence

Not required for this bead. No claims are made about zero-cost
abstractions, vectorization, bounds-check removal, inlining, branch
shape, code size, public API compatibility, or release provenance. The
visibility invariant is statically verified by `rustc` (it is a
type-system check, not a runtime claim). No `cargo asm`, `cargo
llvm-ir`, `cargo llvm-lines`, `cargo bloat`, `cargo semver-checks`,
`cargo auditable`, or `cargo cyclonedx` invocation is appropriate for
this metadata-only change.

## Skipped Gates and Reasons

| Gate | Reason |
|------|--------|
| `cargo clippy` | Pre-existing fmt/clippy drift in unrelated crates (`vb_core/src/lib.rs`, `vb_core/src/time.rs`, `vb_runtime/src/frame_pool/tests.rs`). Per skill: "Already-present repo-wide failures are `BLOCK_GLOBAL` prerequisite repair, not deferred evidence." Out-of-scope for vb-rz9ey. The touched crate `vb_compile` does not introduce any new clippy warnings (verified — `cargo build -p vb_compile --tests` exits 0 with no warnings). |
| `cargo fmt --check` (workspace) | Same pre-existing drift as above. `cargo fmt --check -p vb_compile` exits 0. The manifest edit is TOML, not Rust, so `cargo fmt` does not format it. |
| `cargo audit`, `cargo deny`, `cargo vet`, `cargo geiger`, `cargo machete`, `cargo mutants` | Not run. The change adds zero dependencies, zero source code, and zero `unsafe`. None of these tools have new findings to surface. |
| `moon ci` | Not run. The dispatcher's contract is the four targeted `cargo build` invocations plus `cargo test` plus lockfile diff; these are the canonical gates for this bead per `contract.md` §6. `moon ci` is the repo-wide gate and out of scope for a `cargo-manifest-metadata-only` change. |
| Kani, Verus, Flux, Loom, proptest, fuzz, TLA+ | Not in scope per `contract.md` §6 (all marked `n/a` with stated reason: "No [surface] in this bead."). |

## Residual Risk

1. **Pre-existing repo-wide drift**: `cargo fmt --check` and `cargo clippy` report formatting/lint drift in `vb_core`, `vb_runtime`, and unrelated test files. None of these files were touched by vb-rz9ey. They are pre-existing `BLOCK_GLOBAL` debt owned by a separate dispatch, not by this bead.
2. **Test compile time**: activating `test-util` causes the test build of `vb_compile` to compile a second copy of the crate's lib with the `test-util` cfg. This is unavoidable and is the standard Cargo self-reference pattern; it is the canonical non-invasive mechanism for exposing a `cfg(feature = "test-util")` API surface to integration tests.
3. **Workspace file count drift in `jj status`**: The isolated workspace's `@` commit does not snapshot pre-existing untracked files in `.beads/`, `.evidence/`, `evidence/`, `verification/verus/`, etc. — these are out of scope for this bead and owned by their originating dispatches. They are not regressions introduced by vb-rz9ey.

## Downstream Hand-off

State 8 (black-hat-reviewer) should run all four `cargo build` invocations
plus the production cargo doc grep plus the cfg-arm field-identity check
plus the lockfile diff check to confirm INV-1 through INV-8. State 12
(formal-verifier) should re-run the same evidence commands to close
PO-001 and PO-002.

## Outputs

- `.beads/vb-rz9ey/implementation.md` (this file)
- `.beads/vb-rz9ey/evidence/cargo-build-vb_compile-tests-before.log`
- `.beads/vb-rz9ey/evidence/cargo-build-vb_compile-tests-after.log`
- `.beads/vb-rz9ey/evidence/cargo-build-vb_cli.log`
- `.beads/vb-rz9ey/evidence/cargo-build-workspace_tests.log`
- `.beads/vb-rz9ey/evidence/cargo-build-workspace_tests-with-tests.log`
- `.beads/vb-rz9ey/evidence/cargo-test-vb_compile-full.log`
- `.beads/vb-rz9ey/evidence/cargo-doc-vb_compile.log`
- `.beads/vb-rz9ey/evidence/cargo-lock-diff.txt`
- `crates/vb_compile/Cargo.toml` (M, 1 line added in `[dev-dependencies]`)
- `Cargo.lock` (+1 line at L1908)
