# CI, Fuzz, Formal, and Evidence Defects

## Status Update 2026-06-03

Closed: formal task names (`vb-481r.1`), Section 37 fuzz target names (`vb-481r.8`), vb-fzgdn State 12 evidence closure (`vb-u831a`), partial TLA RRO bridge closure (`vb-b69gz`), and the stale numeric timer trusted-base claim (`vb-uwg7d`).

Still open: TLC fail-open/path issues (`vb-481r.2`), hardcoded Kani shapes (`vb-481r.3`), Verus production binding gaps (`vb-481r.4`, `vb-481r.5`), Miri/coverage smoke-only gates (`vb-481r.6`, `vb-481r.7`), root Cargo profiles (`vb-esq9.1`), sanitizer pipeline omission (`vb-481r.10`), and Section 39 benchmark evidence (`vb-a7t6`, `vb-a7t6.1`-.`4`).

<!-- RESOLVED 2026-05-24: Sections 1 (formal task names) and 2 (fuzz target names) resolved via vb-481r.1, vb-481r.8 -->

## ✅ P0: Moon pipeline references nonexistent formal task names [RESOLVED: vb-481r.1]

Evidence:

- `.moon.yml:11` calls `kani-verify`.
- `.moon.yml:24` calls `verus-verify`.
- `.moon.yml:25` calls `tlc-verify`.
- `.moon/tasks/kani.yml:14` defines `verify-kani`; `.moon/tasks/kani.yml:31` defines `verify-kani-vb-validate`.
- `.moon/tasks/verus.yml:13` defines `verify-verus`; `.moon/tasks/verus.yml:30` defines `verify-verus-all`.
- `.moon/tasks/tlc.yml:12`, `35`, and `53` define `verify-tlc`, `verify-tlc-workflow`, and `verify-tlc-idempotency`.

Master violated:

- Section 40: `moon ci` is the canonical gate.
- Section 44 point 23: full current-scope gates must pass.

Impact: Formal lanes are not reliably part of the canonical pipeline.

Suggested bead: `P0 fix moon ci formal task name wiring`

## ✅ P0: Required fuzz target names do not match executable Cargo fuzz targets [RESOLVED: vb-481r.8]

Evidence:

- `fuzz/Cargo.toml:7` sets `autobins = false`.
- `fuzz/Cargo.toml:84-89` declares exact target `journal_event`.
- `fuzz/Cargo.toml:441-467` declares `compiled_ir_fuzz`, `ipc_frame_fuzz`, `expression_fuzz`, and `yaml_events_fuzz`, not the required exact names.
- `.moon/tasks/all.yml:458-460` runs `yaml_events`, `ipc_frame`, `journal_event`, and `compiled_ir`; it omits `expression`.

Master violated:

- Section 37: required fuzz targets are `yaml_events`, `expression`, `ipc_frame`, `journal_event`, and `compiled_ir`.
- Section 40: fuzz-smoke is required in Moon CI.

Impact: Four required fuzz targets are not wired under required names, and expression fuzz is not run by fuzz-smoke.

Suggested bead: `P0 make Section 37 fuzz targets executable by required names`

## P0: Miri and coverage tasks are smoke-only

Evidence:

- `.moon/tasks/all.yml:377-390` runs one `vb_core` Miri test: `ids::tests::run_id_zero_constant`.
- `.moon/tasks/all.yml:392-412` runs one `vb_core` llvm-cov test: `action::tests::validate_action_outcome_failed_always_succeeds`.

Master violated:

- Section 4: Miri required for `vb_core`, `vb_expr`, and `vb_compile`.
- Section 40: `cargo llvm-cov --workspace --all-features` required.
- Section 44 point 23.

Impact: Passing Moon cannot prove required Miri/coverage scope.

Suggested bead: `P0 replace miri and coverage smokes with master-required gates`

## P0: TLC gate is fail-open and path-broken

Evidence:

- `.moon/tasks/tlc.yml:19-24` appends `|| true` to every TLC run in `verify-tlc`.
- `.moon/tasks/tlc.yml:42-43` references `verification/tla/specs/WorkflowBoundedAdmission.tla`, while subagent inspection found that spec at `verification/tla/WorkflowBoundedAdmission.tla`.

Master violated:

- Section 40.
- AGENTS Formal Verification Mandate 3: no unbounded/vacuous TLA math and verification must not cheat the math.

Impact: TLC failures can be swallowed, and key specs can be skipped or path-broken.

Suggested bead: `P0 make TLC gate fail-closed and cover root specs`

## P0: Kani harnesses hardcode structural shapes

Evidence:

- Subagent inspection found `kani/pipeline.rs` and `kani/gate_12_14_15.rs` constructing fixed `WorkflowParts` and fixed `RunFrame` shapes.

Master violated:

- AGENTS Formal Verification Mandate 1: Kani harnesses must not hardcode structural inputs such as `WorkflowParts` or `RunFrame` with fixed dummy data.

Impact: Proofs cover toy structures, not arbitrary core structures.

Suggested bead: `P0 replace hardcoded Kani workflow/frame shapes with arbitrary generators`

## P0: Verus proofs are largely not bound to production exec functions

Evidence:

- Subagent inspection found `verification/verus/step_budget.rs` proving standalone integer spec functions instead of binding to production `StepBudget::try_take`.
- Subagent inspection found `verification/verus/run_frame_invariant.rs` using `SpecRunFrame` rather than production `RunFrame::new` semantics.

Master violated:

- AGENTS Formal Verification Mandate 2: no vacuum Verus proofs.

Impact: Verus evidence does not prove actual production Rust satisfies the model.

Suggested bead: `P0 bind Verus proofs to production exec functions`

## P1: Root Cargo profiles required by master are missing

Evidence:

- Root `Cargo.toml` ends at lints and lacks `[profile.release]` and `[profile.bench]`.
- `.moon/tasks/all.yml:248` uses `--profile hardened`.
- `.moon/tasks/all.yml` also contains maxperf profile tasks per subagent inspection.

Master violated:

- Section 34 profile contract.
- Section 40 release gate expectations.

Impact: Release/bench/hardened/maxperf tasks may fail or use unintended profile policy.

Suggested bead: `P1 restore master-required Cargo profiles or document waiver`

## P1: Sanitizer task exists but is omitted from pipeline

Evidence:

- `.moon/tasks/all.yml:479-490` defines `sanitizer-address-check`.
- `.moon.yml:7-27` pipeline omits it.

Master violated:

- Section 40: nightly sanitizer jobs required for runtime, IPC, storage, and binary decoding crates.

Impact: Canonical CI can pass without sanitizer coverage.

Suggested bead: `P1 add sanitizer jobs to moon ci pipeline`

## P1: Benchmark evidence is below Section 39 acceptance

Evidence:

- Subagent inspection found benchmark metadata saying `instructions=not-collected` and `allocations=allocator-external`.
- Existing evidence files contain only partial metrics and no full p50/p95/p99, instruction counts, allocation counts, bytes allocated, CPU/governor/kernel/RUSTFLAGS matrix.

Master violated:

- Section 39 mandatory benchmark metadata.
- Section 44 point 22.

Impact: No current speed/performance claim can be accepted.

Suggested bead: `P1 produce Section 39 complete benchmark evidence`
