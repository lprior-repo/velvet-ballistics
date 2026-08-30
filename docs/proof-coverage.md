# Proof Coverage

The repository ships three formal-verification layers — Kani (bounded model
checking), TLC (TLA+ state-machine model checking), and Verus (deductive Rust
proofs). The CI baseline (`moon run :ci`) only gates on a small slice of each.

## Totals

| Verifier | Total artifacts | In CI baseline | Manual only |
| --- | --- | --- | --- |
| Kani harnesses (`#[kani::proof]`) | 331 | 8 (4 `vb_core` + 4 `vb_validate`) | 323 |
| TLA+ root specs (`.tla`) | 26 | 2 (`WorkflowBoundedAdmission`, `IdempotencySafety`) | 24 |
| Verus spec files (`verification/verus/*.rs`) | 70 | 0 | 70 |

Kani counts are sourced from `.evidence/kani-list/*.json` (regenerate with
`bash scripts/kani-list.sh <pkg>`); see `.evidence/kani/baseline/README.md`
for the harness-to-claim map and current baseline status.

## CI Baseline

### Kani (8 harnesses, `moon run :kani-baseline`)

**`vb_core`** — `cargo kani --lib -p vb_core --all-features`, logs to
`.evidence/kani/baseline/vb_core.log`:

| # | Harness filter | Claim class | Inventory under filter |
| - | --- | --- | --- |
| 1 | `kani_step_budget_try_take_arbitrary::kani_step_budget_try_take_arbitrary` | arithmetic / no-overflow | 1 |
| 2 | `kani_idempotency_gates::` | idempotency / contract | 17 |
| 3 | `kani_taint::` | taint / lattice-no-panic | 6 |
| 4 | `kani_workflow_budget_harnesses::` | workflow-budget / bounded-compute | 5 |

**`vb_validate`** — `cargo kani --lib -p vb_validate --all-features`, logs to
`.evidence/kani/baseline/vb_validate.log`:

| # | Harness filter | Claim class | Inventory under filter |
| - | --- | --- | --- |
| 5 | `kani_gate_08_arbitrary_parts_valid_accessors_pass` | gate-08 / structural-valid-accessors | 1 |
| 6 | `kani_gate_08_arbitrary_parts_root_oob_rejected` | gate-08 / structural-root-oob-rejection | 1 |
| 7 | `kani_gate_08_arbitrary_parts_symbol_oob_rejected` | gate-08 / structural-symbol-oob-rejection | 1 |
| 8 | `kani_step_primitives::` | step-primitives / constant-content | 0 (orphan) |

The `vb_validate.log` file exists but contains a compilation failure — not
verification output. The `kani-baseline` and `kani-baseline-heavy` moon tasks
are CI dependencies but cannot produce `VERIFICATION:- SUCCESSFUL` logs because
vb_core Kani harnesses fail to compile under `cfg(kani)`. The root cause is
missing `input_slots` field initializers in `WorkflowParts` builders in
`crates/vb_core/src/replay/kani_harnesses.rs`,
`crates/vb_core/src/kani_workflow_arbitrary.rs`, and
`crates/vb_core/src/kani_step_harnesses.rs`. See
`.evidence/kani/baseline/README.md` for the raw failure log. The fail-closed
sentinel (`grep -q 'VERIFICATION:- SUCCESSFUL'`) would reject the existing log.

### TLA+ (2 specs, `moon run :verify-tlc`)

| Spec | Bounded model | CI baseline reason |
| --- | --- | --- |
| `verification/tla/WorkflowBoundedAdmission.tla` | Bounded step-budget workflow admission | Small state, fast (`-terse -metadir`) |
| `verification/tla/IdempotencySafety.tla` | Crash-recovery + duplicate-replay safety | Highest-value bounded model; covered by 9 sub-configs |

Run via:

```bash
TLC=(tlc) moon run :verify-tlc
# or with bundled JAR
TLA2TOOLS_JAR=/path/to/tla2tools.jar moon run :verify-tlc
```

### Verus (0 in CI)

`moon run :verify-verus` reads `contracts/proof_obligations.yaml` and runs
the registry's `verus:` targets via `scripts/verify-verus.sh`. It is marked
`runInCI: true` in `.moon/tasks/verus.yml` but is **not** a dep of the
`ci:` pipeline in `.moon/tasks/all.yml` (verus requires the `verus` binary
plus 120-minute budget per PR). The full-workspace sweep
`moon run :verify-verus-all` is `runInCI: false`.

The 70 `.rs` files under `verification/verus/` (excluding `extern_*` thin
extern-spec mirrors) are reachable only through manual invocation of the
above commands.

## Exploratory Kani Harnesses (Top 10 by Package)

The 323 non-baseline harnesses are distributed across seven packages. The
five highest-leverage modules outside the CI baseline:

| Rank | Module | Package | Harnesses | Subject |
| - | --- | --- | --- | --- |
| 1 | `vb_core::budget::tests_and_verification` | `vb_core` | 12 | Aggregate budget try-add, overflow/underflow, symbolics |
| 2 | `vb_core::frame::frame_kani_harnesses` | `vb_core` | 11 | Frame transition validity, slot bounds, PC bounds |
| 3 | `vb_core::frame::parallel_in_flight_kani` | `vb_core` | 6 | Parallel-in-flight counter no-panic |
| 4 | `vb_core::engine::expr_eval::kani_stack` | `vb_core` | 9 | Expr-eval stack push/pop overflow |
| 5 | `vb_core::engine::expr_eval::kani_div_zero` | `vb_core` | 3 | f64 / i64 division-by-zero, MIN/-1 trap |
| 6 | `vb_core::kani_capability_harnesses` | `vb_core` | multiple | Capability construction parity |
| 7 | `vb_compile::mod_compile_lowering::kani_proofs::*` | `vb_compile` | 8 | Digest determinism across foreach loops |
| 8 | `vb_compile::mod_compile_lowering::kani::*` | `vb_compile` | 4 | Choose-body / choose-slots / choose-width / choose-lowering |
| 9 | `vb_compile::kani::*` (digest family) | `vb_compile` | ~10 | Ask-prompt sensitivity, timeout sentinel, resource contract |
| 10 | `vb_core::kani_step_harnesses` / `kani_workflow_budget_harnesses::kani_workflow_budget_generators` (extra) | `vb_core` | 5 | Arbitrary workflow-budget generators beyond baseline module filter |

Per-package totals: `vb_core` 206, `vb_compile` 65, `vb_validate` 27,
`vb_runtime` 26, `vb_verification` 3, `vb_storage` 3, `vb_yaml` 1.

## Gaps

### Kani

- **323 of 331 harnesses are exploratory.** Only 8 short-circuit the PR gate;
  the rest depend on developer invocation of `cargo kani --harness <name>`
  per package.
- **Baseline logs do not pass CI.** The `kani-baseline` and `kani-baseline-heavy`
  moon tasks are listed as CI dependencies. `vb_validate.log` exists in the
  baseline directory but contains compilation failure output (7
  `missing field input_slots` errors); it does not contain the
  `VERIFICATION:- SUCCESSFUL` sentinel that the fail-closed check requires.
  vb_core.log and vb_core_heavy.log are not yet generated. The root blocker is
  vb_core Kani harness compilation failures. See `.evidence/kani/baseline/README.md`.
- **`vb_compile`, `vb_runtime`, `vb_storage`, `vb_yaml`, `vb_verification`
  have zero harnesses in `kani-baseline`.** The single-filter
  `cargo kani --harness <name>` form used in `.moon/tasks/kani.yml` (legacy
  `verify-kani` / `verify-kani-vb-validate`) is not in `ci:` either.
- **Module-level filter granularity hides harnesses.** `kani_idempotency_gates::`
  emits 17 reachable harnesses in one CBMC run but they are not enumerated
  individually in the CI evidence.
- **Orphan harness (resolved).** `kani_step_primitives::` was previously
  unreachable because `mod kani_step_primitives` was not declared in
  `crates/vb_validate/src/verification/mod.rs`. The module is now declared
  with `#[cfg(kani)]` (line 24) so the filter resolves to 4 harnesses. The
  harness count in proof-coverage remains at 0 only until the baseline logs
  are generated.

### TLA+

- **24 of 26 root specs are exploratory.** Only `WorkflowBoundedAdmission`
  and `IdempotencySafety` are wired into `verify-tlc`. Heavy specs
  (`StepBudgetSuspension`, `VbKyyfReplayDeterminism`, `YamlE2eChain`) stay
  out of CI until they have bounded, time-safe `.cfg` files.
- **IdempotencySafety has 9 sub-configs** (`IdempotencySafetyOverflow`,
  `IdempotencySafetyCrashRecoverDuplicate`, etc.); only the root
  `IdempotencySafety.cfg` runs in CI.

### Verus

- **0 of 70 spec files are in `ci:`.** `verify-verus` and `verify-verus-all`
  are both manual. The `verify-verus` task reads from
  `contracts/proof_obligations.yaml` and currently lists 41 `verus:` keys.
- **Coverage is registry-driven, not pipeline-gated.** A Verus spec added
  under `verification/verus/` without a matching `verus:` registry entry
  ships unverified.

## Expanding CI Coverage

Sequenced, low-risk steps that close the F-006 gap without changing
production code:

0. **Fix vb_core Kani compilation.** Add `input_slots: Default::default()` to
   the `WorkflowParts` initializers in:
   - `crates/vb_core/src/replay/kani_harnesses.rs:22`
   - `crates/vb_core/src/kani_workflow_arbitrary.rs:369`
   - `crates/vb_core/src/kani_step_harnesses.rs:53,158,230,291,436`
   This unblocks the entire `kani-baseline` and `kani-baseline-heavy` CI
   pipeline. Once fixed, re-run the baseline and commit the generated logs.

1. **Wire the orphan harness.** The `mod kani_step_primitives;` declaration
    already exists in `crates/vb_validate/src/verification/mod.rs` (line 24,
    gated by `#[cfg(kani)]`). This was fixed in a prior bead. This step is
    superseded; the baseline harness count for vb_validate is now 8
    (4 gate-08 + 4 step-primitives) pending vb_core compilation fix.

2. **Add per-package `kani-baseline-*` tasks** mirroring the existing
   `kani-baseline` task for `vb_compile`, `vb_runtime`, `vb_storage`,
   `vb_yaml`, and `vb_verification`. Promote one high-value harness per
   package (e.g. `kani_digest_determinism` in `vb_compile`,
   `kani_recovery_hydrate` in `vb_runtime`) and gate them through `ci:`.
3. **Promote additional `vb_core` harnesses.** The
   `kani_idempotency_gates::` and `kani_taint::` module-level filters
   already compile 17 + 6 harnesses per run; split each into a
   per-function filter to make CBMC cost (and CI time) proportional to the
   harnesses actually required for the PR.
4. **Gate `verify-tlc` from `ci:`** for the two baseline specs and add a
   `tlc-baseline` task in `.moon/tasks/tlc.yml` analogous to `kani-baseline`
   with a `TLC_RUN_SUCCESSFUL` sentinel grep. This makes the TLA+ gate
   fail-closed like Kani.
5. **Promote TLA+ sub-configs.** Move the nine `IdempotencySafety*` sub-
   configs into `verify-tlc` after bounding their state space to the
   values exercised by the root spec.
6. **Wire `verify-verus` into `ci:`** with a 60-minute timeout and
   registry-pinned target list. Defer `verify-verus-all` to nightly.
7. **Inventory sync.** Regenerate `.evidence/kani-list/*.json` on every
   harness edit and pin the expected total in `kani-baseline` so a
   deleted harness blocks the PR.

All steps above are `.moon/tasks/*.yml`, `.beads/`, or inventory-JSON
changes — no production code, no installs.
