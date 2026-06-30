// SPDX-License-Identifier: MIT
//
// Extern surface for `vb_cli_commands_journal_trace` Verus spec.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is a thin re-export surface for the production mirror at
// `verification/verus/production_inner/cli_commands_journal_trace_production.rs`,
// which is a structural mirror of `crates/vb_cli/src/commands_journal.rs`.
//
// The companion spec file (`vb_cli_commands_journal_trace.rs`)
// attaches spec contracts to the projections via
// `assume_specification`, and every proof below the bridge exercises
// the production wrappers through exec wrappers. There are zero
// vacuous proofs in the rewritten spec.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production mirror bodies are `#[verifier::external]` so Verus
// skips body verification; the production contract is attached via
// `assume_specification` in the companion spec file. Drift between
// the mirror and the production source is reported as binding-debt
// outside Verus.
//
// NOTE: this extern file is plain Rust (NOT wrapped in `verus!`)
// because the production mirror uses `write!`, `format!`, `Debug`
// derive, and closure patterns that Verus cannot model in spec mode.
// The companion spec file (`vb_cli_commands_journal_trace.rs`)
// references the types and fns through `production::*` after the
// `#[path]` inclusion below.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK BINDING)
// ============================================================================
#[path = "production_inner/cli_commands_journal_trace_production.rs"]
pub mod prod_src;

// Re-export everything from prod_src into the production module so
// the spec file's `production::Type` references resolve correctly.
pub use prod_src::*;