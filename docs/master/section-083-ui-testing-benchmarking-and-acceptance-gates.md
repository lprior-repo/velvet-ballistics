---
section: 83
title: "UI Testing, Benchmarking, and Acceptance Gates"
parent: velvet-ballistics-MASTER.md
---

## 83. UI Testing, Benchmarking, and Acceptance Gates


> **Removed.** Makepad UI is not part of the current core feature set. This section is historical residue only; no current backend bead may be blocked by UI testing, benchmarking, snapshot, token, or acceptance gate requirements.

### UI Tests

Required tests:

- `ui_model_schema_versions_are_stable`
- `ui_artifacts_match_cli_output_kinds`
- `workflow_graph_view_has_no_missing_nodes`
- `workflow_graph_edges_reference_valid_nodes`
- `event_rows_are_bounded`
- `ai_context_redacts_secrets`
- `incident_report_has_replay_safety`
- `verification_certificate_maps_all_gates`
- `action_ticket_hides_raw_idempotency_key`
- `ui_tokens_generate_makepad_and_contract_outputs`
- `all_screens_have_demo_fixtures`

### UI Snapshot Tests

Required deterministic snapshot fixtures:

```text
fixtures/ui/execution_overview.fixture
fixtures/ui/workflow_graph_authoring.fixture
fixtures/ui/execution_details.fixture
fixtures/ui/verification_certificate.fixture
fixtures/ui/replay_theater.fixture
fixtures/ui/incident_failure.fixture
fixtures/ui/action_registry.fixture
fixtures/ui/storage_doctor_ai_context.fixture
```

Snapshot command:

```bash
cargo xtask ui-snapshot --all --emit yaml
```

Snapshot report:

```yaml
kind: UiSnapshotReport
status: pass
screens:
  - screen: execution_overview
    png: tests/ui_snapshots/execution_overview.png
    overlap_check: pass
    clipping_check: pass
    spelling_check: pass
    token_check: pass
```

### UI Performance Benchmarks

Required UI benchmarks:

| Benchmark | Requirement |
|----------|-------------|
| `ui_graph_pan_zoom_256_nodes` | Smooth interaction, no unbounded allocation. |
| `ui_graph_packet_animation_512_packets` | Animation remains within frame budget. |
| `ui_timeline_2000_events_clustered` | Timeline remains responsive. |
| `ui_event_table_scroll_10000_bounded` | Virtualized/bounded rendering only. |
| `ui_replay_scrub_1000_events` | Scrub updates selected graph/event without full relayout. |
| `ui_fixture_load_all_screens` | Demo fixtures load under bounded memory. |

### UI Acceptance Commands

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy -p vb_ui_model -p vb_ui_makepad --all-targets --all-features -- -D warnings
cargo +nightly nextest run -p vb_ui_model -p vb_ui_makepad
cargo xtask ui-tokens --check
cargo xtask ui-snapshot --all
cargo xtask ui-overlap-check --all
cargo xtask ui-perf-smoke
cargo xtask forbidden-scan --changed
cargo xtask hotpath-scan --changed
```

### UI Definition of Done

The Makepad UI is accepted only when:

1. All eight required screens exist and are reachable from shared app chrome.
2. Every screen consumes typed `vb_ui_model` artifacts.
3. CLI/UI parity exists for all displayed artifact kinds.
4. Figma token source and Makepad token output are synchronized.
5. No UI panel overlap, clipping, or unreadable primary label exists in 1920x1080 baseline screenshots.
6. All secret-sensitive values are redacted or summarized.
7. Graph, replay, incident, and verification views expose journal/digest/evidence concepts accurately.
8. Motion is bounded, meaningful, and can be disabled or frozen for deterministic snapshots.
9. UI code does not introduce Makepad, HTTP, JSON, async runtimes, or web dependencies into runtime core crates.
10. UI snapshot, token, model, parity, redaction, performance-smoke, lint, and test gates pass with evidence.
