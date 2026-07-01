# Trusted Base Plan — vb-tsjnz

STATUS: PLANNED (proof-planner State 4). No closure or PASS is claimed.

## Trusted Surfaces

The patch is metadata-only. The trusted base is small: every trusted
item below is either compiler-enforced, a stable Cargo behavior, or a
build-tool property. No behavior-affecting obligation is waived; no
runtime function, no Rust expression, no library internals are in
scope for this bead.

### Cargo semantics (compiler + toolchain enforced)

| ID | Surface | Location | Trusted Kind | Reason | Scope |
|----|---------|----------|--------------|--------|-------|
| TB-001 | `version.workspace = true` resolution | `crates/vb_queue_semantics/Cargo.toml:3` | cargo-spec | Cargo 1.74+ resolves `version.workspace = true` to `[workspace.package].version` at metadata resolution time | Version equality check in PO-004 |
| TB-002 | `[lints]\nworkspace = true` resolution | `crates/vb_queue_semantics/Cargo.toml` terminal block | cargo-spec | Cargo 1.74+ pulls `[workspace.lints.rust]` and `[workspace.lints.clippy]` en bloc when `[lints].workspace = true` is set | Lint-policy gate in PO-001/PO-002 |
| TB-003 | `[workspace.package].version = "0.1.0"` | `Cargo.toml:19` | cargo-spec | The workspace root is the canonical source; resolved value is `"0.1.0"` (matches the prior literal `"0.1.0"`) | Version-equality anchor for PO-004 |
| TB-004 | `[workspace.lints.rust]` table | `Cargo.toml:54-59` | cargo-spec | Defines `unsafe_code = "forbid"`, `unused_must_use = "deny"`, `unreachable_pub = "deny"`, `rust_2018_idioms = { level = "deny", priority = -1 }`, `unexpected_cfgs = { level = "warn", check-cfg = [...] }` | Lint-policy gate in PO-001 |
| TB-005 | `[workspace.lints.clippy]` table | `Cargo.toml:61-82` | cargo-spec | Defines `correctness / suspicious / perf / complexity = { level = "deny", priority = -1 }`, `unwrap_used / expect_used / panic / panic_in_result_fn / todo / unimplemented / dbg_macro = "forbid"`, `indexing_slicing / string_slice / get_unwrap / arithmetic_side_effects / as_conversions / let_underscore_must_use / await_holding_lock = "deny"`, `large_stack_arrays / large_types_passed_by_value / result_large_err = "warn"` | Lint-policy gate in PO-002 |

### Tooling pipeline (build-tool surface)

| ID | Surface | Location | Trusted Kind | Reason | Scope |
|----|---------|----------|--------------|--------|-------|
| TB-006 | `cargo check -p vb_queue_semantics --all-targets` exit code | `cargo` (nightly pinned by `rust-toolchain.toml`) | build-tool | Standard cargo behavior; exit 0 ⟺ compile success under workspace lints | PO-001 evidence |
| TB-007 | `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` exit code | `cargo` + `cargo-clippy` (nightly pinned) | build-tool | Standard cargo behavior; `-D warnings` promotes warn to error | PO-002 evidence |
| TB-008 | `cargo test -p workspace_tests --test ...` exit code | `cargo` (nightly pinned) | build-tool | Standard cargo test runner; exit 0 ⟺ all listed tests pass | PO-003 evidence |
| TB-009 | `cargo metadata --no-deps --format-version 1` JSON shape | `cargo` (nightly pinned) | build-tool | Stable JSON output; `packages[].name` and `packages[].version` are authoritative | PO-004 evidence |
| TB-010 | `jj diff --stat` output | `jj` (workspace-scoped to isolated worktree) | build-tool | Standard jj output; one-line-per-file summary; stable path + line-count format | PO-004 evidence |
| TB-011 | `jq '.packages[] \| select(.name=="vb_queue_semantics") \| .version'` | `jq` (system PATH) | build-tool | Stable jq filter over stable JSON | PO-004 evidence |

### Held-Invariants (out-of-scope for the patch)

| ID | Surface | Location | Trusted Kind | Reason | Scope |
|----|---------|----------|--------------|--------|-------|
| TB-012 | `crates/vb_queue_semantics/src/lib.rs` (423 lines) | out-of-scope; owned by `vb-2lu1` | out-of-scope | Patch MUST NOT touch this file. Diff-audit in PO-004 enforces this; the `vb-2lu1` source-length exception at `.config/source-length-exceptions.txt:323` remains valid | Out-of-scope preservation |
| TB-013 | `.config/source-length-exceptions.txt:323` | exception file | out-of-scope | Pre-existing 427-line exception for `lib.rs` under bead `vb-2lu1`. Diff-audit in PO-004 enforces non-modification | Out-of-scope preservation |

## Trusted Base Debt

None. All trusted surfaces are either:

- Cargo spec semantics (compiler/toolchain enforced)
- Build-tool exit codes (`cargo`, `cargo-clippy`, `jj`)
- Held invariants that are explicitly out-of-scope for this bead

No Rust function, no proof model, no harness, and no behavior is
trusted-but-unverified.

## Model Reductions

| Reduction | Justification |
|-----------|---------------|
| `cargo check -p vb_queue_semantics --all-targets` does not exercise benches, examples, or builds that do not match the filter | The patch is metadata-only; the only Rust target is `lib.rs`, which is exercised by the default `cargo check` filter. `-all-targets` is required by the contract but cannot discover additional Rust targets in this stub crate. |
| `cargo metadata --no-deps` skips the dependency graph | The dependency graph is unchanged (no new dep, no version bump); `--no-deps` is the contract-mandated form. |
| `jj diff --stat` does not enumerate removed files | `vb-tsjnz` adds no new files; the only permitted modification is to an existing file. |
| The four planned obligations do not model concurrency, UB, panic, type-state, refinement | The patch is metadata-only; the only Rust surface in scope is `lib.rs`, and that surface is held invariant by the out-of-scope rule for `vb-tsjnz`. Any finding by the build (PO-001 / PO-002) becomes a follow-up bead, not a waiver. |

## Compensating Evidence

| Trusted Base Item | Compensating Evidence | Status |
|-------------------|----------------------|--------|
| TB-012 `lib.rs` untouched | PO-004 diff-audit verifies the file is unmentioned in `jj diff`. | Compensating evidence exists in the planned obligation |
| TB-013 exception preserved | PO-004 diff-audit verifies `.config/source-length-exceptions.txt` is unmodified via `git diff` over the patch's path scope. | Compensating evidence exists in the planned obligation |

## Repair Triggers

If the proof-writer introduces any of the following, the proof plan
becomes stale and the new surface requires new obligations:

- Adding `unsafe` code → Miri becomes required.
- Adding a Verus `requires`/`ensures` to a Rust function in the patch →
  Verus becomes required and a `production_binding` block must be
  present on each new Verus obligation.
- Adding Flux annotations or extern specs → Flux-rs becomes required
  on those annotated functions.
- Adding a concurrency primitive (lock, atomic, channel) → Loom becomes
  required for the affected module.
- Adding a property test or fuzz harness → the corresponding lane
  becomes required.

## Behavior-affecting waiver candidates

None. `waiver-candidates.jsonl` contains zero rows. See
`proof-strategy.md` §Forbidden repairs for the cross-cutting rules
that keep this list empty:

- MUST NOT lower workspace lint priority.
- MUST NOT remove any workspace lint.
- MUST NOT add `#[allow(...)]` to `crates/vb_queue_semantics/src/lib.rs`.
- MUST NOT edit `.config/source-length-exceptions.txt:323`.
- MUST NOT edit `rust-toolchain.toml`.
- MUST NOT edit contract artifacts retroactively.

If PO-001 or PO-002 fails, the recovery is `Failed::LintFailure` per
Holzman-Rust doctrine: the patch does not land; the source cleanup is
handed to a follow-up bead owned by the original `lib.rs` author. No
behavior-affecting waiver is filed against the workspace policy.
