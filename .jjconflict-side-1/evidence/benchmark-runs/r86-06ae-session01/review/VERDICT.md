# ACCEPT — limited recorded-session mean nonregression

Reviewed immutable session /home/lewis/tmp/vb-f1f5-benchmark-sessions-r86/session01, all four predeclared A1/B1/B2/A2 runs and separate candidate counter phase. Accept the three observed-session mean comparisons under the frozen recipe and its unchanged 3%/5%/3% limits. This accepts neither a noise-free/causal speedup claim, individual-operation tail improvement, counter improvement, policy publication, generated/maxperf performance, nor full F1–F5/release acceptance. No run was selected, removed, retried or reweighted by this review.

## Source and execution authority

Candidate exact 06ae752b84172c5484f09e45386208b794cba954 versus baseline exact 7f505d498fa35acb5b70b083ac6f02cd40f3a891 with only approved benchmark overlay 0175f57b1a63ef7a9a36c67ea0422106c7b4917d124c26944542b9383b968fed. Independently read Git blobs and actual file bytes: all 2,546 candidate and 2,178 baseline relevant inputs/modes match their source fingerprints, with exact overlay applied in memory. No staged or untracked relevant files. Both Cargo.lock hashes match. Baseline production is unmodified; the comparison measures the whole candidate pin, including upstream integration, against that overlaid baseline rather than attributing change to any single F1/F2 repair.

Verified every FROZEN.sha256 and BUILD-FROZEN.sha256 entry, plan/protocol hashes, exact run order, all five invocation/exit/source-before/source-after records, build/preflight binary receipts, shared YAML and IPC fixture hashes. Both actual native preflight logs contain ENGINE_PREFLIGHT_OK (Continue/I64(1)/pc1 and Finished(I64(1))) plus IPC_PREFLIGHT_OK (SubmitRun run5/input[1,2,3,4]), exit0. Read actual preflight source: compile/execute/encode/decode comparisons return errors on mismatches. No preflight/native rerun was launched. Bench artifacts on both sides use opt3/debuginfo2/debug_assertions=false/overflow_checks=false, the bench profile rather than hardened; pinned nightly-2026-04-28, RUSTFLAGS=-Dwarnings. Actual latency commands all pin CPU14 and use sample-size100/warmup3s/measurement5s. Source guard/export remain 100000us; no guard or threshold was raised.

The r86 refusal names Moon1473894 and cargo-fmt1474009. Recipe control flow refuses before preflight invocation or any run directory/timing creation; the later r86b native preflight succeeds. No measured data came from that refused attempt. Subsequent latency and counter runner logs complete with exit0. This is not adoption of unrelated suite/proof success or a canonical gate-pass claim.

## Independent arithmetic

All 12 raw new/sample.json + new/estimates.json pairs were inspected with independent Python Fraction/Decimal parsing. Each mean estimate matches the arithmetic sample mean to normal floating serialization precision. Side means use exactly two unrounded decimal mean estimates with equal run weight, not iteration weighting. All six final integer-ps values round the exact side mean once. Each side pools all 200 time/iteration ratios; nearest-rank p50/p95/p99 and printed percent deltas match within the documented 18-place rendering. 176 recorded recipe/protocol/source/latency checks passed; additional source/host/counter checks are recorded separately.

| Metric | Baseline mean ns | Candidate mean ns | Delta | Limit | Worst of all four A/B mean pairings |
|---|---:|---:|---:|---:|---:|
| SaveConst step | 43.475893927706702 | 40.879784926938365 | -5.971375781450846% | +3% | -1.986707026756677% |
| IPC frame decode | 69.15586775889006 | 70.713892427994255 | +2.252917531938437% | +5% | +3.602274558853563% |
| budget10 small workflow | 67.652184247104685 | 60.0373197328068475 | -11.255903411017703% | +3% | -8.639580232321096% |

Worst-pair analysis is sensitivity evidence, not a replacement acceptance rule or permission to select runs. Every pairing still meets its predeclared mean limit, so the aggregate verdict does not depend on averaging a failing single-run mean with a winning one.

## Host and variance judgment

All ten host snapshots agree on machine/kernel/model/profile and governor/driver/frequency limits/boost. CPU14 is core14, SMT sibling30, performance/amd-pstate-epp, 32MiB L3 shared by CPUs8–15/24–31, with boost enabled. Recorded Tctl endpoints range54.125–67.5C across latency/counters; no thermal-policy change or concrete throttling event is established. Endpoints do not prove absence of transient throttling. Host identity/source/cpu placement remain consistent.

SaveConst B1=42.43618383575015ns versus B2=39.32338601812658ns: -7.335% run drift, sample CV3.711%/9.234%. B2 includes high sample means up to61.540ns; its third index-quartile mean41.586ns versus roughly38.3–38.8ns elsewhere shows bursts, not uniformly steady timing. Baseline engine A1=69.53549044486147ns versus A2=65.7688780493479ns: -5.417%; A1 sample CV4.932%, A2 1.113%. Candidate engine means remain close,60.08672306237707/59.987916403236625ns. These observations preclude presenting exact aggregate deltas as stable causal optimization gains.

The recorded five-second prewindow sample has CPU14 busy5/501 ticks=0.9980%, sibling30 busy13/500=2.6%. It is not a continuous timing-window isolation record. vmstat intervals (initial since-boot average excluded) show nonzero background swap-in, occasional block activity, and system idle averaging94.94%,96.00%,95.36%,92.06% in A1/B1/B2/A2. B1 briefly reports10 blocked tasks and76KiB/s swap-out; this is system-wide, not attribution to benchmark memory. Normal desktop/model-server tasks remain visible. A new opencode.exe at230% and ChatGPT102% appear at B2's after snapshot, with128%/51% immediately before A2. These ps values are lifetime averages, not proof of activity on CPU14/30 during a particular sample. Snapshots and vmstat contain no observed competing build/proof job during the successful runs. CPU PSI some avg10 is0 at latency endpoints, then22.06 after the instrumented counter phase; profiler workers sharing pinned affinity can themselves create scheduling pressure, so this does not establish foreign latency contamination.

No continuous per-CPU/per-process/frequency monitor exists. Concrete timed foreign-work overlap or a causal link between background work and drift cannot be established or excluded. I found no specific host evidence that invalidates the limited observed-session mean comparison; accepting it does not certify a wholly idle host. Sensitivity across every mean pairing supports bounded nonregression despite that uncertainty. No claim about broad loaded-workflow or durable I/O performance follows from these three small filtered IR subjects.

SaveConst pooled sample-mean p99 rises44.203054374711286013→51.543366956864872864ns (+16.6059%); p95 also slightly rises. This is a material adverse tail observation retained in the verdict. It is not individual-operation p99, and the recipe's mean threshold pass does not imply tail nonregression. Pooled ratios represent Criterion sample averages over many operations, not200 independent operation latencies or200 independent benchmark runs. Only two runs per side were taken.

## Counter parsing and limits

Independently parsed all three raw perf.csv files: exactly one numeric instructions:u row,100.00% running, no scaled/multiplexed or unsupported result. Parsed every histogram row as exact allocation size/count; count agrees with both heaptrack raw stats and heaptrack_print calls to allocation functions. Histogram size*count sums match aggregate bytes. All six perf/heaptrack records use10 samples. Exact output counts:

| Metric | User instructions | Allocation calls | Bytes allocated |
|---|---:|---:|---:|
| SaveConst | 48605463648 | 11376611 | 136253159 |
| IPC decode | 31626032474 | 8454037 | 240554614 |
| budget10 workflow | 34396748570 | 11138671 | 134745305 |

These are candidate-only whole filtered-process totals from separate perf and heaptrack invocations, including startup, all suite fixture registration/setup, calibration/warmup and measured work. Different tools use different adaptive iteration totals (e.g. SaveConst perf about17M measured versus heaptrack about1.6M); totals cannot be combined into same-work per-operation costs. No baseline counter comparison or zero-allocation transition claim is established. Instrumented timings are excluded from all latency statistics. Raw compressed heaptrack data exists and is hashed/preserved, not reprocessed with a new native profiler job during HOLD.

## Freeze and scope

INPUTS.json hashes all218 session files; reread verification confirms all unchanged. SOURCE-AND-COUNTER-AUDIT.json, LATENCY-AUDIT.json, HOST-AND-COMMAND-AUDIT.json and CHECKS.json retain independent results. This review writes only its own external directory/archive. No source, gate, recipe, policy/evidence-publication or raw measurement bytes changed; no build/proof/fmt/Moon/GOTO/native job or benchmark retry launched; no JJ operations or foreign process signals. HOLD remains until parent release. Other pending protected approvals and unfinished core baseline claim remain outside this verdict.

Reference guidance used: /home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md and runtime-performance-architecture.md. Review does not request any new rerun.
