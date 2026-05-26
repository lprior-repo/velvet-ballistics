// Flux RS refinement specification for Span paired invariant
// PO-F01: Flux refinement on Span (C1.3)
// STATUS: WAIVED (2026-05-25)
//
// Waiver rationale:
// - Flux annotations require editing production source (crates/vb_core/src/span.rs)
//   which is outside proof-writer scope.
// - Kani PO-K01 is the canonical bounded proof for the Span paired invariant.
// - Waiver tracked in .beads/vb-xi2f.9/waiver-candidates.jsonl
//
// The annotations below document what WOULD be applied if Flux were the
// primary verification lane. They serve as a reference for future
// implementation agents.
//
// ```rust
// use flux_rs::attrs::*;
//
// #[refined_by(line_present: bool)]
// pub struct Span {
//     pub start: u32,
//     pub end: u32,
//     pub line: Option<u32>,
//     pub column: Option<u32>,
// }
//
// impl Span {
//     #[sig(fn() -> Span[|s| s.line_present == false])]
//     pub const ZERO: Self = Self { start: 0, end: 0, line: None, column: None };
//
//     #[sig(fn(u32, u32) -> Span[|s| s.line_present == false])]
//     pub const fn new(start: u32, end: u32) -> Self { ... }
//
//     #[sig(fn(u32, u32, u32, u32) -> Span[|s| s.line_present == true])]
//     pub const fn with_location(start: u32, end: u32, line: u32, column: u32) -> Self { ... }
// }
// ```
//
// Invariant: for all publicly constructible Spans,
// self.line.is_some() == self.column.is_some()
// which is expressed as the refined_by field `line_present: bool`
// where `line_present == true` iff line and column are both Some.
//
// Verification command: cargo flux --crate vb_core
// Expected: no refinement violations reported.
//
// Trusted base: TB-007 (flux-rs toolchain availability), TB-026 (light refinement)
// Flux refinement is defensive-in-depth; Kani PO-K01 is the canonical bounded proof
// for the paired invariant.
