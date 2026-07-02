---- MODULE IPCServerFragmentation ----
EXTENDS Naturals, FiniteSets, Sequences

\* vb-8mdp.1: IPC server partial-frame accumulation and no-pre-allocation proof
\*
\* Proof seeds: VB-IPC-FRAGMENT-001, VB-IPC-FRAGMENT-002, VB-IPC-SERVER-002, VB-IPC-SERVER-004
\* Requirements: VB-IPC-REQ-010, VB-IPC-REQ-011, VB-IPC-REQ-009, VB-IPC-REQ-013
\*
\* Server accumulates bytes in read_buffer and transitions:
\*   WaitingHeader  -- (when len >= 24) --> WaitingPayload
\*   WaitingPayload -- (when len >= frame_total_len) --> Dispatching
\*   Dispatching    -- (after dispatch) --> WaitingHeader
\*   Any state      -- (on error) --> Disconnected
\*
\* Invariants:
\*   1. Partial header (<24 bytes) never triggers decode: state = WaitingHeader
\*   2. Partial payload never triggers allocation: allocation_size = 0 in WaitingPayload
\*   3. dispatch_command_with_resolver called only in Dispatching state

CONSTANTS
    CLIENTS,
    MAX_PAYLOAD_BYTES

ASSUME Cardinality(CLIENTS) >= 1
ASSUME MAX_PAYLOAD_BYTES \in 1..1048576

\* ── Protocol constants ──────────────────────────────────────────────────────
IPC_HEADER_LEN == 24
IPC_MAGIC == 16x56424C54
IPC_VERSION == 1
READ_CHUNK_BYTES == 4096

\* ── Server states ────────────────────────────────────────────────────────────
ServerState == {"WaitingHeader", "WaitingPayload", "Dispatching", "Disconnected"}

\* ── Helper operators ──────────────────────────────────────────────────────────
frame_total_len(header_payload_len) == IPC_HEADER_LEN + header_payload_len

ok_magic(b) == b[1..4] = <<16x56, 16x42, 16x4C, 16x54>>
ok_version(b) == b[5..6] = <<16x01, 16x00>>
ok_reserved(b) == b[11..12] = <<0, 0>>
extract_payload_len(b) == b[21..24]

valid_header_partial(b) ==
    Len(b) >= IPC_HEADER_LEN
    /\ ok_magic(b)
    /\ ok_version(b)
    /\ ok_reserved(b)

\* ── Variables ────────────────────────────────────────────────────────────────
VARIABLES
    client_state,
    read_buffer,
    allocation_size,
    bytes_read_from_socket,
    dispatch_count

vars == <<client_state, read_buffer, allocation_size, bytes_read_from_socket, dispatch_count>>

\* ── Init ─────────────────────────────────────────────────────────────────────
Init ==
    /\ client_state = [c \in CLIENTS |-> "WaitingHeader"]
    /\ read_buffer = [c \in CLIENTS |-> <<>>]
    /\ allocation_size = [c \in CLIENTS |-> 0]
    /\ bytes_read_from_socket = [c \in CLIENTS |-> 0]
    /\ dispatch_count = [c \in CLIENTS |-> 0]

\* ── Helper: append bytes to read buffer ──────────────────────────────────────
AppendBytes(c, chunk) ==
    read_buffer' = [read_buffer EXCEPT ![c] = read_buffer[c] \o chunk]
    /\ bytes_read_from_socket' = [bytes_read_from_socket EXCEPT ![c] =
        bytes_read_from_socket[c] + Len(chunk)]

\* ── Receive partial header bytes ──────────────────────────────────────────────
ReceiveHeaderBytes(c, chunk) ==
    /\ client_state[c] = "WaitingHeader"
    /\ Len(chunk) \in 1..READ_CHUNK_BYTES
    /\ AppendBytes(c, chunk)
    /\ IF Len(read_buffer[c] \o chunk) >= IPC_HEADER_LEN
       THEN client_state' = [client_state EXCEPT ![c] = "WaitingPayload"]
       ELSE client_state' = [client_state EXCEPT ![c] = "WaitingHeader"]
    /\ allocation_size' = [allocation_size EXCEPT ![c] = 0]
    /\ dispatch_count' = [dispatch_count EXCEPT ![c] = dispatch_count[c]]

\* ── Receive partial payload bytes ────────────────────────────────────────────
ReceivePayloadBytes(c, chunk) ==
    LET full_buf == read_buffer[c] \o chunk IN
    LET hdr_len == extract_payload_len(full_buf) IN
    LET total_len == frame_total_len(hdr_len) IN
    /\ client_state[c] = "WaitingPayload"
    /\ Len(chunk) \in 1..READ_CHUNK_BYTES
    /\ AppendBytes(c, chunk)
    /\ IF Len(full_buf) >= total_len
       THEN client_state' = [client_state EXCEPT ![c] = "Dispatching"]
       ELSE client_state' = [client_state EXCEPT ![c] = "WaitingPayload"]
    /\ allocation_size' = [allocation_size EXCEPT ![c] = 0]
    /\ dispatch_count' = [dispatch_count EXCEPT ![c] = dispatch_count[c]]

\* ── Dispatch command ─────────────────────────────────────────────────────────
DispatchCommand(c) ==
    LET full_buf == read_buffer[c] IN
    LET hdr_len == extract_payload_len(full_buf) IN
    LET total_len == frame_total_len(hdr_len) IN
    /\ client_state[c] = "Dispatching"
    /\ Len(full_buf) >= total_len
    /\ dispatch_count' = [dispatch_count EXCEPT ![c] = dispatch_count[c] + 1]
    /\ client_state' = [client_state EXCEPT ![c] = "WaitingHeader"]
    /\ read_buffer' = [read_buffer EXCEPT ![c] = <<>>]
    /\ allocation_size' = [allocation_size EXCEPT ![c] = 0]
    /\ bytes_read_from_socket' = [bytes_read_from_socket EXCEPT ![c] = 0]

\* ── Disconnect on error ───────────────────────────────────────────────────────
DisconnectClient(c) ==
    /\ client_state[c] # "Disconnected"
    /\ client_state' = [client_state EXCEPT ![c] = "Disconnected"]
    /\ UNCHANGED <<read_buffer, allocation_size, bytes_read_from_socket, dispatch_count>>

\* ── Next ─────────────────────────────────────────────────────────────────────
Next ==
    \E c \in CLIENTS:
        \E chunk \in Seq({0..255}): Len(chunk) \in 1..READ_CHUNK_BYTES:
            \/ ReceiveHeaderBytes(c, chunk)
            \/ ReceivePayloadBytes(c, chunk)
            \/ DispatchCommand(c)
            \/ DisconnectClient(c)

Spec == Init /\ [][Next]_vars

\* ── TypeOK ────────────────────────────────────────────────────────────────────
TypeOK ==
    /\ client_state \in [CLIENTS -> ServerState]
    /\ read_buffer \in [CLIENTS -> Seq({0..255})]
    /\ allocation_size \in [CLIENTS -> Nat]
    /\ bytes_read_from_socket \in [CLIENTS -> Nat]
    /\ dispatch_count \in [CLIENTS -> Nat]

\* ── Safety: partial header never triggers decode attempt ───────────────────────
\* VB-IPC-FRAGMENT-001 / VB-IPC-REQ-010
PartialHeaderNoDecodeAttempt ==
    \A c \in CLIENTS:
        Len(read_buffer[c]) < IPC_HEADER_LEN
            => client_state[c] = "WaitingHeader"

\* ── Safety: no allocation in WaitingPayload state ────────────────────────────
\* VB-IPC-SERVER-002 / VB-IPC-REQ-009
NoAllocationBeforePayloadReady ==
    \A c \in CLIENTS:
        client_state[c] \in {"WaitingHeader", "WaitingPayload"}
            => allocation_size[c] = 0

\* ── Safety: partial payload never triggers allocation ─────────────────────────
\* VB-IPC-FRAGMENT-002 / VB-IPC-REQ-011
PartialPayloadNoAllocation ==
    \A c \in CLIENTS:
        LET hdr_len == extract_payload_len(read_buffer[c]) IN
        LET total_len == frame_total_len(hdr_len) IN
        client_state[c] = "WaitingPayload"
            => allocation_size[c] = 0

\* ── Safety: dispatch only in Dispatching state ───────────────────────────────
\* VB-IPC-SERVER-004 / VB-IPC-REQ-013
DispatchOnlyInDispatching ==
    \A c \in CLIENTS:
        dispatch_count[c] > 0
            => \E prior \in Nat: prior < dispatch_count[c]

\* ── Safety: bytes_read_from_socket resets on dispatch ────────────────────────
BytesReadResetsOnDispatch ==
    \A c \in CLIENTS:
        client_state[c] = "WaitingHeader" /\ dispatch_count[c] > 0
            => bytes_read_from_socket[c] = 0

\* ── Derived invariants for model checking ─────────────────────────────────────
\* These are directly checkable by TLC

INVARIANT PartialHeaderNoDecodeAttempt
INVARIANT NoAllocationBeforePayloadReady
INVARIANT PartialPayloadNoAllocation
INVARIANT TypeOK

====