bead_id: vb-ahfl
bead_title: ui-model: Enforce artifact schema bounds and CLI parity
phase: State 2
updated_at: 2026-05-15T00:00:00Z

# Codebase Map

- Master contract: `velvet-ballistics-MASTER.md` is the authoritative source for this bead.
- Required master scope: lines 5316-5344 define `vb_ui_model` required top-level types and require bounded collections for all UI model structs.
- Required master schema: lines 5765-5775 require every UI artifact to carry `schema_version`, `kind`, `generated_at`, `source`, and `redaction_status`.
- Required graph schema: lines 5777-5800 require graph nodes to expose `step_idx`, `step_id`, `kind`, `status`, `output_slot`, `taint`, `badges`, and `position`; graph edges require `from_step_idx`, `to_step_idx`, `edge_kind`, `condition_summary`, `is_failure_path`, `is_taint_path`, and `packet_state`.
- Required event schema: lines 5802-5813 require event rows to expose `seq`, `timestamp`, `run_id`, `step_idx`, `event_kind`, `status`, `evidence_digest`, and `attempt`.
- Required redaction schema: lines 5830-5839 require secret-sensitive values to render only as `redacted`, `taint`, `digest`, and bounded `summary` fields.
- UI model crate: `crates/vb_ui_model/src/lib.rs` re-exports domain primitives, declares `UiScreenKind`, and `UiAppSnapshot`; it is cold-path plain data and currently has no shared universal artifact metadata type.
- Workflow model surface: `crates/vb_ui_model/src/workflow.rs` defines `WorkflowGraphView`, `WorkflowNodeView`, and `WorkflowEdgeView`; current gaps include unbounded `Vec` fields, skipped positional `Vec<f32>` fields, node `label` instead of required `step_id`, missing node `status`, `output_slot`, `taint`, `badges`, `position`, and missing required edge parity fields.
- Verification model surface: `crates/vb_ui_model/src/verify.rs` defines `VerificationReportView`, `VerificationCertificate`, and `GateResult`; current gaps include unbounded `warnings` and `gate_results`, and missing universal artifact metadata.
- Runtime inspection surface: `crates/vb_ui_model/src/run.rs` defines `RunSummaryView`, `RunInspectionView`, `StepStateView`, `SlotDiffView`, `RunEventsView`, and `RunEventView`; `RunEventsView` has limit/cursor fields but still uses unbounded `Vec`, and `RunEventView` names do not match required `step_idx`, `event_kind`, `status`, `evidence_digest`, and `attempt` fields.
- Incident model surface: `crates/vb_ui_model/src/incident.rs` defines `IncidentReportView` and `EvidenceChain`; current gaps include unbounded `repair_hints`, no universal artifact metadata, and no explicit redaction wrapper for secret-sensitive failure details.
- Makepad UI boundary: `crates/vb_ui_makepad/src/lib.rs` exports UI widget modules only; it does not currently pull `vb_ui_model` in this top-level file, so implementation must preserve Makepad isolation and avoid runtime-core dependency creep.
- CLI boundary: `crates/velvet_ballastics/src/main.rs` supports structured `--json|--jsonl` output for many commands and action/status/AI context surfaces, but this bead must add tests proving CLI-emitted artifacts and `vb_ui_model` artifacts canonicalize equivalently before implementation.
- Likely implementation files for later states: `crates/vb_ui_model/src/lib.rs`, `workflow.rs`, `verify.rs`, `run.rs`, `incident.rs`, `replay.rs`, `storage.rs`, `system.rs`, `ai.rs`, `emitter.rs`, and CLI parity tests under the relevant `crates/velvet_ballastics` or `crates/vb_ui_model` test surfaces.
- Out of scope for State 2: no production code edits, no test edits, no proof edits, no generated artifacts beyond this map and `delivery-scope.jsonl`.
