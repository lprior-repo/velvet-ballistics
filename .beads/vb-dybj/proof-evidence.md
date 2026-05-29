# Proof Evidence - vb-dybj State 5 final review repair attempt 7

bead_id: vb-dybj  
writer_invocation_id: proof-writer-vb-dybj-state5-final-007  
state: 5  
sublane: final-review-repair

## Archived rejected State 6 artifacts

Active rejected State 6 artifacts were removed from the active bead surface and archived under `.beads/vb-dybj/archive/state6-rejected-20260525-final-003/`:

- `proof-review.md` sha256 `422578e4d5318db3859d742c39602185c9d56834c77e47bd850b9b6cf340432e`
- `proof-findings.jsonl` sha256 `8d0a7d27982699bc7a3ec7d21658fe168d3db12c25bce68dd1ab583d2f8ca1f9`
- `transcript-state6-proof-reviewer.md` sha256 `219b7d922cc6ffe20f0d064e2374fb91f1ae5dd4d8694c0e0528cc7f33e5ac41`

## Trailing-byte exact-boundary repair - PO-VB-DYBJ-013, PO-VB-DYBJ-014, PO-VB-DYBJ-015

Repair: the proof/test/fuzz artifacts now model an explicit exact Postcard decode boundary with `postcard::take_from_bytes::<WorkflowDigest>` and reject `Ok((_value, remaining))` when `remaining` is nonempty. This addresses the reviewer counterexample without weakening the property to prefix acceptance and without editing production behavior.

Touched artifacts:

- `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`
- `crates/workspace_tests/src/kani_vb_dybj_trailing_decode.rs`
- `fuzz/fuzz_targets/vb_dybj_trailing_decode.rs`

### Proptest command - PO-VB-DYBJ-014

```bash
rtk cargo test -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests trailing_bytes -- --nocapture
```

Exit: PASS.

```text
cargo test: 1 passed, 8 filtered out (1 suite, 0.01s)
```

### Kani command - PO-VB-DYBJ-013

```bash
cargo kani -p velvet-ballistics-workspace-tests --harness kani_vb_dybj_trailing_bytes_rejected --output-format regular
```

Exit: PASS. Full raw output captured at `/home/lewis/.local/share/opencode/tool-output/tool_e617c407c001SQhenBHqjTIA6X`.

```text
Kani Rust Verifier 0.67.0 (cargo plugin)
CBMC 6.8.0 (cbmc-6.8.0)
SUMMARY:
 ** 0 of 238 failed (5 unreachable)
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

Bounds/assumptions: `suffix_len` is symbolic and constrained to `1_usize..=8_usize`; `suffix_byte` and the 32 digest bytes are symbolic; loop unwind bound is `#[kani::unwind(9)]`. Kani emitted unsupported-construct warnings for `caller_location` and a foreign function, with no reachable failure in this harness.

### cargo-fuzz trailing command - PO-VB-DYBJ-015

```bash
cargo fuzz run vb_dybj_trailing_decode --target x86_64-unknown-linux-gnu -- -max_total_time=10 -runs=1000
```

Exit: PASS.

```text
#1000 DONE   cov: 19 ft: 20 corp: 1/1b lim: 11 exec/s: 0 rss: 46Mb
Done 1000 runs in 0 second(s)
```

## cargo-fuzz storage-short planned bound - PO-VB-DYBJ-012

Command:

```bash
cargo fuzz run vb_dybj_storage_short_decode --target x86_64-unknown-linux-gnu -- -max_total_time=60 -runs=10000
```

Exit: PASS.

```text
#10000 DONE   cov: 181 ft: 197 corp: 2/61b lim: 98 exec/s: 0 rss: 46Mb
Done 10000 runs in 0 second(s)
```

This satisfies the planned run-count bound with an explicit GNU target selection retained from prior sanitizer compatibility evidence.

## Flux evidence - PO-VB-DYBJ-005

Command:

```bash
cargo flux --manifest-path verification/flux/Cargo.toml
```

Exit: RECORDED_TOOLING_GAP (Flux attribute crate unresolved).

```text
error[E0463]: can't find crate for `flux_rs`
error: cannot find attribute `sig` in this scope
error: cannot find attribute `refined_by` in this scope
error: cannot find attribute `field` in this scope
error: could not compile `vb-dybj-flux-artifacts` (lib) due to 6 previous errors
```

Disposition: PO-VB-DYBJ-005 is not discharged in State 5. The isolated Flux package lacks a resolvable Flux attribute crate/integration boundary.

## Existing evidence retained from prior State 5 attempt

- PO-VB-DYBJ-001/004/007: Verus artifacts verify as standalone mapped models, including planned `--extern` invocations, but remain a production-binding trust boundary.
- PO-VB-DYBJ-002: RunId Kani harness verified successfully under symbolic `u64` bounds.
- PO-VB-DYBJ-008/010: selected vb_storage Kani harnesses remain blocked by unrelated existing cfg(kani) compile errors in `kani_recovery_hydrate.rs`.
- PO-VB-DYBJ-016: TLC lifecycle model previously passed with 52,165 generated states, 14,641 distinct states, depth 9.

## Final State 5 disposition

The concrete trailing-byte counterexample has been repaired in proof/test/fuzz artifact scope by selecting an explicit exact/no-trailing decode boundary. Storage-short fuzz now meets the planned bound. Flux, vb_storage Kani, and Verus production-binding gaps remain honestly recorded trust boundaries. No final proof success is claimed for unresolved obligations.
