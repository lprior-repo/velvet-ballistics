---- MODULE RecoveryReplayBadSeq ----

EXTENDS Integers, Sequences

VARIABLE journal

Init == journal = <<>>

AppendBad == journal' = <<[seq |-> 1], [seq |-> 0]>>

Next == AppendBad

vars == <<journal>>

Spec == Init /\ [][Next]_vars

ReplaySeqOrder ==
    \A i, j \in 1..Len(journal) : i < j => journal[i].seq <= journal[j].seq

====
