//! velvet-ballistics rebuild — disciplined port plan.
//!
//! Master: `velvet-ballistics-MASTER.md` at repo root.
//!
//! Goal: ~250k LOC of clean, NASA-discipline code ported piece by piece from
//! the existing `crates/` tree into `rebuild/crates/`. Each piece is a
//! vertical slice that:
//!
//!   1. Cites the master doc section at the top of every source file
//!   2. Stays within file-size (≤300) and hot-fn (≤25) limits
//!   3. Carries sharp, mutation-resistant tests
//!   4. Verifies (Verus requires/ensures, Kani Arbitrary) when applicable
//!   5. Replaces the legacy equivalent — no dual maintenance
//!
//! Phases (master §35):
//!   0  Discipline gates + this scaffold                       [in progress]
//!   1  vb_core (§14) — IDs, FiniteF64, SlotValue, RunFrame     [next]
//!   2  vb_yaml (§25) — strict parser
//!   3  vb_validate (§26) — schema, refs, control, type/taint
//!   4  vb_expr (§27) — lexer, parser, bytecode, evaluator
//!   5  vb_compile (§28) — IR, full v1 primitive lowering
//!   6  vb_storage (§29) — Fjall 9 keyspaces, journal, recovery
//!   7  vb_runtime (§30) — shard, primitives, frame pool, action
//!   8  vb_ipc (§31) — mio socket loop, 11 commands
//!   9  vb_cli (§33) — 16 commands, typed envelopes
//!  10  Verification re-binding (Verus/Kani/Flux/loom)
//!  11  Evidence + release gates (real bench, fuzz, miri, coverage)
//!
//! Each piece is independently useful; you can stop after any phase and
//! have a working CLI tool. Each phase replaces a slice of the legacy tree;
//! no green-field rewrite of the whole codebase at once.

#![forbid(unsafe_code)]
#![warn(clippy::unwrap_used, clippy::expect_used)]