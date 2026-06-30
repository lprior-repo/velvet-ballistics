STATUS: PASS

# State 10 supply-chain repair for `vb-nf2u`

## Root cause

State 10 wired the `ui_redaction_artifact` cargo-fuzz target in `fuzz/Cargo.toml`. That preserved the requested fuzz coverage but added three resolved fuzz/build transitive dependencies to `Cargo.lock` without corresponding cargo-vet store metadata:

- `arbitrary:1.4.2`
- `jobserver:0.1.34`
- `libfuzzer-sys:0.4.12`

`moon run velvet-ballistics:supply-chain` runs `cargo vet --store-path supply-chain --locked`; cargo-vet therefore failed with `Vetting Failed!` and each of those dependencies missing `safe-to-deploy`.

## Files changed

- `supply-chain/config.toml`
  - Added cargo-vet exemptions for `arbitrary 1.4.2`, `jobserver 0.1.34`, and `libfuzzer-sys 0.4.12` with criterion `safe-to-deploy`.
  - This uses the repository's existing cargo-vet exemption mechanism and does not remove the fuzz target or weaken cargo-deny/cargo-vet command policy.

## Commands and results

1. `bd prime`
   - PASS for workflow context load.
   - Note: automatic Dolt push warning remained unrelated to this repair (`non-fast-forward`).

2. `moon run velvet-ballistics:supply-chain`
   - FAIL before repair.
   - Verbatim actionable summary: `Vetting Failed!`; `3 unvetted dependencies:`; `arbitrary:1.4.2 missing ["safe-to-deploy"]`; `jobserver:0.1.34 missing ["safe-to-deploy"]`; `libfuzzer-sys:0.4.12 missing ["safe-to-deploy"]`.

3. `rustup run nightly-2026-04-28 cargo vet --store-path supply-chain fmt && moon run velvet-ballistics:supply-chain`
   - FAIL before verification because cargo-vet rejected the option placement: `error: the subcommand 'fmt' cannot be used with '--store-path <STORE_PATH>'`.

4. `rustup run nightly-2026-04-28 cargo vet fmt --store-path supply-chain && moon run velvet-ballistics:supply-chain`
   - PASS.
   - Verbatim Moon summary: `Tasks: 1 completed`; `Time: 46s 417ms`.
   - Full captured output: `/home/lewis/.local/share/opencode/tool-output/tool_e104eaddb0016lvv3Lb9GADCuV`.

5. `moon ci --base HEAD --head HEAD`
   - PASS.
   - Verbatim Moon summary: `Tasks: 20 completed (2 cached)`; `Time: 8m 7s 204ms`.
   - Full captured output: `/home/lewis/.local/share/opencode/tool-output/tool_e104f1a2a0016by1qW9bqt3T1L`.

## Residual risks

- The repair records cargo-vet exemptions, not third-party audit proofs. This matches the existing store pattern for dependency acceptance but remains a supply-chain trust decision.
- `cargo audit` still prints allowed warnings for pre-existing advisory/dependency conditions; the supply-chain task treats them as warnings and completed successfully.
- `cargo geiger` remains reporting-only per `.moon/tasks/all.yml`; transitive unsafe remains visible but is governed by vet/deny policy.
- No performance claim was made.
