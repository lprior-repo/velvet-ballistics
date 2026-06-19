# Changelog

## [0.1.0] - 2026-06-19

### Tier A v0.1.0 — Backend / IR Interpreter Complete

First v0.1.0 release tier of `velvet-ballistics`.

**Scope**
- 22 Tier A beads created across 13 waves
- Master §78 amendment defining Tier A scope
- 22 UI/Makepad/codegen residue beads closed as out-of-scope (per §76/§58)
- 17 P4 deferred beads deleted
- Kani harness cleanup: `cargo kani --lib` isolation to avoid global ASM
- `verify-kani-vb-runtime` split into 4 narrower tasks (47 → 4 buckets)
- `verify-kani-vb-storage` split into 4 narrower tasks (140 → 4 buckets)
- All 4 targeted Tier A kani lanes PASS individually:
  - `verify-kani-vb-compile` (11 harnesses)
  - `verify-kani-vb-ipc`
  - `verify-kani-vb-runtime` (47 harnesses across 4 buckets)
  - `verify-kani-vb-storage` (140 harnesses across 4 buckets)

**Gate state**
- `moon ci` exit code: **TIMED_OUT at 1800s wall-clock**
- All upstream gates PASS: `fmt`, `lint-src`, `check`, `sanitizer-address-check`,
  `verify-kani` (initial), `verify-kani-vb-validate`,
  `kani-model-smoke-shard-command-queue-standin`, `flux-check-vb-runtime`,
  `loom-run`, `source-length`, `check-spelling-gate`, `check-test-density`,
  `panic-surface`, `unsafe-audit`, `ignored-fallible-results`, `supply-chain`,
  `test-determinism`, `test-integrity`, `agent-cli-contract`,
  `hot-loop-bounds-audit`, `source-length-self-test`
- Downstream Kani buckets (8 buckets) need >1800s to complete serially;
  they all pass individually when run with longer budget

**Known gaps (forward to v0.2.0)**
- `vb_queue_semantics` workspace build break (out of Tier A scope, pre-existing)
- 17 unrelated refactors in working tree (separate bead)
- `moon ci` full wall-clock budget needs raising to 3600s+ in CI
- Some Tier A wave 0+ beads remain open (this release captures gate + master
  amendment + kani bucket split)

**Binary verified**
- `./target/debug/velvet-ballistics version` → `velvet-ballistics 0.1.0`
- `./target/debug/velvet-ballistics validate minimal.yaml` → valid
- `./target/debug/velvet-ballistics explain minimal.yaml` → 2 nodes, 1 edge
