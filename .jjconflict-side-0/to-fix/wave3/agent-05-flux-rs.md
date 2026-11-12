# Wave 3 — Agent 05: flux-rs (refinement types) Review

**Chunk:** 9 IDs (`vb-2ljzq`, `vb-30zsu`, `vb-32pmb`, `vb-36fly`, `vb-4pwif`, `vb-4rao8`, `vb-55qnv`, `vb-574zr`, `vb-596cp`)
**Scope:** storage / recovery / codec / digest
**flux-rs surface in `vb_storage`:** `crates/vb_storage/src/codec/mod.rs` and `crates/vb_storage/src/codec/flux_validation.rs` (artifact, gated behind missing `flux_rs` dep — `mod.rs:171-174`)

## Flux-surface map for this chunk

- `vb_storage/src/codec/mod.rs` — public codec module, NOT annotated (artifact module wired under `#[cfg(feature = "flux")]` only)
- `vb_storage/src/codec/flux_validation.rs` — 15 trusted models, all `#[flux_rs::trusted]` + `#[sig(fn(...) -> bool[true])]` — never compiled in this workspace

None of the 9 bug ID source paths touch the codec module. All flux-surface flags resolve to `NO`.

## Verdict Table

| bug-id  | pri | flux-surface | source-fix | test | flux-cmd | flux-result | cargo-result | verdict | evidence |
|---------|-----|--------------|------------|------|----------|-------------|--------------|---------|----------|
| vb-2ljzq | P2 | NO | `recovery/replay/core.rs:228-234` — `load_snapshot` STILL matches `Ok(None) \| Err(PostcardDecodeFailed) => Err(RecoveryError::CorruptSnapshot{run,seq})`. Missing/corrupt leak intact. | `recovery::tests` (213/213 ok) | `bash scripts/flux-check-package.sh vb_storage` | `Finished flux profile ... 2.34s` | `1270 passed; 0 failed` (vb_storage lib) | NOT-PATCHED | source at `crates/vb_storage/src/recovery/replay/core.rs:223-235`; bead close reason "Closed" with no commit ref; fix absent |
| vb-30zsu | P3 | NO | `journal/internal.rs:70` — function renamed `append_queued_unpersisted` → `append_queued_unfsynced` (commit `e7b2cb7d6 bead vb-s9iyv`, present in main). | `journal::tests::append_queued_unfsynced_*` (4/4 ok) | same | same | `72 passed; 0 failed` (journal+batch+tests) | PATCHED | rename landed; SA-016 misleading-name fix verified by code + tests |
| vb-32pmb | P2 | NO | Source paths `crates/vb_core/src/workflow/compiled_slug/codec.rs:26-31` and `compiled_query/mod.rs:54-59` no longer exist; refactored into `vb_core/src/compiled_workflow.rs`. `try_from_parts` (line 27) now calls `validate_parts` then `validate_budget` BEFORE any materialization; parent bead `vb-dyulo` is still `IN_PROGRESS` (so fix claim cannot be verified). | `workflow::tests::compiled_workflow_*` (7/7 ok) | same | same | `2143 passed; 0 failed` (vb_core lib) | UNKNOWN | file path no longer exists; cannot verify original bug location; parent bead open |
| vb-36fly | P3 | NO | `crates/vb_storage/src/admission/persistence.rs:34` (close-reason cite) does NOT exist — file refactored into `admission.rs`. Live code at `admission.rs:340, 354, 385, 398, 411, 413, 420` STILL uses `.map_err(\|_\| JournalError::ArtifactMalformed)?`. No `JournalError::from(postcard::Error)` / `From<postcard::Error>` import or usage found anywhere in `vb_storage/src/admission.rs`. | `admission::tests` (82/82 ok) | same | same | `82 passed; 0 failed` (admission::tests) | NOT-PATCHED | grep `JournalError::from` in `crates/vb_storage/src/admission.rs` returns 0 matches; source still swallows postcard errors |
| vb-4pwif | P3 | NO | `shard/impl_parts/chunk_001.rs:142-156` — referenced function `lock_admission` does NOT exist in the tree (`rtk grep "fn lock_admission" -r crates/` returns 0 matches). Current code at the cited lines is `reserve_index_set_slot` (no mutex poisoning). No `AdmissionPoisoned` variant in error taxonomy. Bead status `OPEN`. | `shard::impl_parts::chunk_001` tests pass | same (vb_storage only) | same | `1733 passed; 2 failed` (vb_runtime lib, pre-existing engine/execute failures unrelated to RA-014) | NOT-PATCHED | bead status OPEN; cited function absent; cited Recovery/Admission variant absent |
| vb-4rao8 | P4 | NO | `binary.rs:1-9` module doc says "Binary serialization helpers for record encoding/decoding" — no Endianness contract paragraph. `keys.rs:2-5` says "in big-endian byte order" only. Cross-cutting LE/BE rationale absent in BOTH files. Bead status `IN_PROGRESS`. | `binary::tests` (28/28 ok), `keys::tests` | same | same | `28 passed` (binary), keys tests pass | NOT-PATCHED | `rtk grep "Endianness\|endian contract\|endianness" crates/vb_storage/src/{binary.rs,keys.rs}` returns 0 matches; status IN_PROGRESS |
| vb-55qnv | P4 | NO | `vb_core/src/replay/mod.rs:219-227` — `new_replay_frame` now calls `.map_err(engine_to_replay_err)` (line 226), routing `RunFrame::new`'s `InvalidProgramCounter { step }` through the same evidence-preserving helper as CE-004. | `replay::tests` (3/3 ok) | same | same | `3 passed; 0 failed` (vb_core replay) | PATCHED | line 226 matches CE-004 follow-up fix |
| vb-574zr | P2 | NO | `recovery/replay/summary.rs:746-752` — `legacy_slot_taint` STILL matches `SlotValue::Bool(false) => Taint::Clean`. Asymmetric leak intact. Close-reason claim ("unconditionally returns Taint::Secret") does not match code. | `recovery::tests` (213/213 ok) | same | same | `213 passed; 0 failed` (recovery::tests) | NOT-PATCHED | live source at `crates/vb_storage/src/recovery/replay/summary.rs:746-752` still has the leak |
| vb-596cp | P3 | NO | `trimming/logic.rs:82-99` — hoisted scratch `key_buf` reused per iteration; per-iteration `Vec::to_vec()` AND Arc-cheap `key.clone()` both eliminated (CC-003 supersedes SC-008). The "RESOLVED_REJECTED" close reason predates the CC-003 fix landing on main. | `trimming::tests` (37/37 ok) | same | same | `37 passed; 0 failed` (trimming::tests) | PATCHED | commit `4395a598f fix(vb_storage/trimming): SC-008 alloc fix + compute_retained_terminal_seq`; source uses hoisted buffer |

## Flux Trusted/Ignore Abuse Cases

| artifact | location | abuse pattern | status |
|----------|----------|---------------|--------|
| `flux_validation.rs` (entire file) | `crates/vb_storage/src/codec/flux_validation.rs:11-167` | 15 `#[flux_rs::trusted]` models, all with `#[sig(fn(...) -> bool[true])]` — tautologically true specs prove nothing. `is_known_record_kind`, `validate_kind_family`, `contiguous_check`, `gap_detection`, `duplicate_detection`, all kind-stable models — every body is `#[flux_rs::trusted]`. | NOT COMPILED — module is gated under `#[cfg(feature = "flux")]` and `flux_rs` is not in workspace `[dependencies]` (`Cargo.toml` has no `flux_rs` entry). Comment at `codec/mod.rs:171-174` preserves it as an artifact. Re-enabling it today would force every model through the flux verifier with tautological sigs → 0 actual proof obligation. |

## Pre-existing test failures observed

- `engine::execute::execute_tests::execute_reduce_start_errors_on_uninitialized_input` — fails: `workflow validation failed: backward edge from StepIdx(0) to StepIdx(0)` at `crates/vb_runtime/src/engine/execute/execute_tests.rs:69:13`
- `engine::execute::execute_tests::execute_repeat_start_single_attempt_no_panic` — same panic at the same line
- Both unrelated to any of the 9 bugs in this chunk (engine/execute vs recovery/codec/replay).

## Counts

- bugs-checked: **9**
- PATCHED: **3** (vb-30zsu, vb-55qnv, vb-596cp)
- NOT-PATCHED: **5** (vb-2ljzq, vb-36fly, vb-4pwif, vb-4rao8, vb-574zr)
- UNKNOWN: **1** (vb-32pmb — file path gone, parent open)

## Top-3 NOT-PATCHED with reason

1. **vb-2ljzq (P2, SR-004)** — `load_snapshot` (`crates/vb_storage/src/recovery/replay/core.rs:223-235`) STILL conflates `Ok(None)` with `Err(PostcardDecodeFailed)` into `RecoveryError::CorruptSnapshot`. The "snapshot missing" case never returns a distinct error variant. Bead is CLOSED but source is unchanged. Fix should split the match arms: `Ok(None) => RecoveryError::SnapshotMissing { run, seq }` and `Err(PostcardDecodeFailed) => RecoveryError::CorruptSnapshot { run, seq }`.

2. **vb-574zr (P2, SR-013)** — `legacy_slot_taint` (`crates/vb_storage/src/recovery/replay/summary.rs:746-752`) STILL maps `SlotValue::Bool(false) => Taint::Clean`. Close reason claims the function "now unconditionally returns Taint::Secret"; that is false on `main`. Fix should drop the `Bool(false)` arm (treat like `_ => Taint::Secret`) and add a regression test that constructs a `SlotValue::Bool(false)` and asserts `Taint::Secret` (currently no such test).

3. **vb-36fly (P3, SA-015)** — `crates/vb_storage/src/admission.rs` STILL calls `.map_err(|_| JournalError::ArtifactMalformed)?` at lines 340, 354, 385, 398, 411, 413, 420. Close reason claims `JournalError::from` preserves the postcard error; no such usage exists. Fix should `impl From<postcard::Error> for JournalError { ... }` (or per-call `.map_err(JournalError::from)`) and route the inner error to a typed variant.

## Flux-abuse summary

- 1 artifact file with 15 tautological `#[flux_rs::trusted]` + `bool[true]` models
- Currently inert (gated behind missing `flux_rs` dep), but ready to re-enable as a proof blocker — every model would prove `true` regardless of the underlying `validation::is_known_record_kind` semantics
- No live `#[trusted]` / `#[ignore]` / `#[opaque]` abuse on production Rust in this chunk

## File-path written

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-05-flux-rs.md`