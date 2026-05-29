# vb-3vo5q fuzz-smoke evidence

## Scope

Triage the `moon ci` fuzz-smoke failure that reported a stale
`fuzz_targets/journal_cancel_kill_record_kind.rs` call to
`vb_storage::encode_record`.

## Findings

- No current first-party source file matching
  `fuzz_targets/journal_cancel_kill_record_kind.rs` exists in `fuzz/`.
- Current fuzz sources that call `vb_storage::encode_record` use the current
  safe five-argument API shape.
- A direct cargo-fuzz build and the Moon fuzz-smoke lane now pass without code
  changes.

## Command evidence

- `env RUSTFLAGS="-Dwarnings" rustup run nightly-2026-04-28 cargo fuzz build --target x86_64-unknown-linux-gnu`
  - PASS: finished release profile in 0.30s.
- `moon run velvet-ballistics:fuzz-smoke`
  - PASS: task completed in 8.734s.

## Fuzz target contract

- Input model: existing cargo-fuzz targets only; no new target added.
- Oracle: existing harness oracles unchanged.
- Resource bounds: existing Moon smoke bounds unchanged (`-max_total_time=1`,
  30s wrapper per selected target).
- Sanitizer lane: existing cargo-fuzz build/smoke lane unchanged.

## Residual risk

The stale failure was not reproducible after rebuilding the current fuzz target
set. Full `moon ci` still has unrelated global blockers recorded under
`vb-p7v8q` evidence and the latest Moon output.
