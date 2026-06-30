---- MODULE Vt2fRuntimeLifecycle ----
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS Runs, MaxQueue, MaxEvents, MaxStep
VARIABLES runs, queued, trace, journal, errors, shutdown, steps

vars == <<runs, queued, trace, journal, errors, shutdown, steps>>
States == {"missing", "queued", "running", "await_action", "await_ask", "finished", "failed"}
Errors == {"lower_run_not_found", "facade_invalid_action_completion", "wrong_ask_rejected", "shutdown_seen"}
BoundedAppend(seq, item) == IF Len(seq) < MaxEvents THEN Append(seq, item) ELSE seq

Init ==
    /\ runs = [r \in Runs |-> "missing"]
    /\ queued = <<>>
    /\ trace = <<>>
    /\ journal = <<>>
    /\ errors = {}
    /\ shutdown = FALSE
    /\ steps = 0

CanStep == steps < MaxStep

Submit(r) ==
    /\ CanStep /\ ~shutdown /\ runs[r] = "missing" /\ Len(queued) < MaxQueue
    /\ runs' = [runs EXCEPT ![r] = "queued"]
    /\ queued' = Append(queued, [kind |-> "submit", run |-> r])
    /\ trace' = BoundedAppend(trace, [kind |-> "submit_called", run |-> r])
    /\ journal' = journal /\ errors' = errors /\ shutdown' = shutdown
    /\ steps' = steps + 1

TickSubmit(r) ==
    /\ CanStep /\ Len(queued) > 0 /\ Head(queued).kind = "submit" /\ Head(queued).run = r
    /\ runs' = [runs EXCEPT ![r] = "running"]
    /\ queued' = Tail(queued)
    /\ trace' = BoundedAppend(trace, [kind |-> "run_submitted", run |-> r])
    /\ journal' = BoundedAppend(journal, [kind |-> "RunSubmitted", run |-> r])
    /\ errors' = errors /\ shutdown' = shutdown /\ steps' = steps + 1

SuspendAction(r) ==
    /\ CanStep /\ ~shutdown /\ runs[r] = "running"
    /\ runs' = [runs EXCEPT ![r] = "await_action"]
    /\ queued' = queued
    /\ trace' = BoundedAppend(trace, [kind |-> "await_action", run |-> r])
    /\ journal' = journal /\ errors' = errors /\ shutdown' = shutdown
    /\ steps' = steps + 1

CompleteAction(r) ==
    /\ CanStep /\ ~shutdown /\ runs[r] = "await_action"
    /\ runs' = [runs EXCEPT ![r] = "finished"]
    /\ queued' = queued
    /\ trace' = BoundedAppend(BoundedAppend(trace, [kind |-> "slot_written", run |-> r]), [kind |-> "action_completed", run |-> r])
    /\ journal' = BoundedAppend(BoundedAppend(journal, [kind |-> "SlotWritten", run |-> r]), [kind |-> "ActionCompleted", run |-> r])
    /\ errors' = errors /\ shutdown' = shutdown /\ steps' = steps + 1

SuspendAsk(r) ==
    /\ CanStep /\ ~shutdown /\ runs[r] = "running"
    /\ runs' = [runs EXCEPT ![r] = "await_ask"]
    /\ queued' = queued
    /\ trace' = BoundedAppend(trace, [kind |-> "await_ask", run |-> r])
    /\ journal' = journal /\ errors' = errors /\ shutdown' = shutdown
    /\ steps' = steps + 1

AnswerAsk(r) ==
    /\ CanStep /\ ~shutdown /\ runs[r] = "await_ask"
    /\ runs' = [runs EXCEPT ![r] = "finished"]
    /\ queued' = queued
    /\ trace' = BoundedAppend(trace, [kind |-> "ask_answered", run |-> r])
    /\ journal' = BoundedAppend(BoundedAppend(journal, [kind |-> "SlotWritten", run |-> r]), [kind |-> "AskAnswered", run |-> r])
    /\ errors' = errors /\ shutdown' = shutdown /\ steps' = steps + 1

WrongAsk(target, other) ==
    /\ CanStep /\ target # other /\ runs[target] = "await_ask"
    /\ runs' = runs /\ queued' = queued /\ trace' = trace /\ journal' = journal
    /\ errors' = errors \cup {"wrong_ask_rejected"}
    /\ shutdown' = shutdown /\ steps' = steps + 1

LowerFailAbsent(r) ==
    /\ CanStep /\ runs[r] = "missing"
    /\ runs' = runs /\ queued' = queued /\ trace' = trace /\ journal' = journal
    /\ errors' = errors \cup {"lower_run_not_found"}
    /\ shutdown' = shutdown /\ steps' = steps + 1

FacadeFailAbsent(r) ==
    /\ CanStep /\ runs[r] = "missing" /\ Len(queued) < MaxQueue
    /\ runs' = runs /\ queued' = Append(queued, [kind |-> "runtime_action_failed", run |-> r])
    /\ trace' = trace /\ journal' = journal /\ errors' = errors
    /\ shutdown' = shutdown /\ steps' = steps + 1

TickFacadeFailAbsent(r) ==
    /\ CanStep /\ Len(queued) > 0 /\ Head(queued).kind = "runtime_action_failed" /\ Head(queued).run = r
    /\ runs' = runs /\ queued' = Tail(queued) /\ trace' = trace /\ journal' = journal
    /\ errors' = errors \cup {"facade_invalid_action_completion"}
    /\ shutdown' = shutdown /\ steps' = steps + 1

Shutdown ==
    /\ CanStep /\ shutdown = FALSE
    /\ runs' = runs /\ queued' = <<>> /\ trace' = trace /\ journal' = journal
    /\ errors' = errors \cup {"shutdown_seen"}
    /\ shutdown' = TRUE /\ steps' = steps + 1

Quiesce ==
    /\ shutdown \/ steps = MaxStep
    /\ runs' = runs /\ queued' = queued /\ trace' = trace /\ journal' = journal
    /\ errors' = errors /\ shutdown' = shutdown /\ steps' = steps

Progress ==
    \/ \E r \in Runs : Submit(r)
    \/ \E r \in Runs : TickSubmit(r)
    \/ \E r \in Runs : SuspendAction(r)
    \/ \E r \in Runs : CompleteAction(r)
    \/ \E r \in Runs : SuspendAsk(r)
    \/ \E r \in Runs : AnswerAsk(r)
    \/ \E t \in Runs : \E o \in Runs : WrongAsk(t, o)
    \/ \E r \in Runs : LowerFailAbsent(r)
    \/ \E r \in Runs : FacadeFailAbsent(r)
    \/ \E r \in Runs : TickFacadeFailAbsent(r)
    \/ Shutdown

Next == Progress \/ Quiesce

Spec == Init /\ [][Next]_vars /\ WF_vars(Progress)

TypeOK ==
    /\ runs \in [Runs -> States]
    /\ queued \in Seq([kind : {"submit", "runtime_action_failed"}, run : Runs])
    /\ Len(queued) <= MaxQueue
    /\ Len(trace) <= MaxEvents
    /\ Len(journal) <= MaxEvents
    /\ errors \subseteq Errors
    /\ shutdown \in BOOLEAN
    /\ steps \in 0..MaxStep

EventuallyTerminalOrSuspendedOrTypedErrorWithinBounds ==
    <>(shutdown \/ steps = MaxStep \/ errors # {} \/ \A r \in Runs : runs[r] \in {"missing", "await_action", "await_ask", "finished", "failed"})

NoDeadlockWithoutHeartbeatMask ==
    [](shutdown \/ steps = MaxStep \/ ENABLED Progress)

TraceJournalBounded == Len(trace) <= MaxEvents /\ Len(journal) <= MaxEvents
ShutdownMarker == "shutdown_seen" \in errors => shutdown
WrongAskRejectedCodeBounded == "wrong_ask_rejected" \in errors => "wrong_ask_rejected" \in Errors
LowerFacadeCodesAvailable == "lower_run_not_found" \in Errors /\ "facade_invalid_action_completion" \in Errors

====
