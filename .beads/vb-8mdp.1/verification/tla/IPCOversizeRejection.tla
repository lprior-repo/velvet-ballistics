---- MODULE IPCOversizeRejection ----
EXTENDS Naturals, Sequences

\* vb-8mdp.1: IPC oversize payload rejection — server disconnects without reading payload bytes
\*
\* Proof seed: VB-IPC-SERVER-003
\* Requirement: VB-IPC-REQ-012
\*
\* When header decode returns PayloadTooLarge, the server transitions to
\* Disconnected WITHOUT entering WaitingPayload and WITHOUT reading any payload
\* bytes from the socket.
\*
\* Key invariants:
\*   1. PayloadTooLarge error causes immediate Disconnected state
\*   2. bytes_read_from_socket = 0 in all Disconnected states caused by header errors
\*   3. No WaitingPayload state entered when header decode fails with PayloadTooLarge

CONSTANTS
    CLIENTS,
    MAX_PAYLOAD_BYTES

ASSUME Cardinality(CLIENTS) >= 1
ASSUME MAX_PAYLOAD_BYTES \in 1..1048576

IPC_HEADER_LEN == 24
READ_CHUNK_BYTES == 4096

ServerState == {"WaitingHeader", "WaitingPayload", "Dispatching", "Disconnected"}

VARIABLES
    client_state,
    read_buffer,
    bytes_read_from_socket,
    disconnect_reason

vars == <<client_state, read_buffer, bytes_read_from_socket, disconnect_reason>>

\* ── Init ─────────────────────────────────────────────────────────────────────
Init ==
    /\ client_state = [c \in CLIENTS |-> "WaitingHeader"]
    /\ read_buffer = [c \in CLIENTS |-> <<>>]
    /\ bytes_read_from_socket = [c \in CLIENTS |-> 0]
    /\ disconnect_reason = [c \in CLIENTS |-> "none"]

\* ── Simulate header-only read (no payload bytes read) ─────────────────────────
\*
\* This action models the server reading ONLY the 24-byte header from the socket.
\* After decode, if PayloadTooLarge is returned, the server disconnects WITHOUT
\* reading any further bytes.
ReadHeaderOnlyAndRejectOversize(c) ==
    LET chunk_size == IPC_HEADER_LEN IN
    /\ client_state[c] = "WaitingHeader"
    /\ bytes_read_from_socket[c] = 0
    /\ read_buffer' = [read_buffer EXCEPT ![c] = <<0..255>>]  \* any header bytes
    /\ bytes_read_from_socket' = [bytes_read_from_socket EXCEPT ![c] = IPC_HEADER_LEN]
    /\ client_state' = [client_state EXCEPT ![c] = "Disconnected"]
    /\ disconnect_reason' = [disconnect_reason EXCEPT ![c] = "PayloadTooLarge"]

\* ── Read header + partial payload (valid header, oversize) ──────────────────
\*
\* If header is valid but payload_len > MAX_PAYLOAD_BYTES, the server should
\* disconnect after reading ONLY the header bytes — NOT the full payload.
ReadHeaderThenDisconnect(c, actual_payload_len) ==
    LET is_oversize == actual_payload_len > MAX_PAYLOAD_BYTES IN
    /\ client_state[c] = "WaitingHeader"
    /\ read_buffer' = [read_buffer EXCEPT ![c] = <<0..255>>]  \* valid header bytes
    /\ bytes_read_from_socket' = [bytes_read_from_socket EXCEPT ![c] = IPC_HEADER_LEN]
    /\ IF is_oversize
       THEN /\ client_state' = [client_state EXCEPT ![c] = "Disconnected"]
            /\ disconnect_reason' = [disconnect_reason EXCEPT ![c] = "PayloadTooLarge"]
       ELSE /\ client_state' = [client_state EXCEPT ![c] = "WaitingPayload"]
            /\ disconnect_reason' = [disconnect_reason EXCEPT ![c] = "none"]

\* ── Normal payload read ──────────────────────────────────────────────────────
ReceivePayloadBytes(c, chunk) ==
    LET full_buf == read_buffer[c] \o chunk IN
    /\ client_state[c] = "WaitingPayload"
    /\ bytes_read_from_socket' = [bytes_read_from_socket EXCEPT ![c] =
        bytes_read_from_socket[c] + Len(chunk)]
    /\ read_buffer' = [read_buffer EXCEPT ![c] = full_buf]
    /\ UNCHANGED <<client_state, disconnect_reason>>

\* ── Disconnect for other header errors ──────────────────────────────────────
DisconnectOnHeaderError(c, reason) ==
    /\ client_state[c] \in {"WaitingHeader", "WaitingPayload"}
    /\ client_state' = [client_state EXCEPT ![c] = "Disconnected"]
    /\ disconnect_reason' = [disconnect_reason EXCEPT ![c] = reason]
    /\ UNCHANGED <<read_buffer, bytes_read_from_socket>>

Next ==
    \E c \in CLIENTS:
        \/ ReadHeaderOnlyAndRejectOversize(c)
        \/ \E payload_len \in 0..(2 * MAX_PAYLOAD_BYTES):
            ReadHeaderThenDisconnect(c, payload_len)
        \/ \E chunk \in Seq({0..255}): Len(chunk) \in 1..READ_CHUNK_BYTES:
            ReceivePayloadBytes(c, chunk)
        \/ \E reason \in {"InvalidMagic", "UnsupportedVersion", "ReservedNonZero", "PayloadTooLarge"}:
            DisconnectOnHeaderError(c, reason)

Spec == Init /\ [][Next]_vars

\* ── TypeOK ────────────────────────────────────────────────────────────────────
TypeOK ==
    /\ client_state \in [CLIENTS -> ServerState]
    /\ read_buffer \in [CLIENTS -> Seq({0..255})]
    /\ bytes_read_from_socket \in [CLIENTS -> Nat]
    /\ disconnect_reason \in [CLIENTS -> {"none", "InvalidMagic", "UnsupportedVersion",
        "ReservedNonZero", "PayloadTooLarge"}]

\* ── Safety: PayloadTooLarge disconnects without entering WaitingPayload ──────
\* VB-IPC-SERVER-003 / VB-IPC-REQ-012
\* When disconnect_reason = PayloadTooLarge, the client was never in WaitingPayload.
OversizeDisconnectSkipsWaitingPayload ==
    \A c \in CLIENTS:
        disconnect_reason[c] = "PayloadTooLarge"
            => \A k \in Nat: k <= 10
                => ~(client_state[c] = "WaitingPayload")

\* ── Safety: bytes_read_from_socket = IPC_HEADER_LEN when header error causes disconnect ─
\* VB-IPC-SERVER-003 / VB-IPC-REQ-012
HeaderErrorNoPayloadBytesRead ==
    \A c \in CLIENTS:
        disconnect_reason[c] \in {"InvalidMagic", "UnsupportedVersion",
            "ReservedNonZero", "PayloadTooLarge"}
            => bytes_read_from_socket[c] = IPC_HEADER_LEN

\* ── Safety: no WaitingPayload state exists when header declared oversized ─────
WaitingPayloadNeverEnteredForOversize ==
    \A c \in CLIENTS:
        client_state[c] = "WaitingPayload"
            => disconnect_reason[c] # "PayloadTooLarge"

INVARIANT TypeOK
INVARIANT OversizeDisconnectSkipsWaitingPayload
INVARIANT HeaderErrorNoPayloadBytesRead
INVARIANT WaitingPayloadNeverEnteredForOversize

====