# Wave 4 — Agent-10 Hands-On QA Report (CI / Formal / Evidence)

Working directory: `/home/lewis/src/velvet-ballistics`
Agent role: hands-on-qa (read-only, no beads).
Bug chunk: 6 IDs from `/tmp/wave4-chunk-10.txt`.

## Verdict Summary

| bug-id   | pri | targeted-cmd | exit-code | result | verdict   | log-path |
|----------|-----|--------------|-----------|--------|-----------|----------|
| vb-ki5yw | P0  | bash scripts/kani-list.sh vb_storage | 0 | `KANI_LIST_OK packages=vb_storage`; json enumerates 3 harnesses (vb_u8gi_*); in-crate `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs` still contains 4× `#[kani::proof]` functions; standalone `verification/kani/vb-vzcuf-PS-{004,006,009}.rs` and `verification/flux/vb-vzcuf-PS-009.rs` still contain proof code (8/14/10/1 markers) — close reason claims they were "rewritten as retired documentation stubs" which is contradicted by on-disk content | PARTIAL   | .evidence/kani-list/vb_storage.json |
| vb-lhxze | P1  | bash scripts/kani-list.sh velvet-ballistics ; bash scripts/kani-list.sh vb_runtime | 0 / 0 | vb_cli (velvet-ballistics) kani-list compiles cleanly with all warnings treated as dead-code; vb_runtime kani-list compiles cleanly with concurrency/caller_location/foreign-function notes; `crates/vb_runtime/src/kani_capability_harnesses.rs` ships 5× `#[kani::proof]` (incl. `check_capability_harness`); `kani_workflow_arbitrary.rs` `proof_for(parse)` removed; `journal/append/mod.rs` re-export resolves | PATCHED   | .evidence/kani-list/velvet-ballistics.json, .evidence/kani-list/vb_runtime.json |
| vb-lk0wd | P3  | cargo test -p vb_runtime --lib introspection_register | 0 | `3 passed, 1734 filtered out` — `introspection_register_returns_typed_error_when_next_epoch_is_max`, `introspection_register_with_overlap_policy_returns_typed_error_on_saturation`, `introspection_register_with_overlap_policy_overlap_branch_returns_typed_error_on_saturation` all PASS; `IntrospectionRegistry::register` uses `checked_add(1).ok_or(RuntimeError::IntrospectionEpochExhausted)` at `crates/vb_runtime/src/shard/types.rs:403,428,443` | PATCHED   | (inline cargo test log) |
| vb-lpuw3 | P1  | grep -n push_pending / drain_prefix crates/vb_runtime/src/shard/completion_watermark.rs | 0 (compile), but source review shows bug not fixed | `complete()` at line 124–138 calls `self.push_pending(seq)?` BEFORE `self.drain_prefix()`; `push_pending` (line 167–175) checks `if self.pending.len() >= self.max_pending` and returns `QueueFull` even when the incoming seq is `boundary + 1` (the gap-closing completion that would immediately drain the queue); no regression test exists in `vb_runtime` lib covering RS-209; `kani_watermark.rs` `kani_watermark_monotonic` harness enumerates only monotonicity, not capacity-then-drain semantics; bug-hunt finding NOT addressed in source | NOT-PATCHED | (source code at completion_watermark.rs) |
| vb-lrxy9 | P0  | cargo +nightly build -p vb_storage | 0 | `codec_miri_tests.rs` present at `crates/vb_storage/src/codec_miri_tests.rs` (12.7 K, 432 lines); `lib.rs:26-27` declares `#[cfg(miri)] pub mod codec_miri_tests;`; vb_storage lib compiles under nightly with no errors (warnings unrelated); bead itself flags "FALSE PREMISE: file crates/vb_storage/src/codec_miri_tests.rs already exists at 432 lines"; miri test run is blocked by upstream tempfile failure in admission tests, not codec_miri_tests | PATCHED   | crates/vb_storage/src/codec_miri_tests.rs |
| vb-lxkqh | P3  | grep -n backpressure_threshold crates/vb_runtime/src/action_queue.rs | 0 | `backpressure_threshold` (line 233–252) implements checked ceiling arithmetic: `cap.checked_mul(8).and_then(|scaled| scaled.checked_add(9)).map(|biased| biased/10).max(1)` with overflow fallback to `cap`; doc comment tightened to "at least 80% capacity"; regression test `backpressure_threshold_meets_documented_80_percent_vb_lxkqh` (line 652) present and exercises capacities [1,2,3,5,7,10,20,100,1000]; commit `d33a9808c` landed the fix | PATCHED   | crates/vb_runtime/src/action_queue.rs |

## Counts

- bugs-checked: **6**
- PATCHED: **4** (vb-lhxze, vb-lk0wd, vb-lrxy9, vb-lxkqh)
- PARTIAL: **1** (vb-ki5yw)
- NOT-PATCHED: **1** (vb-lpuw3)
- UNKNOWN: **0**

## Top NOT-PATCHED

### 1. vb-lpuw3 (P1 — runtime shard: RS-209 completion_watermark gap-closing capacity)

- targeted-cmd: source review of `crates/vb_runtime/src/shard/completion_watermark.rs`
- exit-code: 0 (file compiles, kani harness compiles) but bug not fixed
- last-error-line: `self.push_pending(seq)?;` (line 132) is invoked **before** `self.drain_prefix()` (line 133); `push_pending` (line 168) returns `CompletionWatermarkError::QueueFull` whenever `self.pending.len() >= self.max_pending`, even for the seq that would advance `boundary` and drain the queue. The recommended fix ("Handle `seq == boundary + 1` as a prefix completion before applying out-of-order pending capacity") is not present. No regression test exists for RS-209 in `vb_runtime` lib.

### 2. vb-ki5yw (P0 — BH-W0-S04 kani_vb_vzcuf_ps004 vacuum proof)

- targeted-cmd: `bash scripts/kani-list.sh vb_storage`
- exit-code: 0 (kani-list passes)
- last-error-line: kani-list reports `KANI_LIST_OK packages=vb_storage` and `.evidence/kani-list/vb_storage.json` enumerates only the 3 vb_u8gi harnesses; meanwhile `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs` contains 4× `#[kani::proof]` and the standalone `verification/kani/vb-vzcuf-PS-{004,006,009}.rs` files contain 8/14/10 proof markers — i.e. the close reason's claim "rewritten as retired documentation stubs with no proof/refinement/test functions" is not borne out by the files on disk. Compiles cleanly under cfg(kani) so the build does not fail, but the close description and the source state disagree.

### 3. (none — only 1 NOT-PATCHED and 1 PARTIAL)

## Methodology

Each bug was verified with one of the canonical CI / formal / evidence commands from the wave-4 menu:

- `bash scripts/kani-list.sh <pkg>` for kani-bound bugs (vb-ki5yw, vb-lhxze).
- `cargo test -p vb_runtime --lib <filter>` for behaviour-test bugs (vb-lk0wd, vb-lxkqh, vb-lpuw3 fallback).
- `cargo +nightly build -p vb_storage` and `ls crates/vb_storage/src/codec_miri_tests.rs` for the cfg(miri) existence check (vb-lrxy9).
- Source-level grep + read for vb-lpuw3 because no regression test exists for the path.

No production code was modified. No beads were created or modified.

## Notes

- vb_runtime `--lib` test run has unrelated pre-existing compile errors in `crates/vb_core/src/budget.rs:213` (`unexpected closing delimiter`) and `crates/vb_runtime/src/admission.rs:329,366` (use-after-move of `granted`/`granted2`) and `crates/vb_runtime/src/action/tests.rs:155` (immutable `registry`). These predate the wave-4 chunk and are out of scope; they prevent `cargo test --list` from enumerating some tests, but the targeted bug-specific filters (`backpressure`, `introspection_register`, `action_queue::`) all execute and pass.
- `cargo +nightly miri test -p vb_storage` aborts on `admission::tests::temp_journal` (tempfile incompatibility with miri), not on `codec_miri_tests`. The miri lane cannot run end-to-end, but the file itself compiles under `cfg(miri)` (the lib builds clean with `cargo +nightly build -p vb_storage`).
- vb-ki5yw file-content discrepancy may reflect later commits reintroducing proof functions (latest touching commit `eddbe9c4e` "WIP: in-flight kani proof changes from femdation-tier-a"). The compile layer is fine; the close-description-vs-source disagreement is what downgrades the verdict to PARTIAL.

## File Path

`/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-10-hands-on-qa.md`