# Collect V48 independent TEST review

Date: 2026-10-23 UTC. Scope: the seven corrected Collect unit/property test cases and their native receipt. This is not full F1/F2 acceptance, a production code audit, or a new runtime execution.

## Verdict

No contract-weakening defect found in the reviewed changes. They correct the previous premature-completion expectations to the authoritative MASTER contract: non-empty CollectStart enters body; exhausted CollectNext enters done; genuinely missing pagination state remains an error. Native receipt records 176 passed, 0 failed, 0 ignored, 1813 filtered out. The focused gate passed with strict warnings, exit 0, 2.947 seconds elapsed. This does not establish a full runtime-suite or canonical CI pass.

## Exact pins

- Comparison base: `6b591076ebc4423dbe01e320bd738f4b9eaa5aa7`.
- Tested source recorded before and after execution: `78d7eab5f1f4a62a00627072887a5330edd8a4cc`.
- Receipt commit: `72460f0cb3fa119c053f87479ae6f99fcdfdd146`, whose parent is exactly the tested-source commit above.
- Both source and receipt have crates tree `0a3aeb00290db5b3358706a78d5f43bbf4070089`.
- Source-to-receipt delta is Beads-only: the existing contract record and seven added receipt files. No source/configuration changes in that delta.
- Receipt directory: `.beads/vb-8eocj.3.5/native-collect-v48/`.

## Contract and assertion review

Authoritative `velvet-ballistics-MASTER.md` at the tested pin, lines 2236–2284: line 2246 requires empty start to go to done and non-empty start to go to body; line 2271 requires exhausted next to go to done; line 2272 requires InvalidCompiledWorkflow if pagination state is missing.

Reviewed changes in `crates/vb_runtime/src/primitives/collect/tests.rs`:

1. `collect_start_page_size_at_limit_boundary`: requires body on non-empty exact-limit input.
2. `collect_start_uses_source_as_collector_when_output_is_none_for_non_empty`: checks exact first-page values, then explicit next, done, empty collector, and removed state.
3. `collect_next_cursor_at_item_count_goes_to_done`: requires retained state after start and successful exhausted next, not an artificial missing-state error.
4. `collect_repeated_start_next_cycles`: requires first-cycle body, explicit next, completion and state cleanup before another cycle.
5. `collect_next_writes_empty_page_and_removes_state_after_last_item`: checks successful terminal next, empty collector and removed state.
6. `collect_start_exact_page_limit_finishes_without_active_pagination_state`: now checks body first, exact first-page values, retained cursor/item_count, then terminal next and cleanup. Its unchanged name is stale and misleading, but the assertions are correct.

The seventh change, `prop4_collect_pagination_reentry` in `crates/vb_runtime/src/primitives/reentry_tests.rs:1555–1632`, removes the single-page early-return escape. Every non-empty case now exercises body running/succeeded transitions and next routing through completion. It is a primitive re-entry property, not a source-admission test or a complete page-payload oracle.

Preserved negatives were inspected directly: `collect/tests.rs:799–825` still checks the exact missing-state error; duplicate, stale and future-page cases at lines 3283, 3422 and 3468 retain typed order errors and state-preservation checks. These names and the seven corrected cases appear passing in the raw stdout. No ignores, skips, limits or gate relaxations were added by the reviewed test diff.

## Receipt verification

Actual recorded command: `env CARGO_TARGET_DIR=<existing native target> RUSTFLAGS=-Dwarnings RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 cargo test -p vb_runtime --lib collect -- --nocapture --test-threads=1`.

The command's JSON records exit 0, 2.947 seconds, 300-second timeout, and timed_out false. Stdout ends with `176 passed; 0 failed; 0 ignored; 0 measured; 1813 filtered out`. Stderr records normal compilation and test launch, not an omitted execution.

Independently recomputed SHA-256 hashes:

| File | SHA-256 |
|---|---|
| manifest.json | `0d731aba2915726f4ac90aae1abbed5d3bf75ee1b68bbc01a0f4ea8159d3dfbc` |
| assessment.md | `f6dce77d7dc88ae0ea4099bddc8d0305fa10d5061eaf3401dad6346524ec65ad` |
| ROOT_F2_COLLECT_RUNTIME_V48_NATIVE.json | `916b9ac87ad19e20ebc85783871f7b0e81253411c1d2bde83b983ba0d73598a3` |
| ROOT_F2_COLLECT_RUNTIME_V48_NATIVE.stdout | `9602c7922edb468c4d48fba8dd5749a6513976ccbee346803fedcfa463a40a94` |
| ROOT_F2_COLLECT_RUNTIME_V48_NATIVE.stderr | `bcaa66e22c2fd3c29a5747d85f0ef08223197e843995bf0e3461ba8e7a2e74db` |
| ROOT_F2_COLLECT_RUNTIME_V48_NATIVE.source-before.json | `20f47336f5f9ca8ce700a4fa5deecc525cbce0267df5da95ade607b322d9c2c0` |
| ROOT_F2_COLLECT_RUNTIME_V48_NATIVE.source-after.json | `f79cfab89c854936b8e729f74a4264dcd00bfceb5b286baedbb7f2e80bd3bd75` |

All five raw-file hashes match manifest.json. Before/after source snapshots contain equal 1811-file hash maps and both report aggregate source SHA-256 `462dcc49717092a908c492cff76be25c28c18637027230b4c05edb9ac1e3c78d`. I independently compared the two changed test files, Collect production file and MASTER blobs at the tested commit to their recorded hashes:

| Source file | SHA-256 |
|---|---|
| crates/vb_runtime/src/primitives/collect/tests.rs | `77569cd06bdf8fae064b44a482a0bd52ed9263ebc4171bfabc69ac218c4733f9` |
| crates/vb_runtime/src/primitives/reentry_tests.rs | `731135b42ff56d94f93a2d7100073a5a82ee5cf44d529f6539bf543c14cde443` |
| crates/vb_runtime/src/primitives/collect.rs | `f8100d53768046c32473d1bffe3475514c47aba55003ffe6ff516662498235f3` |
| velvet-ballistics-MASTER.md | `25fcd502b827a4908afd4bedfcb77ff2f1649cdd8007d695e4487e89bd0d419a` |

I did not independently regenerate all 1811 repository-content hashes or the aggregate-hash algorithm. Equal snapshot maps and the relevant blob checks are the precise independently verified scope.

## Check limitations and review basis

No Rust command, native runtime probe, source/test edit, or main push was performed by this reviewer. This review inspected authoritative source, diffs, raw native receipts and hashes. Full source-to-IR-to-runtime nested-flow acceptance, unrelated runtime tests, compiler gates and canonical CI are outside this focused result. The older 68-test retrospective receipt remains separately qualified by its explicitly unrecorded contemporaneous tested SHA/tree; this stronger V48 receipt does not repair that older gap.

Guidance used: repository AGENTS.md, authoritative MASTER, `nasa-jpl-standards.md`, and `runtime-performance-architecture.md`.
