//! Verus specification and proof for vb_runtime action module — vb-rxru0 (revised).
//!
//! Production bindings:
//! - `spec_dispatch_generic` → `action.rs:14-34`
//! - `spec_issue_action_ticket` → `vb_core/src/action.rs:863-882`
//!
//! Spec functions model production behavior. No exec fns
//! (production code is plain Rust, not Verus-compiled).

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // Model: Abstract ActionTicket and ActionOutcome
    //
    // These model the essential fields of vb_core::action::ActionTicket and
    // vb_core::action::ActionOutcome. They are Verus-only models that capture
    // the mathematical structure of the production types.
    // ===========================================================================

    pub struct AbstractTicket {
        pub run: u64,
        pub step: u64,
        pub seq: u64,
        pub action: u64,
        pub attempt: u16,
        pub idempotency_key: u128,
        pub capacity: u16,
        pub mock: u8,
    }

    impl AbstractTicket {
        pub closed spec fn run(&self) -> u64 { self.run }
        pub closed spec fn step(&self) -> u64 { self.step }
        pub closed spec fn seq(&self) -> u64 { self.seq }
        pub closed spec fn action(&self) -> u64 { self.action }
        pub closed spec fn attempt(&self) -> u16 { self.attempt }
        pub closed spec fn idempotency_key(&self) -> u128 { self.idempotency_key }
        pub closed spec fn capacity(&self) -> u16 { self.capacity }
        pub closed spec fn mock(&self) -> u8 { self.mock }
    }

    pub enum AbstractOutcome {
        Suspended(AbstractTicket),
        Completed(AbstractTicket),
        Failed,
    }

    // ===========================================================================
    // Spec: dispatch_generic
    //
    // Production binding: vb_runtime::action::dispatch_generic (action.rs:14-34)
    //
    //   fn dispatch_generic(input: &ActionInput, contract: &ActionContract)
    //       -> ActionResult<ActionOutcome>
    //   {
    //       validate_input_bytes(input, contract)?;
    //       let mock = MockMarker::from_contract_name(contract.name.as_str());
    //       let ticket = ActionTicket {
    //           run: input.run, step: input.step, seq: input.ticket.seq,
    //           action: input.action, attempt: input.ticket.attempt,
    //           idempotency_key: input.ticket.idempotency_key,
    //           capacity: 1, mock,
    //       };
    //       Ok(ActionOutcome::Suspended(ticket))
    //   }
    //
    // The spec captures:
    //   1. Input validation: returns Err when max_input_bytes == 0 && input_slot_count > 0
    //   2. Mock derivation: from contract name (not input ticket)
    //   3. Field preservation: each ticket field from correct input source
    //   4. Capacity: always 1 (hardcoded in production)
    //   5. Outcome: always Suspended (not Completed or Failed)
    // ===========================================================================

    pub closed spec fn spec_dispatch_generic(
        input_run: u64,
        input_step: u64,
        input_seq: u64,
        input_action: u64,
        input_attempt: u16,
        input_idempotency_key: u128,
        contract_max_input_bytes: u32,
        contract_input_slot_count: u16,
        contract_name: &str,
        input_mock: u8,
    ) -> AbstractOutcome {
        if contract_max_input_bytes == 0 && contract_input_slot_count > 0 {
            AbstractOutcome::Failed
        } else {
            let mock = spec_mock_from_contract_name(contract_name);
            AbstractOutcome::Suspended(AbstractTicket {
                run: input_run,
                step: input_step,
                seq: input_seq,
                action: input_action,
                attempt: input_attempt,
                idempotency_key: input_idempotency_key,
                capacity: 1,
                mock,
            })
        }
    }

    // ===========================================================================
    // Spec: issue_action_ticket
    //
    // Production binding: vb_core::action::issue_action_ticket (vb_core/src/action.rs:863-882)
    //
    //   pub fn issue_action_ticket(
    //       run: RunId, step: StepIdx, seq: SeqNo, action: ActionId,
    //       attempt: u16, idempotency_key: u128, capacity: u16,
    //   ) -> ActionTicket
    //   {
    //       ActionTicket {
    //           run, step, seq, action, attempt, idempotency_key, capacity,
    //           ..Default::default()
    //       }
    //   }
    //
    // The spec captures that each field equals its corresponding parameter.
    // The mock field uses Default::default() → 0 in the model.
    // ===========================================================================

    pub closed spec fn spec_issue_action_ticket(
        p_run: u64, p_step: u64, p_seq: u64, p_action: u64,
        p_attempt: u16, p_idempotency_key: u128, p_capacity: u16,
    ) -> AbstractTicket {
        AbstractTicket {
            run: p_run,
            step: p_step,
            seq: p_seq,
            action: p_action,
            attempt: p_attempt,
            idempotency_key: p_idempotency_key,
            capacity: p_capacity,
            mock: 0, // Default::default() for MockMarker
        }
    }

    // ===========================================================================
    // Spec: MockMarker derivation from contract name
    //
    // Production binding: MockMarker::from_contract_name (vb_core/src/action.rs)
    //
    // Maps action contract names to MockMarker byte values.
    // ===========================================================================

    pub closed spec fn spec_mock_from_contract_name(contract_name: &str) -> u8 {
        match contract_name {
            "github.issue.create" => 0u8,
            "ai.classify.ticket" => 1u8,
            "http.put" => 2u8,
            _ => 0u8,
        }
    }

    // ===========================================================================
    // Proof: dispatch_generic field preservation
    //
    // Each output field comes from its declared input source.
    // ===========================================================================

    pub proof fn proof_dispatch_generic_field_preservation(
        input_run: u64,
        input_step: u64,
        input_seq: u64,
        input_action: u64,
        input_attempt: u16,
        input_idempotency_key: u128,
        contract_max_input_bytes: u32,
        contract_input_slot_count: u16,
        contract_name: &str,
        input_mock: u8,
    )
        ensures
            (contract_max_input_bytes == 0 && contract_input_slot_count > 0)
                ==> match spec_dispatch_generic(
                    input_run, input_step, input_seq, input_action, input_attempt,
                    input_idempotency_key, contract_max_input_bytes, contract_input_slot_count,
                    contract_name, input_mock,
                ) {
                    AbstractOutcome::Failed => true,
                    _ => false,
                }
            && (contract_max_input_bytes > 0 || contract_input_slot_count == 0)
                ==> match spec_dispatch_generic(
                    input_run, input_step, input_seq, input_action, input_attempt,
                    input_idempotency_key, contract_max_input_bytes, contract_input_slot_count,
                    contract_name, input_mock,
                ) {
                    AbstractOutcome::Suspended(t) => {
                        t.run() == input_run
                        && t.step() == input_step
                        && t.seq() == input_seq
                        && t.action() == input_action
                        && t.attempt() == input_attempt
                        && t.idempotency_key() == input_idempotency_key
                        && t.capacity() == 1
                        && t.mock() == spec_mock_from_contract_name(contract_name)
                    }
                    _ => false,
                },
    {
        // Verify each field preservation clause directly from spec definition.
        assert((contract_max_input_bytes == 0 && contract_input_slot_count > 0)
            ==> match spec_dispatch_generic(
                input_run, input_step, input_seq, input_action, input_attempt,
                input_idempotency_key, contract_max_input_bytes, contract_input_slot_count,
                contract_name, input_mock,
            ) {
                AbstractOutcome::Failed => true,
                _ => false,
            }) by (compute);
        assert((contract_max_input_bytes > 0 || contract_input_slot_count == 0)
            ==> match spec_dispatch_generic(
                input_run, input_step, input_seq, input_action, input_attempt,
                input_idempotency_key, contract_max_input_bytes, contract_input_slot_count,
                contract_name, input_mock,
            ) {
                AbstractOutcome::Suspended(t) => {
                    t.run() == input_run
                    && t.step() == input_step
                    && t.seq() == input_seq
                    && t.action() == input_action
                    && t.attempt() == input_attempt
                    && t.idempotency_key() == input_idempotency_key
                    && t.capacity() == 1
                    && t.mock() == spec_mock_from_contract_name(contract_name)
                }
                _ => false,
            }) by (compute);
    }

    // ===========================================================================
    // Proof: dispatch_generic is deterministic
    // ===========================================================================

    pub proof fn proof_dispatch_generic_deterministic(
        input_run: u64,
        input_step: u64,
        input_seq: u64,
        input_action: u64,
        input_attempt: u16,
        input_idempotency_key: u128,
        contract_max_input_bytes: u32,
        contract_input_slot_count: u16,
        contract_name: &str,
        input_mock: u8,
    )
        ensures
            spec_dispatch_generic(
                input_run, input_step, input_seq, input_action, input_attempt,
                input_idempotency_key, contract_max_input_bytes, contract_input_slot_count,
                contract_name, input_mock,
            ) == spec_dispatch_generic(
                input_run, input_step, input_seq, input_action, input_attempt,
                input_idempotency_key, contract_max_input_bytes, contract_input_slot_count,
                contract_name, input_mock,
            ),
    {
        // Determinism: identical inputs produce identical outputs.
        assert(spec_dispatch_generic(
            input_run, input_step, input_seq, input_action, input_attempt,
            input_idempotency_key, contract_max_input_bytes, contract_input_slot_count,
            contract_name, input_mock,
        ) == spec_dispatch_generic(
            input_run, input_step, input_seq, input_action, input_attempt,
            input_idempotency_key, contract_max_input_bytes, contract_input_slot_count,
            contract_name, input_mock,
        )) by (compute);
    }

    // ===========================================================================
    // Proof: Mock derivation from contract, NOT from input
    //
    // Two dispatch calls with same contract name but different input mocks
    // produce the SAME mock in the output ticket.
    // ===========================================================================

    pub proof fn proof_dispatch_derives_mock_from_contract(
        contract_name: &str,
        input_mock_a: u8,
        input_mock_b: u8,
        input_run: u64,
        input_step: u64,
        input_seq: u64,
        input_action: u64,
        input_attempt: u16,
        input_idempotency_key: u128,
    )
        requires
            contract_name != "",
        ensures
            match spec_dispatch_generic(
                input_run, input_step, input_seq, input_action, input_attempt,
                input_idempotency_key, 1, 0, contract_name, input_mock_a,
            ) {
                AbstractOutcome::Suspended(t) => t.mock() == spec_mock_from_contract_name(contract_name),
                _ => false,
            }
            && match spec_dispatch_generic(
                input_run, input_step, input_seq, input_action, input_attempt,
                input_idempotency_key, 1, 0, contract_name, input_mock_b,
            ) {
                AbstractOutcome::Suspended(t) => t.mock() == spec_mock_from_contract_name(contract_name),
                _ => false,
            },
    {
        // Both dispatches with same contract but different input mocks
        // must produce the same mock value derived from the contract.
        assert(match spec_dispatch_generic(
            input_run, input_step, input_seq, input_action, input_attempt,
            input_idempotency_key, 1, 0, contract_name, input_mock_a,
        ) {
            AbstractOutcome::Suspended(t) => t.mock() == spec_mock_from_contract_name(contract_name),
            _ => false,
        }) by (compute);
        assert(match spec_dispatch_generic(
            input_run, input_step, input_seq, input_action, input_attempt,
            input_idempotency_key, 1, 0, contract_name, input_mock_b,
        ) {
            AbstractOutcome::Suspended(t) => t.mock() == spec_mock_from_contract_name(contract_name),
            _ => false,
        }) by (compute);
    }

    // ===========================================================================
    // Proof: issue_action_ticket field preservation
    // ===========================================================================

    pub proof fn proof_issue_action_ticket_field_preservation(
        p_run: u64, p_step: u64, p_seq: u64, p_action: u64,
        p_attempt: u16, p_idempotency_key: u128, p_capacity: u16,
    )
        ensures
            spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).run == p_run
            && spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).step == p_step
            && spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).seq == p_seq
            && spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).action == p_action
            && spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).attempt == p_attempt
            && spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).idempotency_key == p_idempotency_key
            && spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).capacity == p_capacity
            && spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).mock == 0,
    {
        // Each field in the spec equals its corresponding parameter.
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).run == p_run) by (compute);
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).step == p_step) by (compute);
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).seq == p_seq) by (compute);
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).action == p_action) by (compute);
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).attempt == p_attempt) by (compute);
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).idempotency_key == p_idempotency_key) by (compute);
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).capacity == p_capacity) by (compute);
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity).mock == 0) by (compute);
    }

    // ===========================================================================
    // Proof: issue_action_ticket determinism
    // ===========================================================================

    pub proof fn proof_issue_action_ticket_deterministic(
        p_run: u64, p_step: u64, p_seq: u64, p_action: u64,
        p_attempt: u16, p_idempotency_key: u128, p_capacity: u16,
    )
        ensures
            spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity)
                == spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity),
    {
        // Determinism: identical inputs to issue_action_ticket produce identical tickets.
        assert(spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity)
            == spec_issue_action_ticket(p_run, p_step, p_seq, p_action, p_attempt, p_idempotency_key, p_capacity)) by (compute);
    }

    // ===========================================================================
    // Proof: dispatch_generic vs issue_action_ticket — capacity difference
    //
    // dispatch_generic hardcodes capacity: 1, but issue_action_ticket uses
    // the parameter capacity.
    // ===========================================================================

    pub proof fn proof_dispatch_vs_issue_capacity_difference(ticket_capacity: u16)
        ensures
            match spec_dispatch_generic(0, 0, 0, 0, 0, 0, 1, 0, "http.get", 0) {
                AbstractOutcome::Suspended(t) => t.capacity() == 1,
                _ => false,
            }
            && spec_issue_action_ticket(0, 0, 0, 0, 0, 0, ticket_capacity).capacity() == ticket_capacity
            && (ticket_capacity != 1 ==>
                (match spec_dispatch_generic(0, 0, 0, 0, 0, 0, 1, 0, "http.get", 0) {
                    AbstractOutcome::Suspended(t) => t.capacity(),
                    _ => 0,
                } != spec_issue_action_ticket(0, 0, 0, 0, 0, 0, ticket_capacity).capacity())),
    {
        // dispatch_generic hardcodes capacity=1, issue_action_ticket uses its parameter.
        // When ticket_capacity != 1, the two capacities must differ.
        let dispatched = spec_dispatch_generic(0, 0, 0, 0, 0, 0, 1, 0, "http.get", 0);
        assert(match dispatched {
            AbstractOutcome::Suspended(t) => t.capacity() == 1,
            _ => false,
        }) by (compute);
        assert(spec_issue_action_ticket(0, 0, 0, 0, 0, 0, ticket_capacity).capacity() == ticket_capacity) by (compute);
        match dispatched {
            AbstractOutcome::Suspended(t) => {
                assert(ticket_capacity != 1 ==> t.capacity() != spec_issue_action_ticket(0, 0, 0, 0, 0, 0, ticket_capacity).capacity()) by (compute);
            }
            _ => {
                assert(ticket_capacity != 1 ==> 0 != spec_issue_action_ticket(0, 0, 0, 0, 0, 0, ticket_capacity).capacity()) by (compute);
            }
        }
    }

    // ===========================================================================
    // Spec: Validation guard
    //
    // Production validate_input_bytes (action.rs:37-48):
    //   if contract.max_input_bytes == 0 && contract.input_slot_count > 0 {
    //       return Err(ActionError::PayloadTooLarge { ... });
    //   }
    //   Ok(())
    // ===========================================================================

    pub closed spec fn spec_dispatch_validates_first(max_input_bytes: u32, input_slot_count: u16) -> bool {
        max_input_bytes > 0 || input_slot_count == 0
    }

    pub proof fn proof_validation_passes_when_safe(
        max_input_bytes: u32, input_slot_count: u16,
    )
        requires max_input_bytes > 0 || input_slot_count == 0
        ensures spec_dispatch_validates_first(max_input_bytes, input_slot_count),
    {
        assert(spec_dispatch_validates_first(max_input_bytes, input_slot_count)) by (compute);
    }

    pub proof fn proof_validation_fails_when_unsafe(
        max_input_bytes: u32, input_slot_count: u16,
    )
        requires max_input_bytes == 0 && input_slot_count > 0
        ensures !spec_dispatch_validates_first(max_input_bytes, input_slot_count),
    {
        assert(!spec_dispatch_validates_first(max_input_bytes, input_slot_count)) by (compute);
    }

} // verus!
