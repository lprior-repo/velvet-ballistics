# Wave 4 Agent 14 — Ad-hoc moon-pipeline-expert deep-dive

Scope: `.moon.yml` pipeline, `.moon/tasks/*.yml` task definitions, fail-closed behavior, sanitizer inclusion, profile coverage (Section 34).
Chunk: `/tmp/wave4-chunk-14.txt` — 5 IDs: `vb-ykph4`, `vb-yt5wq`, `vb-z6gpb`, `vb-z8q3q`, `vb-zsapv`.
Mode: read-only, no bead creation.

## Pipeline baseline (canonical `moon ci`)

`.moon.yml:7-26` (includes `all.yml`, `kani.yml`, `verus.yml`, `tlc.yml`):
```yaml
pipeline:
  - fmt, lint-src, check, verify-kani, nightly-feature-gate,
    nightly-feature-cargo-probe, source-length, supply-chain,
    feature-powerset, hardened-build, test, doc-test, doc,
    mutants-smoke, fuzz-smoke, miri, verify-verus, verify-tlc,
    coverage, bench-build
```

- `verify-kani` (`kani.yml:14`) — fail-closed (`set -euo pipefail`), `runInCI: true`, in pipeline ✓
- `verify-verus` (`verus.yml:13`) — fail-closed, `runInCI: true`, in pipeline ✓
- `verify-tlc` (`tlc.yml:14`) — fail-closed (exits 1/127 on missing cfg/spec), `runInCI: true`, in pipeline ✓
- `verify-kani-vb-validate` (`kani.yml:36`) — fail-closed, `runInCI: true`, **NOT in `.moon.yml` pipeline** ✗
- `verify-loom` (`loom.yml:11`) — fail-closed, `runInCI: true`, **NOT in `.moon.yml` pipeline** ✗ (loom file is never even included in `.moon.yml` includes block)
- `sanitizer-address-check` (`all.yml:549`) — `runInCI: true`, **NOT in `.moon.yml` pipeline** ✗
- Root `Cargo.toml` only defines `[profile.hardened]` and `[profile.maxperf]` (l.94, 103). Section 34 contract (MASTER.md:1390, 1396) requires explicit `[profile.release]` and `[profile.bench]`. Profile mismatch ✗
- `verify-verus-all` (`verus.yml:32`) — `runInCI: false`, not in pipeline (expected; deep lane).
- `quick` (`all.yml:784`) — `runInCI: false` (developer convenience, expected).
- Fail-open markers: only `set +e` at `all.yml:379` (geiger run, bounded by explicit status check + 90s timeout and required output file presence — not fail-open). No `|| true` in any moon task script body. Tasks are fail-closed.

## Per-bug table

| bug-id | pri | moon-task | fail-closed | in-pipeline | profile | verdict | evidence |
|---|---|---|---|---|---|---|---|
| vb-ykph4 | P3 | (n/a — RS-218 runtime shard introspect formatter run mismatch; source code defect at `crates/vb_runtime/src/shard/introspection.rs:216`) | n/a | n/a | n/a | UNKNOWN | Not a moon-pipeline bug. Source-level formatter defect. No moon task names, no fail-open surface, no profile impact. Out of agent-14 scope. |
| vb-yt5wq | P4 | (n/a — CE-003 follow-up adding taint-lattice regression tests) | n/a | n/a | n/a | UNKNOWN | CLOSED test-addition bead. No moon-pipeline surface; taint tests run under existing `test` lane (`all.yml:292`, in pipeline). Out of agent-14 scope. |
| vb-z6gpb | P2 | (n/a — CW-003 `validate_node_bounds` missing `on_error` + kind-specific targets at `crates/vb_core/src/engine/validate.rs:16-29`) | n/a | n/a | n/a | UNKNOWN | Pure source-code validator bug. No moon task names, fail-closed status, pipeline-inclusion, or profile angle. Out of agent-14 scope. |
| vb-z8q3q | P1 | (n/a — VERIFY-NEW-5: TEST-C0-01 orphan `[[bin]]` paths in `fuzz/Cargo.toml`) | n/a | n/a | n/a | UNKNOWN | CLOSED fuzz manifest defect (4 of 48 paths missing files). Surfaced through Wave 1 verification, not via moon pipeline. `fuzz-smoke` (`all.yml:507`, in pipeline) would catch rebuild failures but not orphan declarations. Out of agent-14 scope. |
| vb-zsapv | P2 | (n/a — CV-104 `RuntimeLimitsProfile::new` accepted `journal_writer_queue_capacity` above `MAX_JOURNAL_BATCH_BYTES`) | n/a | n/a | n/a | UNKNOWN | CLOSED source-code defect at `crates/vb_core/src/policy/contract.rs:206`. Not a moon task / pipeline issue. Out of agent-14 scope. |

## Summary

- bugs-checked: 5
- pass: 0
- partial: 0
- fail (NOT-PATCHED): 0
- unknown: 5 — none of the 5 IDs in this chunk are moon-pipeline bugs; each is a source-code, test, or manifest defect outside this agent's domain.
- chunk mis-assignment note: chunk 14's IDs map to defects in `vb_runtime/src/shard`, `vb_core/src/engine`, `vb_core/src/policy`, `vb_core::lattice`, and `fuzz/Cargo.toml` — none touch `.moon.yml`, `.moon/tasks/*.yml`, sanitizer inclusion, fail-closed shell scripts, or root Cargo.toml profile sections.

## Fail-open gates (moon-wide, not per-bug)

None. Every CI-gated (`runInCI: true`) moon task script body begins with `set -euo pipefail` and chains via `&&`. The only `set +e` (`all.yml:379`, cargo-geiger wrapper) is bounded by an explicit status + non-empty-output check before re-enabling `set -e` at line 382. No `|| true` anywhere in `.moon/tasks/*.yml`.

## Profile mismatches (moon-wide, not per-bug)

- Root `Cargo.toml` has only `[profile.hardened]` and `[profile.maxperf]`. Section 34 (`velvet-ballistics-MASTER.md:1390-1400`) mandates explicit `[profile.release]` (opt-level=3, lto=thin, codegen-units=1, strip=symbols) and `[profile.bench]` (inherits release, debug=true). `hardened` and `maxperf` both `inherits = "release"`, so they currently resolve to cargo defaults. `to-fix/04-ci-formal-evidence-defects.md:109,115` already records this defect.
- `phase0_scaffold_test.rs:492,496,500,507,511,518,525` enforces `[profile.maxperf]` invariants but no analogous test enforces `[profile.release]` or `[profile.bench]`.

## Pipeline gaps (moon-wide, not per-bug)

These CI-gated tasks are missing from the canonical `moon ci` pipeline:
- `verify-kani-vb-validate` (`kani.yml:36`, `runInCI: true`) — runs Kani on `vb_validate` crate, only `verify-kani` for `vb_core` is in the pipeline.
- `verify-loom` (`loom.yml:11`, `runInCI: true`) — entire `loom.yml` is not even `includes`'d by `.moon.yml:1-5`.
- `sanitizer-address-check` (`all.yml:549`, `runInCI: true`) — Section 40 / `to-fix/04-ci-formal-evidence-defects.md:126-133` flags this; ASan lane is configured but never invoked by `moon ci`.

## Top-3 NOT-PATCHED with reason

1. None — no moon-pipeline bug is present in chunk 14. All 5 IDs are source/test/manifest defects outside this agent's domain.
2. (n/a)
3. (n/a)

For completeness, the moon-pipeline-level defects that *do* exist (root Cargo.toml Section 34 profile gap, `sanitizer-address-check`/`verify-kani-vb-validate`/`verify-loom` not in `moon ci` pipeline, `loom.yml` not included by `.moon.yml`) are pipeline-wide and would be reported by an agent assigned a chunk containing those bugs — not by any of these 5 IDs.

## File path

Written: `/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-14-adhoc-moon-pipeline.md`
