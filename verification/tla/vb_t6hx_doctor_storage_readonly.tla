---- MODULE vb_t6hx_doctor_storage_readonly ----
EXTENDS Naturals, FiniteSets, Sequences, TLC

CONSTANTS MaxCommands
VARIABLES pc, index, command, mutation, parsed, opened, attemptedMutation, unsupported

Commands == {"scan", "get", "append", "persist", "delete", "compact", "migrate", "syntheticRun"}
ReadOnlyCommands == {"scan", "get"}
MutationCommands == Commands \ ReadOnlyCommands
Init == /\ pc = "Parse" /\ index = 0 /\ command \in Commands
        /\ mutation = FALSE /\ parsed = FALSE /\ opened = FALSE
        /\ attemptedMutation = FALSE /\ unsupported = FALSE

Parse == /\ pc = "Parse" /\ parsed' = TRUE
         /\ IF command \in ReadOnlyCommands THEN /\ pc' = "OpenReadOnly" /\ unsupported' = FALSE
            ELSE /\ pc' = "Rejected" /\ unsupported' = TRUE
         /\ UNCHANGED <<index, command, mutation, opened, attemptedMutation>>
OpenReadOnly == /\ pc = "OpenReadOnly" /\ parsed /\ command \in ReadOnlyCommands
                /\ opened' = TRUE /\ pc' = "Query"
                /\ UNCHANGED <<index, command, mutation, parsed, attemptedMutation, unsupported>>
Query == /\ pc = "Query" /\ opened /\ index < MaxCommands
          /\ index' = index + 1 /\ pc' \in {"Query", "Done"}
          /\ attemptedMutation' = FALSE /\ mutation' = FALSE
          /\ UNCHANGED <<command, parsed, opened, unsupported>>
QueryLimitDone == /\ pc = "Query" /\ opened /\ index = MaxCommands
                  /\ pc' = "Done" /\ attemptedMutation' = FALSE /\ mutation' = FALSE
                  /\ UNCHANGED <<index, command, parsed, opened, unsupported>>
ForbiddenMutationAttempt == /\ pc \in {"OpenReadOnly", "Query"} /\ command \in MutationCommands
                            /\ attemptedMutation' = TRUE /\ mutation' = TRUE /\ pc' = "Mutated"
                            /\ UNCHANGED <<index, command, parsed, opened, unsupported>>
Rejected == /\ pc = "Rejected" /\ unsupported /\ ~opened /\ ~mutation
            /\ UNCHANGED <<pc, index, command, mutation, parsed, opened, attemptedMutation, unsupported>>
Done == /\ pc = "Done" /\ UNCHANGED <<pc, index, command, mutation, parsed, opened, attemptedMutation, unsupported>>
Next == Parse \/ OpenReadOnly \/ Query \/ QueryLimitDone \/ Rejected \/ Done \/ ForbiddenMutationAttempt

NoMutation == mutation = FALSE
ParseBeforeOpen == opened => parsed
FailClosedReadOnlyUnsupported == command \in MutationCommands => ~opened /\ ~mutation /\ pc \notin {"OpenReadOnly", "Query", "Done"}
NoForbiddenMutationReachable == pc # "Mutated" /\ ~attemptedMutation
TypeOK == /\ pc \in {"Parse", "OpenReadOnly", "Query", "Done", "Rejected", "Mutated"}
          /\ index \in 0..MaxCommands /\ command \in Commands
          /\ mutation \in BOOLEAN /\ parsed \in BOOLEAN /\ opened \in BOOLEAN
          /\ attemptedMutation \in BOOLEAN /\ unsupported \in BOOLEAN

====
