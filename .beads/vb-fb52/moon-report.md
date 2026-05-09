# Moon CI Gate Report — vb-fb52

**Workspace:** `/home/lewis/src/Velvet-ballistics/vb-fb52-ws`
**Date:** 2026-05-09

## Gate Results

| Gate | Command | Exit Code | Status |
|------|---------|-----------|--------|
| `:quick` | `moon run :quick` | 0 | **PASS** |
| `:test` | `moon run :test` | 0 | **PASS** |
| `:ci` | `moon run :ci` | 1 | **NO TASKS** |

## Details

### `:quick`
- **Exit Code:** 0
- **Duration:** ~1m 24s
- **Result:** PASS
- **Output:** Installed rust nightly-2026-04-28, ran "Hello, world!" hello test

### `:test`
- **Exit Code:** 0
- **Duration:** ~4m 52s
- **Result:** PASS
- **Output:** 10301 tests run: 10301 passed, 0 skipped

### `:ci`
- **Exit Code:** 1
- **Result:** FAILED — "No tasks found. Unable to execute action pipeline. For targets :ci."
- **Note:** No `:ci` task is defined in the workspace's moon configuration. See available tasks below.

## Available Tasks
```
:agent-cli-contract  :bench-build         :benchmark-proof    :check
:coverage            :doc                 :doc-test           :feature-powerset
:fmt                 :fuzz-smoke          :hardened-build     :lint-src
:maxperf             :maxperf-native      :miri               :mutants-smoke
:nightly-feature-cargo-probe :nightly-feature-gate :pgo-instrument-build :pgo-optimized-build
:quick               :sanitizer-address-check :source-length    :supply-chain
:test                :verify-all          :verify-deep        :verify-fast
:verify-proof        :verify-standard
```

## Conclusion

- `:quick` — **PASS**
- `:test` — **PASS** (10301/10301 tests)
- `:ci` — **NO TASKS** (`:ci` not defined in moon.yml)
