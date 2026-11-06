# Binary IPC

The fastest control path is direct in-process Rust API. IPC exists for external local processes and must remain binary.

## Current Scope

`vb-ipc` currently exposes bounded in-memory ingress:

```text
MemoryIngress
IngressFrame
BoundedPayload
MaxPayloadBytes
QueueCapacity
```

Producers use `try_submit`. Full queues return `IpcError::Full`. Empty receives return `Ok(None)`.

## Frame Contract

`IngressFrame` carries:

```text
RunId
WorkflowDigest
bounded bytes payload
```

The default maximum payload is 1 MiB. Payload parsing and input-slot mapping are cold boundary work.

## Wire Frame

Production `vb-ipc` uses a fixed 24-byte little-endian header. It is separate
from the 60-byte storage record envelope and therefore has no storage-envelope
BLAKE3 digest or CRC32C fields.

```text
magic:       u32 = 0x56424C54
version:     u16
command:     u16
flags:       u16
reserved:    u16 = 0
correlation: u64
payload_len: u32
payload:     postcard bytes
```

## Forbidden

```text
HTTP ingress
JSON routing
unbounded channels
blocking producer admission
```

Future transports include Unix domain sockets and shared-memory rings, using the same bounded binary frame contract.
