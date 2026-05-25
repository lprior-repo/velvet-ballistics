# Proof Strategy: vb-m5gp

## Scope

- Bead: `vb-m5gp` — pure refactor split of `crates/vb_compile/src/lib.rs`.
- State: 4 proof planning only.
- Workspace: `/home/lewis/src/go-skill-vb-m5gp`.
- Forbidden checkout: `/home/lewis/src/velvet-ballistics` was not used for source edits or discovery.
- No production, test, proof, dependency, or CI implementation artifacts are written by this plan.

## Inputs Consumed

- `STATE.md`
- `baseline-report.md`
- `codebase-map.md`
- `delivery-scope.jsonl`
- `contract.md`
- `domain-model-review.md`
- `tla-spec.md`
- `lean-contract.md`
- `verification-layers.md`
- `proof-obligations.jsonl`
- `traceability-matrix.jsonl`
- State 11 rejection inputs for attempt 3 repair: `formal-verification-report.md`, `regression-diff.md`, `ci-failure-category.txt`

## Discovery Evidence

Executed in `/home/lewis/src/go-skill-vb-m5gp`:

1. `pwd -P && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac && test -s ".beads/vb-m5gp/contract.md" && test -s ".beads/vb-m5gp/traceability-matrix.jsonl" && test -s ".beads/vb-m5gp/delivery-scope.jsonl"`
   - Result: passed; current directory was `/home/lewis/src/go-skill-vb-m5gp`.
2. `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" ...scoped paths...`
   - Result: 568 matches in 41 files. Matches are concentrated in existing tests/Kani assertions, `#![forbid(unsafe_code)]`, idempotency/retry domain code, and existing compile code. No temporal/concurrency trigger was discovered in the scoped refactor surface.
3. `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" ...scoped paths...`
   - Result: 238 matches in 39 files. Relevant lanes discovered: Kani harnesses under `crates/vb_compile/src/kani_idempotency_parity.rs` and top-level `kani/idempotency_gate_parity.rs`, proptest tests, fuzz target compile surface, and `#![forbid(unsafe_code)]`.
4. Kani invocation discovery:
   - `scripts/rust-verification-gauntlet.sh` uses repository-supported form `cargo kani --package vb_compile --harness <harness> --quiet`.
   - `crates/vb_compile/src/kani_idempotency_parity.rs` declares `#[kani::proof] fn idempotency_gate_parity()` behind `#[cfg(kani)]` in `lib.rs`.
   - `cargo kani list --format json` returned `No supported targets were found`, so listing is not reliable in this workspace. The planned executable command uses the repository-supported form from `scripts/rust-verification-gauntlet.sh`: `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet`.

## Risk Classification

| Risk class | Applicability | Strategy |
|---|---:|---|
| Temporal/state-machine | Not applicable | No scheduler, queue, lease, retry lifecycle, distributed protocol, fairness, or liveness behavior is changed. Do not invent TLA+. |
| Rust-local invariant | Applicable | Compile, clippy, source scans, module dependency review, file-length gate. |
| Bounded state | Applicable only to idempotency decision table | Existing Kani idempotency parity harness. |
| Refinement/type-state | Low | No new type-state design; rely on API compile parity and source review. |
| Concurrency | Not applicable | No spawn/tokio/mutex/atomic concurrent behavior in scope. |
| Unsafe/UB | Applicable as governance | `#![forbid(unsafe_code)]`, clippy forbidden constructs, optional Miri lane. |
| Untrusted input | Applicable behaviorally | Existing YAML/compiler tests and workspace integration tests must preserve accepted/rejected behavior. |
| Dependency/supply-chain | Applicable as negative constraint | Explicit dependency/config diff gate. |
| Performance | Not applicable | No performance claim or hot-path redesign. |
| Release-critical gates | Applicable | `moon ci` remains canonical final rollup. |

## Lane Selection

Required executable gates:

1. Workspace isolation and required input presence.
2. Dependency/config no-change diff.
3. Formatting and strict source-target clippy for `vb_compile`.
4. API compatibility via `cargo +nightly test -p vb_compile --all-targets --all-features` and selected downstream workspace tests using the actual package `velvet-ballistics-workspace-tests`.
5. Behavior parity via `moon ci` plus targeted compile/error integration tests.
6. Static structure checks for private module names, private facade, acyclic dependency direction, stale scaffolding non-use, visibility leakage, and file length.
7. Kani idempotency parity using `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet` when Kani tooling is available.

Optional/deep gate:

- Miri: useful for panic/UB regression signal but not mandatory for a behavior-free file move. If unavailable or over budget, formal verifier may record a waiver with required compensating evidence from clippy, tests, Kani, and `moon ci`.

Explicit non-applicable/waived lanes:

- TLA+: non-applicable. This bead has no temporal state machine. Stronger local gates are API, behavior, structure, Kani, clippy, and `moon ci`.
- Verus: waived while the implementation is a pure move. If implementation changes validation/lowering/digest/idempotency semantics, rerun State 3 and add Verus/Kani obligations for changed pure logic.
- Lean/Aeneas/Hax: non-applicable. No theorem-critical kernel is introduced.
- Loom: non-applicable. No concurrency primitive or scheduler behavior in scope.
- Fuzz execution: not required for proof planning; compile/test coverage of existing fuzz target reachability is represented by `moon ci`. Add fuzz execution only if later implementation changes parser semantics.

## Acceptance Rule for Later States

Proof writing/formal verification must not claim success unless `proof-obligations.planned.jsonl` rows marked `required=true` are executed, accepted as not applicable, or explicitly waived with evidence. Any semantic change beyond moving code invalidates the TLA/Verus/theorem waivers and returns the bead to State 3.

## Attempt 3 Obligation Command Repair

State 11 rejected the prior approved plan because three exact obligation commands were locally invalid even though project-equivalent evidence passed:

- `API-002` / `PO-005`: replaced package `workspace_tests` with actual package `velvet-ballistics-workspace-tests` in exact command and evidence wording.
- `ERR-001` / `PO-007`: replaced package `workspace_tests` with actual package `velvet-ballistics-workspace-tests` for the diagnostic integration test command and evidence wording.
- `STATIC-001` / `PO-010`: replaced all-target clippy with the repository-governed strict source lint command `cargo +nightly clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings`. All-target clippy is not retained as required because repository governance states source lint is strict while test clippy is not strict; existing test-target lint debt is outside this bead's source-lint obligation.
