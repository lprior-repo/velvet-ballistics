# V19 RunAdmission policy_pin Fixture Repair

## Root Cause
After adding `policy_pin: vb_storage::RunPolicyPin` to `JournalEvent::RunAdmission`,
the Rust compiler discovered NINE legacy test fixtures that had not been updated to
include the mandatory `policy_pin` field.

## Fix Applied

### Historical/Hardcoded Fixtures (8 locations, 5 files)
Policy: `policy_pin: vb_storage::RunPolicyPin::Unverified`
Rationale: These fixtures are synthetic/historical journal records that never went
through real admission. They cannot bind a real accepted policy identity, so
`Unverified` correctly represents the compatibility state.

| File | Line(s) | Test Function/Context |
|------|---------|----------------------|
| `crates/workspace_tests/tests/derived_status_replay_timeline_tests.rs` | 738 | `replay_timeline_includes_digest_or_record_kind_per_event` |
| `crates/workspace_tests/tests/runtime_fjall_pending_action_recovery.rs` | 352 | `write_pending_action_events_with_input` |
| `crates/workspace_tests/tests/runtime_fjall_pending_action_recovery.rs` | 429 | `write_wait_boundary_events` |
| `crates/workspace_tests/tests/runtime_fjall_pending_action_recovery.rs` | 465 | `write_ask_boundary_events` |
| `crates/workspace_tests/tests/runtime_fjall_pending_action_recovery.rs` | 505 | `write_ask_boundary_events_with_prompt` |
| `crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs` | 173 | `run_admission_event` helper |
| `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` | 524 | `journal_events` helper |
| `crates/velvet_ballistics/src/commands_journal/tests.rs` | 375 | `trace_one_run_admission_maps_correct_fields` |

### Actual CLI Exporter Test (1 location, 1 file)
Policy: `policy_pin: vb_storage::RunPolicyPin::Pinned(artifact.policy_digest)`
Rationale: This is NOT a historical/hardcoded fixture. `test_submit_controls` runs
the real CLI submit operation, the run is genuinely admitted, and the resulting
`JournalEvent::RunAdmission` in the journal MUST contain the actual retained
accepted policy digest. The test loads the real `AcceptedArtifact` from the journal
via `vb_storage::AcceptedArtifact::from_bytes(&record.ir)` and uses
`artifact.policy_digest` as the pin — proving the exporter records the authentic
persisted policy identity, not `Unverified`, not an artifact digest, not zero.

| File | Line(s) | Test Function/Context |
|------|---------|----------------------|
| `crates/workspace_tests/tests/cli/submit_input_boundary.rs` | 216 | `test_submit_controls` |

## Verification

1. `cargo fmt --all` — all source reformatted
2. `python3 /tmp/vb-run-gate.py 300 .beads/vb-ny068/evidence/LOCAL_V19_FIXTURES_FMT cargo fmt --all --check` — exit 0, formatting verified
3. All affected test suites compile and pass:
   - `cargo test --test submit_input_boundary` — 6 passed (CLI exporter with actual policy pin)
   - `cargo test --test derived_status_replay_timeline_tests` — 18 passed (fixture fixture)
   - `cargo test --test runtime_fjall_pending_action_recovery` — 20 passed (4 historical fixtures)
   - `cargo test --test vb_core_yaml_e2e_chain_contract` — 35 passed (fixture fixture)
   - `cargo test --test vb_kyyf_cross_run_determinism` — 16 passed (fixture fixture)
   - `cargo test -p velvet-ballistics --lib` — 220 passed (fixture fixture)

## Why CLI Expected Pin Differs

Historical test fixtures construct journal events by hand for testing purposes.
They never run through the real admission pipeline, so there is no genuine
accepted artifact to bind. These use `Unverified`.

The CLI exporter test `test_submit_controls` runs the real `velvet-ballistics`
binary via subprocess, which compiles the workflow, admits it through the real
admission pipeline, and journals the resulting `RunAdmission` event. The test
then loads the journal, retrieves the actual `AcceptedArtifact` that was
persisted, and asserts the exported `RunAdmission` event contains
`policy_pin: Pinned(artifact.policy_digest)` — the actual retained accepted
policy identity. This is the only place in the test suite where a real
submission is made, which is why this is the only fixture that uses `Pinned`.
