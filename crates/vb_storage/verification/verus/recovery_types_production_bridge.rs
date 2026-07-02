// SPDX-License-Identifier: MIT
//
// ============================================================================
// VB-STORAGE-RECOVERY-TYPES — production-bound Verus bridge (REDIR)
// ============================================================================
//
// This file is a thin redirect to the canonical Verus bridge at
// `verification/verus/recovery_types_production_bridge.rs`. The
// canonical bridge uses the WEAK production-binding discipline
// (`#[path = "production_inner/recovery_types_production.rs"]`)
// mandated by AGENTS.md / `proof-writer` God-Rule-2 and is the
// single source of truth for the production-bound proofs of:
//   - `RecoveryTerminalState`
//   - `RecoveryRuntimeSummary`
//   - `RecoveryHydration`
//   - `RecoveredStepState`
//   - `UnsupportedRecoveryState`
//
// All 35 spec proofs + 9 production-bound exec wrappers are
// defined in the canonical file.
//
// ============================================================================
// WHY THIS REDIRECT EXISTS
// ============================================================================
//
// The parent black-hat handoff note for `vb-god2f.3` (see
// `vb-god2f` NOTES, paragraph beginning *"2026-06-30 PLANNING
// COMPLETE"*) required retiring the orphan vacuum spec at this
// path:
//
//   "vb-god2f.3 execution MUST retire
//    crates/vb_storage/verification/verus/recovery_types_spec.rs
//    mirror-model file before close (delete or annotate +
//    ALLOWED_EXCEPTIONS)."
//
// The vacuum spec has been DELETED (commit 44cdbb1e + vb-fodzb
// follow-up). The replacement bridge is the canonical file
// listed above. This redirect file is retained so the
// crate-local path continues to exist as a stable entrypoint
// for future Verus work in the `vb_storage` crate.
//
// ============================================================================
// PRODUCTION-BINDING AUDIT
// ============================================================================
//
// `bash scripts/check-verus-production-binding.sh` audits this
// file as WEAK (extern pattern): the canonical bridge is
// referenced via `#[path = "../../../verification/verus/recovery_types_production_bridge.rs"]`
// and that file in turn binds via `production_inner/...`. Both
// the bridge and the production_inner mirror are drift-detected
// against `crates/vb_storage/src/recovery/types.rs:529-621,652-726`.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

#[path = "../../../verification/verus/recovery_types_production_bridge.rs"]
mod canonical_bridge;

// Re-export the canonical bridge's verified symbols so this
// crate-local entrypoint exposes the same proof surface. The
// `canonical_bridge::*` re-export keeps the production-binding
// gate's STRONG/WEAK/VACUUM accounting consistent: the gate sees
// ONE `proof fn` source (the canonical bridge) reachable from
// both this redirect and the top-level path.
pub use canonical_bridge::*;