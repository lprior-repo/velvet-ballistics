STATUS: PASS

# Manual QA Smoke: vb-nf2u

## Scope

- Bead: `vb-nf2u`
- Boundary: `cargo xtask ai-release --bead vb-nf2u`
- Role: hands-on QA smoke only; no code or test changes.
- Residual boundary: evidence is fixture-backed/synthetic and does not claim live Makepad/core runtime parity while `blocked-by-core` / `ui-paused` remain relevant.

## Commands Run

### Context Read

- Read `.beads/vb-nf2u/STATE.md`: exit status N/A, file read succeeded.
- Read `.beads/vb-nf2u/implementation.md`: exit status N/A, file read succeeded.
- Read `tests/vb_nf2u_ui_release_acceptance.rs`: exit status N/A, file read succeeded.

### Beads Context

Command:

```text
bd prime
```

Exit status: `0`

Key output excerpt:

```text
# Beads Workflow Context
> **Context Recovery**: Run `bd prime` after compaction, clear, or new session
```

### Happy Path Boundary

Command:

```text
cargo xtask ai-release --bead vb-nf2u
```

Exit status: `0`

Key output excerpt:

```text
Running `target/debug/xtask ai-release --bead vb-nf2u`
AiRelease profile complete: ProfileEvidence { profile: "ai-release", gates: [GateEvidence { kind: "ui-release", gate_name: "ui_snapshot", command: "cargo xtask ai-release --bead vb-nf2u", exit_code: 0, log: ".evidence/vb-nf2u/ui_snapshot.log", status: Pass, why_failed: None }, GateEvidence { kind: "ui-release", gate_name: "layout_readability", command: "cargo xtask ai-release --bead vb-nf2u", exit_code: 0, log: ".evidence/vb-nf2u/layout_readability.log", status: Pass, why_failed: None }, GateEvidence { kind: "ui-release", gate_name: "redaction", command: "cargo xtask ai-release --bead vb-nf2u", exit_code: 0, log: ".evidence/vb-nf2u/redaction.log", status: Pass, why_failed: None }, GateEvidence { kind: "ui-release", gate_name: "negative_fixture", command: "cargo xtask ai-release --bead vb-nf2u", exit_code: 0, log: ".evidence/vb-nf2u/negative_fixture.log", status: Pass, why_failed: None }, GateEvidence { kind: "ui-release", gate_name: "deterministic_capture", command: "cargo xtask ai-release --bead vb-nf2u", exit_code: 0, log: ".evidence/vb-nf2u/deterministic_capture.log", status: Pass, why_failed: None }, GateEvidence { kind: "ui-release", gate_name: "evidence_shape", command: "cargo xtask ai-release --bead vb-nf2u", exit_code: 0, log: ".evidence/vb-nf2u/evidence_shape.log", status: Pass, why_failed: None }], exit_code: 0 }
```

### Variant Smoke: Unknown Bead Id

Command:

```text
cargo xtask ai-release --bead vb-nf2u-missing
```

Exit status: `0`

Key output excerpt:

```text
Running `target/debug/xtask ai-release --bead vb-nf2u-missing`
AiRelease profile complete: ProfileEvidence { profile: "ai-release", gates: [GateEvidence { kind: "check", gate_name: "check", command: "synthetic fixture-backed gate: check", exit_code: 0, log: ".evidence/vb-nf2u-missing/check.log", status: Skipped { reason: "fixture-backed synthetic evidence; no live runtime/core parity claimed" }, why_failed: None }
EXIT_STATUS=0
```

Observation: arbitrary bead ids route to generic fixture-backed skipped evidence instead of failing. This is non-blocking for `vb-nf2u` smoke because the required bead boundary passed and preserved no-core-parity disclaimers.

### Failure Smoke: Missing Bead Value

Command:

```text
cargo xtask ai-release --bead
```

Exit status: `2`

Key output excerpt:

```text
Running `target/debug/xtask ai-release --bead`
error: a value is required for '--bead <BEAD>' but none was supplied

For more information, try '--help'.
EXIT_STATUS=2
```

## Evidence Files Inspected

- `.evidence/vb-nf2u/ai-release.yaml`
- `.evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml`
- `.evidence/vb-nf2u/negative-fixtures.txt`
- `.evidence/vb-nf2u/determinism.txt`
- `.evidence/vb-nf2u/animation-freeze.txt`

## Evidence Excerpts

### Release Gate Shape

```text
profile: ai-release
bead_id: vb-nf2u
status: passed
fixture_backed: true
core_runtime_parity_claim: unsupported
subgates:
  - name: ui_snapshot
  - name: layout_readability
  - name: redaction
  - name: negative_fixture
  - name: deterministic_capture
  - name: evidence_shape
```

### Snapshot Screen Coverage

```text
status: pass
total_screens: 8
passed_screens: 8
failed_screens: 0
fixture_backed: true
core_runtime_parity_claim: unsupported
screen_name: execution_overview
screen_name: workflow_graph_authoring
screen_name: execution_details
screen_name: verification_certificate
screen_name: replay_theater
screen_name: incident_failure
screen_name: action_registry
screen_name: storage_doctor_ai_context
```

### Layout/Readability/Redaction Checks

```text
kind: Overlap
kind: Clipping
kind: Bounds
kind: ChipReadability
kind: SelectedState
kind: PngValidity
kind: Redaction
passed: true
diagnostics: []
```

### Redaction Evidence

```text
placeholder: [REDACTED:sentinel]
placeholder: [REDACTED:api_key]
placeholder: [REDACTED:token]
placeholder: [REDACTED:password]
placeholder: [REDACTED:idempotency_key]
placeholder: [REDACTED:tainted_fixture_value]
raw_matches: 0
approved_placeholders_seen: 1
```

Raw denied-value scan result:

```text
No files found
```

Pattern scanned:

```text
vb_nf2u_secret_sentinel|sk_test_vb_nf2u_raw_secret|Bearer vb_nf2u_token|password=hunter2|Idempotency-Key: idem_vb_nf2u_secret|tainted_fixture_value_vb_nf2u
```

### Negative Fixtures

```text
negative_fixtures:
  - fixture_id: intentional_overlap_fixture
    status: expected-failed
    gate: layout_readability
    diagnostic_code: layout_violation
    screen_id: execution_overview
    control_id: run_button
    second_control_id: stop_button
    predicate: overlap
    overlap_area_px: 600
    action: keep release gate failing on overlapping controls
  - fixture_id: intentional_secret_fixture
    status: expected-failed
    gate: redaction
    diagnostic_code: redaction_violation
    screen_id: storage_doctor_ai_context
    artifact_path: target/vb-nf2u-negative-fixtures/intentional_secret_fixture.txt
    secret_class: api_key
    redacted_sample: [REDACTED:api_key]
    action: keep release gate failing on raw secret exposure
```

### Determinism And Animation Freeze

```text
deterministic_capture: passed
snapshot_timestamp: 2026-05-09T00:00:00Z
hidden_animations_paused: true
wall_clock_used: false
fixture_backed: true
core_runtime_parity_claim: unsupported
```

```text
hidden_animations_paused: true
visible_animation_time_source: fixed
execution_marker: vb-nf2u-animation-freeze
```

## PASS/FAIL Checklist

- PASS: Required boundary `cargo xtask ai-release --bead vb-nf2u` exits `0`.
- PASS: `.evidence/vb-nf2u/ai-release.yaml` exists, is non-empty, and reports `status: passed`.
- PASS: All six required UI subgates are present: `ui_snapshot`, `layout_readability`, `redaction`, `negative_fixture`, `deterministic_capture`, `evidence_shape`.
- PASS: Snapshot report includes all eight canonical screens.
- PASS: Snapshot report includes `Overlap`, `Clipping`, `Bounds`, `ChipReadability`, `SelectedState`, `PngValidity`, and `Redaction` checks.
- PASS: Layout/readability/redaction markers include `passed: true`, `diagnostics: []`, and execution markers.
- PASS: Evidence contains approved placeholders for sentinel, api key, token, password, idempotency key, and tainted fixture value.
- PASS: Direct raw denied-value scan over `.evidence/vb-nf2u` found no raw denied values.
- PASS: Evidence explicitly marks `fixture_backed: true` and `core_runtime_parity_claim: unsupported`.
- PASS: Negative fixture evidence records intentional overlap and intentional secret fixtures as `expected-failed`.
- PASS: Determinism evidence reports fixed timestamp/no wall clock and animation-freeze evidence reports hidden animations paused/fixed time source.
- PASS: Missing `--bead` value fails cleanly with exit status `2` and a clap usage error.
- PASS: Report file was written for State 7 smoke handoff.

## Residual Risks

- Fixture-backed/synthetic evidence proves release evidence shape and redaction/overlap gate wiring, not live Makepad rendering.
- `core_runtime_parity_claim: unsupported` remains appropriate; this smoke does not prove live core/runtime parity.
- Unknown bead ids produce generic skipped fixture-backed evidence with exit status `0`; this does not block `vb-nf2u`, but it is a CLI UX/strictness observation if future release workflows require rejecting unknown bead ids.
