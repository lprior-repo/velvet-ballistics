---- MODULE YamlE2eChain ----
EXTENDS Naturals, Sequences

\* Obligations: PO-001, PO-002, PO-003.
\* Pure bounded lifecycle model for YAML-origin strict execution.
\* Byte-level digest and parser correctness are abstracted as booleans.

Bool == {TRUE, FALSE}
Phase == {"Cold", "YamlValidated", "SourcePersisted", "ArtifactPersisted",
          "RunHeaderPersisted", "Accepted", "Admitted", "Running",
          "Suspended", "Finished", "Failed", "Restarted", "Recovered"}
Event == {"RunAccepted", "RunAdmission", "RunFinished", "RunFailed",
          "AdmissionRejected", "RecoveryRejected"}
Ack == {"none", "accepted", "admitted", "running", "finished",
        "suspended", "failed", "recovered"}
Projection == {"none", "accepted", "admitted", "terminal", "failed", "recovered"}

VARIABLES phase, sourceStored, artifactStored, acceptedEnvelope,
          runHeaderStored, journal, ack, yamlParserUsedAfterAdmission,
          digestOk, artifactOk, gateOk, proofOk, capabilityOk,
          replayOk, inspectStatus, eventsProjection

vars == <<phase, sourceStored, artifactStored, acceptedEnvelope,
          runHeaderStored, journal, ack, yamlParserUsedAfterAdmission,
          digestOk, artifactOk, gateOk, proofOk, capabilityOk,
          replayOk, inspectStatus, eventsProjection>>

TypeOK ==
  /\ phase \in Phase
  /\ sourceStored \in BOOLEAN
  /\ artifactStored \in BOOLEAN
  /\ acceptedEnvelope \in BOOLEAN
  /\ runHeaderStored \in BOOLEAN
  /\ journal \in Seq(Event)
  /\ ack \in Ack
  /\ yamlParserUsedAfterAdmission \in BOOLEAN
  /\ digestOk \in BOOLEAN
  /\ artifactOk \in BOOLEAN
  /\ gateOk \in BOOLEAN
  /\ proofOk \in BOOLEAN
  /\ capabilityOk \in BOOLEAN
  /\ replayOk \in BOOLEAN
  /\ inspectStatus \in Projection
  /\ eventsProjection \in Projection

AcceptedEnvelopeOk ==
  sourceStored /\ artifactStored /\ runHeaderStored /\ acceptedEnvelope
  /\ digestOk /\ artifactOk /\ gateOk /\ proofOk /\ capabilityOk

HasEvent(e) == \E i \in 1..Len(journal): journal[i] = e

EventIndex(e) == CHOOSE i \in 1..Len(journal): journal[i] = e

EventBeforeOrAt(a, b) ==
  HasEvent(a) /\ HasEvent(b) /\ EventIndex(a) <= EventIndex(b)

DurableAcceptedPrefix ==
  sourceStored /\ artifactStored /\ runHeaderStored /\ HasEvent("RunAccepted")

DurableAdmissionPrefix ==
  DurableAcceptedPrefix /\ HasEvent("RunAdmission")

TerminalJournaled == HasEvent("RunFinished") \/ HasEvent("RunFailed")

Init ==
  /\ phase = "Cold"
  /\ sourceStored = FALSE
  /\ artifactStored = FALSE
  /\ acceptedEnvelope = FALSE
  /\ runHeaderStored = FALSE
  /\ journal = <<>>
  /\ ack = "none"
  /\ yamlParserUsedAfterAdmission = FALSE
  /\ digestOk \in Bool
  /\ artifactOk \in Bool
  /\ gateOk \in Bool
  /\ proofOk \in Bool
  /\ capabilityOk \in Bool
  /\ replayOk \in Bool
  /\ inspectStatus = "none"
  /\ eventsProjection = "none"

RejectSourceDigest ==
  /\ phase = "Cold"
  /\ ~digestOk
  /\ phase' = "Failed"
  /\ journal' = Append(journal, "RunFailed")
  /\ ack' = "failed"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, yamlParserUsedAfterAdmission, digestOk,
                  artifactOk, gateOk, proofOk, capabilityOk, replayOk,
                  inspectStatus, eventsProjection>>

ValidateYaml ==
  /\ phase = "Cold"
  /\ digestOk
  /\ phase' = "YamlValidated"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, journal, ack, yamlParserUsedAfterAdmission,
                  digestOk, artifactOk, gateOk, proofOk, capabilityOk,
                  replayOk, inspectStatus, eventsProjection>>

PersistSource ==
  /\ phase = "YamlValidated"
  /\ phase' = "SourcePersisted"
  /\ sourceStored' = TRUE
  /\ UNCHANGED <<artifactStored, acceptedEnvelope, runHeaderStored, journal,
                  ack, yamlParserUsedAfterAdmission, digestOk, artifactOk,
                  gateOk, proofOk, capabilityOk, replayOk, inspectStatus,
                  eventsProjection>>

RejectArtifactDigest ==
  /\ phase = "SourcePersisted"
  /\ ~artifactOk
  /\ phase' = "Failed"
  /\ journal' = Append(journal, "RunFailed")
  /\ ack' = "failed"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, yamlParserUsedAfterAdmission, digestOk,
                  artifactOk, gateOk, proofOk, capabilityOk, replayOk,
                  inspectStatus, eventsProjection>>

PersistArtifact ==
  /\ phase = "SourcePersisted"
  /\ artifactOk
  /\ phase' = "ArtifactPersisted"
  /\ artifactStored' = TRUE
  /\ UNCHANGED <<sourceStored, acceptedEnvelope, runHeaderStored, journal,
                  ack, yamlParserUsedAfterAdmission, digestOk, artifactOk,
                  gateOk, proofOk, capabilityOk, replayOk, inspectStatus,
                  eventsProjection>>

PersistRunHeader ==
  /\ phase = "ArtifactPersisted"
  /\ phase' = "RunHeaderPersisted"
  /\ runHeaderStored' = TRUE
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope, journal,
                  ack, yamlParserUsedAfterAdmission, digestOk, artifactOk,
                  gateOk, proofOk, capabilityOk, replayOk, inspectStatus,
                  eventsProjection>>

AppendRunAccepted ==
  /\ phase = "RunHeaderPersisted"
  /\ sourceStored /\ artifactStored /\ runHeaderStored
  /\ phase' = "Accepted"
  /\ acceptedEnvelope' = TRUE
  /\ journal' = Append(journal, "RunAccepted")
  /\ ack' = "accepted"
  /\ UNCHANGED <<sourceStored, artifactStored, runHeaderStored,
                  yamlParserUsedAfterAdmission, digestOk, artifactOk, gateOk,
                  proofOk, capabilityOk, replayOk, inspectStatus,
                  eventsProjection>>

AdmitAcceptedArtifact ==
  /\ phase = "Accepted"
  /\ AcceptedEnvelopeOk
  /\ phase' = "Admitted"
  /\ journal' = Append(journal, "RunAdmission")
  /\ ack' = "admitted"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, yamlParserUsedAfterAdmission, digestOk,
                  artifactOk, gateOk, proofOk, capabilityOk, replayOk,
                  inspectStatus, eventsProjection>>

RejectAdmission ==
  /\ phase = "Accepted"
  /\ ~AcceptedEnvelopeOk
  /\ phase' = "Failed"
  /\ journal' = Append(Append(journal, "AdmissionRejected"), "RunFailed")
  /\ ack' = "failed"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, yamlParserUsedAfterAdmission, digestOk,
                  artifactOk, gateOk, proofOk, capabilityOk, replayOk,
                  inspectStatus, eventsProjection>>

StartRuntime ==
  /\ phase = "Admitted"
  /\ DurableAdmissionPrefix
  /\ phase' = "Running"
  /\ ack' = "running"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, journal, yamlParserUsedAfterAdmission,
                  digestOk, artifactOk, gateOk, proofOk, capabilityOk,
                  replayOk, inspectStatus, eventsProjection>>

SuspendRuntime ==
  /\ phase = "Running"
  /\ phase' = "Suspended"
  /\ ack' = "suspended"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, journal, yamlParserUsedAfterAdmission,
                  digestOk, artifactOk, gateOk, proofOk, capabilityOk,
                  replayOk, inspectStatus, eventsProjection>>

FinishRuntime ==
  /\ phase \in {"Running", "Suspended"}
  /\ phase' = "Finished"
  /\ journal' = Append(journal, "RunFinished")
  /\ ack' = "finished"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, yamlParserUsedAfterAdmission, digestOk,
                  artifactOk, gateOk, proofOk, capabilityOk, replayOk,
                  inspectStatus, eventsProjection>>

CrashRestart ==
  /\ phase \in {"Accepted", "Admitted", "Running", "Suspended", "Finished"}
  /\ phase' = "Restarted"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, journal, ack, yamlParserUsedAfterAdmission,
                  digestOk, artifactOk, gateOk, proofOk, capabilityOk,
                  replayOk, inspectStatus, eventsProjection>>

RecoverFromJournal ==
  /\ phase = "Restarted"
  /\ DurableAdmissionPrefix
  /\ AcceptedEnvelopeOk
  /\ replayOk
  /\ ~yamlParserUsedAfterAdmission
  /\ phase' = "Recovered"
  /\ ack' = "recovered"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, journal, yamlParserUsedAfterAdmission,
                  digestOk, artifactOk, gateOk, proofOk, capabilityOk,
                  replayOk, inspectStatus, eventsProjection>>

RejectRecovery ==
  /\ phase = "Restarted"
  /\ (~DurableAdmissionPrefix \/ ~AcceptedEnvelopeOk \/ ~replayOk)
  /\ phase' = "Failed"
  /\ journal' = Append(Append(journal, "RecoveryRejected"), "RunFailed")
  /\ ack' = "failed"
  /\ UNCHANGED <<sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, yamlParserUsedAfterAdmission, digestOk,
                  artifactOk, gateOk, proofOk, capabilityOk, replayOk,
                  inspectStatus, eventsProjection>>

Inspect ==
  /\ inspectStatus' = IF phase = "Finished" THEN "terminal"
                      ELSE IF phase = "Recovered" THEN "recovered"
                      ELSE IF phase = "Failed" THEN "failed"
                      ELSE IF HasEvent("RunAdmission") THEN "admitted"
                      ELSE IF HasEvent("RunAccepted") THEN "accepted"
                      ELSE "none"
  /\ UNCHANGED <<phase, sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, journal, ack, yamlParserUsedAfterAdmission,
                  digestOk, artifactOk, gateOk, proofOk, capabilityOk,
                  replayOk, eventsProjection>>

Events ==
  /\ eventsProjection' = IF HasEvent("RunFinished") THEN "terminal"
                         ELSE IF HasEvent("RunFailed") THEN "failed"
                         ELSE IF HasEvent("RunAdmission") THEN "admitted"
                         ELSE IF HasEvent("RunAccepted") THEN "accepted"
                         ELSE "none"
  /\ UNCHANGED <<phase, sourceStored, artifactStored, acceptedEnvelope,
                  runHeaderStored, journal, ack, yamlParserUsedAfterAdmission,
                  digestOk, artifactOk, gateOk, proofOk, capabilityOk,
                  replayOk, inspectStatus>>

Next == RejectSourceDigest \/ ValidateYaml \/ PersistSource \/ RejectArtifactDigest
        \/ PersistArtifact \/ PersistRunHeader \/ AppendRunAccepted
        \/ AdmitAcceptedArtifact \/ RejectAdmission \/ StartRuntime
        \/ SuspendRuntime \/ FinishRuntime \/ CrashRestart \/ RecoverFromJournal
        \/ RejectRecovery \/ Inspect \/ Events

Spec == Init /\ [][Next]_vars /\ WF_vars(RecoverFromJournal \/ RejectRecovery)

StrictAdmissionOnlyAcceptedArtifact ==
  phase \in {"Admitted", "Running", "Suspended", "Finished", "Recovered"}
  => AcceptedEnvelopeOk /\ HasEvent("RunAccepted")

SuccessfulRunHasEvidence ==
  phase \in {"Finished", "Recovered"}
  => sourceStored /\ artifactStored /\ runHeaderStored /\ acceptedEnvelope
     /\ HasEvent("RunAccepted")

PersistBeforeAck ==
  /\ ack \in {"accepted", "admitted", "running", "finished", "suspended", "recovered"}
     => DurableAcceptedPrefix
  /\ ack \in {"admitted", "running", "finished", "suspended"}
     => DurableAdmissionPrefix

JournalPrefixDurable ==
  /\ HasEvent("RunAdmission") => DurableAcceptedPrefix
  /\ HasEvent("RunFinished") => DurableAdmissionPrefix
  /\ HasEvent("RunAdmission") => EventBeforeOrAt("RunAccepted", "RunAdmission")
  /\ HasEvent("RunFinished") => EventBeforeOrAt("RunAdmission", "RunFinished")
  /\ phase = "Recovered" => DurableAdmissionPrefix

InspectEventsReflectJournal ==
  /\ inspectStatus = "accepted" => HasEvent("RunAccepted")
  /\ inspectStatus = "admitted" => HasEvent("RunAdmission")
  /\ inspectStatus = "terminal" => TerminalJournaled
  /\ inspectStatus = "recovered" => phase = "Recovered" /\ DurableAcceptedPrefix
  /\ eventsProjection = "accepted" => HasEvent("RunAccepted")
  /\ eventsProjection = "admitted" => HasEvent("RunAdmission")
  /\ eventsProjection = "terminal" => HasEvent("RunFinished")
  /\ eventsProjection = "failed" => HasEvent("RunFailed")

NoYamlParseAfterAdmission ==
  phase \in {"Accepted", "Admitted", "Running", "Suspended", "Finished",
             "Restarted", "Recovered"}
  => ~yamlParserUsedAfterAdmission

RecoveryInputsPersistedOnly ==
  phase = "Recovered" => DurableAdmissionPrefix /\ AcceptedEnvelopeOk /\ replayOk

RecoveredStateRefinesJournal ==
  phase = "Recovered" => DurableAdmissionPrefix

MismatchFailsClosed ==
  (~digestOk \/ ~artifactOk \/ ~gateOk \/ ~proofOk \/ ~capabilityOk)
  => phase \notin {"Admitted", "Running", "Suspended", "Finished", "Recovered"}

StrictAdmissionOnlyAcceptedArtifactAlways == []StrictAdmissionOnlyAcceptedArtifact

PersistBeforeAckAlways == []PersistBeforeAck

JournalPrefixDurableAlways == []JournalPrefixDurable

NoYamlParseAfterAdmissionAlways == []NoYamlParseAfterAdmission

RecoveryInputsPersistedOnlyAlways == []RecoveryInputsPersistedOnly

AfterRestartEventuallyRecoveredOrTypedFailure ==
  [](phase = "Restarted" => <>(phase = "Recovered" \/ phase = "Failed"))

====
