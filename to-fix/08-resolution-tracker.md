# To-Fix Resolution Tracker

Generated 2026-05-24. Maps each defect from the master doc audit (`vb-3tew`) to resolution bead and status.

## Truth Serum Update 2026-05-31

Live bead DB was repaired back to the authoritative DoltHub remote before this update. The 2026-05-24 tables below are preserved as historical tracker output, but the following rows are now corrected by bead evidence:

| Tracker/doc gap | Current bead disposition |
|---|---|
| 02: Action completion mutates frame before durable evidence | Closed under `vb-w678.4`; umbrella `vb-w678` is closed. |
| 02: Durable action events lose ActionTicket/idempotency key | Closed under `vb-w678.1`; umbrella `vb-w678` is closed. |
| 02: Action output size/taint policy not enforced on completion | Closed under `vb-w678.3`; umbrella `vb-w678` is closed. |
| 02: Runtime::new defaults to dropping all journal events | Closed under `vb-w678.5`. |
| 02: Frame pool allocates on empty pool | Newly tracked as `vb-n70qh`. |
| 04: Root Cargo profiles missing | Tracked as `vb-esq9.1`. |
| 05: File-size drift / source-length gate too narrow | Tracked by `vb-zxgb`, `vb-ui6k`, `vb-9kwz`, and closed `vb-jpq7.47`. |
| 03: Pending action index keyspace not maintained | Tracked as `vb-mrwe.6`. |
| 03: Journaled writer queue group commit not proven | Tracked as `vb-mrwe.7`. |
| 05: Hot shard state uses map-like structures | Tracked as `vb-jpq7.9` and related `vb-9kwz` notes. |
| 05: Timer wheel uses map/vector-backed storage | Tracked as `vb-vi3g`. |
| 05: Workspace shape / deferred codegen residue / duplicate compiler tree | Tracked as `vb-esq9`, `vb-esq9.1`, and `vb-esq9.4`; `vb-esq9.2` and `vb-esq9.3` are closed. |
| BIG audit compile-fail/trybuild silent-pass risk | Resolved under `vb-j58jl`: current master requires `trybuild` only for active public macro/schema contracts, and `xtask ai-release --bead vb-nf2u` fails closed when required negative fixtures are missing. |
| vb-fzgdn State 12 formal blockers and missing raw evidence | Newly tracked as `vb-u831a`. |
| TLA-to-Rust partial RRO bridge rows | Newly tracked as `vb-b69gz`. |
| vb-fzgdn numeric timer trusted-base claim conflicts with `Instant` source | Newly tracked as `vb-uwg7d`. |

Direct checks run for this update: `bd status`, `bd show` for listed bead IDs, `bd search` for untracked terms, `/home/lewis/.cargo/bin/cargo test -p vb_boundary_inventory -- --list`, `/home/lewis/.cargo/bin/cargo test -p vb_doc -- --list`, `! rtk cargo run -p xtask -- ai-release --bead vb-nf2u`, `rtk cargo test -p xtask --test ui_release_gates ai_release_includes_ui_release_gates -- --exact`, `rtk cargo test -p xtask --test ui_release_errors missing_evidence_error_returns_typed_variant_and_diagnostic -- --exact`, `rtk cargo test -p xtask --test ui_release_errors false_pass_fixture_violation_returns_typed_variant_and_diagnostic -- --exact`, and `bash scripts/check-test-integrity.sh --self-test`.

## Resolved (this session)

| Defect | Bead | Status | Resolution |
|---|---|---|---|
| 04: Moon pipeline references nonexistent formal task names | vb-481r.1 | ✓ CLOSED | `.moon.yml` updated: `kani-verify`→`verify-kani`, `verus-verify`→`verify-verus`, `tlc-verify`→`verify-tlc` |
| 04: Required fuzz target names do not match Section 37 | vb-481r.8 | ✓ CLOSED | `fuzz/Cargo.toml` targets renamed to canonical: `compiled_ir`, `expression`, `ipc_frame`, `journal_event`, `yaml_events`. Moon fuzz-smoke includes all 5. |
| 05: CI/evidence: closed proof worktree diffs unlanded | vb-ccyi | ✓ CLOSED | All 6 worktrees retired, directories removed. Inventory at `.beads/vb-ccyi/`. |
| 05: Evidence: vb-jpq7 closure manifest missing rows | vb-rud5 | ✓ CLOSED | Manifest backfilled, check script exits 0. |
| 06: CLI: OutputFormat Json/Jsonl enum variants removed | vb-ne3j | ✓ CLOSED | Json/Jsonl variants removed from OutputFormat; --json/--jsonl CLI flags removed from action parsing. |

## Remaining P0 Defects (open beads exist)

| Defect | Bead(s) | Priority |
|---|---|---|
| 01: Compiler lowering rejects do/choose, nested bodies only accept set | vb-xi2f, vb-xi2f.1, vb-xi2f.3-.24 | P0 |
| 01: Compiler emits unchecked compiled workflows via from_parts_unchecked | vb-xi2f.4 | P0 |
| 01: Trigger schema diverges from master Section 9 | vb-xi2f.5 | P0 |
| 01: Primitive vocabulary stale (parallel/aggregate vs together/reduce) | vb-xi2f.6, .15 | P0 |
| 01: Canonical step output references not implemented | vb-xi2f (umbrella) | P0 |
| 02: Action completion mutates frame before durable evidence | P0 need bead | P0 |
| 02: Durable action events lose ActionTicket/idempotency key | P0 need bead | P0 |
| 02: Action output size/taint policy not enforced on completion | P0 need bead | P0 |
| 02: Runtime taint lattice diverges from normative 3-level | vb-o5zb.1 | P0 |
| 02: Terminal step state can become pending again | vb-o5zb.2 | P0 |
| 02: ResourceContract shape/defaults violate master | vb-o5zb.3 | P0 |
| 03: Storage envelope doesn't reject trailing bytes | vb-mrwe.1 | P0 |
| 03: Compiled IR storage doesn't verify digest before put | vb-mrwe.2 | P0 |
| 03: Full digest check omits action ABI/policy digests | vb-mrwe.3 | P0 |
| 03: Pending action recovery unsupported | vb-mrwe.4 | P0 |
| 03: StepSucceeded mapped to SlotWritten record kind | vb-mrwe.5 | P0 |
| 04: Miri/coverage smoke-only | vb-481r.6, vb-481r.7 | P0 |
| 04: TLC gate fail-open and path-broken | vb-481r.2 | P0 |
| 04: Kani harnesses hardcode structural shapes | vb-481r.3 | P0 |
| 04: Verus proofs not bound to production exec functions | vb-481r.4, vb-481r.5 | P0 |
| 05: File-size drift (378 files over 300 lines) | need bead | P0 |
| 05: Source-length gate too narrow | need bead | P0 |
| 05: Hot runtime dispatcher monolithic | vb-9kwz.1 | P0 |
| 05: Hot shard tick command dispatch oversized | vb-9kwz.2 | P0 |
| 06: Live IPC server buffers before validating magic | vb-k8ut.1 | P0 |

## Remaining P1 Defects

| Defect | Bead(s) | Priority |
|---|---|---|
| 01: Compiled digest doesn't cover full semantics | vb-xi2f.28-.39 | P1 |
| 01: Diagnostics lack real path/span | vb-xi2f.9 | P1 |
| 01: Diagnostic code format not symbolic | vb-xi2f.10 | P1 |
| 02: Runtime::new defaults to dropping all journal events | need bead | P1 |
| 02: Collect primitive reads wall-clock time | need bead | P1 |
| 02: Frame pool allocates on empty pool | need bead | P1 |
| 03: Pending action index keyspace not maintained | need bead | P1 |
| 03: Journaled writer queue not proven group commit | need bead | P1 |
| 04: Root Cargo profiles missing | need bead | P1 |
| 04: Sanitizer task omitted from pipeline | vb-481r.10 | P1 |
| 04: Benchmark evidence below Section 39 | vb-a7t6, vb-a7t6.1-.4 | P1 |
| 05: Hot shard state uses map-like structures | need bead | P1 |
| 05: Timer wheel uses map/vector-backed storage | need bead | P1 |
| 05: Workspace shape drifts from master | need bead | P1 |
| 05: Deferred codegen residue in active graph | need bead | P1 |
| 05: Duplicate compiler module tree | need bead | P1 |
| 06: IPC command set drifted from 11-command master | vb-k8ut.2 | P1 |
| 06: CLI action inspect takes numeric id not name | vb-k8ut.3 | P1 |
| 06: CLI command surface exceeds Section 33 | vb-k8ut.4 | P1 |
| 06: CLI --emit postcard wraps JSON not typed payloads | vb-k8ut.5 | P1 |
