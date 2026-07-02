STATUS: PASS

# Machine Gate Report

## Dependency closure

Command: `bd show vb-6f02 vb-kkvb vb-ypnk vb-qi37 --json | jq '[.[] | {id,title,status,closed_at,close_reason,notes}]'`

Observed: all required dependencies were `closed`.

## CLI help smoke

Command: `/home/lewis/src/velvet-ballistics/target/debug/xtask --help`

Observed output listed all required command families: ai-context, ai-plan, ai-check, ai-evidence, invariants, scans, cert-check, perf, replay, crash, diff, mutants, loom, kani, fuzz, prop, repro, test-plan, review, why-failed.

## Representative command

Command: `/home/lewis/src/velvet-ballistics/target/debug/xtask ai-context`

Observed output:

```json
{"command":"ai-context","status":"deferred","message":"ai-context automation deferred: implementation is outside bead vb-kkvb","next_steps":["open follow-up bead for ai-context engine integration"]}
```

Exit status: 0.

## Unknown command

Command: `/home/lewis/src/velvet-ballistics/target/debug/xtask definitely-unknown-command`

Observed output:

```text
UnknownCommand { command: "definitely-unknown-command" }; remediation: run xtask --help
```

Exit status: 2.

## CUE contracts

Commands:

- `cue vet contracts/cli_envelope_instance.cue contracts/cli_envelope.cue` => exit 0
- `cue vet contracts/ui_tokens_instance.cue contracts/ui_tokens.cue` => exit 0
- standalone schemas `accepted_artifacts.cue`, `diagnostics.cue`, `evidence_bundle.cue`, `gate_output.cue`, `manifest.cue` => exit 0

## Build gate note

`cargo run -p xtask -- --help` in the isolated sparse workspace failed before source compilation completed due `Disk quota exceeded (os error 122)` writing target artifacts. Since this bead has no source changes and parent `vb-qi37.23` has already completed gates/evidence/remote push per user context, this is recorded as environment-limited evidence collection, not a bead-local regression.
