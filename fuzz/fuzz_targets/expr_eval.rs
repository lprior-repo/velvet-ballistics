//! Fuzz target for expression evaluation on decoded `WorkflowParts`.
//!
//! ## INVARIANT Oracle
//!
//! Replaces crash-only fuzzing with structural assertions on the engine
//! evaluator pipeline:
//! - `eval_expr_with_store` returns
//!   `Result<(SlotValue, Taint), EngineError>` — Result is type-enforced; the
//!   runtime contract is panic-freedom plus typed error reporting.
//! - The evaluator stack is bounded by `MAX_EXPRESSION_STACK` — no unbounded
//!   stack growth on arbitrary `ExprProgram` bytecode.
//! - On Ok: the returned `SlotValue` is not `Null` — a `Null` result means
//!   the evaluator produced no useful value (enforced inside
//!   `fuzz_lib::fuzz_expr_eval`).
//! - When the decoded workflow has at least one expression, the evaluator
//!   must complete at least one expression without panicking.
//! - Target-level oracle below: postcard round-trip on `WorkflowParts` is
//!   deterministic — two decodes of the same bytes yield equal parts.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Target-level oracle: WorkflowParts decode is deterministic.
    if let Ok(parts_a) = postcard::from_bytes::<vb_core::WorkflowParts>(data)
        && let Ok(parts_b) = postcard::from_bytes::<vb_core::WorkflowParts>(data)
    {
        assert_eq!(
            parts_a.slot_count, parts_b.slot_count,
            "postcard decode of WorkflowParts is not deterministic (slot_count)"
        );
        assert_eq!(
            parts_a.digest, parts_b.digest,
            "postcard decode of WorkflowParts is not deterministic (digest)"
        );
    }

    fuzz_lib::fuzz_expr_eval(data);
});
