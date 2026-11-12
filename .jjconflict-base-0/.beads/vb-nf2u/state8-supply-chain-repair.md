STATUS: PASS

# State 8 supply-chain repair for `vb-nf2u`

## Root cause

`velvet-ballistics:supply-chain` ran `cargo vet --store-path supply-chain --locked` after `cargo audit` and `cargo deny` passed. The cargo-vet store directory had `config.toml` and `audits.toml`, but was missing the generated `imports.lock` file. With `--locked`, cargo-vet attempted to open `supply-chain/imports.lock` while acquiring the store and failed with `No such file or directory (os error 2)`. Strace evidence from `/tmp/cargo-vet-strace.log` showed `openat(..., "supply-chain/imports.lock", ...) = -1 ENOENT`.

## Files changed

- Added `supply-chain/imports.lock` with the cargo-vet empty lock-file header required by `cargo vet fmt` / `cargo vet --locked`.

## Commands run

- `moon run velvet-ballistics:supply-chain` — FAIL before repair; reproduced `ERROR × Couldn't acquire the store` / `No such file or directory (os error 2)`.
- `rustup run nightly-2026-04-28 cargo vet --help` — PASS; confirmed `--store-path` and `--locked` semantics.
- `rustup run nightly-2026-04-28 cargo vet --store-path supply-chain --locked --verbose trace` — FAIL before repair; trace still reported store acquisition failure.
- `rustup run nightly-2026-04-28 cargo vet --store-path supply-chain --locked` — PASS after adding and formatting `supply-chain/imports.lock`; output: `Vetting Succeeded (403 exempted)`.
- `moon run velvet-ballistics:supply-chain` — PASS after repair; `Tasks: 1 completed`.
- `moon ci --base HEAD --head HEAD` — FAIL observation only; `Tasks: 12 completed (2 cached), 3 failed, 5 skipped`.

## Scope statement

Only the supply-chain gate/store acquisition failure was repaired. Later `lint-src`, `miri`, and other non-supply-chain CI failures were not repaired in this pass.

## Residual next failure

Full CI was run only to observe the next failures. It remained non-green with 3 failed tasks after supply-chain passed. The captured output indicates later failures include the pre-existing `velvet-ballistics:lint-src` and `velvet-ballistics:miri` categories, which are intentionally out of scope for this repair pass.

## Performance layer

No performance claim made. No benchmark/profiler evidence required.
