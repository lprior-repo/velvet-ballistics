---- MODULE vb_7m21_007 ----
EXTENDS Naturals, TLC

(* PO-vb-7m21-030: stale/corrupt snapshot cannot hide newer journal tail. *)
VARIABLES phase, snapshotSeq, tailSeq, snapshotValid, outcome

Outcomes == {"Unknown", "UseSnapshot", "ReplayTail", "StorageError"}
Phases == {"Start", "Checked", "Done"}
Seqs == 0..4

Init ==
  /\ phase = "Start"
  /\ snapshotSeq \in Seqs
  /\ tailSeq \in Seqs
  /\ snapshotValid \in BOOLEAN
  /\ outcome = "Unknown"

Check ==
  /\ phase = "Start"
  /\ phase' = "Checked"
  /\ UNCHANGED <<snapshotSeq, tailSeq, snapshotValid>>
  /\ outcome' = IF ~snapshotValid THEN "StorageError"
                ELSE IF snapshotSeq < tailSeq THEN "ReplayTail"
                ELSE "UseSnapshot"

Finish == /\ phase = "Checked" /\ phase' = "Done" /\ UNCHANGED <<snapshotSeq, tailSeq, snapshotValid, outcome>>
Done == /\ phase = "Done" /\ UNCHANGED <<phase, snapshotSeq, tailSeq, snapshotValid, outcome>>
Next == Check \/ Finish \/ Done
Spec == Init /\ [][Next]_<<phase, snapshotSeq, tailSeq, snapshotValid, outcome>>

TypeOK == /\ phase \in Phases /\ snapshotSeq \in Seqs /\ tailSeq \in Seqs /\ snapshotValid \in BOOLEAN /\ outcome \in Outcomes
StaleSnapshotDoesNotHideTail == (phase \in {"Checked", "Done"} /\ snapshotValid /\ snapshotSeq < tailSeq) => outcome = "ReplayTail"
CorruptSnapshotTyped == (phase \in {"Checked", "Done"} /\ ~snapshotValid) => outcome = "StorageError"
====
