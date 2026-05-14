STATUS: APPROVED

# Red Queen Adversarial QA Report — vb-nf2u

Workspace: `/home/lewis/src/Velvet-ballistics-vb-nf2u-go`
Scope: UI release gate behavior for `cargo xtask ai-release --bead vb-nf2u`.

## Commands run

1. Positive release boundary:
   - Command: `cargo xtask ai-release --bead vb-nf2u`
   - Result: PASS, exit `0`.
   - Observed summary: profile `ai-release`; six UI release gates emitted with exit `0`: `ui_snapshot`, `layout_readability`, `redaction`, `negative_fixture`, `deterministic_capture`, `evidence_shape`.

2. Evidence hygiene scan:
   - Command: Python scanner over `.evidence/vb-nf2u` after the positive boundary run.
   - Result: PASS.
   - Note: an initial scanner harness invocation failed from a path-concatenation bug in the reviewer script, not product behavior; the corrected scanner above passed.
   - Verified required files non-empty: `ai-release.yaml`, `ui_snapshots/ui_snapshot_report.yaml`, `negative-fixtures.txt`, `determinism.txt`, `animation-freeze.txt`.
   - Verified raw denied values absent: sentinel, API key, token, password, idempotency key, tainted fixture value.
   - Verified fixture/no-parity/determinism markers present: `fixture_backed: true`, `core_runtime_parity_claim: unsupported`, fixed timestamp, hidden animations paused.
   - Verified no live parity overclaim markers found.

3. Overlap false-pass hostile fixture:
   - Setup: wrote `target/vb-nf2u-negative-fixtures/intentional_overlap_fixture.txt` with `actual_status=passed` and nonce `adversarial_overlap_false_pass`.
   - Command: `cargo xtask ai-release --bead vb-nf2u`.
   - Result: PASS, release failed closed as expected with exit `1`.
   - Observed evidence: `FalsePassFixtureViolation`, `fixture_id: intentional_overlap_fixture`, `expected_gate: layout`, `actual_status: passed`.

4. Secret false-pass hostile fixture:
   - Setup: wrote `target/vb-nf2u-negative-fixtures/intentional_secret_fixture.txt` with `actual_status=passed`, nonce `adversarial_secret_false_pass`, and raw hostile values.
   - Command: `cargo xtask ai-release --bead vb-nf2u`.
   - Result: PASS, release failed closed as expected with exit `1`.
   - Observed evidence: `FalsePassFixtureViolation`, `fixture_id: intentional_secret_fixture`, `expected_gate: redaction`, `actual_status: passed`.
   - Evidence scan confirmed the raw hostile values did not leak into generated evidence.

5. Overlap negative fixture consumption probe:
   - Setup: wrote changed overlap fixture fields: `adversarial_run`, `adversarial_stop`, `overlap_area_px=77`, nonce `adversarial_overlap_consumed`, custom bounds.
   - Command: `cargo xtask ai-release --bead vb-nf2u`.
   - Result: PASS, exit `0`.
   - Observed evidence reflected hostile input fields and omitted stale canned defaults (`run_button`, `stop_button`, `overlap_area_px: 600`).

6. Acceptance suite:
   - Command: `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance`.
   - Result: PASS, `8 tests run: 8 passed, 0 skipped`.

## Findings

- No blocker found. The required command boundary succeeds for clean fixture-backed evidence.
- False-pass overlap and secret fixtures fail closed through the public command boundary.
- Negative overlap fixture evidence consumes hostile fixture fields rather than only emitting stale canned text.
- Raw denied values were not observed in generated evidence during positive or hostile secret false-pass probes.
- Evidence explicitly remains fixture-backed and does not claim live Makepad/core/runtime parity.

## Residual risks

- This review did not rerun full `moon ci`, mutation, coverage, Kani, Miri, or fuzz sanitizer lanes; `.beads/vb-nf2u/moon-report.md` contains prior machine-gate evidence.
- Evidence remains fixture-backed while `blocked-by-core` / `ui-paused` applies; approval is for honest fixture-backed release gating, not live Makepad/core parity.
