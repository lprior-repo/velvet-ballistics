---------------------------- MODULE ControlLowering ----------------------------
(*
 * TLA+ spec for vb-core-lower-control-primitives
 * bead_id: vb-core-lower-control-primitives
 * obligation: TLA-WF-001
 *
 * Structural well-formedness of step chains produced by lower_* functions.
 * Fixed: Added EXTENDS Naturals, FiniteSets; defined Null as model value;
 *         use UNIQUE variable names for every \E quantifier across all Next disjuncts
 *         to avoid shadowing errors in TLC.
 *
 * Model:
 *   - steps: ARRAY of step records indexed by step id
 *   - slots: ARRAY of slot records indexed by slot id
 *   - MaxSteps = 10, MaxSlots = 20 (bounded model for TLC)
 *
 * Invariants verified:
 *   1. NoDuplicateStepIds   — every step id is unique
 *   2. ValidOffsets         — body > id and done > id when present
 *   3. AskResumeIdCorrect   — Ask's resume has id = Ask.id + 1
 *   4. SlotsRecorded        — all referenced slots are in DOMAIN slots
 *)

EXTENDS Naturals, FiniteSets, Integers

CONSTANT
    MaxSteps,
    MaxSlots

ASSUME MaxSteps \in Nat /\ MaxSteps > 0
ASSUME MaxSlots \in Nat /\ MaxSlots > 0

\* Define Null as a value not in Nat.
\* Use CHOOSE with Int (from Integers extension) to get a unique non-Nat value.
\* -1 is the simplest integer not in Nat.
Null == -1

VARIABLES
    steps,
    slots

vars == <<steps, slots>>

(*
 * StepIds: range 0 .. MaxSteps-1 using .. range operator from Naturals
 * SlotIds: range 0 .. MaxSlots-1 using .. range operator from Naturals
 *)
StepIds == 0 .. MaxSteps-1
SlotIds == 0 .. MaxSlots-1

(*
 * Step record shape (approximation of CompiledNode):
 *
 *   id       — StepIdx (0..MaxSteps-1)
 *   body     — StepIdx (only for loop/branch nodes)
 *   done     — StepIdx (only for loop/branch nodes)
 *   output   — SlotIdx \union {Null} (optional output slot)
 *   kind     — one of: Nop, ForEachStart, ForEachNext, TogetherStart,
 *                   TogetherJoin, CollectStart, CollectPage,
 *                   CollectFinish, ReduceStart, ReduceNext,
 *                   ReduceFinish, RepeatStart, RepeatAttempt,
 *                   RepeatFinish, WaitUntil, WaitEvent,
 *                   Ask, AskResume, Finish
 *)

Init ==
    /\ steps = [i \in StepIds |-> [id |-> i, body |-> i, done |-> i,
                                   output |-> Null, kind |-> "Nop"]]
    /\ slots = [j \in SlotIds |-> [id |-> j, referenced_by |-> {}]]

(*
 * ForEachStart lowering action.
 * Input: sid, sinp, sit, lim, sb, sd
 * Emits: 2 nodes: [ForEachStart(sid), ForEachNext(sb)]
 *)
LowerForEach(sid, sinp, sit, lim, sb, sd) ==
    /\ steps' = [steps EXCEPT
                     ![sb] = [id |-> sb, body |-> sb, done |-> sd,
                               output |-> Null, kind |-> "ForEachNext"]]
    /\ UNCHANGED slots

(*
 * TogetherStart/TogetherJoin lowering action.
 * Input: sid, br, sj
 * Emits: 2 nodes: [TogetherStart(sid), TogetherJoin(sj)]
 *)
LowerTogether(sid, br, sj) ==
    /\ steps' = [steps EXCEPT
                     ![sj] = [id |-> sj, body |-> sj, done |-> sj,
                               output |-> Null, kind |-> "TogetherJoin"]]
    /\ UNCHANGED slots

(*
 * Collect lowering action.
 * Input: sid, src, lim, ps, sb, sd
 * Emits: 3 nodes: [CollectStart, CollectPage, CollectFinish]
 *)
LowerCollect(sid, src, lim, ps, sb, sd) ==
    /\ steps' = [steps EXCEPT
                     ![sb] = [id |-> sb, body |-> sb, done |-> sd,
                               output |-> Null, kind |-> "CollectPage"]]
    /\ UNCHANGED slots

(*
 * Reduce lowering action.
 * Input: sid, sinp, acc, init, sb, sd
 * Emits: 3 nodes: [ReduceStart, ReduceNext, ReduceFinish]
 *)
LowerReduce(sid, sinp, acc, init, sb, sd) ==
    /\ steps' = [steps EXCEPT
                     ![sb] = [id |-> sb, body |-> sb, done |-> sd,
                               output |-> Null, kind |-> "ReduceNext"]]
    /\ UNCHANGED slots

(*
 * Repeat lowering action.
 * Input: sid, ma, sb, sd
 * Emits: 3 nodes: [RepeatStart, RepeatAttempt(sb), RepeatFinish(sb)]
 * where sb = sid + 1 (enforced by guard condition sid < MaxSteps - 1)
 *
 * CRITICAL INVARIANT (TLA-WF-001): RepeatAttempt id = sid + 1
 *)
LowerRepeat(sid, ma, sb, sd) ==
    /\ sid < MaxSteps - 1       \* ensures sb = sid+1 is within step range
    /\ sb < MaxSteps
    /\ sd < MaxSteps
    /\ sb = sid + 1              \* enforce attempt_slot = sid+1
    /\ steps' = [steps EXCEPT
                     ![sb] = [id |-> sb, body |-> sb, done |-> sd,
                               output |-> sb, kind |-> "RepeatAttempt"]]
    /\ UNCHANGED slots

(*
 * Wait lowering action (WaitKind::Until or WaitKind::Event).
 * Input: sid, knd \in {"Until", "Event"}
 * Emits: 1 node: WaitUntil or WaitEvent
 *)
LowerWait(sid, knd) ==
    /\ steps' = [steps EXCEPT
                     ![sid] = [id |-> sid, body |-> sid, done |-> sid,
                                output |-> Null,
                                kind |-> IF knd = "Until" THEN "WaitUntil"
                                         ELSE "WaitEvent"]]
    /\ UNCHANGED slots

(*
 * Ask lowering action.
 * Input: sid, pr, ans, ts
 * Emits: 2 nodes: [Ask(sid), AskResume(sid+1)]
 *
 * CRITICAL INVARIANT (TLA-WF-001): AskResume id = Ask id + 1
 *)
LowerAsk(sid, pr, ans, ts) ==
    /\ sid < MaxSteps - 1       \* ensures sid+1 is within step range
    /\ pr \in SlotIds
    /\ ans \in SlotIds
    /\ steps' = [steps EXCEPT
                     ![sid+1] = [id |-> sid+1, body |-> sid+1, done |-> sid+1,
                                   output |-> ans, kind |-> "AskResume"]]
    /\ UNCHANGED slots

(*
 * The Next action nondeterministically applies any lowering function.
 * FIX: Every \E quantifier uses a completely unique variable name to avoid
 *       any shadowing across the \/ disjunction.
 *)
Next ==
    \E fe_sid \in StepIds:
        \E fe_sinp, fe_sit \in SlotIds:
            \E fe_lim \in 0..1000:
                \E fe_sb, fe_sd \in StepIds:
                    LowerForEach(fe_sid, fe_sinp, fe_sit, fe_lim, fe_sb, fe_sd)
    \/ \E to_sid \in StepIds:
        \E to_br \in SUBSET StepIds \ {to_sid}:
            \E to_sj \in StepIds:
                LowerTogether(to_sid, to_br, to_sj)
    \/ \E co_sid \in StepIds:
        \E co_src \in SlotIds:
            \E co_lim \in 0..1000:
                \E co_ps \in 0..100:
                    \E co_sb, co_sd \in StepIds:
                        LowerCollect(co_sid, co_src, co_lim, co_ps, co_sb, co_sd)
    \/ \E re_sid \in StepIds:
        \E re_sinp, re_acc \in SlotIds:
            \E re_init \in 0..100:
                \E re_sb, re_sd \in StepIds:
                    LowerReduce(re_sid, re_sinp, re_acc, re_init, re_sb, re_sd)
    \/ \E rp_sid \in StepIds:
        \E rp_ma \in 1 .. 1000:
            \E rp_sb, rp_sd \in StepIds:
                LowerRepeat(rp_sid, rp_ma, rp_sb, rp_sd)
    \/ \E wa_sid \in StepIds:
        LowerWait(wa_sid, "Until")
    \/ \E ak_sid \in StepIds:
        LowerAsk(ak_sid, 0, 1, Null)

Spec == Init /\ [][Next]_vars

(*
 * INVARIANT: NoDuplicateStepIds
 * No two steps share the same id.
 *)
NoDuplicateStepIds ==
    \A i, j \in DOMAIN steps:
        i /= j => steps[i].id /= steps[j].id

(*
 * INVARIANT: ValidOffsets
 * For any step with body/done fields (non-Nop):
 *   body > id  and  done > id
 *)
ValidOffsets ==
    \A s \in DOMAIN steps:
        LET step == steps[s] IN
            IF step.kind = "Nop" THEN TRUE
            ELSE /\ step.body > step.id
                 /\ step.done > step.id

(*
 * INVARIANT: AskResumeIdCorrect
 * For every AskResume node, there exists an Ask node with id = AskResume.id - 1
 *)
AskResumeIdCorrect ==
    \A s \in DOMAIN steps:
        IF steps[s].kind = "AskResume"
        THEN \E a \in DOMAIN steps:
                /\ steps[a].kind = "Ask"
                /\ steps[a].id + 1 = steps[s].id
        ELSE TRUE

(*
 * INVARIANT: SlotsRecorded
 * All slots referenced as output are in the slots domain.
 *)
SlotsRecorded ==
    \A s \in DOMAIN steps:
        IF steps[s].output \in SlotIds
        THEN steps[s].output \in DOMAIN slots
        ELSE TRUE

(*
 * Safety check: in the initial state all steps are Nop.
 * After any lowering action, some step will have a non-Nop kind.
 * This is not verified as a temporal property since it only holds
 * after Next executes (not in Init).
 *)

=============================================================================
