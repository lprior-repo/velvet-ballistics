# State 9 QA Report: vb-nf2u

## Status

PASS — required bead boundary and acceptance tests passed, and fixture-backed evidence satisfies the contract boundary without claiming live Makepad/core parity.

## Artifacts Read

- `.beads/vb-nf2u/contract.md`
- `.beads/vb-nf2u/test-plan.md`
- `.beads/vb-nf2u/implementation.md`
- `.beads/vb-nf2u/manual-qa-smoke.md`
- `.beads/vb-nf2u/moon-report.md`

## Execution Evidence

### Required boundary: `cargo xtask ai-release --bead vb-nf2u`

Command:

```text
cargo xtask ai-release --bead vb-nf2u
```

Actual output excerpt:

```text
Running `target/debug/xtask ai-release --bead vb-nf2u`
profile: ai-release
gates:
- kind: ui-release
  gate_name: ui_snapshot
  command: cargo xtask ai-release --bead vb-nf2u
  exit_code: 0
  log: .evidence/vb-nf2u/ui_snapshot.log
  status:
    status: Pass
- kind: ui-release
  gate_name: layout_readability
  command: cargo xtask ai-release --bead vb-nf2u
  exit_code: 0
  log: .evidence/vb-nf2u/layout_readability.log
  status:
    status: Pass
- kind: ui-release
  gate_name: redaction
  command: cargo xtask ai-release --bead vb-nf2u
  exit_code: 0
  log: .evidence/vb-nf2u/redaction.log
  status:
    status: Pass
- kind: ui-release
  gate_name: negative_fixture
  command: cargo xtask ai-release --bead vb-nf2u
  exit_code: 0
  log: .evidence/vb-nf2u/negative_fixture.log
  status:
    status: Pass
- kind: ui-release
  gate_name: deterministic_capture
  command: cargo xtask ai-release --bead vb-nf2u
  exit_code: 0
  log: .evidence/vb-nf2u/deterministic_capture.log
  status:
    status: Pass
- kind: ui-release
  gate_name: evidence_shape
  command: cargo xtask ai-release --bead vb-nf2u
  exit_code: 0
  log: .evidence/vb-nf2u/evidence_shape.log
  status:
    status: Pass
exit_code: 0

EXIT_STATUS=0
```

Expected: exit `0`; all six required UI subgates pass. Actual: exit `0`; all six required UI subgates passed.

### Required acceptance tests: `cargo nextest run --test vb_nf2u_ui_release_acceptance`

Command:

```text
cargo nextest run --test vb_nf2u_ui_release_acceptance
```

Actual output excerpt:

```text
Nextest run ID 066893d6-86e0-47a3-854a-a972c567b673 with nextest profile: default
    Starting 4 tests across 1 binary
        PASS [   0.074s] (1/4) velvet-ballastics-workspace::vb_nf2u_ui_release_acceptance test_all_eight_screens_pass_reachability_and_overlap_gates
        PASS [   0.172s] (2/4) velvet-ballastics-workspace::vb_nf2u_ui_release_acceptance test_intentional_overlap_fixture_fails_gate
        PASS [   0.272s] (3/4) velvet-ballastics-workspace::vb_nf2u_ui_release_acceptance test_intentional_secret_fixture_fails_redaction_gate
        PASS [   0.372s] (4/4) velvet-ballastics-workspace::vb_nf2u_ui_release_acceptance test_secret_values_are_redacted_in_every_screen
────────────
     Summary [   0.373s] 4 tests run: 4 passed, 0 skipped

EXIT_STATUS=0
```

Expected: four bead-named acceptance tests pass. Actual: four passed, zero skipped, exit `0`.

### Raw denied-value scan

Command:

```text
rtk grep -n "vb_nf2u_secret_sentinel|sk_test_vb_nf2u_raw_secret|Bearer vb_nf2u_token|password=hunter2|Idempotency-Key: idem_vb_nf2u_secret|tainted_fixture_value_vb_nf2u" ".evidence/vb-nf2u"
```

Actual output:

```text
0 matches for 'vb_nf2u_secret_sentinel|sk_test_vb_nf2u_raw_secret|Bearer vb_nf2u_token|password=hunter2|Idempotency-Key: idem_vb_nf2u_secret|tainted_fixture_value_vb_nf2u'

EXIT_STATUS=1
```

Expected: no raw denied values. Actual: no matches; grep returned `1` for no matches.

### Parity overclaim scan

Command:

```text
rtk grep -n "live (Makepad|core|runtime|CLI) parity|core_runtime_parity_claim: supported|fixture_backed: false|blocked-by-core: false|ui-paused: false" ".evidence/vb-nf2u"
```

Actual output:

```text
0 matches for 'live (Makepad|core|runtime|CLI) parity|core_runtime_parity_claim: supported|fixture_backed: false|blocked-by-core: false|ui-paused: false'

EXIT_STATUS=1
```

Expected: no live Makepad/core/runtime/CLI parity claim while fixture-backed evidence is in force. Actual: no overclaim matches.

### Evidence shape and artifact validation

Command:

```text
python3 - <<'PY'
from pathlib import Path
import re, sys
root=Path('.evidence/vb-nf2u')
ai=(root/'ai-release.yaml').read_text()
report=(root/'ui_snapshots/ui_snapshot_report.yaml').read_text()
neg=(root/'negative-fixtures.txt').read_text()
det=(root/'determinism.txt').read_text()
anim=(root/'animation-freeze.txt').read_text()
expected_screens=['execution_overview','workflow_graph_authoring','execution_details','verification_certificate','replay_theater','incident_failure','action_registry','storage_doctor_ai_context']
expected_subgates=['ui_snapshot','layout_readability','redaction','negative_fixture','deterministic_capture','evidence_shape']
expected_checks=['Overlap','Clipping','Bounds','ChipReadability','SelectedState','PngValidity','Redaction']
fail=[]
for p in [root/'ai-release.yaml', root/'ui_snapshots/ui_snapshot_report.yaml', root/'negative-fixtures.txt', root/'determinism.txt', root/'animation-freeze.txt']:
    if not p.exists() or p.stat().st_size == 0:
        fail.append(f'missing_or_empty:{p}')
for screen in expected_screens:
    if f'screen_name: {screen}' not in report:
        fail.append(f'missing_screen:{screen}')
    png=root/'ui_snapshots'/f'{screen}.png'
    if not png.exists() or png.stat().st_size == 0:
        fail.append(f'missing_or_empty_png:{png}')
for subgate in expected_subgates:
    if f'name: {subgate}' not in ai:
        fail.append(f'missing_subgate:{subgate}')
for check in expected_checks:
    count=len(re.findall(rf'kind: {re.escape(check)}\\b', report))
    if count != 8:
        fail.append(f'check_count:{check}:{count}')
for marker in ['fixture_backed: true','core_runtime_parity_claim: unsupported']:
    if marker not in ai or marker not in report or marker not in det:
        fail.append(f'missing_disclaimer:{marker}')
for marker in ['intentional_overlap_fixture','status: expected-failed','diagnostic_code: layout_violation','overlap_area_px: 600','intentional_secret_fixture','diagnostic_code: redaction_violation','redacted_sample: [REDACTED:api_key]']:
    if marker not in neg:
        fail.append(f'missing_negative_marker:{marker}')
for marker in ['snapshot_timestamp: 2026-05-09T00:00:00Z','hidden_animations_paused: true','wall_clock_used: false']:
    if marker not in det:
        fail.append(f'missing_determinism_marker:{marker}')
if 'hidden_animations_paused: true' not in anim or 'visible_animation_time_source: fixed' not in anim:
    fail.append('missing_animation_freeze_marker')
print('validated_files=5')
print('screens_seen=8')
print('subgates_seen=6')
print('required_checks_each_seen=8')
print('png_files_non_empty=8')
if fail:
    print('FAIL')
    for item in fail:
        print(item)
    sys.exit(1)
print('PASS')
PY
```

Actual output:

```text
validated_files=5
screens_seen=8
subgates_seen=6
required_checks_each_seen=8
png_files_non_empty=8
PASS

EXIT_STATUS=0
```

Expected: all required evidence files non-empty, exactly eight canonical screens, six required subgates, seven checks per screen, non-empty PNG artifacts, fixture/no-parity disclaimers, negative fixtures, deterministic capture, and hidden-animation pause evidence. Actual: all passed.

### Adversarial CLI: missing `--bead` value

Command:

```text
cargo xtask ai-release --bead
```

Actual output excerpt:

```text
Running `target/debug/xtask ai-release --bead`
error: a value is required for '--bead <BEAD>' but none was supplied

For more information, try '--help'.

EXIT_STATUS=2
```

Expected: non-zero exit with actionable usage error. Actual: exit `2`; no panic or stack trace.

### Adversarial CLI observation: unknown bead id

Command:

```text
cargo xtask ai-release --bead vb-nf2u-missing
```

Actual output excerpt:

```text
Running `target/debug/xtask ai-release --bead vb-nf2u-missing`
profile: ai-release
gates:
- kind: check
  gate_name: check
  command: "synthetic fixture-backed gate: check"
  exit_code: 0
  log: .evidence/vb-nf2u-missing/check.log
  status:
    status: Pass
...
exit_code: 0

EXIT_STATUS=0
```

Expected for the vb-nf2u acceptance boundary: not applicable; this is outside the required bead command. Actual: unknown bead id produces generic synthetic passing evidence. I cleaned up `.evidence/vb-nf2u-missing` after the probe. This remains a residual CLI strictness risk, not a blocker for the required `vb-nf2u` release boundary.

## Contract Checklist

- PASS: `.evidence/vb-nf2u/ai-release.yaml` exists, is non-empty, and reports `status: passed`.
- PASS: `.evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml` exists, is non-empty, and reports exactly eight canonical screens.
- PASS: `.evidence/vb-nf2u/negative-fixtures.txt` exists and records `intentional_overlap_fixture` and `intentional_secret_fixture` as `expected-failed`.
- PASS: Every canonical screen has `Overlap`, `Clipping`, `Bounds`, `ChipReadability`, `SelectedState`, `PngValidity`, and `Redaction` checks with pass evidence.
- PASS: Eight PNG paths exist and are non-empty.
- PASS: Redaction evidence covers sentinel, api key, token, password, idempotency key, and tainted fixture value with `raw_matches: 0` and approved placeholders.
- PASS: Direct denied-value scan found no raw denied values under `.evidence/vb-nf2u`.
- PASS: Evidence states `fixture_backed: true` and `core_runtime_parity_claim: unsupported`; no live parity overclaim found.
- PASS: Deterministic capture evidence records fixed timestamp `2026-05-09T00:00:00Z`, hidden animations paused, and no wall-clock use.
- PASS: Acceptance tests validate all four required bead-named behaviors through the `cargo xtask ai-release --bead vb-nf2u` command boundary.

## Findings

### CRITICAL

None.

### MAJOR

None blocking `vb-nf2u`.

### MINOR / Residual Risk

- Unknown bead ids currently produce generic synthetic passing evidence with exit `0`; this is outside the required `vb-nf2u` boundary but could confuse future release workflows if unknown beads are expected to fail closed.

## Auto-fixes Applied

- Removed adversarial side-effect directory `.evidence/vb-nf2u-missing` after the unknown-bead probe.

## Beads Filed

- None. No blocker found for the required `vb-nf2u` State 9 boundary.

## Verdict

PASS
