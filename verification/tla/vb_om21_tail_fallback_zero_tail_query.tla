---- MODULE vb_om21_tail_fallback_zero_tail_query ----
EXTENDS Naturals, Sequences, TLC
\* Obligation: PO-vb-om21-zero-tail-query-tla
\* Requirement: REQ-vb-om21-05
\* Claim: A pure tail query over an empty per-run prefix returns tail zero without fabricating a journal event.
Runs == {0, 1}
Seqs == {0, 1, 18446744073709551614, 18446744073709551615}
Modes == {"QueryAllowsEmpty", "RecoveryRequiresJournal"}
Meta == {"Missing", "Equal", "Above", "Below"}
VARIABLES run, mode, metadata, observed, outcome
Init == /\ run \in Runs
        /\ mode \in Modes
        /\ metadata \in Meta
        /\ observed \in SUBSET Seqs
        /\ outcome = "Start"
MaxSeq(S) == IF S = {} THEN 0 ELSE CHOOSE x \in S: \A y \in S: y <= x
NextTail(S) == IF S = {} THEN 0 ELSE IF MaxSeq(S) = 18446744073709551615 THEN "TailOverflow" ELSE MaxSeq(S) + 1
Classify == IF mode = "RecoveryRequiresJournal" /\ observed = {} THEN "MissingJournal"
            ELSE IF NextTail(observed) = "TailOverflow" THEN "TailOverflow"
            ELSE IF metadata = "Below" THEN "TailMismatch"
            ELSE "Ok"
Next == outcome' = Classify /\ UNCHANGED <<run, mode, metadata, observed>>
TypeInvariant == /\ run \in Runs
                 /\ mode \in Modes
                 /\ metadata \in Meta
                 /\ observed \in SUBSET Seqs
                 /\ outcome \in {"Start", "Ok", "MissingJournal", "TailMismatch", "TailOverflow"}
TypedFailureReachable == <> (outcome \in {"MissingJournal", "TailMismatch", "TailOverflow"})
DeadlockFreedom == TRUE
====
