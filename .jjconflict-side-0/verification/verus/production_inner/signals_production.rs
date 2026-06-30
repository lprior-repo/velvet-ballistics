// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for signals.rs
// ============================================================================
//
// This file is a verbatim copy of `crates/vb_core/src/engine/signals.rs`
// with the following minimal substitutions:
//
//   1. `StepBudget::remaining` is declared `pub` instead of private
//      (production at signals.rs:15). Verus's `external_type_specification`
//      requires the production-side field to be visible to the
//      spec mirror; production keeps the field private as a Rust-API
//      hardening. This mirror relaxes only visibility; field NAME
//      and TYPE are preserved byte-for-byte. Any drift in field
//      NAME breaks the verification build.
//
//   2. `StepBudget::from_env` body is wrapped in
//      `#[verifier::external_body]` because the closure pattern
//      `|_| EngineError::BudgetParse { reason: ... }` (production at
//      signals.rs:84) is rejected by Verus 0.2026.05.05 (Rust 1.95.0)
//      as "only variables are supported here, not general patterns".
//      The body is opaque to Verus. The signature and field name
//      remain production-identical so any drift breaks this Verus build.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_core/src/engine/signals.rs` whenever production changes.
// The mirror is annotated at the top of every section with the
// originating production line range so regeneration is mechanical.
//
// This file is included by the companion extern file
// (`verification/verus/extern_signals_invariant.rs`) via `#[path]`
// inside `verus!` (without module-level `#[verifier::external]`) so
// the type declarations are nameable in spec mode. Each production
// method body that Verus cannot verify is marked
// `#[verifier::external_body]` so the body is opaque while the
// signature participates in `assume_specification` binding in the
// companion spec file `signals_invariant.rs`.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// Opaque payload stubs — mirrors of crates/vb_core/src/value.rs
// ============================================================================
//
// Production `EngineSignal::Finished(SlotValue, Taint)` carries payload
// types defined in `crates/vb_core/src/value.rs`. The signals-invariant
// spec only cares about the discriminant set (specifically the
// `StepCounterOverflow` variant used by `StepBudget::try_take`), so we
// mirror the relevant enum variants opaquely.

#[derive(Clone)]
pub enum SlotValue {
    Null,
    Bool(bool),
    I64(i64),
}

#[derive(Clone, Copy)]
pub enum Taint {
    Clean,
    DerivedFromSecret,
    Secret,
}

// ============================================================================
// EngineError stub — mirrors the production error variants used by signals.rs
// ============================================================================

#[derive(Clone)]
pub enum EngineError {
    StepCounterOverflow,
    BudgetParse {
        reason: &'static str,
    },
}

// ============================================================================
// Production type mirrors
// ============================================================================

/// Stub for the production `MAX_STEP_BUDGET` u64 constant
/// (production at `crates/vb_core/src/limits.rs:94 = 10_000`).
pub const MAX_STEP_BUDGET: u64 = 10_000;

/// Mirror of production `StepBudget` at
/// `crates/vb_core/src/engine/signals.rs:13-16`. Field name `remaining`
/// and type `u64` match production exactly; the only relaxation is
/// visibility (production: private; mirror: `pub`) so the spec-mode
/// `#[verifier::external_type_specification]` bridge can read the
/// field. Drift in field NAME still breaks the build.
///
/// The struct is marked `#[verifier::external]` so the type is
/// opaque to Verus until bridged via `external_type_specification`
/// in the companion spec file. The spec accesses the `remaining`
/// field through the bridge alias `ExStepBudget` declared in
/// `signals_invariant.rs`.
#[verifier::external]
#[derive(Debug, Clone)]
pub struct StepBudget {
    /// Mirror of production private field `StepBudget::remaining`.
    /// Visibility relaxed to `pub` so Verus's
    /// `external_type_specification` mirror can read it.
    pub remaining: u64,
}

impl StepBudget {
    /// Mirror of `StepBudget::MAX` at signals.rs:19-22.
    pub const MAX: Self = Self {
        remaining: MAX_STEP_BUDGET,
    };

    /// Mirror of `StepBudget::new(value: u64) -> Self` at
    /// signals.rs:26-35. Verbatim body; the spec contract is attached
    /// via `assume_specification` in the companion spec file.
    /// Function marked `#[verifier::external]` because the mirror
    /// is included inside `verus!` (so the type is usable in spec
    /// mode) but Verus should not attempt to verify the body — the
    /// body is bound to production via `assume_specification`.
    #[verifier::external]
    pub const fn new(value: u64) -> Self {
        Self {
            remaining: if value > MAX_STEP_BUDGET {
                MAX_STEP_BUDGET
            } else {
                value
            },
        }
    }

    /// Mirror of `StepBudget::try_take(&mut self) -> Result<bool, EngineError>`
    /// at signals.rs:50-60. Verbatim body; spec contract attached via
    /// `assume_specification`. Marked external for the same reason
    /// as `new`.
    #[verifier::external]
    pub fn try_take(&mut self) -> Result<bool, EngineError> {
        if self.remaining > MAX_STEP_BUDGET {
            return Err(EngineError::StepCounterOverflow);
        }
        if self.remaining == 0 {
            Ok(false)
        } else {
            self.remaining = self.remaining.saturating_sub(1);
            Ok(true)
        }
    }

    /// Mirror of `StepBudget::remaining(&self) -> u64` at
    /// signals.rs:62-65. Verbatim body; marked external for the
    /// same reason as `new`.
    #[verifier::external]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Mirror of `StepBudget::BENCH_LATENCY_BUDGET_US` at signals.rs:69.
    const BENCH_LATENCY_BUDGET_US: &'static str = "VB_BENCH_LATENCY_BUDGET_US";

    /// Mirror of `StepBudget::DEFAULT_BUDGET` at signals.rs:72.
    const DEFAULT_BUDGET: u64 = MAX_STEP_BUDGET;

    /// Mirror of `StepBudget::from_env() -> Result<Self, EngineError>`
    /// at signals.rs:80-94. Body marked `#[verifier::external_body]`
    /// because the closure pattern `|_| EngineError::BudgetParse { reason }`
    /// is rejected by Verus 0.2026.05.05 (Rust 1.95.0) as "only
    /// variables are supported here, not general patterns".
    /// Signature is production-identical so any drift breaks the
    /// verification build.
    #[verifier::external_body]
    pub fn from_env() -> Result<Self, EngineError> {
        match std::env::var(Self::BENCH_LATENCY_BUDGET_US) {
            Ok(raw) => {
                let parsed = raw.parse::<u64>().map_err(|_| EngineError::BudgetParse {
                    reason: "invalid u64 value",
                })?;
                Ok(Self::new(parsed))
            }
            Err(std::env::VarError::NotPresent) => Ok(Self::new(Self::DEFAULT_BUDGET)),
            Err(_) => Err(EngineError::BudgetParse {
                reason: "env var access error",
            }),
        }
    }
}

/// Mirror of `EngineSignal` at `crates/vb_core/src/engine/signals.rs:99-115`.
/// Discriminant set matches production exactly (7 variants).
#[derive(Clone)]
pub enum EngineSignal {
    Continue,
    Finished(SlotValue, Taint),
    StepBudgetExhausted,
    AwaitingAction,
    ActionFailureUnhandled,
    AwaitingWait,
    AwaitingAsk,
}

} // verus!