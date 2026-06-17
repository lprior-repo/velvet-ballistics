// Verification artifact: kani_resource_contract_encoding_injectivity.rs
// PO: PO-K12
// Bead: vb-xi2f.35
// Verifier: Kani
// Command: cargo kani --harness prove_encoding_no_collision --unwind 2
// Workdir: crates/vb_core
//
// Proof obligation: Prove that the domain-tagged field encoding prevents
// collisions between different contracts. Different contracts produce
// different encoded byte sequences, hence different hashes.
//
// GOD RULE 1: Uses kani::any() + bounded pairs; no hardcoded dummy structs.
// GOD RULE 2: Binds to contract field values and encoding logic.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::contract_encoding::encode_contract_bytes;
use vb_core::workflow::ResourceContract;

// --- Bounded contract generator ---
fn bounded_contract_extreme() -> ResourceContract {
    let max_steps: u16 = kani::any();
    kani::assume(max_steps == 0 || max_steps == 1 || max_steps == u16::MAX);
    let max_slots: u16 = kani::any();
    kani::assume(max_slots == 0 || max_slots == 1 || max_slots == u16::MAX);
    let max_constants: u16 = kani::any();
    kani::assume(max_constants == 0 || max_constants == 1 || max_constants == u16::MAX);
    let max_accessors: u16 = kani::any();
    kani::assume(max_accessors == 0 || max_accessors == 1 || max_accessors == u16::MAX);
    let max_expressions: u16 = kani::any();
    kani::assume(max_expressions == 0 || max_expressions == 1 || max_expressions == u16::MAX);
    let max_expr_stack: u8 = kani::any();
    kani::assume(max_expr_stack == 0 || max_expr_stack == 1 || max_expr_stack == 255);
    let max_step_budget_per_tick: u64 = kani::any();
    kani::assume(max_step_budget_per_tick == 0 || max_step_budget_per_tick == 1 || max_step_budget_per_tick == u64::MAX);
    let max_transitions_per_tick: u64 = kani::any();
    kani::assume(max_transitions_per_tick == 0 || max_transitions_per_tick == 1 || max_transitions_per_tick == u64::MAX);
    let max_input_bytes: u32 = kani::any();
    kani::assume(max_input_bytes == 0 || max_input_bytes == 1 || max_input_bytes == u32::MAX);
    let max_output_bytes: u32 = kani::any();
    kani::assume(max_output_bytes == 0 || max_output_bytes == 1 || max_output_bytes == u32::MAX);
    let max_blob_bytes: u64 = kani::any();
    kani::assume(max_blob_bytes == 0 || max_blob_bytes == 1 || max_blob_bytes == u64::MAX);
    let max_ipc_payload_bytes: u32 = kani::any();
    kani::assume(max_ipc_payload_bytes == 0 || max_ipc_payload_bytes == 1 || max_ipc_payload_bytes == u32::MAX);
    let max_retry_attempts: u16 = kani::any();
    kani::assume(max_retry_attempts == 0 || max_retry_attempts == 1 || max_retry_attempts == u16::MAX);
    let max_fanout: u16 = kani::any();
    kani::assume(max_fanout == 0 || max_fanout == 1 || max_fanout == u16::MAX);
    let max_collect_items: u32 = kani::any();
    kani::assume(max_collect_items == 0 || max_collect_items == 1 || max_collect_items == u32::MAX);
    let max_queue_depth: u32 = kani::any();
    kani::assume(max_queue_depth == 0 || max_queue_depth == 1 || max_queue_depth == u32::MAX);
    let max_journal_batch_bytes: u32 = kani::any();
    kani::assume(max_journal_batch_bytes == 0 || max_journal_batch_bytes == 1 || max_journal_batch_bytes == u32::MAX);
    let allows_secret_results: bool = kani::any();

    ResourceContract {
        max_steps, max_slots, max_constants, max_accessors, max_expressions,
        max_expr_stack, max_step_budget_per_tick, max_transitions_per_tick,
        max_input_bytes, max_output_bytes, max_blob_bytes, max_ipc_payload_bytes,
        max_retry_attempts, max_fanout, max_collect_items, max_queue_depth,
        max_journal_batch_bytes, allows_secret_results,
    }
}

/// PO-K12: Prove that domain-tagged encoding is injective for bounded contracts.
/// When two contracts differ, their encodings must differ.
#[kani::proof]
#[kani::unwind(10)]
fn prove_encoding_no_collision() {
    let a = bounded_contract_extreme();
    let b = bounded_contract_extreme();

    // Only check inequality when contracts actually differ
    kani::assume(a != b);

    let encoded_a = encode_contract_bytes(&a);
    let encoded_b = encode_contract_bytes(&b);

    kani::assert(encoded_a != encoded_b,
        "Domain-tagged encoding must be injective: different contracts produce different encodings.\n\
         a={:?}\nb={:?}", a, b, "assertion failed");
}

/// PO-K12 variant: All-zeros vs all-ones edge case.
#[kani::proof]
#[kani::unwind(10)]
fn prove_encoding_no_collision_zeros_vs_ones() {
    let all_zeros = ResourceContract {
        max_steps: 0, max_slots: 0, max_constants: 0, max_accessors: 0,
        max_expressions: 0, max_expr_stack: 0, max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0, max_input_bytes: 0, max_output_bytes: 0,
        max_blob_bytes: 0, max_ipc_payload_bytes: 0, max_retry_attempts: 0,
        max_fanout: 0, max_collect_items: 0, max_queue_depth: 0,
        max_journal_batch_bytes: 0, allows_secret_results: false,
    };

    let all_ones = ResourceContract {
        max_steps: u16::MAX, max_slots: u16::MAX, max_constants: u16::MAX,
        max_accessors: u16::MAX, max_expressions: u16::MAX,
        max_expr_stack: u8::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_input_bytes: u32::MAX, max_output_bytes: u32::MAX,
        max_blob_bytes: u64::MAX, max_ipc_payload_bytes: u32::MAX,
        max_retry_attempts: u16::MAX, max_fanout: u16::MAX,
        max_collect_items: u32::MAX, max_queue_depth: u32::MAX,
        max_journal_batch_bytes: u32::MAX, allows_secret_results: true,
    };

    let encoded_zeros = encode_contract_bytes(&all_zeros);
    let encoded_ones = encode_contract_bytes(&all_ones);

    kani::assert(encoded_zeros != encoded_ones, "All-zeros contract must encode differently from all-ones contract");
}
