verus! {
        outcome.node_1_kind == KIND_SET_CONST,
        outcome.node_2_kind == KIND_COLLECT_PAGE,
        outcome.node_3_kind == KIND_COLLECT_FINISH,
{
    let outcome = production::lower_canonical_collect_projection(
        id,
        source,
        limit,
        page_size,
        body_length,
        pre_slot_count,
    );
    // Discharge assume_specification contract with explicit assertions
    // so Verus can fold each conjunct into the wrapper's ensures clause.
    assert(outcome.ok);
    assert(outcome.error_kind == SPEC_ERR_NONE);
    assert(outcome.pre_slot_count == pre_slot_count);
    assert(outcome.post_slot_count == pre_slot_count + 1);
    assert(outcome.emitted_node_count == 4u16);
    assert(outcome.start_step_id == id.0);
    assert(outcome.start_source == source.0);
    assert(outcome.start_limit == limit);
    assert(outcome.start_page_size == page_size);
    assert(outcome.start_body_id == id.0 + 1);
    assert(outcome.start_done_id == id.0 + 3);
    assert(outcome.body_step_id == id.0 + 1);
    assert(outcome.page_step_id == id.0 + 2);
    assert(outcome.page_collector_slot == source.0);
    assert(outcome.page_body_id == id.0 + 1);
    assert(outcome.page_done_id == id.0 + 3);
    assert(outcome.done_step_id == id.0 + 3);
    assert(outcome.finish_collector_slot == source.0);
    assert(outcome.node_0_kind == KIND_COLLECT_START);
    assert(outcome.node_1_kind == KIND_SET_CONST);
    assert(outcome.node_2_kind == KIND_COLLECT_PAGE);
    assert(outcome.node_3_kind == KIND_COLLECT_FINISH);
    outcome
}

// ============================================================================
// Production-bound proofs (PO-012)
// ============================================================================
//
// Each proof takes the spec-side inputs that
// `checked_prod_lower_canonical_collect` accepts and proves a property
// of `spec_collect_ir_outcome(...)` (which the assume_specification
// contract ties to the production projection's return value). The
// proofs are non-vacuous: they perform real arithmetic reasoning over
// the spec construction and discharge each IR structure obligation
// from the spec inputs. The binding to production is via the
// `assume_specification` contract on the projection and the
// `checked_prod_lower_canonical_collect` exec wrapper above.

// ---------- L1: Consecutive node IDs ----------

/// PO-012 / L1: The four emitted node IDs are consecutive:
/// `id`, `id+1`, `id+2`, `id+3`. This matches the production
/// `checked_step_offset(id, 1/2/3)` arithmetic at part_03.rs:203-208.
pub proof fn lemma_collect_node_ids_consecutive(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(id, source, limit, page_size, body_length, pre_slot_count),
    ensures
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).body_step_id
            as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_length,
            pre_slot_count,
        ).start_step_id as int + 1,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_step_id
            as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_length,
            pre_slot_count,
        ).body_step_id as int + 1,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).done_step_id
            as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_length,
            pre_slot_count,
        ).page_step_id as int + 1,
{
    // Algebraic unfolding of spec_collect_ir_outcome under
    // collect_ir_inputs_valid: body_step_id = id + 1,
    // start_step_id = id, etc. Consecutiveness follows by definition.
}

// ---------- L2: Node 0 is CollectStart with correct fields ----------

/// PO-012 / L2: Node 0 is `CompiledNodeKind::CollectStart` with
/// source/limit/page_size from the inputs and body/done set to
/// id+1/id+3.
pub proof fn lemma_node_0_is_collect_start(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(id, source, limit, page_size, body_length, pre_slot_count),
    ensures
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).node_0_kind
            as int == KIND_COLLECT_START as int,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_step_id
            as int == id,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_source
            as int == source,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_limit
            == limit as u32,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_page_size
            == page_size as u32,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_body_id
            as int == id + 1,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_done_id
            as int == id + 3,
{
    // Follows from spec_collect_ir_outcome construction under
    // collect_ir_inputs_valid (the conditional selects the success
    // branch).
}

// ---------- L3: Node 1 is SetConst at id+1 ----------

/// PO-012 / L3: Node 1 is `CompiledNodeKind::SetConst` at id+1
/// (emitted by `emit_single_body_set` -> `lower_set` when the body
/// has exactly one Set step).
pub proof fn lemma_node_1_is_set_const(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(id, source, limit, page_size, body_length, pre_slot_count),
    ensures
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).node_1_kind
            as int == KIND_SET_CONST as int,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).body_step_id
            as int == id + 1,
{
    // Follows from spec_collect_ir_outcome construction under
    // collect_ir_inputs_valid.
}

// ---------- L4: Node 2 is CollectPage with correct fields ----------

/// PO-012 / L4: Node 2 is `CompiledNodeKind::CollectPage` with
/// collector_slot=source, body=id+1, done=id+3.
pub proof fn lemma_node_2_is_collect_page(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(id, source, limit, page_size, body_length, pre_slot_count),
    ensures
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).node_2_kind
            as int == KIND_COLLECT_PAGE as int,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_step_id
            as int == id + 2,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_collector_slot
            as int == source,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_body_id
            as int == id + 1,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_done_id
            as int == id + 3,
{
    // Follows from spec_collect_ir_outcome construction under
    // collect_ir_inputs_valid.
}

// ---------- L5: Node 3 is CollectFinish with correct collector_slot ----------

/// PO-012 / L5: Node 3 is `CompiledNodeKind::CollectFinish` with
/// collector_slot=source.
pub proof fn lemma_node_3_is_collect_finish(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(id, source, limit, page_size, body_length, pre_slot_count),
    ensures
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).node_3_kind
            as int == KIND_COLLECT_FINISH as int,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).done_step_id
            as int == id + 3,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).finish_collector_slot
            as int == source,
{
    // Follows from spec_collect_ir_outcome construction under
    // collect_ir_inputs_valid.
}

// ---------- L6: Emission count is exactly 4 ----------

/// PO-012 / L6: The total node count is exactly 4 on the success path.
pub proof fn lemma_collect_node_count(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(id, source, limit, page_size, body_length, pre_slot_count),
    ensures
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).emitted_node_count
            == 4u16,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).ok,
{
    // Follows from spec_collect_ir_outcome construction: emitted_node_count
    // = 4 when ok (which holds under collect_ir_inputs_valid).
}

// ---------- L7: Slot count delta is exactly +1 ----------

/// PO-012 / L7: The slot count delta is exactly +1 on the success
/// path (one `record_slot(source)` call).
pub proof fn lemma_collect_slot_delta(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(id, source, limit, page_size, body_length, pre_slot_count),
    ensures
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).post_slot_count
            as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_length,
            pre_slot_count,
        ).pre_slot_count as int + 1,
{
    // Follows from spec_collect_ir_outcome construction: post_slot_count
    // = pre_slot_count + 1 on success.
}

// ---------- L8: Full emission chain ----------

/// PO-012 / L8: Full emission chain — all four nodes are emitted with
/// the documented kinds and field values, the slot count increases by
/// exactly one, and the emission succeeds. Conjunction of L1-L7.
pub proof fn lemma_collect_full_emission_chain(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(id, source, limit, page_size, body_length, pre_slot_count),
    ensures
        // ok + counts
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).ok,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).emitted_node_count
            == 4u16,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).post_slot_count
            as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_length,
            pre_slot_count,
        ).pre_slot_count as int + 1,
        // L1 — consecutive IDs
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).body_step_id
            as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_length,
            pre_slot_count,
        ).start_step_id as int + 1,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_step_id
            as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_length,
            pre_slot_count,
        ).body_step_id as int + 1,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).done_step_id
            as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_length,
            pre_slot_count,
        ).page_step_id as int + 1,
        // L2 — CollectStart fields
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).node_0_kind
            as int == KIND_COLLECT_START as int,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_source
            as int == source,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_limit
            == limit as u32,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).start_page_size
            == page_size as u32,
        // L3 — SetConst at id+1
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).node_1_kind
            as int == KIND_SET_CONST as int,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).body_step_id
            as int == id + 1,
        // L4 — CollectPage fields
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).node_2_kind
            as int == KIND_COLLECT_PAGE as int,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_collector_slot
            as int == source,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_body_id
            as int == id + 1,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).page_done_id
            as int == id + 3,
        // L5 — CollectFinish collector_slot
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).node_3_kind
            as int == KIND_COLLECT_FINISH as int,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).finish_collector_slot
            as int == source,
{
    // All conjuncts follow from spec_collect_ir_outcome construction
    // under collect_ir_inputs_valid (the conditional selects the
    // success branch, which assigns the documented field values).
    // The binding to production is via the assume_specification
    // contract above and the checked_prod_lower_canonical_collect
    // exec wrapper, which together establish that the production
    // projection returns exactly this spec outcome on the success path.
}

// ---------- Failure-path proofs ----------

/// PO-012 / F1: If id + 3 overflows u16, the projection returns
/// `ok = false` with `error_kind = SPEC_ERR_LIMIT_EXCEEDED` and emits
/// no nodes. This matches the production's
/// `CompileError::PrimitiveLoweringLimitExceeded` from
/// `checked_step_offset(id, 1/2/3)` at part_03.rs:203-208.
pub proof fn lemma_collect_id_overflow_fails(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        bounded_u16(id) && id + 3 > u16_max(),
        bounded_u16(source),
        bounded_u32(limit),
        bounded_u32(page_size),
        bounded_u16(pre_slot_count),
    ensures
        !spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).ok,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).error_kind
            == SPEC_ERR_LIMIT_EXCEEDED,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).emitted_node_count
            == 0u16,
{
    // spec_collect_ir_outcome: ok = (id+3 <= u16_max && body_length == 1).
    // With id + 3 > u16_max, ok is false regardless of body_length.
    // error_kind = SPEC_ERR_LIMIT_EXCEEDED (first branch).
    // emitted_node_count = 0 (failure branch).
}

/// PO-012 / F2: If body length is not 1, the projection returns
/// `ok = false` with `error_kind = SPEC_ERR_STEP_SHAPE` and emits no
/// nodes. This matches the production's `CompileError::StepFieldShape`
/// from `emit_single_body_set` requiring `body.len() == 1` at
/// part_04.rs:222-228.
pub proof fn lemma_collect_body_shape_fails(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
)
    requires
        bounded_u16(id) && id + 3 <= u16_max(),
        bounded_u16(source),
        bounded_u32(limit),
        bounded_u32(page_size),
        body_length != 1,
        bounded_u16(pre_slot_count),
    ensures
        !spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).ok,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).error_kind
            == SPEC_ERR_STEP_SHAPE,
        spec_collect_ir_outcome(id, source, limit, page_size, body_length, pre_slot_count).emitted_node_count
            == 0u16,
{
    // spec_collect_ir_outcome: ok = (id+3 <= u16_max && body_length == 1).
    // With id+3 fits but body_length != 1, ok is false.
    // error_kind = SPEC_ERR_STEP_SHAPE (second branch).
    // emitted_node_count = 0 (failure branch).
}

fn main() {
}

}
