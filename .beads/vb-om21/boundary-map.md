# Boundary Map — vb-om21

## Pure Core

Pure logic should be separable from Fjall iteration:

- Build/check byte prefix shape from typed `RunId`.
- Decode sequence bytes from a validated `RunEventKey`.
- Fold observed sequences into `max_seq`.
- Convert max sequence to `JournalTail` using checked arithmetic.
- Compare declared metadata tail with reconstructed tail.
- Classify empty prefix as zero-tail query or `MissingJournal` recovery failure.

## Imperative Storage Shell

- `FjallJournal` owns the `events` keyspace.
- Tail scan uses Fjall snapshot/range/prefix iteration.
- Storage bytes are parsed at the boundary and admitted into pure core only after prefix and length checks.
- No writes are needed for tail reconstruction.

## Recovery Boundary

- Recovery APIs such as `recover_runtime_summary`, `recover_runtime_frame_seed`, and any new tail fallback helper must translate storage/journal errors into `RecoveryError` without string erasure.
- Recovery must decide tail trust before replay can truncate committed events.

## Parser/Codec Boundary

- Fjall key bytes are not self-validating; validate length and prefix before sequence extraction.
- Journal payload decoding remains responsible for record magic, schema, digest, and Postcard validation during replay.
- Tail scan by final key should not require payload decode, but replay cannot skip payload validation.

## Concurrency / Snapshot Boundary

- Use an immutable Fjall snapshot or equivalent consistent read view for scan consistency.
- Do not expose partially changing tail observations during concurrent appends.
- No async/task spawning is needed; storage remains synchronous and bounded by prefix.

## Forbidden Boundaries

- No runtime YAML, JSON, HTTP, or text command parsing.
- No external Restate code/API/layout copying.
- No global mutable state or cross-run tail cache without a separate contract.
- No unsafe code.
