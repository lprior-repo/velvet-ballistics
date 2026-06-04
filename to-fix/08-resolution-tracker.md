# To-Fix Resolution Tracker

Generated 2026-05-24. Maps each defect from the master doc audit (`vb-3tew`) to resolution bead and status.

## Status Reconciliation 2026-06-03

`bd` is the live tracker. This file is only a human-readable mirror, and stale rows must not override bead status. This pass removed stale untracked-bead claims, closed compiler umbrella beads whose molecular blockers were already closed, and left only currently open defects in the remaining-work tables.

Direct checks for this reconciliation: `bd ready`, `bd show vb-w678`, `bd children vb-xi2f`, `bd show vb-xi2f`, grouped `bd list --all --flat --id ... --json` queries for the audit bead families, `grep` for stale tracker text, `grep` for `SystemTime::now` in `vb_runtime` collect primitives, and `date +%F`.

## Closed or Reconciled Since the Original Audit

| Area | Bead(s) | Current disposition |
|---|---|---|
| 01: Compiler v1 primitive lowering, nested bodies, trigger schema, vocabulary, references, diagnostics, and digest coverage | `vb-xi2f`, all 40 children | CLOSED. Stale umbrella beads `vb-xi2f.18`, `.19`, `.20`, `.8`, and parent `vb-xi2f` were closed during this reconciliation after `bd children vb-xi2f` showed every molecular child closed. |
| 02: Action completion validate-persist-mutate, full ticket evidence, output bounds, taint policy, and explicit non-durable runtime construction | `vb-w678`, `vb-w678.1`-.`5` | CLOSED. |
| 02: Frame pool allocates on empty pool | `vb-n70qh` | CLOSED. |
| 04: Moon pipeline references nonexistent formal task names | `vb-481r.1` | CLOSED. |
| 04: Required fuzz target names do not match Section 37 | `vb-481r.8` | CLOSED. |
| 04: vb-fzgdn State 12 formal blockers and missing raw evidence | `vb-u831a` | CLOSED with follow-up waived-lane beads retained. |
| 04: TLA-to-Rust partial RRO bridge rows | `vb-b69gz` | CLOSED. |
| 04: vb-fzgdn numeric timer trusted-base claim conflicts with `Instant` source | `vb-uwg7d` | CLOSED. |
| 05: 300-line Rust file policy and source-length gate coverage | `vb-zxgb`, `vb-ui6k`, `vb-jpq7.47` | CLOSED for enumeration/gate policy; hot split work remains under `vb-9kwz.*` and `vb-jpq7.9`. |
| 05: Timer wheel uses map/vector-backed storage | `vb-vi3g` | CLOSED. |
| 05: Workspace membership and deferred codegen graph residue | `vb-esq9.2`, `vb-esq9.3` | CLOSED. Cargo profile and duplicate compiler tree work remain open. |
| BIG audit compile-fail/trybuild silent-pass risk | `vb-j58jl` | CLOSED. Current master requires `trybuild` only for active public macro/schema contracts; `xtask ai-release --bead vb-nf2u` fails closed when required negative fixtures are missing. |
| 06: CLI OutputFormat Json/Jsonl enum variants removed | `vb-ne3j` | CLOSED. |
| 05: CI/evidence closed proof worktree diffs unlanded | `vb-ccyi` | CLOSED. |
| 05: Evidence closure manifest missing rows | `vb-rud5` | CLOSED. |

## Remaining P0 Defects

| Defect | Bead(s) | Priority | Status |
|---|---|---|---|
| 02: Runtime taint lattice diverges from normative 3-level lattice | `vb-o5zb.1` | P0 | OPEN |
| 02: Terminal step state can become pending again | `vb-o5zb.2` | P0 | OPEN |
| 02: ResourceContract shape/defaults violate master | `vb-o5zb.3` | P0 | OPEN |
| 03: Storage envelope does not reject trailing bytes | `vb-mrwe.1` | P0 | OPEN |
| 03: Compiled IR storage does not verify digest before put | `vb-mrwe.2` | P0 | OPEN |
| 03: Full digest check omits action ABI/policy digests | `vb-mrwe.3` | P0 | OPEN |
| 03: Pending action recovery unsupported | `vb-mrwe.4` | P0 | OPEN |
| 03: StepSucceeded mapped to SlotWritten record kind | `vb-mrwe.5` | P0 | OPEN |
| 04: TLC gate is fail-open and path-broken | `vb-481r.2` | P0 | OPEN |
| 04: Kani harnesses hardcode structural shapes | `vb-481r.3` | P0 | OPEN |
| 04: Verus step-budget proof not bound to production exec function | `vb-481r.4` | P0 | OPEN |
| 04: Verus RunFrame invariant proof not bound to production constructor semantics | `vb-481r.5` | P0 | OPEN |
| 04: Miri gate is still smoke-only | `vb-481r.6` | P0 | OPEN |
| 04: Coverage gate is still smoke-only | `vb-481r.7` | P0 | OPEN |
| 05: Hot runtime state uses map-like live structures and related boundedness gaps | `vb-jpq7.9` | P0 | IN_PROGRESS |
| 05: Hot runtime dispatcher is monolithic | `vb-9kwz.1` | P0 | OPEN |
| 05: Hot shard tick command dispatch is oversized | `vb-9kwz.2` | P0 | OPEN |
| 06: Live IPC server buffers before validating magic | `vb-k8ut.1` | P0 | OPEN |

## Remaining P1 Defects

| Defect | Bead(s) | Priority | Status |
|---|---|---|---|
| 03: Pending action index keyspace is not maintained | `vb-mrwe.6` | P1 | OPEN |
| 03: Journaled writer queue group commit is not proven | `vb-mrwe.7` | P1 | OPEN |
| 02: Collect primitive reads wall-clock time in runtime primitive logic | `vb-trq7b` | P1 | OPEN |
| 04: Root Cargo profiles missing | `vb-esq9.1` | P1 | OPEN |
| 04: Sanitizer task omitted from pipeline | `vb-481r.10` | P1 | OPEN |
| 04: Benchmark evidence below Section 39 | `vb-a7t6`, `vb-a7t6.1`-.`4` | P1 | OPEN |
| 05: Workspace/profile/deferred residue umbrella | `vb-esq9` | P1 | OPEN until `vb-esq9.1` and `vb-esq9.4` close. |
| 05: Duplicate compiler module tree | `vb-esq9.4` | P1 | OPEN |
| 05: Architecture drift umbrella for hot dispatcher splits | `vb-9kwz` | P1 | OPEN until `vb-9kwz.1`, `vb-9kwz.2`, and `vb-jpq7.9` close. |
| 06: IPC command set drifted from 11-command master | `vb-k8ut.2` | P1 | OPEN |
| 06: CLI `action inspect` takes numeric id not name | `vb-k8ut.3` | P1 | OPEN |
| 06: CLI command surface exceeds Section 33 without reconciliation | `vb-k8ut.4` | P1 | OPEN |
| 06: CLI `--emit postcard` wraps JSON not typed payloads | `vb-k8ut.5` | P1 | OPEN |
| Cross-cutting: fuzz package/docs still use `velvet-ballastics-fuzz` spelling | `vb-zsec2` | P1 | OPEN |

## Historical 2026-05-31 Evidence Notes

Live bead DB was repaired back to the authoritative DoltHub remote before the 2026-05-31 update. Direct checks run then: `bd status`, `bd show` for listed bead IDs, `bd search` for untracked terms, `/home/lewis/.cargo/bin/cargo test -p vb_boundary_inventory -- --list`, `/home/lewis/.cargo/bin/cargo test -p vb_doc -- --list`, `! rtk cargo run -p xtask -- ai-release --bead vb-nf2u`, `rtk cargo test -p xtask --test ui_release_gates ai_release_includes_ui_release_gates -- --exact`, `rtk cargo test -p xtask --test ui_release_errors missing_evidence_error_returns_typed_variant_and_diagnostic -- --exact`, `rtk cargo test -p xtask --test ui_release_errors false_pass_fixture_violation_returns_typed_variant_and_diagnostic -- --exact`, and `bash scripts/check-test-integrity.sh --self-test`.
