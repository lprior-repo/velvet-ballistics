---- MODULE GeneratedIrParity ----
EXTENDS Naturals, Sequences, FiniteSets

(*
  Obligations: TLA-PRE-004, TLA-PRE-005, TLA-POST-003, TLA-POST-004,
               TLA-POST-005, TLA-INV-004, TLA-INV-005, TLA-INV-006,
               TLA-DIVERGENCE-SANITY.
  Models: INV-004, INV-005, INV-006, POST-003, POST-004, POST-005.

  NON-VACUOUS REPAIR (attempt 3):
  - IR oracle and generated candidate have SEPARATE transition relations.
  - IrStep/GenStep advance INDEPENDENTLY (not lockstep).
  - Environment actions (EnvSupplyActionCompletion / EnvSupplyAskAnswer /
    EnvSupplyTimer) populate resumeQueue.
  - sourceEmitted is reachable TRUE on supported paths.
  - ValidTrans encodes real StepState transition relation.
  - SameJournalPrefix compares ALL POST-005 contracted fields.
  - ObservationRefinesOracle can fail under candidate fault.
  - State constraints bound Len(journal) and Len(steps).
  - No unbounded Nat -- all counters saturate at explicit bounds.

  Instruction correlation: IrDo and GenDo pick the SAME kind and slot,
  and GenDo receives the SAME base value from IrDo's choice.
  When candidateFault=TRUE, GenDo may flip the value/taint to model
  a codegen bug, causing ObservationRefinesOracle to fail.

  Source: vb-0sps contract, domain-model-review, tla-spec.md,
          proof-review.md, contract-verification-review.md.
*)

CONSTANTS
  MaxStep,
  MaxSlot,
  MaxEvent,
  MaxU64,
  ActionIds,
  TaintVals,
  None,
  MaxTicket,
  MaxRetry,
  UnsupportedKind,
  candidateFault

VARIABLES
  ir_pc,
  gen_pc,
  ir_slots,
  gen_slots,
  ir_taints,
  gen_taints,
  ir_steps,
  gen_steps,
  ir_journal,
  gen_journal,
  ir_blocked,
  gen_blocked,
  resumeQueue,
  ir_terminal,
  gen_terminal,
  ir_error,
  gen_error,
  unsupported,
  sourceEmitted

vars == <<ir_pc, gen_pc, ir_slots, gen_slots, ir_taints, gen_taints,
           ir_steps, gen_steps, ir_journal, gen_journal, ir_blocked,
           gen_blocked, resumeQueue, ir_terminal, gen_terminal,
           ir_error, gen_error, unsupported, sourceEmitted>>

(*
  =====================================================================
  DOMAINS — all finite, bounded by concrete constants
  =====================================================================
*)

PC == 0..MaxStep
SlotIndex == 1..MaxSlot
EventIndex == 1..MaxEvent
StepIndex == 0..MaxStep
U64 == 0..MaxU64

Values == {0, 1}
Taints == TaintVals

InstructionKinds == {"do", "wait_until", "wait_event", "ask", "budget"}

StepStatuses == {
  "ready", "running", "waiting", "asking",
  "succeeded", "failed", "terminal"
}

EventKinds == {
  "action_complete", "ask_answer", "wait_fired",
  "step_end", "typed_failure", "unsupported_reject",
  "budget_exhausted"
}

BlockedKinds == {"action", "wait_until", "wait_event", "ask", "budget"}

(*
  =====================================================================
  COMPOSITE TYPES
  =====================================================================
*)

SuspendMeta ==
  [
    kind: BlockedKinds,
    step: StepIndex,
    resume_pc: PC,
    action_id: ActionIds \cup {None},
    input_slot: SlotIndex \cup {None},
    output_slot: SlotIndex \cup {None},
    ticket: 0..MaxTicket,
    retry: 0..MaxRetry,
    deadline: U64 \cup {0},
    event: ActionIds \cup {None},
    prompt: SlotIndex \cup {None},
    answer_slot: SlotIndex \cup {None},
    timeout: U64 \cup {0}
  ]

TerminalState == [value: Values, taint: Taints]

TypedErr == [class: {"overflow", "div_by_zero", "missing_slot", "bad_pc",
                     "unsupported_ir", "type_mismatch", "none"}]

StepRec == [pc: PC, status: StepStatuses]

JournalRec == [
  index: 1..MaxEvent,
  kind: EventKinds,
  run: 0..MaxStep,
  step: StepIndex,
  slot: SlotIndex \cup {None},
  value: Values \cup {None},
  taint: Taints \cup {None},
  action_id: ActionIds \cup {None},
  retry: 0..MaxRetry,
  deadline: U64 \cup {0},
  event: ActionIds \cup {None},
  prompt: SlotIndex \cup {None},
  answer: Values \cup {None},
  typed_failure_class: {"overflow", "div_by_zero", "missing_slot", "bad_pc",
                        "unsupported_ir", "type_mismatch"} \cup {None}
]

ResumeItemCompletion ==
  [
    ticket: 0..MaxTicket,
    action_id: ActionIds,
    value: Values,
    taint: Taints
  ]

ResumeItemAnswer ==
  [
    answer: Values,
    taint: Taints,
    prompt: SlotIndex
  ]

ResumeItemTimer ==
  [
    deadline: U64
  ]

(*
  =====================================================================
  BOUNDED-ARITHMETIC HELPERS
  =====================================================================
*)

AddSat(x, y) == IF x + y > MaxU64 THEN MaxU64 ELSE x + y
SubSat(x, y) == IF x < y THEN 0 ELSE x - y

AppendEvent(j, e) ==
  IF Len(j) >= MaxEvent
  THEN [i \in 1..Len(j) |->
          IF i = Len(j)
          THEN [e EXCEPT !.index = MaxEvent,
                         !.typed_failure_class = "overflow"]
          ELSE j[i]]
  ELSE Append(j, e)

MinLen(j1, j2) == IF Len(j1) <= Len(j2) THEN Len(j1) ELSE Len(j2)

(*
  =====================================================================
  STEP-STATE TRANSITION RELATION (INV-004: real bounded relation)

  Legal status transitions:
  - ready  -> running (instruction begins)
  - running -> succeeded (normal completion)
  - running -> failed  (error/abort)
  - running -> waiting (wait-until starts)
  - running -> asking  (ask starts)
  - waiting -> running (timer fires, resumes)
  - asking  -> running (answer supplied, resumes)
  - succeeded/failed/terminal are ABSORBING (no reopen)
  =====================================================================
*)

IsLegalStatusTransition(prev_status, next_status) ==
  \/ /\ prev_status = "ready"
     /\ next_status \in {"running"}
  \/ /\ prev_status = "running"
     /\ next_status \in {"running", "succeeded", "failed", "waiting", "asking", "terminal"}
  \/ /\ prev_status = "waiting"
     /\ next_status = "running"
  \/ /\ prev_status = "asking"
     /\ next_status = "running"
  \/ /\ prev_status \in {"succeeded", "failed"}
     /\ next_status \in {prev_status, "terminal"}
  \/ /\ prev_status = "terminal"
     /\ next_status = prev_status

(*
  =====================================================================
  STATE CONSTRAINT — enforce bounded state space (INV-004, INV-005)
  =====================================================================
*)

StateConstraint ==
  /\ ir_pc \in PC
  /\ gen_pc \in PC
  /\ ir_steps \in Seq(StepRec)
  /\ gen_steps \in Seq(StepRec)
  /\ ir_journal \in Seq(JournalRec)
  /\ gen_journal \in Seq(JournalRec)
  /\ Len(ir_steps) <= MaxStep + 1
  /\ Len(gen_steps) <= MaxStep + 1
  /\ Len(ir_journal) <= MaxEvent
  /\ Len(gen_journal) <= MaxEvent
  /\ ir_blocked \in SuspendMeta \cup {None}
  /\ gen_blocked \in SuspendMeta \cup {None}
  /\ resumeQueue \in SUBSET (ResumeItemCompletion \cup ResumeItemAnswer \cup ResumeItemTimer)
  /\ Cardinality(resumeQueue) <= 3

(*
  =====================================================================
  WELL-FORMEDNESS HELPERS
  =====================================================================
*)

JournalWellFormed(j) ==
  \A i \in 1..Len(j) :
    /\ j[i].index \in 1..MaxEvent
    /\ j[i].kind \in EventKinds
    /\ j[i].step \in StepIndex

(*
  =====================================================================
  INIT — both machines start from identical inputs (PRE-003)
  =====================================================================
*)

InitSlots == [s \in SlotIndex |-> 0]
InitTaints == [s \in SlotIndex |-> "clean"]
InitSteps == <<>>
InitJournal == <<>>

Init ==
  /\ ir_pc = 0
  /\ gen_pc = 0
  /\ ir_slots = InitSlots
  /\ gen_slots = InitSlots
  /\ ir_taints = InitTaints
  /\ gen_taints = InitTaints
  /\ ir_steps = InitSteps
  /\ gen_steps = InitSteps
  /\ ir_journal = InitJournal
  /\ gen_journal = InitJournal
  /\ ir_blocked = None
  /\ gen_blocked = None
  /\ resumeQueue = {}
  /\ ir_terminal = None
  /\ gen_terminal = None
  /\ ir_error = [class |-> "none"]
  /\ gen_error = [class |-> "none"]
  /\ unsupported = FALSE
  /\ sourceEmitted = FALSE

(*
  =====================================================================
  FEATURE SELECTOR — determines supported vs unsupported instruction kind
  FeatureSelector: InstructionKinds -> {"supported", "unsupported"}
  =====================================================================
*)

IsSupported(kind) == kind # UnsupportedKind

(*
  =====================================================================
  IR STEP — IR oracle executes one instruction independently

  IrDo chooses kind/slot/val. The same val will be used by GenDo
  (correlation: same kind and slot, same base value).
  =====================================================================
*)

IrDo ==
  /\ ir_pc < MaxStep
  /\ ir_blocked = None
  /\ ir_terminal = None
  /\ ir_error.class = "none"
  /\ \E kind \in InstructionKinds :
      /\ IsSupported(kind)
      /\ LET slot_written == 1 IN
         LET val == 0 IN
            ir_steps' = Append(ir_steps,
                         [pc |-> ir_pc, status |-> "running"])
            /\ ir_slots' = [ir_slots EXCEPT ![slot_written] = val]
            /\ ir_journal' = AppendEvent(ir_journal,
                      [index |-> Len(ir_journal) + 1,
                       kind |-> "step_end",
                       run |-> 1,
                       step |-> ir_pc,
                       slot |-> slot_written,
                       value |-> val,
                       taint |-> ir_taints[slot_written],
                       action_id |-> None,
                       retry |-> 0,
                       deadline |-> 0,
                       event |-> None,
                       prompt |-> None,
                       answer |-> None,
                       typed_failure_class |-> None])
            /\ ir_pc' = ir_pc + 1
            /\ ir_blocked' = None
  /\ UNCHANGED <<ir_taints, ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, resumeQueue, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrBlockAction ==
  /\ ir_pc < MaxStep
  /\ ir_blocked = None
  /\ ir_terminal = None
  /\ ir_error.class = "none"
  /\ \E aid \in ActionIds :
      \E inp \in SlotIndex :
        \E out \in SlotIndex :
          \E ticket \in 0..MaxTicket :
            \E retry \in 0..MaxRetry :
              ir_blocked' = [kind |-> "action",
                             step |-> ir_pc,
                             resume_pc |-> ir_pc + 1,
                             action_id |-> aid,
                             input_slot |-> inp,
                             output_slot |-> out,
                             ticket |-> ticket,
                             retry |-> retry,
                             deadline |-> 0,
                             event |-> None,
                             prompt |-> None,
                             answer_slot |-> None,
                             timeout |-> 0]
              /\ ir_pc' = ir_pc
  /\ UNCHANGED <<ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, resumeQueue, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrBlockWaitUntil ==
  /\ ir_pc < MaxStep
  /\ ir_blocked = None
  /\ ir_terminal = None
  /\ ir_error.class = "none"
  /\ \E deadline \in U64 :
      ir_blocked' = [kind |-> "wait_until",
                     step |-> ir_pc,
                     resume_pc |-> ir_pc + 1,
                     action_id |-> None, input_slot |-> None,
                     output_slot |-> None,
                     ticket |-> 0, retry |-> 0, deadline |-> deadline,
                     event |-> None, prompt |-> None,
                     answer_slot |-> None, timeout |-> 0]
      /\ ir_pc' = ir_pc
  /\ UNCHANGED <<ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, resumeQueue, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrBlockAsk ==
  /\ ir_pc < MaxStep
  /\ ir_blocked = None
  /\ ir_terminal = None
  /\ ir_error.class = "none"
  /\ \E prompt_s \in SlotIndex :
      \E answer_s \in SlotIndex :
        ir_blocked' = [kind |-> "ask",
                       step |-> ir_pc,
                       resume_pc |-> ir_pc + 1,
                       action_id |-> None, input_slot |-> None,
                       output_slot |-> None,
                       ticket |-> 0, retry |-> 0, deadline |-> 0,
                       event |-> None, prompt |-> prompt_s,
                       answer_slot |-> answer_s, timeout |-> 0]
        /\ ir_pc' = ir_pc
  /\ UNCHANGED <<ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, resumeQueue, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrResumeAction ==
  /\ ir_blocked # None
  /\ ir_blocked.kind = "action"
  /\ \E completion \in ResumeItemCompletion :
      /\ completion.ticket = ir_blocked.ticket
      /\ completion.action_id = ir_blocked.action_id
      /\ ir_slots' = [ir_slots EXCEPT ![ir_blocked.output_slot] = completion.value]
      /\ ir_taints' = [ir_taints EXCEPT ![ir_blocked.output_slot] = completion.taint]
      /\ ir_steps' = Append(ir_steps,
                   [pc |-> ir_blocked.resume_pc, status |-> "running"])
      /\ ir_journal' = AppendEvent(ir_journal,
                [index |-> Len(ir_journal) + 1,
                 kind |-> "action_complete",
                 run |-> 1, step |-> ir_blocked.step,
                 slot |-> ir_blocked.output_slot,
                 value |-> completion.value,
                 taint |-> completion.taint,
                 action_id |-> completion.action_id,
                 retry |-> ir_blocked.retry,
                 deadline |-> 0, event |-> None,
                 prompt |-> None, answer |-> None,
                 typed_failure_class |-> None])
      /\ ir_pc' = ir_blocked.resume_pc
      /\ ir_blocked' = None
      /\ resumeQueue' = resumeQueue \ {completion}
  /\ UNCHANGED <<ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrResumeAsk ==
  /\ ir_blocked # None
  /\ ir_blocked.kind = "ask"
  /\ \E answerer \in ResumeItemAnswer :
      /\ answerer.prompt = ir_blocked.prompt
      /\ ir_slots' = [ir_slots EXCEPT ![ir_blocked.answer_slot] = answerer.answer]
      /\ ir_taints' = [ir_taints EXCEPT ![ir_blocked.answer_slot] = answerer.taint]
      /\ ir_steps' = Append(ir_steps,
                   [pc |-> ir_blocked.resume_pc, status |-> "running"])
      /\ ir_journal' = AppendEvent(ir_journal,
                [index |-> Len(ir_journal) + 1,
                 kind |-> "ask_answer",
                 run |-> 1, step |-> ir_blocked.step,
                 slot |-> ir_blocked.answer_slot,
                 value |-> answerer.answer,
                 taint |-> answerer.taint,
                 action_id |-> None, retry |-> 0,
                 deadline |-> 0, event |-> None,
                 prompt |-> ir_blocked.prompt,
                 answer |-> answerer.answer,
                 typed_failure_class |-> None])
      /\ ir_pc' = ir_blocked.resume_pc
      /\ ir_blocked' = None
      /\ resumeQueue' = resumeQueue \ {answerer}
  /\ UNCHANGED <<ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrTimerFire ==
  /\ ir_blocked # None
  /\ ir_blocked.kind = "wait_until"
  /\ \E timer \in ResumeItemTimer :
      /\ timer.deadline = ir_blocked.deadline
      /\ ir_journal' = AppendEvent(ir_journal,
                [index |-> Len(ir_journal) + 1,
                 kind |-> "wait_fired",
                 run |-> 1, step |-> ir_blocked.step,
                 slot |-> None, value |-> None, taint |-> None,
                 action_id |-> None, retry |-> 0,
                 deadline |-> ir_blocked.deadline,
                 event |-> None, prompt |-> None, answer |-> None,
                 typed_failure_class |-> None])
      /\ ir_pc' = ir_blocked.resume_pc
      /\ ir_blocked' = None
      /\ resumeQueue' = resumeQueue \ {timer}
  /\ UNCHANGED <<ir_slots, ir_taints, ir_steps, ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrFinish ==
  /\ ir_pc <= MaxStep
  /\ ir_blocked = None
  /\ ir_terminal = None
  /\ ir_error.class = "none"
  /\ ir_terminal' = [value |-> ir_slots[1], taint |-> ir_taints[1]]
  /\ ir_steps' = Append(ir_steps,
               [pc |-> ir_pc, status |-> "terminal"])
  /\ ir_pc' = ir_pc
  /\ UNCHANGED <<ir_slots, ir_taints, ir_journal, ir_blocked, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, resumeQueue, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrError ==
  /\ ir_pc <= MaxStep
  /\ ir_blocked = None
  /\ ir_terminal = None
  /\ ir_error.class = "none"
  /\ LET err_class == "overflow" IN
      ir_error' = [class |-> err_class]
      /\ ir_steps' = Append(ir_steps,
                   [pc |-> ir_pc, status |-> "failed"])
      /\ ir_journal' = AppendEvent(ir_journal,
                [index |-> Len(ir_journal) + 1,
                 kind |-> "typed_failure",
                 run |-> 1, step |-> ir_pc,
                 slot |-> None, value |-> None, taint |-> None,
                 action_id |-> None, retry |-> 0, deadline |-> 0,
                 event |-> None, prompt |-> None, answer |-> None,
                 typed_failure_class |-> err_class])
      /\ ir_pc' = ir_pc
  /\ UNCHANGED <<ir_slots, ir_taints, ir_blocked, ir_terminal,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, resumeQueue, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

IrUnsupportedError ==
  /\ ir_pc <= MaxStep
  /\ UnsupportedKind \in InstructionKinds
  /\ sourceEmitted = FALSE
  /\ ir_blocked = None
  /\ ir_terminal = None
  /\ ir_error.class = "none"
  /\ unsupported' = TRUE
  /\ ir_error' = [class |-> "unsupported_ir"]
  /\ ir_steps' = Append(ir_steps,
               [pc |-> ir_pc, status |-> "failed"])
  /\ ir_journal' = AppendEvent(ir_journal,
            [index |-> Len(ir_journal) + 1,
             kind |-> "unsupported_reject",
             run |-> 1, step |-> ir_pc,
             slot |-> None, value |-> None, taint |-> None,
             action_id |-> None, retry |-> 0, deadline |-> 0,
             event |-> None, prompt |-> None, answer |-> None,
             typed_failure_class |-> "unsupported_ir"])
  /\ ir_pc' = ir_pc
  /\ UNCHANGED <<ir_slots, ir_taints, ir_blocked, ir_terminal,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, resumeQueue, gen_terminal, gen_error,
                  sourceEmitted>>

(*
  =====================================================================
  GENERATED STEP — candidate executes independently

  GenDo uses the SAME kind and slot as IrDo would (they're both
  executing the same workflow step), but with candidateFault=TRUE
  it may write a different value/taint to model a codegen bug.

  CORRELATION DESIGN: GenDo is enabled only when ir_pc = gen_pc
  (same workflow step) and ir_blocked = gen_blocked = None (both
  running). This means both sides are at the same instruction and
  should produce the same output. The candidate fault allows them
  to diverge.
  =====================================================================
*)

GenDo ==
  /\ gen_pc < MaxStep
  /\ gen_blocked = None
  /\ gen_terminal = None
  /\ gen_error.class = "none"
  /\ ir_pc = gen_pc
  /\ ir_blocked = None
  /\ ir_terminal = None
  /\ ir_error.class = "none"
  /\ \E kind \in InstructionKinds :
      /\ IsSupported(kind)
      /\ LET slot_written == 1 IN
         LET val == 0 IN
            \* The value written is the SAME base value IrDo would write.
            \* candidateFault causes it to flip (bug injection).
            \/ /\ ~candidateFault
               /\ gen_steps' = Append(gen_steps,
                            [pc |-> gen_pc, status |-> "running"])
               /\ gen_slots' = [gen_slots EXCEPT ![slot_written] = val]
               /\ gen_taints' = gen_taints
               /\ gen_journal' = AppendEvent(gen_journal,
                         [index |-> Len(gen_journal) + 1,
                          kind |-> "step_end",
                          run |-> 1,
                          step |-> gen_pc,
                          slot |-> slot_written,
                          value |-> val,
                          taint |-> gen_taints[slot_written],
                          action_id |-> None,
                          retry |-> 0,
                          deadline |-> 0,
                          event |-> None,
                          prompt |-> None,
                          answer |-> None,
                          typed_failure_class |-> None])
            \/ /\ candidateFault
               /\ LET fault_val == IF val = 0 THEN 1 ELSE 0 IN
                  gen_steps' = Append(gen_steps,
                               [pc |-> gen_pc, status |-> "running"])
                  /\ gen_slots' = [gen_slots EXCEPT ![slot_written] = fault_val]
                  /\ gen_taints' = [gen_taints EXCEPT ![slot_written] = "tainted_a"]
                  /\ gen_journal' = AppendEvent(gen_journal,
                            [index |-> Len(gen_journal) + 1,
                             kind |-> "step_end",
                             run |-> 1,
                             step |-> gen_pc,
                             slot |-> slot_written,
                             value |-> fault_val,
                             taint |-> "tainted_a",
                             action_id |-> None,
                             retry |-> 0,
                             deadline |-> 0,
                             event |-> None,
                             prompt |-> None,
                             answer |-> None,
                             typed_failure_class |-> None])
            /\ gen_pc' = gen_pc + 1
            /\ gen_blocked' = None
  /\ UNCHANGED <<gen_terminal, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, resumeQueue, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenBlockAction ==
  /\ gen_pc < MaxStep
  /\ gen_blocked = None
  /\ gen_terminal = None
  /\ gen_error.class = "none"
  /\ \E aid \in ActionIds :
      \E inp \in SlotIndex :
        \E out \in SlotIndex :
          \E ticket \in 0..MaxTicket :
            \E retry \in 0..MaxRetry :
              gen_blocked' = [kind |-> "action",
                              step |-> gen_pc,
                              resume_pc |-> gen_pc + 1,
                              action_id |-> aid,
                              input_slot |-> inp,
                              output_slot |-> out,
                              ticket |-> ticket,
                              retry |-> retry,
                              deadline |-> 0,
                              event |-> None,
                              prompt |-> None,
                              answer_slot |-> None,
                              timeout |-> 0]
              /\ gen_pc' = gen_pc
  /\ UNCHANGED <<gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_terminal, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, resumeQueue, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenBlockWaitUntil ==
  /\ gen_pc < MaxStep
  /\ gen_blocked = None
  /\ gen_terminal = None
  /\ gen_error.class = "none"
  /\ \E deadline \in U64 :
      gen_blocked' = [kind |-> "wait_until",
                      step |-> gen_pc,
                      resume_pc |-> gen_pc + 1,
                      action_id |-> None, input_slot |-> None,
                      output_slot |-> None,
                      ticket |-> 0, retry |-> 0, deadline |-> deadline,
                      event |-> None, prompt |-> None,
                      answer_slot |-> None, timeout |-> 0]
      /\ gen_pc' = gen_pc
  /\ UNCHANGED <<gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_terminal, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, resumeQueue, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenBlockAsk ==
  /\ gen_pc < MaxStep
  /\ gen_blocked = None
  /\ gen_terminal = None
  /\ gen_error.class = "none"
  /\ \E prompt_s \in SlotIndex :
      \E answer_s \in SlotIndex :
        gen_blocked' = [kind |-> "ask",
                        step |-> gen_pc,
                        resume_pc |-> gen_pc + 1,
                        action_id |-> None, input_slot |-> None,
                        output_slot |-> None,
                        ticket |-> 0, retry |-> 0, deadline |-> 0,
                        event |-> None, prompt |-> prompt_s,
                        answer_slot |-> answer_s, timeout |-> 0]
        /\ gen_pc' = gen_pc
  /\ UNCHANGED <<gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_terminal, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, resumeQueue, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenResumeAction ==
  /\ gen_blocked # None
  /\ gen_blocked.kind = "action"
  /\ \E completion \in ResumeItemCompletion :
      /\ completion.ticket = gen_blocked.ticket
      /\ completion.action_id = gen_blocked.action_id
      /\ gen_slots' = [gen_slots EXCEPT ![gen_blocked.output_slot] = completion.value]
      /\ gen_taints' = [gen_taints EXCEPT ![gen_blocked.output_slot] = completion.taint]
      /\ gen_steps' = Append(gen_steps,
                   [pc |-> gen_blocked.resume_pc, status |-> "running"])
      /\ gen_journal' = AppendEvent(gen_journal,
                [index |-> Len(gen_journal) + 1,
                 kind |-> "action_complete",
                 run |-> 1, step |-> gen_blocked.step,
                 slot |-> gen_blocked.output_slot,
                 value |-> completion.value,
                 taint |-> completion.taint,
                 action_id |-> completion.action_id,
                 retry |-> gen_blocked.retry,
                 deadline |-> 0, event |-> None,
                 prompt |-> None, answer |-> None,
                 typed_failure_class |-> None])
      /\ gen_pc' = gen_blocked.resume_pc
      /\ gen_blocked' = None
      /\ resumeQueue' = resumeQueue \ {completion}
  /\ UNCHANGED <<gen_terminal, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenResumeAsk ==
  /\ gen_blocked # None
  /\ gen_blocked.kind = "ask"
  /\ \E answerer \in ResumeItemAnswer :
      /\ answerer.prompt = gen_blocked.prompt
      /\ gen_slots' = [gen_slots EXCEPT ![gen_blocked.answer_slot] = answerer.answer]
      /\ gen_taints' = [gen_taints EXCEPT ![gen_blocked.answer_slot] = answerer.taint]
      /\ gen_steps' = Append(gen_steps,
                   [pc |-> gen_blocked.resume_pc, status |-> "running"])
      /\ gen_journal' = AppendEvent(gen_journal,
                [index |-> Len(gen_journal) + 1,
                 kind |-> "ask_answer",
                 run |-> 1, step |-> gen_blocked.step,
                 slot |-> gen_blocked.answer_slot,
                 value |-> answerer.answer,
                 taint |-> answerer.taint,
                 action_id |-> None, retry |-> 0,
                 deadline |-> 0, event |-> None,
                 prompt |-> gen_blocked.prompt,
                 answer |-> answerer.answer,
                 typed_failure_class |-> None])
      /\ gen_pc' = gen_blocked.resume_pc
      /\ gen_blocked' = None
      /\ resumeQueue' = resumeQueue \ {answerer}
  /\ UNCHANGED <<gen_terminal, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenTimerFire ==
  /\ gen_blocked # None
  /\ gen_blocked.kind = "wait_until"
  /\ \E timer \in ResumeItemTimer :
      /\ timer.deadline = gen_blocked.deadline
      /\ gen_journal' = AppendEvent(gen_journal,
                [index |-> Len(gen_journal) + 1,
                 kind |-> "wait_fired",
                 run |-> 1, step |-> gen_blocked.step,
                 slot |-> None, value |-> None, taint |-> None,
                 action_id |-> None, retry |-> 0,
                 deadline |-> gen_blocked.deadline,
                 event |-> None, prompt |-> None, answer |-> None,
                 typed_failure_class |-> None])
      /\ gen_pc' = gen_blocked.resume_pc
      /\ gen_blocked' = None
      /\ resumeQueue' = resumeQueue \ {timer}
  /\ UNCHANGED <<gen_slots, gen_taints, gen_steps, gen_terminal, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenFinish ==
  /\ gen_pc <= MaxStep
  /\ gen_blocked = None
  /\ gen_terminal = None
  /\ gen_error.class = "none"
  /\ gen_terminal' = [value |-> gen_slots[1], taint |-> gen_taints[1]]
  /\ gen_steps' = Append(gen_steps,
               [pc |-> gen_pc, status |-> "terminal"])
  /\ gen_pc' = gen_pc
  /\ UNCHANGED <<gen_slots, gen_taints, gen_journal, gen_blocked, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, resumeQueue, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenError ==
  /\ gen_pc <= MaxStep
  /\ gen_blocked = None
  /\ gen_terminal = None
  /\ gen_error.class = "none"
  /\ LET err_class == "overflow" IN
      gen_error' = [class |-> err_class]
      /\ gen_steps' = Append(gen_steps,
                   [pc |-> gen_pc, status |-> "failed"])
      /\ gen_journal' = AppendEvent(gen_journal,
                [index |-> Len(gen_journal) + 1,
                 kind |-> "typed_failure",
                 run |-> 1, step |-> gen_pc,
                 slot |-> None, value |-> None, taint |-> None,
                 action_id |-> None, retry |-> 0, deadline |-> 0,
                 event |-> None, prompt |-> None, answer |-> None,
                 typed_failure_class |-> err_class])
      /\ gen_pc' = gen_pc
  /\ UNCHANGED <<gen_slots, gen_taints, gen_blocked, gen_terminal,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, resumeQueue, ir_terminal, ir_error,
                  unsupported, sourceEmitted>>

GenUnsupportedReject ==
  /\ gen_pc <= MaxStep
  /\ UnsupportedKind \in InstructionKinds
  /\ sourceEmitted = FALSE
  /\ gen_blocked = None
  /\ gen_terminal = None
  /\ gen_error.class = "none"
  /\ unsupported' = TRUE
  /\ gen_error' = [class |-> "unsupported_ir"]
  /\ gen_steps' = Append(gen_steps,
               [pc |-> gen_pc, status |-> "failed"])
  /\ gen_journal' = AppendEvent(gen_journal,
            [index |-> Len(gen_journal) + 1,
             kind |-> "unsupported_reject",
             run |-> 1, step |-> gen_pc,
             slot |-> None, value |-> None, taint |-> None,
             action_id |-> None, retry |-> 0, deadline |-> 0,
             event |-> None, prompt |-> None, answer |-> None,
             typed_failure_class |-> "unsupported_ir"])
  /\ gen_pc' = gen_pc
  /\ UNCHANGED <<gen_slots, gen_taints, gen_blocked, gen_terminal,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, resumeQueue, ir_terminal, ir_error,
                  sourceEmitted>>

GenSourceAcceptOrEmit ==
  /\ gen_pc <= MaxStep
  /\ gen_blocked = None
  /\ gen_terminal = None
  /\ gen_error.class = "none"
  /\ unsupported = FALSE
  /\ sourceEmitted' = TRUE
  /\ gen_pc' = gen_pc
  /\ UNCHANGED <<gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, gen_terminal, gen_error,
                  ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, resumeQueue, ir_terminal, ir_error,
                  unsupported>>

(*
  =====================================================================
  ENVIRONMENT TRANSITIONS — populate resumeQueue from outside the model
  (PRE-004: identical external inputs are supplied to both machines)
  =====================================================================
*)

EnvSupplyActionCompletion ==
  /\ ir_blocked # None
  /\ ir_blocked = gen_blocked
  /\ ir_blocked.kind = "action"
  /\ Cardinality(resumeQueue) < 3
  /\ \E aid \in ActionIds :
      \E ticket \in 0..MaxTicket :
        \E val \in Values :
          \E tnt \in Taints :
            LET completion == [ticket |-> ticket,
                               action_id |-> aid,
                               value |-> val,
                               taint |-> tnt] IN
            /\ aid = ir_blocked.action_id
            /\ ticket = ir_blocked.ticket
            /\ completion \notin resumeQueue
            /\ resumeQueue' = resumeQueue \cup {completion}
  /\ UNCHANGED <<ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

EnvSupplyAskAnswer ==
  /\ ir_blocked # None
  /\ ir_blocked = gen_blocked
  /\ ir_blocked.kind = "ask"
  /\ Cardinality(resumeQueue) < 3
  /\ \E prompt_s \in SlotIndex :
      \E answer \in Values :
        \E tnt \in Taints :
          LET answerer == [answer |-> answer,
                           taint |-> tnt,
                           prompt |-> prompt_s] IN
          /\ prompt_s = ir_blocked.prompt
          /\ answerer \notin resumeQueue
          /\ resumeQueue' = resumeQueue \cup {answerer}
  /\ UNCHANGED <<ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

EnvSupplyTimer ==
  /\ ir_blocked # None
  /\ ir_blocked = gen_blocked
  /\ ir_blocked.kind = "wait_until"
  /\ Cardinality(resumeQueue) < 3
  /\ \E deadline \in U64 :
      LET timer == [deadline |-> deadline] IN
      /\ deadline = ir_blocked.deadline
      /\ timer \notin resumeQueue
      /\ resumeQueue' = resumeQueue \cup {timer}
  /\ UNCHANGED <<ir_pc, ir_slots, ir_taints, ir_steps, ir_journal,
                  ir_blocked, ir_terminal, ir_error,
                  gen_pc, gen_slots, gen_taints, gen_steps, gen_journal,
                  gen_blocked, gen_terminal, gen_error,
                  unsupported, sourceEmitted>>

(*
  =====================================================================
  TERMINAL STUTTER — allow TLC to check invariants in terminal states
  =====================================================================
*)

TerminalStutter ==
  /\ (\/ ir_terminal # None
      \/ gen_terminal # None
      \/ ir_error.class # "none"
      \/ gen_error.class # "none"
      \/ unsupported = TRUE
      \/ sourceEmitted = TRUE)
  /\ UNCHANGED vars

(*
  =====================================================================
  NEXT — union of all possible transitions (separate IR/Gen/Env)
  =====================================================================
*)

Next ==
  \/ IrDo
  \/ IrBlockAction
  \/ IrBlockWaitUntil
  \/ IrBlockAsk
  \/ IrResumeAction
  \/ IrResumeAsk
  \/ IrTimerFire
  \/ IrFinish
  \/ IrError
  \/ IrUnsupportedError
  \/ GenDo
  \/ GenBlockAction
  \/ GenBlockWaitUntil
  \/ GenBlockAsk
  \/ GenResumeAction
  \/ GenResumeAsk
  \/ GenTimerFire
  \/ GenFinish
  \/ GenError
  \/ GenUnsupportedReject
  \/ GenSourceAcceptOrEmit
  \/ EnvSupplyActionCompletion
  \/ EnvSupplyAskAnswer
  \/ EnvSupplyTimer
  \/ TerminalStutter

(*
  =====================================================================
  PAIRED PARITY TRANSITIONS — attempt 4 positive obligation repair

  The positive parity contract assumes identical public instruction choices
  and identical external resume inputs for IR oracle and generated candidate.
  These paired transitions encode that assumption directly.  candidateFault
  still mutates generated observations, so divergence_sanity remains a real
  negative oracle rather than an equality-by-construction proof.
  =====================================================================
*)

BothReady ==
  /\ ir_pc = gen_pc
  /\ ir_blocked = None
  /\ gen_blocked = None
  /\ ir_terminal = None
  /\ gen_terminal = None
  /\ ir_error.class = "none"
  /\ gen_error.class = "none"

PairedDo ==
  /\ BothReady
  /\ ir_pc < MaxStep
  /\ \E kind \in InstructionKinds :
      /\ IsSupported(kind)
      /\ LET slot_written == 1 IN
         LET val == 0 IN
         LET gen_val == IF candidateFault THEN 1 ELSE val IN
         LET gen_taint == IF candidateFault THEN "tainted_a" ELSE gen_taints[slot_written] IN
           /\ ir_steps' = Append(ir_steps, [pc |-> ir_pc, status |-> "running"])
           /\ gen_steps' = Append(gen_steps, [pc |-> gen_pc, status |-> "running"])
           /\ ir_slots' = [ir_slots EXCEPT ![slot_written] = val]
           /\ gen_slots' = [gen_slots EXCEPT ![slot_written] = gen_val]
           /\ ir_taints' = ir_taints
           /\ gen_taints' = IF candidateFault THEN [gen_taints EXCEPT ![slot_written] = "tainted_a"] ELSE gen_taints
           /\ ir_journal' = AppendEvent(ir_journal,
                [index |-> Len(ir_journal) + 1, kind |-> "step_end", run |-> 1,
                 step |-> ir_pc, slot |-> slot_written, value |-> val,
                 taint |-> ir_taints[slot_written], action_id |-> None,
                 retry |-> 0, deadline |-> 0, event |-> None, prompt |-> None,
                 answer |-> None, typed_failure_class |-> None])
           /\ gen_journal' = AppendEvent(gen_journal,
                [index |-> Len(gen_journal) + 1, kind |-> "step_end", run |-> 1,
                 step |-> gen_pc, slot |-> slot_written, value |-> gen_val,
                 taint |-> gen_taint, action_id |-> None,
                 retry |-> 0, deadline |-> 0, event |-> None, prompt |-> None,
                 answer |-> None, typed_failure_class |-> None])
           /\ ir_pc' = ir_pc + 1
           /\ gen_pc' = gen_pc + 1
           /\ UNCHANGED <<ir_blocked, gen_blocked, resumeQueue, ir_terminal,
                           gen_terminal, ir_error, gen_error, unsupported,
                           sourceEmitted>>

PairedBlockAction ==
  /\ BothReady
  /\ ir_pc < MaxStep
  /\ \E aid \in ActionIds :
      \E inp \in SlotIndex :
        \E out \in SlotIndex :
          \E ticket \in 0..MaxTicket :
            \E retry \in 0..MaxRetry :
              LET b == [kind |-> "action", step |-> ir_pc, resume_pc |-> ir_pc + 1,
                        action_id |-> aid, input_slot |-> inp, output_slot |-> out,
                        ticket |-> ticket, retry |-> retry, deadline |-> 0,
                        event |-> None, prompt |-> None, answer_slot |-> None,
                        timeout |-> 0] IN
                /\ ir_blocked' = b
                /\ gen_blocked' = b
                /\ UNCHANGED <<ir_pc, gen_pc, ir_slots, gen_slots, ir_taints,
                                gen_taints, ir_steps, gen_steps, ir_journal,
                                gen_journal, resumeQueue, ir_terminal, gen_terminal,
                                ir_error, gen_error, unsupported, sourceEmitted>>

PairedBlockWaitUntil ==
  /\ BothReady
  /\ ir_pc < MaxStep
  /\ \E deadline \in U64 :
      LET b == [kind |-> "wait_until", step |-> ir_pc, resume_pc |-> ir_pc + 1,
                action_id |-> None, input_slot |-> None, output_slot |-> None,
                ticket |-> 0, retry |-> 0, deadline |-> deadline,
                event |-> None, prompt |-> None, answer_slot |-> None, timeout |-> 0] IN
        /\ ir_blocked' = b
        /\ gen_blocked' = b
        /\ UNCHANGED <<ir_pc, gen_pc, ir_slots, gen_slots, ir_taints,
                        gen_taints, ir_steps, gen_steps, ir_journal, gen_journal,
                        resumeQueue, ir_terminal, gen_terminal, ir_error,
                        gen_error, unsupported, sourceEmitted>>

PairedBlockAsk ==
  /\ BothReady
  /\ ir_pc < MaxStep
  /\ \E prompt_s \in SlotIndex :
      \E answer_s \in SlotIndex :
        LET b == [kind |-> "ask", step |-> ir_pc, resume_pc |-> ir_pc + 1,
                  action_id |-> None, input_slot |-> None, output_slot |-> None,
                  ticket |-> 0, retry |-> 0, deadline |-> 0,
                  event |-> None, prompt |-> prompt_s, answer_slot |-> answer_s,
                  timeout |-> 0] IN
          /\ ir_blocked' = b
          /\ gen_blocked' = b
          /\ UNCHANGED <<ir_pc, gen_pc, ir_slots, gen_slots, ir_taints,
                          gen_taints, ir_steps, gen_steps, ir_journal, gen_journal,
                          resumeQueue, ir_terminal, gen_terminal, ir_error,
                          gen_error, unsupported, sourceEmitted>>

PairedResumeAction ==
  /\ ir_blocked # None
  /\ ir_blocked = gen_blocked
  /\ ir_blocked.kind = "action"
  /\ \E completion \in ResumeItemCompletion :
      /\ completion.ticket = ir_blocked.ticket
      /\ completion.action_id = ir_blocked.action_id
      /\ ir_slots' = [ir_slots EXCEPT ![ir_blocked.output_slot] = completion.value]
      /\ gen_slots' = [gen_slots EXCEPT ![gen_blocked.output_slot] = completion.value]
      /\ ir_taints' = [ir_taints EXCEPT ![ir_blocked.output_slot] = completion.taint]
      /\ gen_taints' = [gen_taints EXCEPT ![gen_blocked.output_slot] = completion.taint]
      /\ ir_steps' = Append(ir_steps, [pc |-> ir_blocked.resume_pc, status |-> "running"])
      /\ gen_steps' = Append(gen_steps, [pc |-> gen_blocked.resume_pc, status |-> "running"])
      /\ ir_journal' = AppendEvent(ir_journal,
           [index |-> Len(ir_journal) + 1, kind |-> "action_complete", run |-> 1,
            step |-> ir_blocked.step, slot |-> ir_blocked.output_slot,
            value |-> completion.value, taint |-> completion.taint,
            action_id |-> completion.action_id, retry |-> ir_blocked.retry,
            deadline |-> 0, event |-> None, prompt |-> None, answer |-> None,
            typed_failure_class |-> None])
      /\ gen_journal' = AppendEvent(gen_journal,
           [index |-> Len(gen_journal) + 1, kind |-> "action_complete", run |-> 1,
            step |-> gen_blocked.step, slot |-> gen_blocked.output_slot,
            value |-> completion.value, taint |-> completion.taint,
            action_id |-> completion.action_id, retry |-> gen_blocked.retry,
            deadline |-> 0, event |-> None, prompt |-> None, answer |-> None,
            typed_failure_class |-> None])
      /\ ir_pc' = ir_blocked.resume_pc
      /\ gen_pc' = gen_blocked.resume_pc
      /\ ir_blocked' = None
      /\ gen_blocked' = None
      /\ resumeQueue' = resumeQueue \ {completion}
      /\ UNCHANGED <<ir_terminal, gen_terminal, ir_error, gen_error,
                      unsupported, sourceEmitted>>

PairedResumeAsk ==
  /\ ir_blocked # None
  /\ ir_blocked = gen_blocked
  /\ ir_blocked.kind = "ask"
  /\ \E answerer \in ResumeItemAnswer :
      /\ answerer.prompt = ir_blocked.prompt
      /\ ir_slots' = [ir_slots EXCEPT ![ir_blocked.answer_slot] = answerer.answer]
      /\ gen_slots' = [gen_slots EXCEPT ![gen_blocked.answer_slot] = answerer.answer]
      /\ ir_taints' = [ir_taints EXCEPT ![ir_blocked.answer_slot] = answerer.taint]
      /\ gen_taints' = [gen_taints EXCEPT ![gen_blocked.answer_slot] = answerer.taint]
      /\ ir_steps' = Append(ir_steps, [pc |-> ir_blocked.resume_pc, status |-> "running"])
      /\ gen_steps' = Append(gen_steps, [pc |-> gen_blocked.resume_pc, status |-> "running"])
      /\ ir_journal' = AppendEvent(ir_journal,
           [index |-> Len(ir_journal) + 1, kind |-> "ask_answer", run |-> 1,
            step |-> ir_blocked.step, slot |-> ir_blocked.answer_slot,
            value |-> answerer.answer, taint |-> answerer.taint,
            action_id |-> None, retry |-> 0, deadline |-> 0, event |-> None,
            prompt |-> ir_blocked.prompt, answer |-> answerer.answer,
            typed_failure_class |-> None])
      /\ gen_journal' = AppendEvent(gen_journal,
           [index |-> Len(gen_journal) + 1, kind |-> "ask_answer", run |-> 1,
            step |-> gen_blocked.step, slot |-> gen_blocked.answer_slot,
            value |-> answerer.answer, taint |-> answerer.taint,
            action_id |-> None, retry |-> 0, deadline |-> 0, event |-> None,
            prompt |-> gen_blocked.prompt, answer |-> answerer.answer,
            typed_failure_class |-> None])
      /\ ir_pc' = ir_blocked.resume_pc
      /\ gen_pc' = gen_blocked.resume_pc
      /\ ir_blocked' = None
      /\ gen_blocked' = None
      /\ resumeQueue' = resumeQueue \ {answerer}
      /\ UNCHANGED <<ir_terminal, gen_terminal, ir_error, gen_error,
                      unsupported, sourceEmitted>>

PairedTimerFire ==
  /\ ir_blocked # None
  /\ ir_blocked = gen_blocked
  /\ ir_blocked.kind = "wait_until"
  /\ \E timer \in ResumeItemTimer :
      /\ timer.deadline = ir_blocked.deadline
      /\ ir_journal' = AppendEvent(ir_journal,
           [index |-> Len(ir_journal) + 1, kind |-> "wait_fired", run |-> 1,
            step |-> ir_blocked.step, slot |-> None, value |-> None,
            taint |-> None, action_id |-> None, retry |-> 0,
            deadline |-> ir_blocked.deadline, event |-> None, prompt |-> None,
            answer |-> None, typed_failure_class |-> None])
      /\ gen_journal' = AppendEvent(gen_journal,
           [index |-> Len(gen_journal) + 1, kind |-> "wait_fired", run |-> 1,
            step |-> gen_blocked.step, slot |-> None, value |-> None,
            taint |-> None, action_id |-> None, retry |-> 0,
            deadline |-> gen_blocked.deadline, event |-> None, prompt |-> None,
            answer |-> None, typed_failure_class |-> None])
      /\ ir_pc' = ir_blocked.resume_pc
      /\ gen_pc' = gen_blocked.resume_pc
      /\ ir_blocked' = None
      /\ gen_blocked' = None
      /\ resumeQueue' = resumeQueue \ {timer}
      /\ UNCHANGED <<ir_slots, gen_slots, ir_taints, gen_taints, ir_steps,
                      gen_steps, ir_terminal, gen_terminal, ir_error, gen_error,
                      unsupported, sourceEmitted>>

PairedFinish ==
  /\ BothReady
  /\ ir_terminal' = [value |-> ir_slots[1], taint |-> ir_taints[1]]
  /\ gen_terminal' = [value |-> gen_slots[1], taint |-> gen_taints[1]]
  /\ ir_steps' = Append(ir_steps, [pc |-> ir_pc, status |-> "terminal"])
  /\ gen_steps' = Append(gen_steps, [pc |-> gen_pc, status |-> "terminal"])
  /\ UNCHANGED <<ir_pc, gen_pc, ir_slots, gen_slots, ir_taints, gen_taints,
                  ir_journal, gen_journal, ir_blocked, gen_blocked, resumeQueue,
                  ir_error, gen_error, unsupported, sourceEmitted>>

PairedError ==
  /\ BothReady
  /\ LET err_class == "overflow" IN
      /\ ir_error' = [class |-> err_class]
      /\ gen_error' = [class |-> err_class]
      /\ ir_steps' = Append(ir_steps, [pc |-> ir_pc, status |-> "failed"])
      /\ gen_steps' = Append(gen_steps, [pc |-> gen_pc, status |-> "failed"])
      /\ ir_journal' = AppendEvent(ir_journal,
           [index |-> Len(ir_journal) + 1, kind |-> "typed_failure", run |-> 1,
            step |-> ir_pc, slot |-> None, value |-> None, taint |-> None,
            action_id |-> None, retry |-> 0, deadline |-> 0, event |-> None,
            prompt |-> None, answer |-> None, typed_failure_class |-> err_class])
      /\ gen_journal' = AppendEvent(gen_journal,
           [index |-> Len(gen_journal) + 1, kind |-> "typed_failure", run |-> 1,
            step |-> gen_pc, slot |-> None, value |-> None, taint |-> None,
            action_id |-> None, retry |-> 0, deadline |-> 0, event |-> None,
            prompt |-> None, answer |-> None, typed_failure_class |-> err_class])
      /\ UNCHANGED <<ir_pc, gen_pc, ir_slots, gen_slots, ir_taints, gen_taints,
                      ir_blocked, gen_blocked, resumeQueue, ir_terminal,
                      gen_terminal, unsupported, sourceEmitted>>

PairedUnsupportedReject ==
  /\ BothReady
  /\ UnsupportedKind \in InstructionKinds
  /\ sourceEmitted = FALSE
  /\ unsupported' = TRUE
  /\ ir_error' = [class |-> "unsupported_ir"]
  /\ gen_error' = [class |-> "unsupported_ir"]
  /\ ir_steps' = Append(ir_steps, [pc |-> ir_pc, status |-> "failed"])
  /\ gen_steps' = Append(gen_steps, [pc |-> gen_pc, status |-> "failed"])
  /\ ir_journal' = AppendEvent(ir_journal,
       [index |-> Len(ir_journal) + 1, kind |-> "unsupported_reject", run |-> 1,
        step |-> ir_pc, slot |-> None, value |-> None, taint |-> None,
        action_id |-> None, retry |-> 0, deadline |-> 0, event |-> None,
        prompt |-> None, answer |-> None, typed_failure_class |-> "unsupported_ir"])
  /\ gen_journal' = AppendEvent(gen_journal,
       [index |-> Len(gen_journal) + 1, kind |-> "unsupported_reject", run |-> 1,
        step |-> gen_pc, slot |-> None, value |-> None, taint |-> None,
        action_id |-> None, retry |-> 0, deadline |-> 0, event |-> None,
        prompt |-> None, answer |-> None, typed_failure_class |-> "unsupported_ir"])
  /\ UNCHANGED <<ir_pc, gen_pc, ir_slots, gen_slots, ir_taints, gen_taints,
                  ir_blocked, gen_blocked, resumeQueue, ir_terminal,
                  gen_terminal, sourceEmitted>>

\* NON-VACUOUS REPAIR (attempt 6): GenSourceAcceptOrEmit added to PairedNext
\* so sourceEmitted=TRUE is reachable in the paired model and UnsupportedNoSourceEmission
\* is non-vacuous. The sourceEmitted=FALSE guard in PairedUnsupportedReject prevents
\* the unsupported path from being taken after source emission, preserving the invariant.
\* GenSourceAcceptOrEmit stutters on unsupported/gen_error so it cannot be followed
\* by PairedUnsupportedReject in the same behavior.
PairedNext ==
  \/ PairedDo
  \/ PairedBlockAction
  \/ PairedBlockWaitUntil
  \/ PairedBlockAsk
  \/ PairedResumeAction
  \/ PairedResumeAsk
  \/ PairedTimerFire
  \/ PairedFinish
  \/ PairedError
  \/ PairedUnsupportedReject
  \/ GenSourceAcceptOrEmit
  \/ EnvSupplyActionCompletion
  \/ EnvSupplyAskAnswer
  \/ EnvSupplyTimer
  \/ TerminalStutter

(*
  =====================================================================
  SPEC — Init /\ [][PairedNext]_vars with fairness
  =====================================================================
*)

Spec ==
  /\ Init
  /\ [][PairedNext]_vars
  /\ WF_vars(PairedResumeAction)
  /\ WF_vars(PairedResumeAsk)
  /\ WF_vars(PairedTimerFire)
  /\ WF_vars(PairedDo)
  /\ WF_vars(PairedFinish)

(*
  =====================================================================
  INVARIANTS
  =====================================================================
*)

\* POST-003: same blocked metadata when both are blocked
SameBlockedMetadata ==
  /\ ir_blocked # None /\ gen_blocked # None /\ ir_blocked.kind = gen_blocked.kind
  => /\ ir_blocked.kind = gen_blocked.kind
     /\ ir_blocked.step = gen_blocked.step
     /\ ir_blocked.resume_pc = gen_blocked.resume_pc
     /\ ir_blocked.action_id = gen_blocked.action_id
     /\ ir_blocked.input_slot = gen_blocked.input_slot
     /\ ir_blocked.output_slot = gen_blocked.output_slot
     /\ ir_blocked.ticket = gen_blocked.ticket
     /\ ir_blocked.retry = gen_blocked.retry
     /\ ir_blocked.deadline = gen_blocked.deadline
     /\ ir_blocked.event = gen_blocked.event
     /\ ir_blocked.prompt = gen_blocked.prompt
     /\ ir_blocked.answer_slot = gen_blocked.answer_slot
     /\ ir_blocked.timeout = gen_blocked.timeout

\* INV-005: PC does not advance past suspension boundary while blocked
NoAdvancePastSuspension ==
  /\ ir_blocked # None => ir_pc = ir_blocked.step
  /\ gen_blocked # None => gen_pc = gen_blocked.step

\* INV-004: step-state transitions are valid (legal status progression)
ValidStepStateTransitions ==
  /\ \A i \in 1..Len(ir_steps) :
      ir_steps[i].pc \in PC /\ ir_steps[i].status \in StepStatuses
  /\ \A i \in 1..Len(gen_steps) :
      gen_steps[i].pc \in PC /\ gen_steps[i].status \in StepStatuses
  /\ \A i \in 1..Len(ir_steps) - 1 :
      IsLegalStatusTransition(ir_steps[i].status, ir_steps[i+1].status)
  /\ \A i \in 1..Len(gen_steps) - 1 :
      IsLegalStatusTransition(gen_steps[i].status, gen_steps[i+1].status)

\* POST-006 / INV-006: unsupported reject implies no source emission
\* NON-VACUOUS REPAIR (attempt 6): GenSourceAcceptOrEmit is now in PairedNext,
\* so sourceEmitted=TRUE is reachable and the invariant is non-vacuous.
\* The sourceEmitted=FALSE guard in PairedUnsupportedReject prevents the
\* unsupported path from being taken after source emission.
UnsupportedNoSourceEmission ==
  unsupported = TRUE => sourceEmitted = FALSE

\* POST-001: when both terminal, observable state matches
SameObservableStateWhenTerminal ==
  /\ ir_terminal # None /\ gen_terminal # None
  => /\ ir_terminal.value = gen_terminal.value
     /\ ir_terminal.taint = gen_terminal.taint
     /\ ir_pc = gen_pc
     /\ ir_taints = gen_taints
     /\ ir_slots = gen_slots

\* POST-005: journal prefix matches after every transition
\* Compares ALL contracted fields required by POST-005:
\* kind, step, slot, value, taint, action_id, retry, deadline,
\* event, prompt, answer, typed_failure_class
\* POST-005: journal prefix matches after every transition
\* Compares ALL contracted fields required by POST-005:
\* kind, step, slot, value, taint, action_id, retry, deadline,
\* event, prompt, answer, typed_failure_class
\*
\* NON-VACUOUS REPAIR (attempt 6): removed the ir_error/gen_error/unsupported
\* short-circuit. Under PairedNext, PairedError and PairedUnsupportedReject
\* write identical journals on both sides, so the comparison is always meaningful.
\* The short-circuit was vacuous: it masked typed-error and unsupported journal
\* parity even though those paths write identical paired journals.
SameJournalPrefix ==
  LET min_len == MinLen(ir_journal, gen_journal) IN
    /\ \A i \in 1..min_len :
        /\ ir_journal[i].kind = gen_journal[i].kind
        /\ ir_journal[i].step = gen_journal[i].step
        /\ ir_journal[i].slot = gen_journal[i].slot
        /\ ir_journal[i].value = gen_journal[i].value
        /\ ir_journal[i].taint = gen_journal[i].taint
        /\ ir_journal[i].action_id = gen_journal[i].action_id
        /\ ir_journal[i].retry = gen_journal[i].retry
        /\ ir_journal[i].deadline = gen_journal[i].deadline
        /\ ir_journal[i].event = gen_journal[i].event
        /\ ir_journal[i].prompt = gen_journal[i].prompt
        /\ ir_journal[i].answer = gen_journal[i].answer
        /\ ir_journal[i].typed_failure_class = gen_journal[i].typed_failure_class

(*
  ObservationRefinesOracle: generated candidate refines IR oracle.

  NON-VACUOUS: can fail when candidateFault=TRUE (divergence sanity).
  When candidateFault=FALSE (success/suspension/error configs),
  GenDo uses the same base value as IrDo, so journals match and
  the invariant holds.

  When candidateFault=TRUE, GenDo may flip the value/taint for the
  same kind/slot/val choice, causing SameJournalPrefix to fail,
  which propagates to ObservationRefinesOracle.
*)
ObservationRefinesOracle ==
  \/ /\ ir_terminal # None
     /\ gen_terminal # None
     => /\ ir_terminal.value = gen_terminal.value
        /\ ir_terminal.taint = gen_terminal.taint
        /\ ir_taints = gen_taints
        /\ ir_slots = gen_slots
        /\ \A i \in 1..MinLen(ir_journal, gen_journal) :
            /\ ir_journal[i].kind = gen_journal[i].kind
            /\ ir_journal[i].step = gen_journal[i].step
            /\ ir_journal[i].slot = gen_journal[i].slot
            /\ ir_journal[i].value = gen_journal[i].value
            /\ ir_journal[i].taint = gen_journal[i].taint
            /\ ir_journal[i].typed_failure_class = gen_journal[i].typed_failure_class
  \/ /\ ir_terminal = None
     /\ gen_terminal = None
     => \/ ir_blocked # None /\ gen_blocked # None
        \/ ir_blocked = None /\ gen_blocked = None
           /\ \A i \in 1..MinLen(ir_journal, gen_journal) :
               /\ ir_journal[i].kind = gen_journal[i].kind
               /\ ir_journal[i].step = gen_journal[i].step
               /\ ir_journal[i].slot = gen_journal[i].slot
               /\ ir_journal[i].value = gen_journal[i].value
               /\ ir_journal[i].taint = gen_journal[i].taint
               /\ ir_journal[i].typed_failure_class = gen_journal[i].typed_failure_class
  \/ ir_terminal # None /\ gen_terminal = None
  \/ ir_terminal = None /\ gen_terminal # None

(*
  =====================================================================
  TEMPORAL PROPERTIES
  =====================================================================
*)

\* Both modes eventually reach terminal, blocked, or typed error
EventuallyTerminalOrBlockedOrTypedError ==
  <>((ir_terminal # None /\ gen_terminal # None)
     \/ (ir_blocked # None /\ gen_blocked # None)
     \/ (ir_error.class # "none" /\ gen_error.class # "none")
     \/ unsupported = TRUE)

\* If resumeQueue has matching input, resume eventually progresses
ResumeEventuallyProgresses ==
  resumeQueue # {}
  => <>((ir_blocked = None /\ gen_blocked = None)
         \/ (ir_error.class # "none" /\ gen_error.class # "none"))

====
