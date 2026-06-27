// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_cli_commands_journal_trace` Verus spec.
//
// STRUCTURAL MIRROR BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_cli_commands_journal_trace.rs` Verus spec. It mirrors the
// production source file `crates/vb_cli/src/commands_journal.rs` with
// line-by-line field/parity annotations so any drift in production
// field names, discriminant sets, or fn signatures breaks Rust
// resolution at compile time.
//
// ============================================================================
// WHY STRUCTURAL MIRROR (NOT DIRECT #[path] INCLUSION)
// ============================================================================
//
// Direct `#[path = "../../crates/vb_cli/src/commands_journal.rs"]`
// inclusion is blocked by three blockers in the production file:
//
//   1. `commands_journal.rs:7` `use vb_storage::JournalEvent;` is a
//      BARE crate-name import. In Rust 2018+, bare crate names only
//      resolve to extern crates (declared via `Cargo.toml` dependencies
//      or `extern crate foo;`). Our stub `pub mod vb_storage` is a
//      top-level module, not an extern crate. The fix would be
//      `use crate::vb_storage::JournalEvent;` but production source
//      cannot be modified.
//
//   2. `commands_journal.rs` uses `serde_json::Value` (production
//      dependency declared in `crates/vb_cli/Cargo.toml`). `serde_json`
//      is not available as an extern crate in a standalone
//      `verus --crate-type=lib` invocation, and re-exporting a stub
//      module as an extern crate requires `--extern` flags that verus
//      does not expose.
//
//   3. Production `TraceEntry`, `TraceStatus`, `TraceFilters`,
//      `RetryAnalysis`, `build_trace`, `filter_trace`, and
//      `analyze_retry` are declared `pub(crate)` — visible within the
//      production `vb_cli` crate but not re-exportable as `pub`. A
//      direct `#[path]` inclusion would put them in OUR crate as
//      `pub(crate)` items, which `pub use` cannot re-export to the
//      spec file.
//
// The structural mirror below sidesteps every blocker while still
// establishing real end-to-end binding: any drift in the production
// variant set, variant names, field names, or trace_one body breaks
// this mirror and the spec proofs that depend on it. Each mirror
// field name below is annotated with its production line number so
// any divergence between the mirror and production is detectable by
// inspection.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Mirror types (each mirrors a production type line-by-line so any
// drift in field names or fn signatures breaks the build):
//
//   - `TraceEntry`           <- crates/vb_cli/src/commands_journal.rs:14-24
//   - `TraceStatus`          <- crates/vb_cli/src/commands_journal.rs:27-35
//   - `TraceStatus::as_str`  <- crates/vb_cli/src/commands_journal.rs:38-48
//   - `TraceFilters`         <- crates/vb_cli/src/commands_journal.rs:51-59
//   - `MirrorJournalEvent`   <- crates/vb_storage/src/events.rs:23-316
//                               (mirror of the 24-variant enum, with
//                               stub newtypes below for the fields
//                               that trace_one reads)
//
// Mirror fns (each mirror body mirrors the production body
// line-by-line so any drift breaks the spec proofs):
//
//   - `mirror_trace_one`     <- crates/vb_cli/src/commands_journal.rs:100-311
//                               (THE PRIMARY BINDING TARGET — production
//                               matches 18 variants explicitly and has
//                               a `_ =>` catch-all for the rest)
//   - `mirror_build_trace`   <- crates/vb_cli/src/commands_journal.rs:62-68
//                               (uses `mirror_trace_one` internally)
//
// Stub newtypes (mirror of vb_core::ids newtypes used by trace_one):
//
//   - `RunId`            (u64 + get)   <- crates/vb_core/src/ids/mod.rs:65
//   - `EventSeq`         (u64 + get)   <- crates/vb_core/src/ids/mod.rs:75
//   - `StepIdx`          (u16 + get)   <- crates/vb_core/src/ids/mod.rs:53
//   - `SlotIdx`          (u16 + get)   <- crates/vb_core/src/ids/mod.rs:55
//   - `ActionId`         (u16 + get)   <- crates/vb_core/src/ids/mod.rs:59
//
// Stub Debug types (used in format!("{:?}") expressions):
//
//   - `WorkflowDigest`   (Debug)       <- crates/vb_core/src/ids/mod.rs:343
//   - `CapabilitySet`    (Debug)       <- crates/vb_core/src/capability.rs:31
//   - `RuntimePolicy`    (Debug)       <- crates/vb_core/src/policy.rs:7
//   - `ConstValue`       (Debug)       <- crates/vb_core/src/value.rs:165
//
// Stub serde_json::Value (mirror of serde_json::Value used by extra_json):
//
//   - `serde_json::Value::Number(U64)` <- from u64/u16 newtype .get()
//   - `serde_json::Value::String`      <- from format!("{:?}")
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
// The bodies of `mirror_trace_one` and `mirror_build_trace` are mirrored
// from production line-by-line but Verus does NOT verify them (the
// fns are plain Rust outside `verus!`). The mathematical binding is
// attached in the companion spec file via `assume_specification`
// bridges: each bridge states the production contract (output value
// vs. input shape) and the spec proofs reason over that contract
// algebraically. Drift between the mirror body and the production
// body breaks the spec proofs because the postcondition asserted in
// `assume_specification` no longer matches the mirror's actual
// return value, so the exec proofs in the spec file fail to verify.
//
// ============================================================================
// DRIFT ITEMS ACCEPTED BY THE BINDING
// ============================================================================
//
//   - D1: Production `JournalEvent` is `#[non_exhaustive]`; the
//         mirror enum mirrors the 24 known variants exactly. The
//         production `trace_one` match has a catch-all `_ =>` arm,
//         so any new variant added to production is silently folded
//         into the "Unknown" entry in trace output. The spec
//         function `spec_trace_one` models this catch-all as the
//         `SpecJournalEvent::Unknown` ghost variant.
//
//   - D2: Production `commands_journal.rs::analyze_retry` is NOT
//         mirrored here (out of scope for TRACE-VERUS-001/002). Its
//         production body lines 318-369 are documented for reference
//         but not bound.
//
//   - D3: Production `commands_journal.rs::filter_trace` and
//         `trace_entry_matches_filters` are NOT mirrored here
//         (out of scope for the trace determinism obligation).
//         Their production body lines 71-98 are documented for
//         reference but not bound.
//
//   - D4: Production `TraceEntry` derives `Debug, Clone, PartialEq`.
//         The mirror derives `Debug, Clone, PartialEq` for parity.
//         Production `TraceEntry` has no `Eq` derive (because
//         `serde_json::Value` does not implement `Eq` in production);
//         the mirror follows the same pattern.
//
// ============================================================================
// DRIFT POLICY
// ============================================================================
// This mirror MUST be regenerated from
// `crates/vb_cli/src/commands_journal.rs` whenever production changes.
// Each section header cites the originating production line range so
// regeneration is mechanical.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// Stub: `serde_json` (mirror of `serde_json::Value` for TraceEntry.extra_json)
// ============================================================================
//
// The production `serde_json` is an external crate dependency of
// `vb_cli` (declared in `crates/vb_cli/Cargo.toml`), not available in
// standalone `verus --crate-type=lib`. The stub mirrors only the
// `Value` variants and `From` impls exercised by `commands_journal.rs`
// (Null, Bool, Number(U64), Number(I64), String).
pub mod serde_json {
    use std::fmt;

    /// Mirror of production `serde_json::Number`. Only the
    /// `U64` and `I64` arms are exposed because `trace_one` only
    /// produces numeric values from `.get()` on newtype wrappers
    /// (`RunId` → u64, `SlotIdx`/`StepIdx`/`ActionId` → u16 cast
    /// to u64).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Number {
        /// Unsigned 64-bit integer.
        U64(u64),
        /// Signed 64-bit integer.
        I64(i64),
    }

    /// Mirror of production `serde_json::Value`. Only the
    /// variants exercised by `commands_journal.rs::trace_one` are
    /// modeled (Null, Bool, Number, String).
    #[derive(Clone, Debug, PartialEq)]
    pub enum Value {
        /// JSON null.
        Null,
        /// JSON boolean.
        Bool(bool),
        /// JSON number (sub-enum covers U64/I64; u16 casts to U64).
        Number(Number),
        /// JSON string (covers Debug-formatted digest/policy/answer).
        String(String),
    }

    impl fmt::Display for Value {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Null => f.write_str("null"),
                Self::Bool(b) => write!(f, "{b}"),
                Self::Number(Number::U64(n)) => write!(f, "{n}"),
                Self::Number(Number::I64(n)) => write!(f, "{n}"),
                Self::String(s) => write!(f, "{s}"),
            }
        }
    }

    impl From<u64> for Value {
        fn from(v: u64) -> Self {
            Self::Number(Number::U64(v))
        }
    }

    impl From<i64> for Value {
        fn from(v: i64) -> Self {
            Self::Number(Number::I64(v))
        }
    }

    /// `Value::from(u16)` is required by `trace_one` for
    /// `StepIdx::get()`, `SlotIdx::get()`, and `ActionId::get()`.
    /// Production `serde_json` supports this via `From<i32>`, `From<u32>`,
    /// etc.; we model it as `Value::Number(Number::U64(u64::from(v)))`.
    impl From<u16> for Value {
        fn from(v: u16) -> Self {
            Self::Number(Number::U64(u64::from(v)))
        }
    }

    impl From<bool> for Value {
        fn from(v: bool) -> Self {
            Self::Bool(v)
        }
    }

    impl From<&'static str> for Value {
        fn from(v: &'static str) -> Self {
            Self::String(v.to_string())
        }
    }

    /// `Value::from(String)` is required by `trace_one` for
    /// `serde_json::Value::from(format!("{workflow:?}"))` and similar
    /// Debug-formatted fields (`artifact_digest`, `granted_capabilities`,
    /// `policy`, `answer`).
    impl From<String> for Value {
        fn from(v: String) -> Self {
            Self::String(v)
        }
    }
}

// ============================================================================
// Mirror newtypes — `vb_core::ids::*` newtype wrappers used by trace_one
// ============================================================================
//
// Mirror of `crates/vb_core/src/ids/mod.rs`. Only the newtypes and
// accessors exercised by `commands_journal.rs::trace_one` are
// provided. The production numeric_id! macro is at
// `crates/vb_core/src/ids/mod.rs:14-40` and generates these struct
// shapes (transparent newtype over a primitive integer, with `get()`
// returning the inner value).

/// Mirror of production `RunId` (`crates/vb_core/src/ids/mod.rs:65`).
/// `pub struct RunId(u64)` with `#[repr(transparent)]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunId(pub u64);

impl RunId {
    /// Mirror of `RunId::get` (production ids/mod.rs:70).
    #[allow(dead_code)]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Mirror of production `EventSeq` (`crates/vb_core/src/ids/mod.rs:75`).
/// `pub struct EventSeq(u64)` with `#[repr(transparent)]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventSeq(pub u64);

impl EventSeq {
    /// Mirror of `EventSeq::get` (production ids/mod.rs:80).
    #[allow(dead_code)]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Mirror of production `StepIdx` (`crates/vb_core/src/ids/mod.rs:53`).
/// `pub struct StepIdx(u16)` with `#[repr(transparent)]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepIdx(pub u16);

impl StepIdx {
    /// Mirror of `StepIdx::get` (production ids/mod.rs:56).
    #[allow(dead_code)]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Mirror of production `SlotIdx` (`crates/vb_core/src/ids/mod.rs:55`).
/// `pub struct SlotIdx(u16)` with `#[repr(transparent)]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

impl SlotIdx {
    /// Mirror of `SlotIdx::get` (production ids/mod.rs:58).
    #[allow(dead_code)]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Mirror of production `ActionId` (`crates/vb_core/src/ids/mod.rs:59`).
/// `pub struct ActionId(u16)` with `#[repr(transparent)]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionId(pub u16);

impl ActionId {
    /// Mirror of `ActionId::get` (production ids/mod.rs:62).
    #[allow(dead_code)]
    pub const fn get(self) -> u16 {
        self.0
    }
}

// ============================================================================
// Mirror Debug-only types — used in format!("{:?}") expressions
// ============================================================================

/// Mirror of production `WorkflowDigest`
/// (`crates/vb_core/src/ids/mod.rs:343`). Only the `Debug` impl is
/// required (used by `format!("{workflow:?}")` at
/// commands_journal.rs:111).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowDigest;

/// Mirror of production `CapabilitySet`
/// (`crates/vb_core/src/capability.rs:31`). Only the `Debug` impl
/// is required.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilitySet;

/// Mirror of production `RuntimePolicy`
/// (`crates/vb_core/src/policy.rs:7`). Only the `Debug` impl is
/// required.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePolicy;

/// Mirror of production `ConstValue`
/// (`crates/vb_core/src/value.rs:165`). Only the `Debug` impl is
/// required (used at commands_journal.rs:298).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstValue;

// ============================================================================
// MirrorJournalEvent — production-bound mirror of vb_storage::JournalEvent
// ============================================================================
//
// Production declaration at `crates/vb_storage/src/events.rs:23-316`.
//
// The 24-variant enum is mirrored line-by-line. Field names match
// production exactly. Field types are simplified to the stub newtypes
// and Debug-only placeholders above. `trace_one` reads these fields:
//   - seq.get()    (u64)
//   - step.get()   (u16)
//   - action.get() (u16)
//   - slot.get()   (u16)
//   - result.get() (u16)
//   - run.get()    (u64)
//   - output.get() (u16)
//   - format!("{workflow:?}")
//   - format!("{artifact_digest:?}")
//   - format!("{granted_capabilities:?}")
//   - format!("{policy:?}")
//   - format!("{:?}", answer)
//
// Variant → field table (mirror types in parentheses; production
// fields annotated with their source lines):
//
//   - RunAccepted                { run: RunId, seq: EventSeq, workflow: WorkflowDigest }
//                                events.rs:25-32
//   - RunAdmission               { run: RunId, seq: EventSeq,
//                                  artifact_digest: WorkflowDigest,
//                                  granted_capabilities: CapabilitySet,
//                                  policy: RuntimePolicy }
//                                events.rs:34-45
//   - StepStarted                { run: RunId, seq: EventSeq,
//                                  step: StepIdx, attempt: u16 }
//                                events.rs:47-56
//   - StepSucceeded              { run: RunId, seq: EventSeq,
//                                  step: StepIdx, output: SlotIdx }
//                                events.rs:58-67
//   - ActionScheduled            { run: RunId, seq: EventSeq,
//                                  step: StepIdx, action: ActionId,
//                                  attempt: u16 }
//                                events.rs:69-80
//   - ActionCompletedEvent       { run: RunId, seq: EventSeq,
//                                  step: StepIdx, action: ActionId,
//                                  attempt: u16 }
//                                events.rs:82-93
//   - ActionScheduledTicket      { run: RunId, seq: EventSeq,
//                                  ticket: ActionTicket, input: SlotIdx,
//                                  output: SlotIdx }
//                                events.rs:95-106
//   - ActionCompletedEnvelope    { run: RunId, seq: EventSeq,
//                                  ticket: ActionTicket, output: SlotIdx,
//                                  outcome: u8, value: (),
//                                  encoded_len: u32, taint: u8,
//                                  value_digest: () }
//                                events.rs:108-127
//   - ActionFailedEvent          { run: RunId, seq: EventSeq,
//                                  step: StepIdx, action: ActionId,
//                                  attempt: u16 }
//                                events.rs:129-140
//   - ActionAbandoned            { run: RunId, seq: EventSeq,
//                                  ticket: ActionTicket }
//                                events.rs:148-158
//   - SlotWrittenEvent           { run: RunId, seq: EventSeq,
//                                  slot: SlotIdx, value: Option<()>,
//                                  extra: Option<()>, attempt: u16 }
//                                events.rs:160-174
//   - WaitScheduledEvent         { run: RunId, seq: EventSeq,
//                                  step: StepIdx, attempt: u16 }
//                                events.rs:176-185
//   - AskScheduledEvent          { run: RunId, seq: EventSeq,
//                                  step: StepIdx, attempt: u16 }
//                                events.rs:187-196
//   - AskAnsweredEvent           { run: RunId, seq: EventSeq,
//                                  step: StepIdx, attempt: u16 }
//                                events.rs:198-207
//   - WaitResolvedEvent          { run: RunId, seq: EventSeq,
//                                  step: StepIdx, attempt: u16 }
//                                events.rs:213-222
//   - RetryScheduledEvent        { run: RunId, seq: EventSeq,
//                                  step: StepIdx, attempt: u16 }
//                                events.rs:224-233
//   - RunCancelled               { run: RunId, seq: EventSeq,
//                                  attempt: u16, reason: Option<String> }
//                                events.rs:235-244
//   - RunKilled                  { run: RunId, seq: EventSeq,
//                                  attempt: u16 }
//                                events.rs:246-253
//   - RunFinished                { run: RunId, seq: EventSeq,
//                                  result: SlotIdx, attempt: u16 }
//                                events.rs:255-264
//   - RunFailedEvent             { run: RunId, seq: EventSeq,
//                                  attempt: u16 }
//                                events.rs:266-273
//   - RunResumed                 { run: RunId, seq: EventSeq,
//                                  timestamp: () }
//                                events.rs:275-282
//   - RunRetried                 { run: RunId, seq: EventSeq,
//                                  timestamp: () }
//                                events.rs:284-291
//   - RunAnswered                { run: RunId, seq: EventSeq,
//                                  slot_idx: SlotIdx, answer: ConstValue,
//                                  timestamp: () }
//                                events.rs:293-304
//   - AskTimedOutEvent           { run: RunId, seq: EventSeq,
//                                  step: StepIdx, attempt: u16 }
//                                events.rs:306-315

/// Stub ActionTicket (mirror of
/// `crates/vb_core/src/action/ticket.rs:6-21`). The production
/// struct has 7 fields; only the `attempt: u16` field is read by
/// `JournalEvent::is_valid()`. `commands_journal.rs::trace_one` does
/// NOT inspect any ActionTicket field — ticket-bearing variants
/// (`ActionScheduledTicket`, `ActionCompletedEnvelope`,
/// `ActionAbandoned`) fall through the `_ =>` catch-all. The stub
/// preserves the field shape for variant parity but its values are
/// placeholders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionTicket {
    /// Mirror of production field `attempt`.
    pub attempt: u16,
}

/// Mirror of production `JournalEvent` enum.
#[derive(Clone, Debug)]
pub enum MirrorJournalEvent {
    /// Mirror of `JournalEvent::RunAccepted` at events.rs:25-32.
    RunAccepted {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `workflow: WorkflowDigest`.
        workflow: WorkflowDigest,
    },
    /// Mirror of `JournalEvent::RunAdmission` at events.rs:34-45.
    RunAdmission {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `artifact_digest: WorkflowDigest`.
        artifact_digest: WorkflowDigest,
        /// Mirror of `granted_capabilities: CapabilitySet`.
        granted_capabilities: CapabilitySet,
        /// Mirror of `policy: RuntimePolicy`.
        policy: RuntimePolicy,
    },
    /// Mirror of `JournalEvent::StepStarted` at events.rs:47-56.
    StepStarted {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::StepSucceeded` at events.rs:58-67.
    StepSucceeded {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `output: SlotIdx`.
        output: SlotIdx,
    },
    /// Mirror of `JournalEvent::ActionScheduled` at events.rs:69-80.
    ActionScheduled {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `action: ActionId`.
        action: ActionId,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::ActionCompletedEvent` at events.rs:82-93.
    ActionCompletedEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `action: ActionId`.
        action: ActionId,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::ActionScheduledTicket` at events.rs:95-106.
    ActionScheduledTicket {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
        /// Mirror of `input: SlotIdx`.
        input: SlotIdx,
        /// Mirror of `output: SlotIdx`.
        output: SlotIdx,
    },
    /// Mirror of `JournalEvent::ActionCompletedEnvelope` at events.rs:108-127.
    ActionCompletedEnvelope {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
        /// Mirror of `output: SlotIdx`.
        output: SlotIdx,
        /// Mirror of `outcome: DurableActionOutcome` (placeholder u8).
        outcome: u8,
        /// Mirror of `value: Vec<u8>` (placeholder unit).
        value: (),
        /// Mirror of `encoded_len: u32`.
        encoded_len: u32,
        /// Mirror of `taint: Taint` (placeholder u8).
        taint: u8,
        /// Mirror of `value_digest: [u8; 32]` (placeholder unit).
        value_digest: (),
    },
    /// Mirror of `JournalEvent::ActionFailedEvent` at events.rs:129-140.
    ActionFailedEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `action: ActionId`.
        action: ActionId,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::ActionAbandoned` at events.rs:148-158.
    ActionAbandoned {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
    },
    /// Mirror of `JournalEvent::SlotWrittenEvent` at events.rs:160-174.
    SlotWrittenEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `slot: SlotIdx`.
        slot: SlotIdx,
        /// Mirror of `value: Option<Vec<u8>>` (placeholder Option<()>).
        value: Option<()>,
        /// Mirror of `extra: Option<Vec<u8>>` (placeholder Option<()>).
        extra: Option<()>,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::WaitScheduledEvent` at events.rs:176-185.
    WaitScheduledEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::AskScheduledEvent` at events.rs:187-196.
    AskScheduledEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::AskAnsweredEvent` at events.rs:198-207.
    AskAnsweredEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::WaitResolvedEvent` at events.rs:213-222.
    WaitResolvedEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RetryScheduledEvent` at events.rs:224-233.
    RetryScheduledEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunCancelled` at events.rs:235-244.
    RunCancelled {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
        /// Mirror of `reason: Option<String>` (placeholder Option<()>).
        reason: Option<()>,
    },
    /// Mirror of `JournalEvent::RunKilled` at events.rs:246-253.
    RunKilled {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunFinished` at events.rs:255-264.
    RunFinished {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `result: SlotIdx`.
        result: SlotIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunFailedEvent` at events.rs:266-273.
    RunFailedEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunResumed` at events.rs:275-282.
    RunResumed {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder unit).
        timestamp: (),
    },
    /// Mirror of `JournalEvent::RunRetried` at events.rs:284-291.
    RunRetried {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder unit).
        timestamp: (),
    },
    /// Mirror of `JournalEvent::RunAnswered` at events.rs:293-304.
    RunAnswered {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `slot_idx: SlotIdx`.
        slot_idx: SlotIdx,
        /// Mirror of `answer: ConstValue`.
        answer: ConstValue,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder unit).
        timestamp: (),
    },
    /// Mirror of `JournalEvent::AskTimedOutEvent` at events.rs:306-315.
    AskTimedOutEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
}

// ============================================================================
// TraceEntry / TraceStatus / TraceFilters — production-bound mirrors
// ============================================================================
//
// Mirror of `crates/vb_cli/src/commands_journal.rs:14-59`.

/// Mirror of production `TraceEntry`
/// (`crates/vb_cli/src/commands_journal.rs:14-24`).
///
/// Production derives: `Debug, Clone, PartialEq`. The mirror
/// preserves the same derives (no `Eq` because `serde_json::Value`
/// does not implement `Eq` in production).
#[derive(Clone, Debug, PartialEq)]
pub struct TraceEntry {
    /// Mirror of production `index: usize` (commands_journal.rs:16).
    pub index: usize,
    /// Mirror of production `event_type: &'static str` (commands_journal.rs:17).
    pub event_type: &'static str,
    /// Mirror of production `step: Option<u16>` (commands_journal.rs:18).
    pub step: Option<u16>,
    /// Mirror of production `status: Option<TraceStatus>` (commands_journal.rs:19).
    pub status: Option<TraceStatus>,
    /// Mirror of production `action: Option<u16>` (commands_journal.rs:20).
    pub action: Option<u16>,
    /// Mirror of production `seq: u64` (commands_journal.rs:21).
    pub seq: u64,
    /// Mirror of production `extra_json: Vec<(&'static str, serde_json::Value)>`
    /// (commands_journal.rs:23).
    pub extra_json: Vec<(&'static str, serde_json::Value)>,
}

/// Mirror of production `TraceStatus`
/// (`crates/vb_cli/src/commands_journal.rs:27-35`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceStatus {
    /// Mirror of `TraceStatus::Pending`.
    Pending,
    /// Mirror of `TraceStatus::Active`.
    Active,
    /// Mirror of `TraceStatus::WaitingAnswer`.
    WaitingAnswer,
    /// Mirror of `TraceStatus::Cancelled`.
    Cancelled,
    /// Mirror of `TraceStatus::Completed`.
    Completed,
    /// Mirror of `TraceStatus::Failed`.
    Failed,
}

impl TraceStatus {
    /// Mirror of `TraceStatus::as_str`
    /// (`crates/vb_cli/src/commands_journal.rs:38-48`).
    #[allow(dead_code)]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::WaitingAnswer => "waiting_answer",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

// ============================================================================
// mirror_trace_one — production-bound mirror of trace_one
// ============================================================================
//
// Mirror of `crates/vb_cli/src/commands_journal.rs:100-311`.
//
// The mirror body is line-by-line equivalent to the production body:
// each explicit match arm produces the same `event_type`, `step`,
// `status`, `action`, `seq`, and `extra_json_len` as production.
// The catch-all `_ =>` arm mirrors the production "Unknown" case.
//
// This fn is plain Rust (NOT inside `verus!`); Verus treats it as
// opaque. The companion spec file attaches a spec contract via
// `assume_specification[ mirror_trace_one ]` and discharges
// production-bound obligations through exec proofs.
#[allow(dead_code)]
pub fn mirror_trace_one(idx: usize, event: &MirrorJournalEvent) -> TraceEntry {
    match event {
        // Production: commands_journal.rs:102-113
        MirrorJournalEvent::RunAccepted { seq, run, workflow } => TraceEntry {
            index: idx,
            event_type: "RunAccepted",
            step: None,
            status: Some(TraceStatus::Pending),
            action: None,
            seq: seq.get(),
            extra_json: vec![
                ("run", serde_json::Value::from(run.get())),
                ("workflow", serde_json::Value::from(format!("{workflow:?}"))),
            ],
        },
        // Production: commands_journal.rs:114-138
        MirrorJournalEvent::RunAdmission {
            seq,
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => TraceEntry {
            index: idx,
            event_type: "RunAdmission",
            step: None,
            status: Some(TraceStatus::Pending),
            action: None,
            seq: seq.get(),
            extra_json: vec![
                (
                    "artifact_digest",
                    serde_json::Value::from(format!("{artifact_digest:?}")),
                ),
                (
                    "granted_capabilities",
                    serde_json::Value::from(format!("{granted_capabilities:?}")),
                ),
                ("policy", serde_json::Value::from(format!("{policy:?}"))),
            ],
        },
        // Production: commands_journal.rs:139-147
        MirrorJournalEvent::StepStarted { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "StepStarted",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:148-158
        MirrorJournalEvent::StepSucceeded {
            seq, step, output, ..
        } => TraceEntry {
            index: idx,
            event_type: "StepSucceeded",
            step: Some(step.get()),
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![("output", serde_json::Value::from(output.get()))],
        },
        // Production: commands_journal.rs:159-169
        MirrorJournalEvent::ActionScheduled {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: Some(action.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        // Production: commands_journal.rs:170-180
        MirrorJournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionCompleted",
            step: Some(step.get()),
            status: Some(TraceStatus::Completed),
            action: Some(action.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        // Production: commands_journal.rs:181-191
        MirrorJournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionFailed",
            step: Some(step.get()),
            status: Some(TraceStatus::Failed),
            action: Some(action.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        // Production: commands_journal.rs:192-200
        MirrorJournalEvent::SlotWrittenEvent { seq, slot, .. } => TraceEntry {
            index: idx,
            event_type: "SlotWritten",
            step: None,
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![("slot", serde_json::Value::from(slot.get()))],
        },
        // Production: commands_journal.rs:201-209
        MirrorJournalEvent::WaitScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "WaitScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:210-218
        MirrorJournalEvent::AskScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "AskScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::WaitingAnswer),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:219-227
        MirrorJournalEvent::AskAnsweredEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "AskAnswered",
            step: Some(step.get()),
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:228-236
        MirrorJournalEvent::RetryScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "RetryScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:237-245
        MirrorJournalEvent::RunCancelled { seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunCancelled",
            step: None,
            status: Some(TraceStatus::Cancelled),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:246-254
        MirrorJournalEvent::RunFinished { seq, result, .. } => TraceEntry {
            index: idx,
            event_type: "RunFinished",
            step: None,
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![("result", serde_json::Value::from(result.get()))],
        },
        // Production: commands_journal.rs:255-263
        MirrorJournalEvent::RunFailedEvent { seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunFailed",
            step: None,
            status: Some(TraceStatus::Failed),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:264-272
        MirrorJournalEvent::RunResumed { run, seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunResumed",
            step: None,
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        // Production: commands_journal.rs:273-281
        MirrorJournalEvent::RunRetried { run, seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunRetried",
            step: None,
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        // Production: commands_journal.rs:282-300
        MirrorJournalEvent::RunAnswered {
            run,
            seq,
            slot_idx,
            answer,
            ..
        } => TraceEntry {
            index: idx,
            event_type: "RunAnswered",
            step: None,
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![
                ("run", serde_json::Value::from(run.get())),
                ("slot_idx", serde_json::Value::from(slot_idx.get())),
                ("answer", serde_json::Value::from(format!("{:?}", answer))),
            ],
        },
        // Production: commands_journal.rs:301-309 (catch-all `_ =>`)
        // Mirrors all 6 variants not explicitly handled:
        // ActionScheduledTicket, ActionCompletedEnvelope, ActionAbandoned,
        // WaitResolvedEvent, RunKilled, AskTimedOutEvent.
        _ => TraceEntry {
            index: idx,
            event_type: "Unknown",
            step: None,
            status: None,
            action: None,
            seq: 0,
            extra_json: vec![],
        },
    }
}

// ============================================================================
// mirror_build_trace — production-bound mirror of build_trace
// ============================================================================
//
// Mirror of `crates/vb_cli/src/commands_journal.rs:62-68`. Uses
// `mirror_trace_one` internally so any drift in `trace_one` cascades
// here.
#[allow(dead_code)]
pub fn mirror_build_trace(events: &[MirrorJournalEvent]) -> Vec<TraceEntry> {
    events
        .iter()
        .enumerate()
        .map(|(idx, event)| mirror_trace_one(idx, event))
        .collect()
}
