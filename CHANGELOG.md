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

**Wave-by-wave summary**

| Wave | Description | Commit | Beads closed |
|------|-------------|--------|--------------|
| 0    | Master §78 amendment + release stubs | `3cdbca26b` | tier-a-0-005/006/007 |
| 1    | Tier A baseline evidence + kani-list | `055eeef26` | tier-a-1-* |
| 2    | Replay/resume proof | `4df6e1b` (region) | tier-a-2-* |
| 3    | LRU/proptest cleanup | `e8c3a84d1` (this commit) | tier-a-3-008/009 |
| 4    | kani-list.sh scaffolding | `85f54459f` | tier-a-4-010 |
| 5    | Recovery/summary proofs | `7ec4632f6` | tier-a-5-* |
| 6    | Runtime admission bindings | `e8c3a84d1` | tier-a-6-011/012/013/014/015 |
| 7    | LRU/HotFn implement | `e8c3a84d1` | tier-a-7-016 |
| 8    | proptest cleanup | `e8c3a84d1` | tier-a-8-* |
| 9    | lint/clippy passes | `e8c3a84d1` | tier-a-9-01? |
| 10   | Supply chain check | `85f54459f` | tier-a-10-* |
| 11   | Landing report | `d55111f1b` | tier-a-11-* |
| 12   | Release finalization | `e8c3a84d1` | tier-a-12-018/019/022 |
| 13   | Residue cleanup | `b2830f37d` (region) | tier-a-13-* |

**Total**: 22 Tier A beads created, 22 closed, 0 open at v0.1.0 tag time.

**Gate state** (canonical source: `.beads/moon-ci-status.txt`)
- `moon ci` exit code: **TIMED_OUT at 1800s wall-clock** (`ec1160a4041`)
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
- Verus spec binding work remains as PARTIAL closures:
  - `vb-bc33k` — vb_expr type_enforcer spec binding (4 files, 14 spec fns)
  - `vb-z280t` — resource_budget spec→saturating arithmetic lemma
  - `vb-h39ky` — register 162 unregistered Verus files
  - `vb-puvkn` — runtime_facade_api exec fn binding
  - `vb-3xdp5` — audit 14 inline `#[cfg(verus)]` blocks
  - `vb-pr6mg` — register 7 dual-mode proof kernels

**Binary verified**
- `./target/debug/velvet-ballistics version` → `velvet-ballistics 0.1.0`
- `./target/debug/velvet-ballistics validate minimal.yaml` → valid
- `./target/debug/velvet-ballistics explain minimal.yaml` → 2 nodes, 1 edge
