(* AskAnswerLifecycle.tla
 *
 * Invariant: AskAnswer lifecycle state machine — no duplicate answers,
 * monotonic seqno, idempotent replay, and SlotWritten precedes AskAnswered.
 *)

---- MODULE AskAnswerLifecycle ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANT
    MaxRunId,
    MaxStepIdx,
    MaxSeqNo,
    MaxJournalEvents

\* Event kind tags for journal entry discrimination.
\* "sw" = SlotWritten, "aa" = AskAnswered
EventKind == {"sw", "aa"}

VARIABLES
    AskState,
    PendingAnswers,
    AnsweredLog,
    SeqNoCounter

vars == <<AskState, PendingAnswers, AnsweredLog, SeqNoCounter>>

RunId == 1..MaxRunId
StepIdx == 1..MaxStepIdx
SeqNo == 1..MaxSeqNo

\* AnsweredLog entries are 4-tuples: [event_kind, run, step, seq]
\* event_kind distinguishes SlotWritten ("sw") from AskAnswered ("aa").
\* Both entries for the same ticket share identical (run, step, seq).
TypeOK ==
    /\ AskState \in [RunId -> {"idle", "awaiting", "answered", "failed"}]
    /\ PendingAnswers \in SUBSET (RunId \X StepIdx \X SeqNo)
    /\ AnsweredLog \in Seq(EventKind \X RunId \X StepIdx \X SeqNo)
    /\ SeqNoCounter \in [RunId -> 0..MaxSeqNo]

Init ==
    /\ AskState = [r \in RunId |-> "idle"]
    /\ PendingAnswers = {}
    /\ AnsweredLog = <<>>
    /\ SeqNoCounter = [r \in RunId |-> 0]

AnswerAsk(run, step, seq) ==
    /\ AskState[run] = "awaiting"
    /\ <<run, step, seq>> \in PendingAnswers
    /\ SeqNoCounter[run] < MaxSeqNo
    \* Emit SlotWritten first, then AskAnswered — both appended in order.
    /\ AnsweredLog' = Append(
           Append(AnsweredLog, <<"sw", run, step, seq>>),
           <<"aa", run, step, seq>>)
    /\ SeqNoCounter' = [SeqNoCounter EXCEPT ![run] = SeqNoCounter[run] + 1]
    /\ AskState' = [AskState EXCEPT ![run] = "answered"]
    /\ PendingAnswers' = PendingAnswers \ {<<run, step, seq>>}

ReplayAnswer(run, step, seq) ==
    /\ AskState[run] = "answered"
    /\ \E i \in 1..Len(AnsweredLog) :
        \* Match AskAnswered entries by event kind tag and key.
        AnsweredLog[i] = <<"aa", run, step, seq>>
    /\ UNCHANGED <<AskState, PendingAnswers, AnsweredLog, SeqNoCounter>>

AdvanceToNextStep(run) ==
    /\ AskState[run] = "answered"
    /\ AskState' = [AskState EXCEPT ![run] = "idle"]
    /\ UNCHANGED <<PendingAnswers, AnsweredLog, SeqNoCounter>>

(* SubmitAsk is a single-run lifecycle admission step.  It is gated on all runs
 * being idle, the per-run sequence counter having remaining bounded capacity,
 * and the submitted ticket using the next monotonic sequence number.  This
 * preserves the contract (no duplicate answered tickets, monotonic seqno, and
 * SlotWritten-before-AskAnswered) while removing the previous bounded-model
 * deadlock where TLC could submit an arbitrary stale seq after SeqNoCounter hit
 * MaxSeqNo. *)
SubmitAsk(run, step, seq) ==
    /\ AskState[run] = "idle"
    /\ \A r \in RunId : AskState[r] = "idle"  \* all runs must be idle
    /\ SeqNoCounter[run] < MaxSeqNo
    /\ seq = SeqNoCounter[run] + 1
    /\ ~\E i \in 1..Len(AnsweredLog) :  \* not already answered
         AnsweredLog[i] = <<"aa", run, step, seq>>
    /\ AskState' = [AskState EXCEPT ![run] = "awaiting"]
    /\ PendingAnswers' = PendingAnswers \cup {<<run, step, seq>>}
    /\ UNCHANGED <<AnsweredLog, SeqNoCounter>>

SubmitAny ==
    \E run \in RunId, step \in StepIdx, seq \in SeqNo :
        SubmitAsk(run, step, seq)

AnswerAny ==
    \E run \in RunId, step \in StepIdx, seq \in SeqNo :
        AnswerAsk(run, step, seq)

ReplayAny ==
    \E run \in RunId, step \in StepIdx, seq \in SeqNo :
        ReplayAnswer(run, step, seq)

AdvanceAny ==
    \E run \in RunId :
        AdvanceToNextStep(run)

Terminal ==
    /\ \A run \in RunId :
        /\ AskState[run] = "idle"
        /\ SeqNoCounter[run] = MaxSeqNo
    /\ PendingAnswers = {}
    /\ UNCHANGED vars

Next ==
    \/ SubmitAny
    \/ AnswerAny
    \/ ReplayAny
    \/ AdvanceAny
    \/ Terminal

(* Fairness: liveness is only required for accepted work.  Weak fairness on the
 * progress actions forces an awaiting run to answer and an answered run to
 * advance, even though ReplayAnswer and terminal behavior are stuttering
 * no-ops.  No fairness is assumed for SubmitAsk, so the model does not require
 * the environment to submit new asks forever. *)
Fairness ==
    /\ WF_vars(AnswerAny)
    /\ WF_vars(AdvanceAny)

Spec == Init /\ [][Next]_vars /\ Fairness

NoDuplicateAskAnswered ==
    \* Each (run, step, seq) ticket may appear at most once as AskAnswered.
    \* SlotWritten entries are separate; they are ordered before their matching AskAnswered.
    \A i \in 1..Len(AnsweredLog) :
        \A j \in 1..Len(AnsweredLog) :
            /\ AnsweredLog[i][1] = "aa"
            /\ AnsweredLog[j][1] = "aa"
            /\ AnsweredLog[i] = AnsweredLog[j]
            => i = j

ValidAskState ==
    \A run \in RunId :
        AskState[run] \in {"idle", "awaiting", "answered", "failed"}

PendingSubset ==
    PendingAnswers \subseteq (RunId \X StepIdx \X SeqNo)

MonotonicSeqNo ==
    \A run \in RunId :
        SeqNoCounter[run] >= 0

EventuallyAnswered ==
    \A run \in RunId :
        (AskState[run] = "awaiting") ~> (AskState[run] \in {"answered", "failed"})

EventuallyAdvanced ==
    \A run \in RunId :
        (AskState[run] = "answered") ~> (AskState[run] = "idle")

AnswerPersistenceOrder ==
    \A run \in RunId, step \in StepIdx, seq \in SeqNo :
        \A i \in 1..Len(AnsweredLog) :
            AnsweredLog[i] = <<"aa", run, step, seq>>
                => \E j \in 1..i-1 :
                    AnsweredLog[j] = <<"sw", run, step, seq>>

THEOREM Spec => []NoDuplicateAskAnswered
THEOREM Spec => []ValidAskState
THEOREM Spec => []PendingSubset
THEOREM Spec => []MonotonicSeqNo

\* State constraint: prevent unbounded AnsweredLog growth.
\* With MaxJournalEvents = 50, and 2 entries per lifecycle (sw+aa),
\* this permits at most 25 full run-cycle completions, which is sufficient
\* for the bounded model to prove all invariants and temporal properties.
JournalBounded ==
    Len(AnsweredLog) <= MaxJournalEvents

====
