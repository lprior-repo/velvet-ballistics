# Audit and repair contract: vb-cyz6d / vb-2a9i3 / vb-fnugl

Source baseline: 856603b8. Master sections 18, 20, 21, 36, 49, 50, 55, 61, 62 and normative 77.3 remain binding. No codegen/GUI/maxperf acceptance is claimed.

The synchronous numeric transition core remains synchronous. The shell owns mio readiness and sockets; the storage adapter owns bounded journal admission/commit. There are no production task regions or Future cancellation points to retrofit with Tokio.

## Ubiquitous language and legal states

A client owns at most one bounded request being assembled and one response awaiting write completion. Readiness is a retained service obligation until WouldBlock, EOF, terminal error, or bounded work-budget rescheduling. An output buffer cannot be overwritten while its response remains incomplete. Invalid frames may transition to closing after their error response is flushed. Overload rejects a new socket and preserves admitted clients. Peer transport failures are observable bounded diagnostics, distinct from reactor/poll infrastructure failures.

States: Reading(magic/header/payload) -> Responding(bytes remaining) -> Reading; invalid header -> Closing(response bytes remaining) -> Closed; EOF/peer write error -> Closed. The implementation may retain buffer/magic DTO fields but response ownership and readiness must be explicit. Empty buffers mean no partial response. A partial response suppresses additional request dispatch. Every poll turn has fixed client, accept, read, and write operation budgets. Budget exhaustion retains a ready obligation and forces a nonblocking next poll; WouldBlock clears the obligation until a new readiness event.

## Invariants and typed errors

Magic and header bounds are checked before payload-buffer growth. A request larger than a 4096-byte read chunk must complete without new peer traffic. A listener burst must accept all admitted clients without a fresh edge. The 257th peer is closed at the 256-client bound. Existing clients remain usable. Request and output Vec growth is bounded, checked and fallibly reserved; existing postcard serialization allocator behavior remains. Pending output submission returns a typed pending-response error without changing bytes. Partial writes track consumed bytes; zero writes close the peer. Deregistration failure is recorded as a typed diagnostic, never silently discarded.

The CLI retains its journal and queue capabilities and services one bounded queue batch before polling and one after runtime progress. Journaled acknowledgement remains queue-admission acknowledgement under master section 18/49; strict persistence-before-ack semantics remain unchanged. Normal service performs real journal commits before shutdown. Flush errors propagate as service failure rather than being counted as successful durable writes. All-idempotent batches require an explicit barrier because Fjall empty batch commit skips persistence. Graceful shutdown still runs the existing runtime durability barrier. The reactor stops admission/dispatch after a successful shutdown response and keeps queued output alive until writable service completes. Runtime-wide shutdown report/signature changes are separately tracked in vb-aaqp7 and require coordination with recovery owner.

## Behavior scenarios

Complete 8216-byte request -> healthy response within bounded poll turns; three preconnected clients -> three correlated healthy responses; 256 admitted clients plus one -> extra EOF and first client still healthy; socket WouldBlock with pending correlation11 then attempted22 -> typed refusal and identical pending bytes; fragmented header/payload and partial response -> no duplicate or overwritten frame; malformed/broken peer -> service keeps other admitted clients usable; queued journal work -> commit visibility during service and unchanged queue on commit failure.

## Hazards and limits

Edge-trigger starvation, response truncation, unbounded drains, overload service denial, allocation failure, shutdown acknowledgement loss, unserviced writer queues, and disconnected model evidence are audit hazards. Existing toy Loom tests do not prove production shutdown. Codec timings do not establish reactor p99 or durable throughput. Broad moon CI belongs to vb-xx331. No full proof-planning pipeline or verifier closure is claimed by this bounded repair contract.
