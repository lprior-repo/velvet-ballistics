bead_id: vb-qi37.16.4
phase: state-8
classification: PASS_AFTER_REPAIR

# State 8 Regression Classification

## First failure

- Category: `FORMAT`
- Classification: `BLOCK_LOCAL`
- Owner state: 8 repair / holzman-rust formatting fallout
- Rerun from: 8 after repair

The first machine-gate failure is formatter drift, including scoped files touched by this bead such as `crates/vb_runtime/src/shard/tests.rs`.

## Additional failures

- `ResourceContract` missing `allows_secret_results` appears in `vb_core`/`vb_codegen`, which are related to answer secret semantics and may be scoped fallout for bead vb-qi37.16.4.
- `vb_ipc/src/server/handlers.rs:243` missing `encoded` was already observed during State 7 smoke as outside immediate `vb_runtime` smoke scope, but it is a global gate failure and must be compared to baseline before landing.

## Decision

Do not advance to State 9. Route a targeted State 8 repair to `holzman-rust` for formatting plus scoped compile fallout, then rerun State 8 gates.

---

## Final classification after release-gate repair packets

The following focused repairs cleared the release-critical gate:

- `state-8-vb-ipc-as-conversions-repair.md`: `vb_ipc` safe conversion lint repaired.
- `state-8-fuzz-let-underscore-repair.md`: fuzz `let_underscore_must_use` repaired.
- `state-8-xtask-panic-lint-repair.md`: `xtask` panic lint repaired.
- `state-8-vb-ui-model-feature-powerset-repair.md`: `vb_ui_model` no-std feature-powerset repaired.

Final rerun evidence from `state-8-release-gates-rerun.md`:

```text
rtk cargo fmt -- --check: PASS
moon run :test: 9863 tests run, 9863 passed, 0 skipped
moon ci: Tasks: 19 completed (1 cached), Time: 3m 52s 48ms
```

Final decision: State 8 is `PASS_AFTER_REPAIR`; `vb-qi37.16.4` may advance to State 9.

---

## Current post Black-Hat repair classification

The current State 8 rerun after the State 11 Black Hat answer-command repair
first failed as:

- Category: `FORMAT`
- Classification: `BLOCK_LOCAL`
- Scoped file: `crates/velvet_ballistics/src/main.rs`
- Owner: State 8 repair via `holzman-rust`

`state-8-format-repair.md` records `STATUS: REPAIRED` with no behavior change.

Post-repair orchestrator gate result:

```text
rtk cargo fmt -- --check: PASS
rtk cargo check -p velvet_ballistics -p vb_ipc --all-targets --all-features: PASS
moon run :test: 9863 tests run, 9863 passed, 0 skipped
moon ci: Tasks: 19 completed (2 cached)
```

Final classification: `PASS_AFTER_FORMAT_REPAIR`. `vb-qi37.16.4` may advance to State 9.

---

## Current post INV-002 repair classification

State 11 Black Hat found a bead-local contract defect:

- Category: `CONTRACT_PARITY`
- Classification: `BLOCK_LOCAL`
- Scoped file: `crates/vb_ipc/src/server/handlers.rs`
- Contract: INV-002 taint enforcement
- Owner: State 6 repair via `holzman-rust`

`state-6-inv002-repair.md` records `STATUS: REPAIRED`.

Post-repair orchestrator gate result:

```text
rtk cargo fmt -- --check: PASS
rtk cargo check -p vb_ipc -p vb_runtime -p velvet_ballistics --all-targets --all-features: PASS
rtk cargo test -p vb_ipc --lib answer: 13 passed
rtk cargo test -p vb_runtime --lib ask_answer: 24 passed
moon run :test: 9867 tests run, 9867 passed, 0 skipped
moon ci: Tasks: 19 completed (1 cached)
```

Final classification: `PASS_AFTER_INV002_REPAIR`. `vb-qi37.16.4` may advance to State 9.
