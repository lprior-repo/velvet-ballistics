---- MODULE V1PrimitiveLowering ----
EXTENDS Naturals, TLC

\* Obligations: POST-006-TLA through POST-012-TLA and INV-002.
\* Bounded lifecycle model over prevalidated compiler-emitted v1 primitive graph
\* shapes. Concrete AST lowering, dense indices, and slot coverage are owned by
\* Verus/tests; this model rejects lifecycle shapes with out-of-range targets or
\* missing primitive-specific routes under the configured finite bounds.

CONSTANTS MaxNodes, BranchBound, AttemptBound, PageBound, InputBound

Primitives == {"ForEach", "Together", "Collect", "Reduce", "Repeat", "Wait", "Ask"}
Phases == {"start", "body", "branch", "join", "page", "attempt", "suspended", "done"}
Nodes == 0..(MaxNodes - 1)
TargetChoices == 0..2
Bool == {TRUE, FALSE}

VARIABLES primitive, phase, target, bodyTarget, doneTarget, joinTarget,
          resumeTarget, exhaustedTarget,
          branches, completedBranches, attempt, maxAttempts,
          page, pageLimit, inputRemaining, suspended,
          delivered, answer, timedOut, finished, finishedCount

vars == <<primitive, phase, target, bodyTarget, doneTarget, joinTarget,
          resumeTarget, exhaustedTarget,
          branches, completedBranches, attempt, maxAttempts,
          page, pageLimit, inputRemaining, suspended,
          delivered, answer, timedOut, finished, finishedCount>>

InitLoweredPrimitiveGraph ==
  /\ primitive \in Primitives
  /\ phase = "start"
  /\ target \in TargetChoices
  /\ bodyTarget \in TargetChoices
  /\ doneTarget \in TargetChoices
  /\ joinTarget \in TargetChoices
  /\ resumeTarget \in TargetChoices
  /\ exhaustedTarget \in TargetChoices
  /\ branches \in 1..BranchBound
  /\ completedBranches = 0
  /\ attempt = 0
  /\ maxAttempts \in 1..AttemptBound
  /\ page = 0
  /\ pageLimit \in 1..PageBound
  /\ inputRemaining \in 0..InputBound
  /\ suspended = FALSE
  /\ delivered = FALSE
  /\ answer = FALSE
  /\ timedOut = FALSE
  /\ finished = FALSE
  /\ finishedCount = 0

Start ==
  /\ phase = "start"
  /\ ~finished
  /\ IF primitive \in {"ForEach", "Reduce"}
        THEN phase' = "body"
        ELSE IF primitive = "Together"
          THEN phase' = "branch"
          ELSE IF primitive = "Collect"
            THEN phase' = "page"
            ELSE IF primitive = "Repeat"
              THEN phase' = "attempt"
              ELSE phase' = "suspended"
  /\ target' = IF primitive \in {"ForEach", "Collect", "Reduce", "Repeat"}
               THEN bodyTarget
               ELSE IF primitive \in {"Wait", "Ask"}
                 THEN resumeTarget
                 ELSE joinTarget
  /\ UNCHANGED <<primitive, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered,
                  answer, timedOut, finished, finishedCount>>

EnterBody ==
  /\ phase = "body"
  /\ primitive \in {"ForEach", "Reduce"}
  /\ target \in Nodes
  /\ UNCHANGED <<primitive, phase, target, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget,
                  branches, completedBranches, attempt, maxAttempts,
                  page, pageLimit, inputRemaining, suspended, delivered,
                  answer, timedOut, finished, finishedCount>>

BodyDone ==
  /\ phase = "body"
  /\ primitive \in {"ForEach", "Reduce"}
  /\ inputRemaining > 0
  /\ inputRemaining' = inputRemaining - 1
  /\ target' = bodyTarget
  /\ UNCHANGED <<primitive, phase, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, suspended, delivered, answer, timedOut,
                  finished, finishedCount>>

AdvanceLoop ==
  /\ phase = "body"
  /\ primitive \in {"ForEach", "Reduce"}
  /\ inputRemaining > 0
  /\ target' = bodyTarget
  /\ UNCHANGED <<primitive, phase, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered,
                  answer, timedOut, finished, finishedCount>>

CompleteLoop ==
  /\ phase = "body"
  /\ primitive \in {"ForEach", "Reduce"}
  /\ inputRemaining = 0
  /\ phase' = "done"
  /\ finished' = TRUE
  /\ finishedCount' = finishedCount + 1
  /\ target' = doneTarget
  /\ UNCHANGED <<primitive, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered,
                  answer, timedOut>>

StartBranches ==
  /\ phase = "branch"
  /\ primitive = "Together"
  /\ completedBranches < branches
  /\ target' = joinTarget
  /\ UNCHANGED <<primitive, phase, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered,
                  answer, timedOut, finished, finishedCount>>

CompleteBranch ==
  /\ phase = "branch"
  /\ primitive = "Together"
  /\ completedBranches < branches
  /\ completedBranches' = completedBranches + 1
  /\ target' = joinTarget
  /\ UNCHANGED <<primitive, phase, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  attempt, maxAttempts, page, pageLimit, inputRemaining,
                  suspended, delivered, answer, timedOut, finished,
                  finishedCount>>

JoinBranches ==
  /\ phase = "branch"
  /\ primitive = "Together"
  /\ completedBranches = branches
  /\ phase' = "done"
  /\ finished' = TRUE
  /\ finishedCount' = finishedCount + 1
  /\ target' = doneTarget
  /\ UNCHANGED <<primitive, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered,
                  answer, timedOut>>

PageDone ==
  /\ phase = "page"
  /\ primitive = "Collect"
  /\ page < pageLimit
  /\ page' = page + 1
  /\ target' = bodyTarget
  /\ UNCHANGED <<primitive, phase, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, pageLimit,
                  inputRemaining, suspended, delivered, answer, timedOut,
                  finished, finishedCount>>

CompleteCollect ==
  /\ phase = "page"
  /\ primitive = "Collect"
  /\ page = pageLimit
  /\ phase' = "done"
  /\ finished' = TRUE
  /\ finishedCount' = finishedCount + 1
  /\ target' = doneTarget
  /\ UNCHANGED <<primitive, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered,
                  answer, timedOut>>

AttemptDone ==
  /\ phase = "attempt"
  /\ primitive = "Repeat"
  /\ attempt < maxAttempts
  /\ attempt' = attempt + 1
  /\ target' = IF attempt + 1 = maxAttempts THEN exhaustedTarget ELSE bodyTarget
  /\ UNCHANGED <<primitive, phase, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, maxAttempts, page, pageLimit,
                  inputRemaining, suspended, delivered, answer, timedOut,
                  finished, finishedCount>>

RetryOrFinish ==
  /\ phase = "attempt"
  /\ primitive = "Repeat"
  /\ attempt = maxAttempts
  /\ phase' = "done"
  /\ finished' = TRUE
  /\ finishedCount' = finishedCount + 1
  /\ target' = doneTarget
  /\ UNCHANGED <<primitive, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered,
                  answer, timedOut>>

Suspend ==
  /\ phase = "suspended"
  /\ primitive \in {"Wait", "Ask"}
  /\ suspended' = TRUE
  /\ target' = resumeTarget
  /\ UNCHANGED <<primitive, phase, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, delivered, answer, timedOut,
                  finished, finishedCount>>

DeliverEvent ==
  /\ phase = "suspended"
  /\ primitive = "Wait"
  /\ suspended
  /\ ~timedOut
  /\ delivered' = TRUE
  /\ phase' = "done"
  /\ finished' = TRUE
  /\ finishedCount' = finishedCount + 1
  /\ target' = doneTarget
  /\ UNCHANGED <<primitive, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, answer, timedOut>>

DeliverAnswer ==
  /\ phase = "suspended"
  /\ primitive = "Ask"
  /\ suspended
  /\ ~timedOut
  /\ answer' = TRUE
  /\ phase' = "done"
  /\ finished' = TRUE
  /\ finishedCount' = finishedCount + 1
  /\ target' = doneTarget
  /\ UNCHANGED <<primitive, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered,
                  timedOut>>

Timeout ==
  /\ phase = "suspended"
  /\ primitive \in {"Wait", "Ask"}
  /\ suspended
  /\ timedOut' = TRUE
  /\ phase' = "done"
  /\ finished' = TRUE
  /\ finishedCount' = finishedCount + 1
  /\ target' = doneTarget
  /\ UNCHANGED <<primitive, bodyTarget, doneTarget, joinTarget,
                  resumeTarget, exhaustedTarget, branches,
                  completedBranches, attempt, maxAttempts, page,
                  pageLimit, inputRemaining, suspended, delivered, answer>>

FinishPrimitive ==
  /\ phase = "done"
  /\ finished
  /\ UNCHANGED vars

Next == Start \/ EnterBody \/ BodyDone \/ AdvanceLoop \/ CompleteLoop
        \/ StartBranches \/ CompleteBranch \/ JoinBranches
        \/ PageDone \/ CompleteCollect
        \/ AttemptDone \/ RetryOrFinish
        \/ Suspend \/ DeliverEvent \/ DeliverAnswer \/ Timeout
        \/ FinishPrimitive

Spec == InitLoweredPrimitiveGraph /\ [][Next]_vars
        /\ WF_vars(Start)
        /\ WF_vars(BodyDone)
        /\ WF_vars(CompleteLoop)
        /\ WF_vars(CompleteBranch)
        /\ WF_vars(JoinBranches)
        /\ WF_vars(PageDone)
        /\ WF_vars(CompleteCollect)
        /\ WF_vars(AttemptDone)
        /\ WF_vars(RetryOrFinish)
        /\ WF_vars(Suspend)
        /\ WF_vars(DeliverEvent)
        /\ WF_vars(DeliverAnswer)
        /\ WF_vars(Timeout)

TargetsInRange ==
  /\ target \in Nodes
  /\ bodyTarget \in Nodes
  /\ doneTarget \in Nodes
  /\ joinTarget \in Nodes
  /\ resumeTarget \in Nodes
  /\ exhaustedTarget \in Nodes

GraphShapePrevalidated ==
  /\ primitive \in Primitives
  /\ TargetChoices \subseteq Nodes
  /\ (primitive \in {"ForEach", "Collect", "Reduce", "Repeat"} => bodyTarget \in Nodes)
  /\ (primitive = "Together" => joinTarget \in Nodes /\ branches \in 1..BranchBound)
  /\ (primitive = "Repeat" => exhaustedTarget \in Nodes /\ maxAttempts \in 1..AttemptBound)
  /\ (primitive \in {"Wait", "Ask"} => resumeTarget \in Nodes)
  /\ doneTarget \in Nodes

NoPrematureTogetherJoin ==
  primitive = "Together" /\ phase = "done" => completedBranches = branches

AttemptNeverExceedsMax ==
  attempt <= maxAttempts

PageNeverExceedsLimit ==
  page <= pageLimit

SingleCompletion ==
  finishedCount <= 1

BranchCountBounded ==
  branches \in 1..BranchBound

ForEachEventuallyDone ==
  primitive = "ForEach" => <>finished

TogetherEventuallyJoin ==
  primitive = "Together" => <> (finished /\ completedBranches = branches)

CollectEventuallyDone ==
  primitive = "Collect" => <>finished

ReduceEventuallyDone ==
  primitive = "Reduce" => <>finished

RepeatEventuallyDone ==
  primitive = "Repeat" => <>finished

WaitEventuallyResumesOrTimesOut ==
  primitive = "Wait" => <> (finished /\ (delivered \/ timedOut))

AskEventuallyResumesOrTimesOut ==
  primitive = "Ask" => <> (finished /\ (answer \/ timedOut))

====
