# Global Readiness Report: vb-jpq7.3

## Status

`GLOBAL_PASS` for final closure readiness. Prior canonical repository gates
passed after Kani seam/evidence and supplemental test-integrity repairs. The
latest rerun after the versioned slot-write extra envelope, full-journal corrupt
taint, ignored-fallible-result scanner, runtime journal encode, and cargo-vet
supply-chain repairs passes.

## Formatting Gate

Command:

```bash
rustup run nightly-2026-04-28 cargo fmt --all -- --check
```

Result: PASS on live rerun.

## Canonical Gate

Command:

```bash
moon ci
```

Prior result: PASS on fresh rerun. Output saved by the shell tool at:

```text
/home/lewis/.local/share/opencode/tool-output/tool_e54429101001QjpToALrkXHR2g
```

Observed final summary:

- `Tasks: 24 completed (3 cached)`.
- `velvet-ballastics:test`: `12165 tests run: 12165 passed (5 slow), 0 skipped`.
- `velvet-ballastics:panic-surface`: `NoViolationFound`.
- `velvet-ballastics:ignored-fallible-results`: `NoViolationFound`.
- `velvet-ballastics:test-integrity`: `PASS base=HEAD`.
- `velvet-ballastics:source-length`: PASS; only `DEFERRED_GLOBAL` compile split notices remain.

Superseded closure rerun after supplemental test-integrity repair:

```text
/home/lewis/.local/share/opencode/tool-output/tool_e5464d5ba001pbGsXBRAO78L6g
```

Observed final summary:

- `Tasks: 24 completed (5 cached)`.
- `velvet-ballastics:test-integrity`: `test integrity: PASS base=HEAD`.
- `velvet-ballastics:test`: `12165 tests run: 12165 passed (5 slow, 14 leaky), 0 skipped`.
- `velvet-ballastics:panic-surface`: `NoViolationFound`.
- `velvet-ballastics:ignored-fallible-results`: `NoViolationFound`.
- `velvet-ballastics:source-length`: PASS; only `DEFERRED_GLOBAL` compile split notices remain.

Latest closure rerun after P0 taint/scanner/supply-chain repair:

```text
/home/lewis/.local/share/opencode/tool-output/tool_e54ad4ea40019LkG7p2r0N30AH
```

Observed final summary:

- `Tasks: 25 completed (5 cached)`.
- `velvet-ballastics:test-integrity`: `test integrity: PASS base=HEAD`.
- `velvet-ballastics:test`: `12167 tests run: 12167 passed (5 slow), 0 skipped`.
- `velvet-ballastics:panic-surface`: `NoViolationFound`.
- `velvet-ballastics:ignored-fallible-results`: fixture checks for embedded/split `.ok()` passed and final scan returned `NoViolationFound`.
- `velvet-ballastics:supply-chain`: completed successfully.
- `velvet-ballastics:source-length`: PASS; only `DEFERRED_GLOBAL` compile split notices remain.

Latest closure rerun after versioned slot-write extra envelope repair:

```text
/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z
```

Observed final summary:

- `Tasks: 25 completed (3 cached)`.
- `velvet-ballastics:test-integrity`: `test integrity: PASS base=HEAD`.
- `velvet-ballastics:test`: `12169 tests run: 12169 passed (5 slow), 0 skipped`.
- `velvet-ballastics:panic-surface`: `NoViolationFound`.
- `velvet-ballastics:ignored-fallible-results`: `NoViolationFound`.
- `velvet-ballastics:supply-chain`: completed successfully.
- `velvet-ballastics:source-length`: PASS; only `DEFERRED_GLOBAL` compile split notices remain.

## Scoped Mitigation

- Prior global blockers were repaired before this rerun:
  - production panic-surface `unreachable!(...)` in `crates/vb_codegen/src/parity.rs`;
  - workspace-test dead-code blockers;
  - source-length hot-function and cargo-mutants residue blockers.
- Scoped vb-jpq7.3 behavior gates still pass.
- Superseded rerun evidence `/home/lewis/.local/share/opencode/tool-output/tool_e5452fd53001Mc2ed6UxB8v3AY` is **not** closure evidence: it failed
  `velvet-ballastics:test-integrity` with `WeakenedAssertion|crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs|removed_exact=2 added_exact=1`.
- The public hydration behavior test now carries supplemental exact source assertions to repair that integrity finding; reruns `tool_e5464d5ba001pbGsXBRAO78L6g`, `tool_e54ad4ea40019LkG7p2r0N30AH`, and latest `tool_e54cfc867001em3UkY7dnDZZ7z` pass.

## Required Before Closure

No global-readiness waiver is required for `moon ci` as of the fresh PASS above.
Remaining closure blockers are proof/reviewer acceptance and scoped evidence
packaging, not canonical Moon CI.
