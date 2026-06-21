# RS-218-core-inspect-formatter-run-mismatch: Snapshot formatter can print a run id different from the snapshot's run

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/introspection.rs:216`
- **Confidence**: confirmed

## Description
`InspectSnapshotFormatter::format_snapshot` accepts a separate `run` parameter even though `InspectResponse::Found` already contains an `InspectSnapshot` with its own run id. The `Found` branch prints the external parameter, so a mismatched call can produce diagnostic output that attributes one run's program counter to another run id.

## Evidence
```rust
216:     pub fn format_snapshot(run: RunId, response: &InspectResponse) -> String {
217:         match response {
218:             InspectResponse::Found(snap) => {
219:                 format!(
220:                     "InspectSnapshot {{ run: {:?}, correlation: {}, pc: {:?}, executed: {} }}",
221:                     run, snap.correlation, snap.pc, snap.executed
222:                 )
223:             }
```

`InspectSnapshot` carries `run`, but this branch ignores `snap.run`. The other branches destructure and print the run id stored inside the response variant, making `Found` inconsistent with the rest of the formatter.

## Adversarial Check
This is not harmless formatting trivia for operators: the formatted output is a diagnostic surface. If a caller accidentally passes a stale or requested run id with a `Found` response from another run, the log/string becomes internally false while still looking well-formed.

## Suggested Fix
Remove the external `run` parameter and format entirely from `InspectResponse`, or use `snap.run` in the `Found` branch to make all variants response-authoritative.
