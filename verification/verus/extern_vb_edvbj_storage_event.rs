// SPDX-License-Identifier: MIT
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance) — extern companion
// ============================================================================
//
// Extern mirror for `verification/verus/vb_edvbj_storage_event.rs`.
//
// This file is the production-binding surface for the `storage_event`
// Verus spec (PO-EDVBJ-001). It includes the in-tree production
// mirror at
// `verification/verus/production_inner/vb_edvbj_storage_event_production.rs`
// via `#[path]` so that:
//   * The companion gate
//     `scripts/check-verus-production-binding.sh` classifies the spec
//     file via the extern chain (`spec → extern → production_inner`).
//   * Any drift in the production body (variant added/removed,
//     per-helper arm changed, fallback error type altered) breaks the
//     spec build at compile time.
//   * The drift-detection stubs in the production_inner mirror force
//     resolution of every mirror method and every 21-variant arm.
//
// The mirror at `production_inner/vb_edvbj_storage_event_production.rs`
// carries the verbatim reproduction of the post-fix body shape
// (chunk_002.rs:270-303) plus the per-layer helpers and
// `runtime_journal_event_kind`. The drift script
// `scripts/check-production-inner-drift.sh` validates that every
// identifier claimed in the mirror's per-section `// Production ...`
// annotations is present in the mirror body.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of `mirror_storage_event`,
// `mirror_run_storage_event`, `mirror_action_storage_event`,
// `mirror_boundary_storage_event`, and
// `mirror_runtime_journal_event_kind` are verified by Verus via the
// production contract attached through `assume_specification` in the
// spec file. The mirror bodies are plain Rust pattern matches (no
// quantifier reasoning), so Verus discharges them directly. Drift
// between this mirror and the production source is detected by the
// `prod_methods_drift_check` function in the production_inner mirror.
//
// Spec calls the mirror via `assume_specification` bridges declared in
// `vb_edvbj_storage_event.rs`; those bridges attach the production
// contract to the mirror's exec methods.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

#[path = "production_inner/vb_edvbj_storage_event_production.rs"]
pub mod production;

// Re-export the production types so the spec file can reference them
// via `crate::production::*`.
pub use production::{
    MirrorActionId, MirrorCapabilitySet, MirrorEventSeq, MirrorJournalEvent,
    MirrorRuntimeError, MirrorRuntimeJournalEvent, MirrorRuntimePolicy, MirrorRuntimeResult,
    MirrorRunAdmission, MirrorRunId, MirrorSlotIdx, MirrorStepIdx, MirrorTaint, MirrorWorkflowDigest,
    mirror_action_storage_event, mirror_boundary_storage_event, mirror_run_storage_event,
    mirror_runtime_journal_event_kind, mirror_storage_event,
};
