# vb-jpq7.24 Raw Evidence Logs

## Verus bound exec

```text
2026-05-23T14:14:57Z
/home/lewis/src/vb-jpq7-24-verus-binding-gpt55
COMMAND=verus verification/verus/vb_jpq724_events_for_run_production.rs
vtsmnwnxqpunpozwsoxoupvwxyuwqxtz ef74f15b903f4ed86dfcb47b2526f1180809a495 vb-jpq7.24 bind Verus evidence to production exec
/home/lewis/.local/bin/verus
Verus
  Version: 0.2026.05.05.d03e906
  Profile: release
  Platform: linux_x86_64
  Toolchain: 1.95.0-x86_64-unknown-linux-gnu

verification results:: 8 verified, 0 errors
EXIT_CODE=0
```

## verusfmt check

```text
2026-05-23T14:15:47Z
/home/lewis/src/vb-jpq7-24-verus-binding-gpt55
COMMAND=verusfmt --check verification/verus/vb_jpq724_events_for_run_production.rs
/home/lewis/.cargo/bin/verusfmt
EXIT_CODE=0
```

## Trust marker scan

```text
2026-05-23T14:15:47Z
/home/lewis/src/vb-jpq7-24-verus-binding-gpt55
COMMAND=rtk grep -n trust markers scoped files
vtsmnwnxqpunpozwsoxoupvwxyuwqxtz ef74f15b903f4ed86dfcb47b2526f1180809a495 vb-jpq7.24 bind Verus evidence to production exec
0 matches for 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom'
RAW_EXIT_CODE=1
EXIT_CODE=0 (no trust markers found)
```

## Scoped Rust seam regression

```text
2026-05-23T14:15:47Z
/home/lewis/src/vb-jpq7-24-verus-binding-gpt55
COMMAND=rtk cargo test -p vb_storage events_for_run -- --nocapture
vtsmnwnxqpunpozwsoxoupvwxyuwqxtz ef74f15b903f4ed86dfcb47b2526f1180809a495 vb-jpq7.24 bind Verus evidence to production exec
cargo test: 24 passed, 1030 filtered out (4 suites, 0.04s)
EXIT_CODE=0
```
