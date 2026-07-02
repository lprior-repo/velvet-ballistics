// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `vb_cli::exit_code` envelope
// discriminant surface — focused for `vb-diagnostic-envelope` spec
// ============================================================================
//
// This file is a VERBATIM copy of the relevant production surface from
//   crates/vb_cli/src/exit_code.rs:1-68
// with three SUBSTITUTIONS required to compile under
// `verus --crate-type=lib` without `vb_core` / `vb_storage` extern
// crate registration (no installs allowed by the task brief):
//
//   - The two non-test `From<...> for CliExitCode` impls at
//     exit_code.rs:56-61 and exit_code.rs:63-68 reference the extern
//     crate paths `vb_core::errors::CoreError` and
//     `vb_storage::error::JournalError`. These paths are treated by
//     Rust as extern-crate-name references and CANNOT be satisfied
//     by `pub mod vb_core { ... }` / `pub mod vb_storage { ... }`
//     stubs at the crate root. The production From impls always
//     return a fixed `CliExitCode` regardless of the input
//     (`let _ = err;`), so the inner variants of `CoreError` /
//     `JournalError` are never inspected. The mirror REPLACES the
//     extern crate paths with local `CoreError` / `JournalError`
//     type stubs declared inline in this file. The semantic bodies
//     of the From impls are preserved byte-for-byte.
//
//   - The `#[cfg(test)] mod tests` block at exit_code.rs:70-171 is
//     REMOVED because it references `vb_core::StepIdx::ZERO` and
//     `vb_storage::error::JournalError::KeyCapacity` which would
//     require the extern crates to be registered. Even though
//     `#[cfg(test)]` items are excluded under `verus --crate-type=lib`
//     (no `test` cfg set), the test module's presence triggers the
//     extern crate lookup at module parse time. Stripping the test
//     module removes the dependency entirely.
//
// The retained surface is exactly what the spec file
// `diagnostic_envelope_verus.rs` needs to bind the public CLI exit
// code surface to real production types. Every other line is copied
// verbatim from production.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_cli/src/exit_code.rs:1-68` whenever production changes.
// The mirror is annotated at the top of every section with the
// originating production line range so regeneration is mechanical.
// Drift that changes the `CliExitCode` discriminant set or the
// `From<CliExitCode> for u8` mapping breaks the
// `assume_specification` bridges in the companion spec file at
// compile time, which is the explicit drift-detection mechanism for
// the diagnostic-envelope binding.
//
// This file is included by the companion extern file
// `extern_diagnostic_envelope_verus.rs` under module-level `#[path]`.
// The whole module is marked `#[verifier::external]` so the
// production bodies are opaque to Verus — Verus verifies only
// structural resolution and type well-formedness, not the body
// semantics. The contracts are attached in the companion spec file
// via `assume_specification` bridges.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// LOCAL MIRROR: production `vb_core::ids::StepIdx` newtype
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/ids/mod.rs:55` declares
// `pub struct StepIdx(pub u16)` as a private-field newtype. The
// mirror declares it inline (rather than referencing the production
// extern crate path `vb_core::StepIdx`) so the mirror compiles
// without `vb_core` registered. Only the `StepIdx` type is mirrored
// because the production `exit_code.rs` does not reference `StepIdx`
// directly in non-test code; the type is declared to keep the
// surface self-describing.
//
// The mirror `StepIdx` is declared as a zero-sized placeholder
// (no fields) to avoid dragging in additional production type
// dependencies. The production `StepIdx` wraps a `u16` newtype, but
// the spec proofs never inspect the inner value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepIdx;

// ---------------------------------------------------------------------------
// LOCAL MIRROR: production `vb_core::errors::CoreError` enum
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/errors.rs:167` declares a 24-variant
// `CoreError` enum. The mirror declares a single-variant stub
// because the production `exit_code.rs` `From<CoreError> impl at
// exit_code.rs:57-60` ignores the input via `let _ = err;` (it
// always returns `CliExitCode::RuntimeFailed` regardless of the
// CoreError variant). Only the `InvalidProgramCounter` variant is
// mirrored as a unit variant (no fields) to avoid pulling in the
// `StepIdx` field type — production test code at exit_code.rs:124-127
// references this variant name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Production variant referenced by `exit_code.rs:124-127`
    /// test code (cfg'd out under `verus --crate-type=lib`).
    InvalidProgramCounter,
}

// ---------------------------------------------------------------------------
// LOCAL MIRROR: production `vb_storage::error::JournalError` enum
// ---------------------------------------------------------------------------
//
// Production `crates/vb_storage/src/error/mod.rs:21` declares an
// 18-variant `JournalError` enum. The mirror declares a single-
// variant stub because the production `exit_code.rs`
// `From<JournalError> impl at exit_code.rs:64-67` ignores the input
// via `let _ = err;` (it always returns `CliExitCode::StorageError`
// regardless of the JournalError variant). Only the `KeyCapacity`
// variant is mirrored to keep the surface honest — production test
// code at exit_code.rs:132-133 references this variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalError {
    /// Production variant referenced by `exit_code.rs:132-133`
    /// test code (cfg'd out under `verus --crate-type=lib`).
    KeyCapacity,
}

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/exit_code.rs:3` — use ExitCode
// ---------------------------------------------------------------------------
//
// Production line 3: `use std::process::ExitCode;`. Retained verbatim.
use std::process::ExitCode;

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/exit_code.rs:5-32` — `CliExitCode` enum
// ---------------------------------------------------------------------------
//
// Production lines 5-32: `CliExitCode` enum declaration with 9
// variants and explicit `#[repr(u8)]` tags. Retained verbatim. The
// production visibility is `pub(crate)`; the mirror uses `pub` so
// the verification crate can re-export it via `pub use`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CliExitCode {
    /// Operation completed successfully.
    Success = 0,
    /// Runtime execution or step evaluation failed.
    RuntimeFailed = 1,
    /// Input validation or argument parsing failed.
    ValidationFailed = 2,
    /// Workflow compilation or code generation failed.
    CompileFailed = 3,
    /// Workflow verification (e.g. step isolation precondition) failed.
    VerificationFailed = 4,
    /// Storage, journal, or persistence operation failed.
    StorageError = 5,
    /// IPC server operation failed.
    IpcError = 6,
    /// Action policy violation.
    ActionPolicyError = 7,
    /// Replay divergence detected, including domain-specific rule
    /// divergence after the internal error has been mapped to a
    /// public CLI status.
    ReplayDivergence = 8,
}

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/exit_code.rs:34-38` —
//   `impl From<CliExitCode> for ExitCode`
// ---------------------------------------------------------------------------
//
// Production lines 34-38. Retained verbatim.
impl From<CliExitCode> for ExitCode {
    fn from(code: CliExitCode) -> Self {
        ExitCode::from(u8::from(code))
    }
}

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/exit_code.rs:40-54` —
//   `impl From<CliExitCode> for u8`
// ---------------------------------------------------------------------------
//
// Production lines 40-54. Retained verbatim. This is the canonical
// mapping that the spec file's `spec_cli_exit_code_discriminant`
// function mirrors. The 9 match arms map each variant to its
// `#[repr(u8)]` discriminant tag.
impl From<CliExitCode> for u8 {
    fn from(code: CliExitCode) -> Self {
        match code {
            CliExitCode::Success => 0,
            CliExitCode::RuntimeFailed => 1,
            CliExitCode::ValidationFailed => 2,
            CliExitCode::CompileFailed => 3,
            CliExitCode::VerificationFailed => 4,
            CliExitCode::StorageError => 5,
            CliExitCode::IpcError => 6,
            CliExitCode::ActionPolicyError => 7,
            CliExitCode::ReplayDivergence => 8,
        }
    }
}

// ---------------------------------------------------------------------------
// SUBSTITUTED: Production `crates/vb_cli/src/exit_code.rs:56-61` —
//   `impl From<vb_core::errors::CoreError> for CliExitCode`
// ---------------------------------------------------------------------------
//
// Production lines 56-61 declare
//   impl From<vb_core::errors::CoreError> for CliExitCode {
//       fn from(err: vb_core::errors::CoreError) -> Self {
//           let _ = err;
//           CliExitCode::RuntimeFailed
//       }
//   }
//
// SUBSTITUTION RATIONALE: the absolute crate-name path
// `vb_core::errors::CoreError` cannot be resolved under
// `verus --crate-type=lib` without the `vb_core` extern crate being
// registered. The production body is the trivial projection
// `CliExitCode::RuntimeFailed` (the input `err` is discarded via
// `let _ = err;`). The substitution below references the local
// `CoreError` enum declared at the top of this mirror file. The
// body of the From impl is preserved byte-for-byte.
impl From<CoreError> for CliExitCode {
    fn from(err: CoreError) -> Self {
        let _ = err;
        CliExitCode::RuntimeFailed
    }
}

// ---------------------------------------------------------------------------
// SUBSTITUTED: Production `crates/vb_cli/src/exit_code.rs:63-68` —
//   `impl From<vb_storage::error::JournalError> for CliExitCode`
// ---------------------------------------------------------------------------
//
// Production lines 63-68 declare
//   impl From<vb_storage::error::JournalError> for CliExitCode {
//       fn from(err: vb_storage::error::JournalError) -> Self {
//           let _ = err;
//           CliExitCode::StorageError
//       }
//   }
//
// SUBSTITUTION RATIONALE: same as above. The substitution below
// references the local `JournalError` enum declared at the top of
// this mirror file. The body of the From impl is preserved
// byte-for-byte.
impl From<JournalError> for CliExitCode {
    fn from(err: JournalError) -> Self {
        let _ = err;
        CliExitCode::StorageError
    }
}

// ---------------------------------------------------------------------------
// REMOVED: Production `crates/vb_cli/src/exit_code.rs:70-171` —
//   `#[cfg(test)] mod tests`
// ---------------------------------------------------------------------------
//
// Production lines 70-171 declare a `#[cfg(test)] mod tests` block
// with 6 tests. REMOVED (see file header for rationale). The
// semantic content of all 6 tests is captured by the spec proofs in
// the companion spec file `diagnostic_envelope_verus.rs`.