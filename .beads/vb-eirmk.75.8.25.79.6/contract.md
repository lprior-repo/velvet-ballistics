# Recovered IPC write-path characterization

Integrate only the existing two-file reconstruction `ba449de0e70bac1ac06ac1044bf184da83dd64ff`, preserving current main `ce3931b28dd18800f7957424737fbb5f0390007d`. The original November 13 implementation and raw review receipts are unavailable; their approvals and execution results do not transfer.

The private shared write helper must retain the current production path's exact pending-byte order, readiness transitions, progress classifications and original I/O error. The production caller remains the existing `mio` stream; no public API, wire format, scheduling budget or execution authority changes. The writer satisfies the standard `Write` contract, including a returned byte count within the offered slice. Deterministic test writers enforce that same contract.

Executable cases must cover ordered partial writes, `WouldBlock`, `Interrupted`, zero progress and fatal write errors, retaining both byte content and readiness. Existing real-stream partial-write tests remain present. No source or assertion weakening is allowed; current pinned package checks and independent CODE/TEST reviews precede central landing.
