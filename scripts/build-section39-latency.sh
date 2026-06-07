#!/usr/bin/env bash
# Build evidence/section39-latency.jsonl from the 3 *.percentiles.jsonl sidecars
# joined with section39-metadata.jsonl on metric/bench_id.
# Reads source from existing per-bench sidecars; emits the consolidated file.
set -euo pipefail

LOGS_DIR="evidence/benchmark-logs"
METADATA="evidence/section39-metadata.jsonl"
OUT="evidence/section39-latency.jsonl"
SCHEMA="section39-latency/v1"

[ -d "$LOGS_DIR" ] || { echo "missing $LOGS_DIR" >&2; exit 1; }
[ -f "$METADATA" ] || { echo "missing $METADATA" >&2; exit 1; }

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

for sidecar in "$LOGS_DIR"/*.percentiles.jsonl; do
    [ -f "$sidecar" ] || continue
    bench_id="$(jq -r '.bench_id' "$sidecar")"
    metric="$bench_id"
    metadata_row="$(jq -c --arg m "$metric" 'select(.metric == $m)' "$METADATA" | head -1)"
    if [ -z "$metadata_row" ]; then
        echo "WARN: no metadata row for $metric" >&2
        metadata_row="{}"
    fi
    jq -c \
        --argjson meta "$metadata_row" \
        --arg schema "$SCHEMA" \
        --arg sidecar "$sidecar" \
        --arg bench_id "$bench_id" \
        '{
            metric: .bench_id,
            bench_id: $bench_id,
            p50_latency_ns: .p50_latency_ns,
            p95_latency_ns: .p95_latency_ns,
            p99_latency_ns: .p99_latency_ns,
            sample_count: .sample_count,
            min_ns: .min_ns,
            max_ns: .max_ns,
            mean_ns: .mean_ns,
            total_ns: .total_ns,
            fixture_digest: $meta.fixture_digest,
            durability_mode: $meta.durability_mode,
            execution_mode: ($meta.execution_mode // $meta.mode),
            commit: $meta.commit,
            rustc: $meta.rustc,
            rustc_commit: $meta.rustc_commit,
            rustflags: $meta.rustflags,
            cpu_governor: $meta.cpu_governor,
            kernel: $meta.kernel,
            timestamp: $meta.timestamp,
            command: $meta.command,
            raw_log: $sidecar,
            tool_kind: "criterion-iter-custom-percentiles",
            schema_version: $schema
        }' "$sidecar" >> "$tmp"
done

mv "$tmp" "$OUT"
echo "wrote $OUT with $(wc -l < "$OUT") rows"
