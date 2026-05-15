# TLA+ Temporal Model Plan: vb-core-ipc-loom-property

## Boundary

- **TLA+-owned temporal behavior**:
  - MemoryIngress bounded queue: multi-producer `try_submit` / `try_recv` channel operations with capacity backpressure
  - IPC server client-map: token uniqueness and concurrent client registration/deregistration in poll loop
  - Write buffer: fill/drain byte conservation on the non-WouldBlock path

- **Rust/core behavior excluded from TLA+** (handled by loom, Kani, Verus):
  - FramePool capacity invariant: proven by loom model + VB-CONC-005 pattern
  - ActionTicket completion/cancel: VB-CONC-002 (existing loom model)
  - TimerWheel fire/cancel ordering: VB-CONC-003 (existing loom model)
  - Shutdown drain ordering: VB-CONC-004 (existing loom model)

- **External systems abstracted**:
  - `crossbeam_channel` library internals (loom tests our usage surface only)
  - `mio` event loop (poll loop is single-threaded; TLA+ models the structural invariant, not the event loop)
  - `HashMap` internal rehashing (model assumes no hash collision attacks; capacity bounded by MAX_CLIENTS=256)

- **Non-applicability rationale**: This bead is about loom property evidence for existing IPC seams. TLA+ is used here to formally specify the invariants that the loom models test. Full TLA+ model-checking of the IPC server is out of scope — loom models cover the concurrent data structure invariants.

---

## TLA+-Owned Clauses

### INV-001: MemoryIngress Bounded Channel Backpressure

**Module**: `MemoryIngressChannel`

**Variables**:
- `queued: [0..CAPACITY]` — current number of frames in channel
- `CAPACITY: Nat` — channel capacity (a constant, bounded for TLC)

**Init**:
```
queued = 0
```

**Actions**:
- `TrySubmit`: `queued < CAPACITY /\ queued' = queued + 1`
- `TryRecv`: `queued > 0 /\ queued' = queued - 1`
- `SubmitFull`: `queued = CAPACITY /\ UNCHANGED queued`
- `RecvEmpty`: `queued = 0 /\ UNCHANGED queued`

**Invariant**:
- `queued <= CAPACITY`
- `queued >= 0` (TLA+ Int is unbounded, this constrains the domain)

**Temporal Property**:
- `[][queued <= CAPACITY]_queued` — safety: never exceeds capacity
- `[]<>(queued >= 0)` — always non-negative (follows from Init + actions)

**Fairness**: Weak fairness on `TrySubmit` and `TryRecv` (channel operations are always enabled when preconditions hold).

**State Constraints**: `queued <= CAPACITY` (for bounded model exploration)

**Refinement to Rust**:
- `MemoryIngress::try_submit` corresponds to `TrySubmit` when channel has capacity, `SubmitFull` when full
- `MemoryIngress::try_recv` corresponds to `TryRecv` when channel has items, `RecvEmpty` when empty
- `queued` is an abstract approximation of `receiver.len()` (crossbeam may use different representation)

**Evidence Command**:
```
cd specs && tlc -config MemoryIngressChannel.cfg MemoryIngressChannel.tla
```

---

### INV-003: IPC Server Client-Map Token Uniqueness

**Module**: `IpcServerClientMap`

**Variables**:
- `clients: [Token -> ClientConnection \/ None]` — partial function from token to connection
- `nextToken: Nat` — monotonic token allocator
- `active: set of Token` — set of currently registered tokens

**Init**:
```
clients = [t \in {} |-> None]
nextToken = 1
active = {}
```

**Actions**:
- `AcceptClient`: Generate new token `t = nextToken`; `nextToken' = nextToken + 1`; `clients' = clients @@ (t :> conn)`; `active' = active \cup {t}`
- `RemoveClient(t)`: `t \in active /\ clients' = clients @@ (t :> None)`; `active' = active \ {t}`
- `GetClient(t)`: `t \in active /\ UNCHANGED (clients, nextToken, active)`

**Invariant**:
- ` Cardinality(active) <= MAX_CLIENTS` (256)
- `dom(clients) = active \cup {t \in dom(clients): clients[t] = None}` (keys never disappear)
- Token injectivity: each token maps to at most one live connection

**Temporal Property**:
- `[][Cardinality(active) <= MAX_CLIENTS]_clients` — never exceed max clients

**Refinement to Rust**:
- `IpcServer::accept_client` refines `AcceptClient`
- `IpcServer::remove_client` refines `RemoveClient`
- `handle_readable/get_mut` refines `GetClient`

**Evidence Command**:
```
cd specs && tlc -config IpcServerClientMap.cfg IpcServerClientMap.tla
```

---

### INV-004: Write Buffer Byte Conservation

**Module**: `WriteBuffer`

**Variables**:
- `buffer: Seq(Byte)` — current write buffer contents
- `written: Nat` — total bytes written to buffer (for tracking)
- `drained: Nat` — total bytes drained from buffer

**Init**:
```
buffer = <<>>
written = 0
drained = 0
```

**Actions**:
- `Fill(bytes)`: `buffer' = buffer \o bytes`; `written' = written + Len(bytes)`
- `Drain(n)`: `n <= Len(buffer) /\ buffer' = SubSeq(buffer, n+1, Len(buffer))`; `drained' = drained + n`
- `WouldBlock`: `UNCHANGED (buffer, written, drained)`

**Invariant**:
- `Len(buffer) = written - drained` (conservation equation)
- `written >= drained` (drained never exceeds written)
- `drained >= 0`

**Temporal Property**:
- `[][Len(buffer) = written - drained]_(buffer, written, drained)` — conservation

**Refinement to Rust**:
- `handle_readable` response encoding refines `Fill`
- `handle_writable` `client.write_buffer.drain(..written)` refines `Drain`
- `WouldBlock` error path corresponds to `WouldBlock` action

**Evidence Command**:
```
cd specs && tlc -config WriteBuffer.cfg WriteBuffer.tla
```

---

## Waivers

- **VB-CONC-001 (JournalWriterQueue)**: Covered by existing loom model; TLA+ not needed for this narrow invariant
- **VB-CONC-002 (ActionTicket)**: Covered by existing loom model; TLA+ would add no value over the existing atomic bool proof
- **VB-CONC-003 (TimerWheel)**: Covered by existing loom model; TLA+ would not improve on the existing mutex-based proof
- **VB-CONC-004 (ShutdownDrain)**: Covered by existing loom model; TLA+ would not add temporal value over existing atomic counter model
- **VB-CONC-005 (BoundedQueue)**: Covered by existing loom model; same pattern as the new FramePool loom model

---

## Bounded Model Limits for TLC

- `CAPACITY = 4` for MemoryIngress model (scales to actual capacity at runtime)
- `MAX_CLIENTS = 256` for client-map (symmetry set of size 4 for token exploration)
- `MaxBytes = 64` for write-buffer model
- All models use `Spec` with fairness disabled for invariant checking; enable for liveness checking
