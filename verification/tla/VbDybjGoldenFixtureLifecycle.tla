---- MODULE VbDybjGoldenFixtureLifecycle ----
EXTENDS Naturals, Sequences, TLC

CONSTANTS Fixtures

VARIABLES pc, bytesChanged, migrationNamePresent

States == {"FixtureFrozen", "EncodedCompared", "MigrationRequired", "Accepted"}

Init == /\ pc \in [Fixtures -> {"FixtureFrozen"}]
        /\ bytesChanged \in [Fixtures -> BOOLEAN]
        /\ migrationNamePresent \in [Fixtures -> BOOLEAN]

Compare(f) == /\ pc[f] = "FixtureFrozen"
              /\ pc' = [pc EXCEPT ![f] = "EncodedCompared"]
              /\ UNCHANGED <<bytesChanged, migrationNamePresent>>

AcceptUnchanged(f) == /\ pc[f] = "EncodedCompared"
                      /\ bytesChanged[f] = FALSE
                      /\ pc' = [pc EXCEPT ![f] = "Accepted"]
                      /\ UNCHANGED <<bytesChanged, migrationNamePresent>>

RequireMigration(f) == /\ pc[f] = "EncodedCompared"
                       /\ bytesChanged[f] = TRUE
                       /\ migrationNamePresent[f] = TRUE
                       /\ pc' = [pc EXCEPT ![f] = "MigrationRequired"]
                       /\ UNCHANGED <<bytesChanged, migrationNamePresent>>

Stutter == UNCHANGED <<pc, bytesChanged, migrationNamePresent>>

Next == \/ \E f \in Fixtures : Compare(f)
        \/ \E f \in Fixtures : AcceptUnchanged(f)
        \/ \E f \in Fixtures : RequireMigration(f)
        \/ Stutter

TypeOK == /\ pc \in [Fixtures -> States]
          /\ bytesChanged \in [Fixtures -> BOOLEAN]
          /\ migrationNamePresent \in [Fixtures -> BOOLEAN]

NoSilentByteChangeAcceptance ==
    \A f \in Fixtures : (pc[f] = "Accepted" => bytesChanged[f] = FALSE)

ChangedBytesNeedNamedMigration ==
    \A f \in Fixtures : ((pc[f] = "MigrationRequired" /\ bytesChanged[f]) => migrationNamePresent[f])

Spec == Init /\ [][Next]_<<pc, bytesChanged, migrationNamePresent>>

====
