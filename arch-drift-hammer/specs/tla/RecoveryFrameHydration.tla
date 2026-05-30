(* RecoveryFrameHydration.tla
 *
 * Frame reconstruction from RecoveryFrameSeed, matching crates/vb_runtime/src/recovery.rs.
 *
 * Hydration pipeline (recovery.rs:64-71):
 *   1. reject_unsupported_live_frame_state (line 74-83)
 *   2. empty_recovered_frame (line 85-93)
 *   3. apply_recovered_steps (line 95-99)  -- two-phase for Waiting/Asking
 *   4. apply_recovered_slots (line 101-107)
 *   5. apply_recovered_pc (line 109-115)
 *
 * Audit fix (May 2026):
 *   - Increased bounds: MaxStepCount=3, MaxSlotCount=2, MaxPendingActionCount=2
 *   - Replaced integer slot values with opaque model values (MaxSlotValue=1)
 *   - Added DuplicateFreeSteps / DuplicateFreeSlots invariants
 *   - Added combined TypeInvariant
 *
 * Proof obligation: PO-TLA-006
 *)

---- MODULE RecoveryFrameHydration ----

EXTENDS Integers, FiniteSets, Sequences, TLC

CONSTANTS MaxStepCount, MaxSlotCount, MaxPendingActionCount, MaxSlotValue

(* Opaque model values for slot payloads, taint, and action identifiers.
   These abstract the Rust SlotValue enum, Taint enum, and ActionId.
   MaxSlotValue = 1 in the model config; we use a single opaque payload
   to keep the state space bounded while preserving the abstraction. *)
SV_A == "SV_A"
TaintClean == "TaintClean"
Action1 == "Action1"

StepStates == {"Pending", "Running", "Succeeded", "Failed",
               "Skipped", "Waiting", "Asking", "Cancelled"}

RecoveredStepStates == {"Running", "Succeeded", "Failed", "Waiting", "Asking"}

TerminalStates == {"Succeeded", "Failed", "Skipped"}

ResumableStates == {"Pending", "Waiting", "Asking", "Running"}

SlotValues == {SV_A}
TaintValues == {TaintClean}
ActionIds == {Action1}

UnsupportedFlags == [slot_values : BOOLEAN, slot_taint : BOOLEAN,
                     action_payloads : BOOLEAN, pending_actions : BOOLEAN]

VARIABLES seed, frame, hydration, pc, step_apply_method

vars == <<seed, frame, hydration, pc, step_apply_method>>

(* --- Helper sets derived from seed sequences --- *)
StepEntrySet == {seed.steps[i] : i \in DOMAIN seed.steps}

SlotEntrySet == {seed.slots[i] : i \in DOMAIN seed.slots}

PendingActionSet == {seed.pending_actions[i] : i \in DOMAIN seed.pending_actions}

(* Type invariant: seed fields must stay within model bounds *)
SeedTypeInvariant ==
    /\ seed.step_count \in 1..MaxStepCount
    /\ seed.slot_count \in 0..MaxSlotCount
    /\ seed.pc \in 0..(seed.step_count - 1)
    /\ seed.first_step \in 0..(seed.step_count - 1)
    /\ Len(seed.steps) <= seed.step_count
    /\ \A i \in DOMAIN seed.steps :
        /\ seed.steps[i].step \in 0..(seed.step_count - 1)
        /\ seed.steps[i].state \in RecoveredStepStates
    /\ Len(seed.slots) <= seed.slot_count
    /\ \A i \in DOMAIN seed.slots :
        /\ seed.slots[i].slot \in 0..(seed.slot_count - 1)
        /\ seed.slots[i].value \in SlotValues
        /\ seed.slots[i].taint \in TaintValues
    /\ Len(seed.pending_actions) <= MaxPendingActionCount
    /\ \A i \in DOMAIN seed.pending_actions :
        /\ seed.pending_actions[i].step \in 0..(seed.step_count - 1)
        /\ seed.pending_actions[i].action \in ActionIds
    /\ seed.unsupported \in UnsupportedFlags

(* Type invariant: frame fields after hydration *)
FrameTypeInvariant ==
    /\ frame.step_count = seed.step_count
    /\ frame.slot_count = seed.slot_count
    /\ frame.run_id = seed.summary_run
    /\ DOMAIN frame.step_states = 0..(seed.step_count - 1)
    /\ \A i \in DOMAIN frame.step_states : frame.step_states[i] \in StepStates
    /\ DOMAIN frame.slots = 0..(seed.slot_count - 1)
    /\ \A i \in DOMAIN frame.slots :
        /\ frame.slots[i].value \in SlotValues
        /\ frame.slots[i].taint \in TaintValues
    /\ pc \in 0..seed.step_count
    /\ hydration \in {"Ok", "Err", "Pending"}
    /\ DOMAIN step_apply_method = 0..(seed.step_count - 1)
    /\ \A i \in DOMAIN step_apply_method :
        step_apply_method[i] \in {"None", "Direct", "TwoPhase"}

(* Combined type invariant *)
TypeInvariant == SeedTypeInvariant /\ FrameTypeInvariant

(* No two step entries in seed.steps share the same step index *)
DuplicateFreeSteps ==
    \A i, j \in DOMAIN seed.steps :
        i # j => seed.steps[i].step # seed.steps[j].step

(* No two slot entries in seed.slots share the same slot index *)
DuplicateFreeSlots ==
    \A i, j \in DOMAIN seed.slots :
        i # j => seed.slots[i].slot # seed.slots[j].slot

(* ALL unsupported flags must reject hydration (contract; recovery.rs:74-83) *)
UnsupportedStateRejected ==
    hydration \in {"Ok", "Err"} =>
        ((seed.unsupported.slot_values = TRUE
          \/ seed.unsupported.slot_taint = TRUE
          \/ seed.unsupported.action_payloads = TRUE
          \/ seed.unsupported.pending_actions = TRUE)
         => hydration = "Err")

(* action_payloads=true => hydration is never Ok *)
ActionPayloadsRejected ==
    seed.unsupported.action_payloads = TRUE => hydration # "Ok"

(* PC must be within bounds when hydration succeeds (recovery.rs:109-112) *)
PcValidAfterHydration ==
    hydration = "Ok" => pc < seed.step_count

(* Out-of-bounds PC is only permitted when hydration failed *)
PcOutOfBoundsMeansErr ==
    pc >= seed.step_count => hydration = "Err"

(* PC must point to a resumable step (recovery.rs:116-124) *)
PcResumable ==
    hydration = "Ok" => frame.step_states[pc] \in ResumableStates

(* All recovered step states must be from the valid recovery set (recovery.rs:196-203) *)
StepStateValid ==
    \A entry \in StepEntrySet : entry.state \in RecoveredStepStates

(* If a slot was restored, its value/taint are in valid ranges *)
SlotValueValid ==
    \A entry \in SlotEntrySet :
        /\ entry.value \in SlotValues
        /\ entry.taint \in TaintValues
        /\ entry.slot \in 0..(seed.slot_count - 1)

(* If hydration succeeds, the frame has exactly the recovered step states *)
StepStateMatch ==
    hydration = "Ok" =>
        \A entry \in StepEntrySet :
            frame.step_states[entry.step] = entry.state

(* Steps not mentioned in seed.steps remain Pending *)
UnmentionedStepsPending ==
    hydration = "Ok" =>
        \A i \in 0..(seed.step_count - 1) :
            (~\E entry \in StepEntrySet : entry.step = i)
            => frame.step_states[i] = "Pending"

(* Two-phase application: Waiting/Asking require mark_running THEN mark_waiting/asking.
   This is modeled by recording the application method for each step. *)
TwoPhaseApplication ==
    hydration = "Ok" =>
        \A entry \in StepEntrySet :
            (entry.state \in {"Waiting", "Asking"}) =>
                step_apply_method[entry.step] = "TwoPhase"

(* --- Initialization: all possible recovery seeds are explored directly --- *)
Init ==
    \E new_step_count \in 1..MaxStepCount :
    \E new_slot_count \in 0..MaxSlotCount :
    \E new_pc \in 0..(new_step_count - 1) :
    \E new_unsupported \in UnsupportedFlags :
    \E num_steps \in 0..new_step_count :
    \E num_slots \in 0..new_slot_count :
    \E num_pending \in 0..MaxPendingActionCount :
    \E step_states_seq \in [1..num_steps -> RecoveredStepStates] :
    \E slot_values_seq \in [1..num_slots -> SlotValues] :
    \E slot_taints_seq \in [1..num_slots -> TaintValues] :
    \E pending_actions_seq \in [1..num_pending -> ActionIds] :
    LET step_entries == [i \in 1..num_steps |->
            [step |-> (i - 1) % new_step_count,
             state |-> step_states_seq[i]]]
        slot_entries == [i \in 1..num_slots |->
            [slot |-> IF new_slot_count > 0 THEN (i - 1) % new_slot_count ELSE 0,
             value |-> slot_values_seq[i],
             taint |-> slot_taints_seq[i]]]
        pending_entries == [i \in 1..num_pending |->
            [step |-> (i - 1) % new_step_count,
             action |-> pending_actions_seq[i]]]
    IN
    /\ seed = [
            summary_run |-> 1,
            first_step |-> 0,
            step_count |-> new_step_count,
            slot_count |-> new_slot_count,
            pc |-> new_pc,
            steps |-> step_entries,
            slots |-> slot_entries,
            pending_actions |-> pending_entries,
            unsupported |-> new_unsupported]
    /\ hydration = "Pending"
    /\ frame = [
            run_id |-> 1,
            step_count |-> new_step_count,
            slot_count |-> new_slot_count,
            step_states |-> [i \in 0..(new_step_count - 1) |-> "Pending"],
            slots |-> [i \in 0..(new_slot_count - 1) |->
                [value |-> SV_A, taint |-> TaintClean]]]
    /\ pc = new_pc
    /\ step_apply_method = [i \in 0..(new_step_count - 1) |-> "None"]

(* FullHydration: atomic pipeline matching recovery.rs:64-71 *)
FullHydration ==
    /\ hydration = "Pending"
    /\ (LET rejected ==
             seed.unsupported.slot_values = TRUE
             \/ seed.unsupported.slot_taint = TRUE
             \/ seed.unsupported.action_payloads = TRUE
             \/ seed.unsupported.pending_actions = TRUE

         steps_valid ==
             \A entry \in StepEntrySet : entry.step < seed.step_count

         slots_valid ==
             \A entry \in SlotEntrySet : entry.slot < seed.slot_count

         step_state_at(i) ==
             IF \E entry \in StepEntrySet : entry.step = i
             THEN (CHOOSE entry \in StepEntrySet : entry.step = i).state
             ELSE "Pending"

         new_step_states == [i \in 0..(seed.step_count - 1) |-> step_state_at(i)]

         slot_entry_at(i) ==
             IF \E entry \in SlotEntrySet : entry.slot = i
             THEN CHOOSE entry \in SlotEntrySet : entry.slot = i
             ELSE [value |-> SV_A, taint |-> TaintClean]

         new_slots == [i \in 0..(seed.slot_count - 1) |->
             [value |-> slot_entry_at(i).value,
              taint |-> slot_entry_at(i).taint]]

         new_step_apply_method == [i \in 0..(seed.step_count - 1) |->
             IF \E entry \in StepEntrySet : entry.step = i
             THEN LET state == (CHOOSE entry \in StepEntrySet : entry.step = i).state
                  IN IF state \in {"Waiting", "Asking"} THEN "TwoPhase" ELSE "Direct"
             ELSE "None"]

         pc_state == step_state_at(seed.pc)

         candidates == {i \in (seed.pc + 1)..(seed.step_count - 1) :
                            new_step_states[i] \in ResumableStates}

         next_pc == IF candidates = {} THEN seed.step_count
                    ELSE CHOOSE i \in candidates : TRUE

         new_pc == IF rejected THEN seed.pc
                  ELSE IF ~steps_valid \/ ~slots_valid THEN seed.pc
                  ELSE IF pc_state \in TerminalStates THEN next_pc
                  ELSE seed.pc
      IN
      /\ hydration' = IF rejected THEN "Err"
                      ELSE IF ~steps_valid \/ ~slots_valid THEN "Err"
                      ELSE IF new_pc >= seed.step_count THEN "Err"
                      ELSE "Ok"
      /\ frame' = [
              run_id |-> seed.summary_run,
              step_count |-> seed.step_count,
              slot_count |-> seed.slot_count,
              step_states |-> new_step_states,
              slots |-> new_slots]
      /\ pc' = new_pc
      /\ step_apply_method' = new_step_apply_method
      /\ seed' = seed)

(* Terminal stutter: allow hydration result to be the final state *)
TerminalStutter ==
    hydration \in {"Ok", "Err"} /\ UNCHANGED vars

Next == FullHydration \/ TerminalStutter

Spec == Init /\ [][Next]_vars

THEOREM Spec => []SeedTypeInvariant
THEOREM Spec => []FrameTypeInvariant
THEOREM Spec => []TypeInvariant
THEOREM Spec => []DuplicateFreeSteps
THEOREM Spec => []DuplicateFreeSlots
THEOREM Spec => []UnsupportedStateRejected
THEOREM Spec => []ActionPayloadsRejected
THEOREM Spec => []PcValidAfterHydration
THEOREM Spec => []PcOutOfBoundsMeansErr
THEOREM Spec => []PcResumable
THEOREM Spec => []StepStateValid
THEOREM Spec => []SlotValueValid
THEOREM Spec => []StepStateMatch
THEOREM Spec => []UnmentionedStepsPending
THEOREM Spec => []TwoPhaseApplication

====
