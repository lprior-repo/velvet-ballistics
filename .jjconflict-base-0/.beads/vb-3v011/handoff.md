# vb-3v011 handoff

## What was fixed

Real production integration bug: the CLI `run`/`run-compiled` producer path built a
**legacy** `AcceptedArtifact` (`VerificationProof::new` → `ArtifactEvidence::Unverified`,
false `*_claimed` flags), which the typed schema-v3 runtime verifier rejects
(`MigrationRequired{1→3}` → `AdmissionError::ArtifactInvalidProofFlag` →
`RuntimeError::AdmissionArtifactInvalid` → `"admission rejected: artifact invalid"`).

## Changes (production, `crates/velvet_ballistics/src/`)

- `run.rs`: `submit_cli_compiled_artifact` now delegates to `vb_storage::submit_artifact`
  (typed evidence, checksum-validated); `store_workflow_artifacts`/`cmd_run` thread `policy`.
- `run_compiled_runtime.rs`: `store_compiled_artifact`/`run_compiled_workflow` thread `policy`.
- `run_compiled.rs`: `runtime_config_for_durability` maps `Journaled → Journaled`
  (was: fell through to `Strict`), `Strict → Strict`, `None → Relaxed`.
- `submit.rs`: `submission_policy` is now `pub(crate)` (single durability→policy source).

## Changes (tests, my 3 assigned files)

- `admission_evidence_integration/chunk_001.rs` + `chunk_002.rs`: converted three
  `set_const_finish_workflow(<forged digest>)` submissions (which asserted obsolete
  "relaxed skips checksum" success) to genuine `vb_compile::compile_workflow` workflows via
  a new `compile_set_const_finish` helper, preserving persistence/lookup/restart intent.
- Added `forged_digest_workflow_rejects_across_relaxed_and_strict` tamper regression:
  placeholder-digest workflow must reject with `ArtifactChecksumMismatch` under both policies.

## Verification

Command (immutable runner):

```
python3 /tmp/vb-run-gate.py 600 .beads/vb-3v011/evidence/cli-regressions cargo test -p velvet-ballistics --test cli_integration --test finish_input_execution --test admission_evidence_integration
```

Result: `exit_code 0`, raw JSON at `.beads/vb-3v011/evidence/cli-regressions.json`.
Logs at `cli-regressions.stdout`/`.stderr`. Counts:

- `admission_evidence_integration`: 25 passed, 0 failed
- `cli_integration`: 123 passed, 0 failed
- `finish_input_execution`: 1 passed, 0 failed

Total 149 passed / 0 failed.

Additional gates:

- `cargo check --tests -p velvet-ballistics` — clean.
- `cargo clippy -p velvet-ballistics` (lib+bin) — clean; `--all-targets` only surfaces
  pre-existing `panic!` in `#[cfg(test)]` modules (`naming_scan/legacy.rs`, `status/tests.rs`),
  which per AGENTS.md test-clippy is not strict.

## Risks / notes

- `runtime_config_for_durability` behavior change is bounded to the `run`/`run-compiled`
  admission path (`bench_run` uses `None → Relaxed`, unchanged). No other caller.
- No public API changed; LOCAL5090 fixtures unaffected (see `integration-note.md`).
- Not committed/pushed/closed (per task: root owns VCS commits and pushes).

## Artifacts

- `.beads/vb-3v011/contract.md`
- `.beads/vb-3v011/integration-note.md`
- `.beads/vb-3v011/handoff.md`
- `.beads/vb-3v011/evidence/cli-regressions.{json,stdout,stderr}`
