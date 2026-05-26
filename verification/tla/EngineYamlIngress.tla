---- MODULE EngineYamlIngress ----
EXTENDS Naturals

\* Obligations: PO-005 / PRE-006 / POST-007. Bounded direct/API and IPC
\* ingress model. Unsupported YAML/JSON/HTTP/text runtime protocols reject with
\* typed diagnostics; full queues reject deterministically without growth.

CONSTANTS Capacity, MaxEvents

VARIABLES direct_queue, ipc_queue, accepted, rejected, ingress_mode,
          protocol_kind, diagnostic_class, full_submit_observed,
          full_submit_rejected, unsupported_submit_observed,
          unsupported_submit_rejected

vars == <<direct_queue, ipc_queue, accepted, rejected, ingress_mode,
          protocol_kind, diagnostic_class, full_submit_observed,
          full_submit_rejected, unsupported_submit_observed,
          unsupported_submit_rejected>>

SupportedProtocol == {"direct_api", "binary_ipc"}
UnsupportedProtocol == {"yaml", "json", "http", "text_command"}
ProtocolKind == SupportedProtocol \cup UnsupportedProtocol
DiagnosticClass == {"none", "accepted_artifact", "unsupported_runtime_protocol",
                    "artifact_not_accepted", "backpressure"}

TypeOK ==
  /\ direct_queue \in 0..Capacity
  /\ ipc_queue \in 0..Capacity
  /\ accepted \in 0..MaxEvents
  /\ rejected \in 0..MaxEvents
  /\ ingress_mode \in {"idle", "direct", "ipc", "direct_backpressure",
                      "ipc_backpressure", "unsupported_protocol",
                      "artifact_not_accepted"}
  /\ protocol_kind \in ProtocolKind
  /\ diagnostic_class \in DiagnosticClass
  /\ full_submit_observed \in BOOLEAN
  /\ full_submit_rejected \in BOOLEAN
  /\ unsupported_submit_observed \in BOOLEAN
  /\ unsupported_submit_rejected \in BOOLEAN

Init ==
  /\ direct_queue = 0
  /\ ipc_queue = 0
  /\ accepted = 0
  /\ rejected = 0
  /\ ingress_mode = "idle"
  /\ protocol_kind = "direct_api"
  /\ diagnostic_class = "none"
  /\ full_submit_observed = FALSE
  /\ full_submit_rejected = FALSE
  /\ unsupported_submit_observed = FALSE
  /\ unsupported_submit_rejected = FALSE

SubmitDirectAccept ==
  /\ direct_queue < Capacity
  /\ accepted + rejected < MaxEvents
  /\ direct_queue' = direct_queue + 1
  /\ accepted' = accepted + 1
  /\ ingress_mode' = "direct"
  /\ protocol_kind' = "direct_api"
  /\ diagnostic_class' = "accepted_artifact"
  /\ UNCHANGED <<ipc_queue, rejected, full_submit_observed, full_submit_rejected,
                 unsupported_submit_observed, unsupported_submit_rejected>>

SubmitIpcAccept ==
  /\ ipc_queue < Capacity
  /\ accepted + rejected < MaxEvents
  /\ ipc_queue' = ipc_queue + 1
  /\ accepted' = accepted + 1
  /\ ingress_mode' = "ipc"
  /\ protocol_kind' = "binary_ipc"
  /\ diagnostic_class' = "accepted_artifact"
  /\ UNCHANGED <<direct_queue, rejected, full_submit_observed, full_submit_rejected,
                 unsupported_submit_observed, unsupported_submit_rejected>>

SubmitDirectReject ==
  /\ direct_queue = Capacity
  /\ accepted + rejected < MaxEvents
  /\ rejected' = rejected + 1
  /\ ingress_mode' = "direct_backpressure"
  /\ protocol_kind' = "direct_api"
  /\ diagnostic_class' = "backpressure"
  /\ full_submit_observed' = TRUE
  /\ full_submit_rejected' = TRUE
  /\ UNCHANGED <<direct_queue, ipc_queue, accepted,
                 unsupported_submit_observed, unsupported_submit_rejected>>

SubmitIpcReject ==
  /\ ipc_queue = Capacity
  /\ accepted + rejected < MaxEvents
  /\ rejected' = rejected + 1
  /\ ingress_mode' = "ipc_backpressure"
  /\ protocol_kind' = "binary_ipc"
  /\ diagnostic_class' = "backpressure"
  /\ full_submit_observed' = TRUE
  /\ full_submit_rejected' = TRUE
  /\ UNCHANGED <<direct_queue, ipc_queue, accepted,
                 unsupported_submit_observed, unsupported_submit_rejected>>

SubmitUnsupportedProtocolReject(k) ==
  /\ k \in UnsupportedProtocol
  /\ accepted + rejected < MaxEvents
  /\ rejected' = rejected + 1
  /\ ingress_mode' = "unsupported_protocol"
  /\ protocol_kind' = k
  /\ diagnostic_class' = "unsupported_runtime_protocol"
  /\ unsupported_submit_observed' = TRUE
  /\ unsupported_submit_rejected' = TRUE
  /\ UNCHANGED <<direct_queue, ipc_queue, accepted,
                 full_submit_observed, full_submit_rejected>>

SubmitArtifactNotAcceptedReject ==
  /\ accepted + rejected < MaxEvents
  /\ rejected' = rejected + 1
  /\ ingress_mode' = "artifact_not_accepted"
  /\ protocol_kind' \in SupportedProtocol
  /\ diagnostic_class' = "artifact_not_accepted"
  /\ UNCHANGED <<direct_queue, ipc_queue, accepted,
                 full_submit_observed, full_submit_rejected,
                 unsupported_submit_observed, unsupported_submit_rejected>>

DrainDirect ==
  /\ direct_queue > 0
  /\ direct_queue' = direct_queue - 1
  /\ UNCHANGED <<ipc_queue, accepted, rejected, ingress_mode,
                 protocol_kind, diagnostic_class, full_submit_observed,
                 full_submit_rejected, unsupported_submit_observed,
                 unsupported_submit_rejected>>

DrainIpc ==
  /\ ipc_queue > 0
  /\ ipc_queue' = ipc_queue - 1
  /\ UNCHANGED <<direct_queue, accepted, rejected, ingress_mode,
                 protocol_kind, diagnostic_class, full_submit_observed,
                 full_submit_rejected, unsupported_submit_observed,
                 unsupported_submit_rejected>>

Stutter == UNCHANGED vars

IngressProgress == SubmitDirectAccept \/ SubmitIpcAccept \/ SubmitDirectReject
                   \/ SubmitIpcReject \/ SubmitArtifactNotAcceptedReject
                   \/ (\E k \in UnsupportedProtocol: SubmitUnsupportedProtocolReject(k))
                   \/ DrainDirect \/ DrainIpc

Next == IngressProgress \/ Stutter

Spec == Init /\ [][Next]_vars /\ WF_vars(IngressProgress)

BoundedIngress ==
  /\ direct_queue >= 0 /\ direct_queue <= Capacity
  /\ ipc_queue >= 0 /\ ipc_queue <= Capacity
  /\ accepted + rejected <= MaxEvents
  /\ protocol_kind \in ProtocolKind
  /\ diagnostic_class \in DiagnosticClass

NoIngressBypass ==
  /\ ingress_mode \in {"idle", "direct", "ipc", "direct_backpressure",
                        "ipc_backpressure", "unsupported_protocol",
                        "artifact_not_accepted"}
  /\ protocol_kind \in UnsupportedProtocol => diagnostic_class = "unsupported_runtime_protocol"
  /\ protocol_kind \in UnsupportedProtocol => ingress_mode /= "direct"
  /\ protocol_kind \in UnsupportedProtocol => ingress_mode /= "ipc"

TypedOperatorOutcome ==
  /\ (diagnostic_class = "accepted_artifact" => accepted > 0)
  /\ (diagnostic_class = "unsupported_runtime_protocol" =>
        /\ protocol_kind \in UnsupportedProtocol
        /\ rejected > 0
        /\ unsupported_submit_observed
        /\ unsupported_submit_rejected)
  /\ (diagnostic_class = "artifact_not_accepted" =>
        /\ protocol_kind \in SupportedProtocol
        /\ rejected > 0)
  /\ (diagnostic_class = "backpressure" =>
        /\ protocol_kind \in SupportedProtocol
        /\ rejected > 0
        /\ full_submit_observed
        /\ full_submit_rejected)

BackpressureRejects ==
  full_submit_observed => full_submit_rejected

UnsupportedProtocolRejects ==
  unsupported_submit_observed => unsupported_submit_rejected

FullQueueRejectsWithoutGrowth ==
  ingress_mode \in {"direct_backpressure", "ipc_backpressure"} =>
    /\ full_submit_observed
    /\ full_submit_rejected
    /\ rejected > 0
    /\ direct_queue <= Capacity
    /\ ipc_queue <= Capacity
    /\ diagnostic_class = "backpressure"

UnsupportedProtocolsNeverAccepted ==
  protocol_kind \in UnsupportedProtocol =>
    /\ diagnostic_class = "unsupported_runtime_protocol"
    /\ ingress_mode = "unsupported_protocol"
    /\ unsupported_submit_rejected

EventuallyAcceptsOrTypedRejects == <>(accepted + rejected > 0)

====
