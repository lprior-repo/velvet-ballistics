#!/usr/bin/env python3
"""Deterministic Section 36/39 coverage audit.

Default mode is an honest audit: it exits 0 after printing COVERED/GAP rows so
the parent bead can split uncovered work without hiding it.  Use
``--strict-complete`` when a release gate must fail on any missing test,
benchmark, or benchmark-metadata requirement.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BENCH_PATH = ROOT / "crates/workspace_tests/benches/velvet_ballistics.rs"
METADATA_PATH = ROOT / "evidence/section39-metadata.jsonl"
BENCH_EVIDENCE_PATH = ROOT / "evidence/benchmark-evidence.jsonl"
LATENCY_PATH = ROOT / "evidence/section39-latency.jsonl"
INSTRUCTION_PATH = ROOT / "evidence/instruction-counts.jsonl"
ALLOC_PATH = ROOT / "evidence/alloc-evidence.jsonl"


@dataclass(frozen=True)
class TokenExpectation:
    section: str
    requirement: str
    tokens: tuple[str, ...]


@dataclass(frozen=True)
class BenchRequirement:
    area: str
    requirement: str
    accepted_ids: tuple[str, ...]


SECTION36_EXPECTATIONS: tuple[TokenExpectation, ...] = (
    TokenExpectation(
        "S36",
        "core value and ID tests",
        (
            "finite_f64_addition_via_expr_yields_finite_result",
            "slot_value_type_name_null_is_stable",
            "const_value_to_slot_value_no_silent_null_fallback",
            "step_budget_exhaustion_returns_false_without_error",
            "run_frame_out_of_bounds_slot_access_returns_typed_error",
            "try_from_parts_rejects_invalid_entry_pc",
        ),
    ),
    TokenExpectation(
        "S36",
        "parser and validator tests",
        (
            "validate_workflow_schema_detects_duplicate_top_level_key",
            "yaml_rejects_anchor_alias_merge_exact_variant",
            "yaml_rejects_ambiguous_yes_scalar_exact_variant",
            "bdd_g13_rejects_direct_cycle",
            "validation_error_duplicate_key_display",
        ),
    ),
    TokenExpectation(
        "S36",
        "engine invariant tests",
        (
            "terminal_succeeded_state_rejects_transition_to_running",
            "failed_step_does_not_become_succeeded_without_error_handler",
            "budget_exhaustion_does_not_advance_pc",
            "missing_output_slot_returns_typed_error",
            "budget_exhaustion_then_resume_advances_correctly",
        ),
    ),
    TokenExpectation(
        "S36",
        "recovery tests",
        (
            "given_scheduled_action_before_crash_when_recovered_then_pending_action_blocks_redispatch",
            "given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable",
            "proptest_valid_slot_events_are_fully_hydrateable",
            "given_explicit_replay_limit_when_more_events_exist_then_too_many_events_and_code_are_returned",
        ),
    ),
    TokenExpectation(
        "S36",
        "IPC tests",
        (
            "bad magic",
            "PayloadTooLarge",
            "backpressure",
            "SubmitRun",
            "malformed",
        ),
    ),
    TokenExpectation(
        "S36",
        "scheduler tests",
        (
            "QueueFull",
            "shutdown_drains_before_journal_drain",
            "action_completion_transitions_run_from_resumable_to_running_then_finished",
            "TimerFired",
        ),
    ),
    TokenExpectation(
        "S36",
        "compile-fail scope documented",
        (
            "Generated Rust compile-fail tests are removed with `vb_codegen`",
            "exit_code_three_on_compile_failure",
        ),
    ),
)


SECTION39_BENCH_REQUIREMENTS: tuple[BenchRequirement, ...] = (
    BenchRequirement("YAML parsing", "small workflow", ("parse_yaml_small",)),
    BenchRequirement("YAML parsing", "large 1 MiB workflow", ("parse_yaml_1mb",)),
    BenchRequirement("Validation", "minimal workflow", ("validate_minimal",)),
    BenchRequirement("Validation", "1000-step workflow", ("validate_1000_steps",)),
    BenchRequirement("Compilation", "minimal workflow", ("compile_ir_minimal",)),
    BenchRequirement("Compilation", "1000-step workflow", ("compile_ir_1000_steps",)),
    BenchRequirement("Expression", "symbol equality", ("expr_eq_symbol",)),
    BenchRequirement("Expression", "number comparison", ("expr_number_compare",)),
    BenchRequirement("Expression", "boolean chain", ("expr_boolean_chain",)),
    BenchRequirement("Expression", "arithmetic", ("expr_arithmetic",)),
    BenchRequirement("Slot operations", "read", ("slot_read", "bench_engine_numeric_slots_read_write_i64")),
    BenchRequirement("Slot operations", "write", ("bench_engine_numeric_slots_read_write_i64",)),
    BenchRequirement("Slot operations", "copy", ("slot_copy", "bench_engine_slot_copy")),
    BenchRequirement("Core transitions", "SetConst", ("bench_engine_step_once_save_const_single_transition",)),
    BenchRequirement("Core transitions", "EvalExpr", ("ir_execution_expr",)),
    BenchRequirement("Core transitions", "Choose 2-branch true", ("bench_engine_choose_true_branch",)),
    BenchRequirement("Core transitions", "Choose 2-branch false", ("bench_engine_choose_false_branch",)),
    BenchRequirement("Core transitions", "Choose 100-branch", ("ir_execution_choose_100",)),
    BenchRequirement("Core transitions", "Finish", ("bench_engine_finish_no_observability", "ir_execution_1_step")),
    BenchRequirement("Run chains", "1-step save chain", ("bench_engine_run_save_chain_1_step",)),
    BenchRequirement("Run chains", "10-step save chain", ("bench_engine_run_save_chain_10_steps",)),
    BenchRequirement("Run chains", "1000-step save chain", ("bench_engine_run_save_chain_1000_steps",)),
    BenchRequirement("Iteration", "for_each", ("for_each", "ir_execution_for_each")),
    BenchRequirement("Iteration", "together", ("together", "ir_execution_together")),
    BenchRequirement("Iteration", "collect", ("collect", "ir_execution_collect")),
    BenchRequirement("Iteration", "reduce", ("reduce", "ir_execution_reduce")),
    BenchRequirement("Iteration", "repeat", ("repeat", "ir_execution_repeat")),
    BenchRequirement("Storage", "Fjall append no-persist", ("bench_fjall_append_run_accepted_no_persist",)),
    BenchRequirement("Storage", "Fjall append journaled", ("bench_fjall_append_run_accepted_journaled",)),
    BenchRequirement("Storage", "Fjall append strict", ("bench_fjall_append_run_accepted_strict",)),
    BenchRequirement("Storage", "Fjall read 1000 events", ("bench_replay_ordered_journal_1000_events",)),
    BenchRequirement("IPC", "frame encode", ("ipc_frame_encode",)),
    BenchRequirement("IPC", "frame decode", ("ipc_frame_decode",)),
    BenchRequirement("Queues", "ArrayQueue push/pop", ("arrayqueue_push_pop", "crossbeam_arrayqueue_push_pop")),
    BenchRequirement("Queues", "rtrb push/pop", ("rtrb_push_pop",)),
    BenchRequirement("Trace", "trace event push", ("trace_event_push",)),
    BenchRequirement("Trace", "ring full policy", ("trace_ring_full_policy",)),
    BenchRequirement("Writer queue", "journal writer queue push", ("journal_writer_queue_push",)),
    BenchRequirement("Writer queue", "group commit batch 1", ("journal_writer_group_commit_1",)),
    BenchRequirement("Writer queue", "group commit batch 64", ("journal_writer_group_commit_64",)),
    BenchRequirement("Writer queue", "group commit batch 1024", ("journal_writer_group_commit_1024",)),
    BenchRequirement("Scheduler", "shard submit-to-start", ("shard_submit_to_start",)),
    BenchRequirement("Scheduler", "shard submit-to-finish", ("shard_submit_to_finish",)),
    BenchRequirement("Direct API", "submit-to-finish", ("direct_api_submit_to_finish",)),
    BenchRequirement("Async primitives", "ask answer resume", ("ask_answer_resume",)),
    BenchRequirement("Async primitives", "action complete resume", ("action_complete_resume",)),
    BenchRequirement("Async primitives", "wait timer resume", ("wait_timer_resume",)),
)

MASTER_METADATA_FIELDS: tuple[str, ...] = (
    "git commit",
    "rustc version",
    "nightly date",
    "CPU model",
    "CPU governor",
    "kernel version",
    "build profile",
    "RUSTFLAGS",
    "benchmark tool and version",
    "sample count or instruction count",
    "input fixture digest",
    "durability profile",
    "execution mode",
    "p50/p95/p99 latency",
    "instruction counts",
    "allocation count",
    "bytes allocated",
    "Fjall write latency",
    "direct API latency",
    "IPC latency",
)


def rust_text() -> str:
    chunks: list[str] = []
    for path in sorted(ROOT.rglob("*.rs")):
        if "/target/" in path.as_posix():
            continue
        chunks.append(path.read_text(encoding="utf-8", errors="ignore"))
    chunks.append((ROOT / "velvet-ballistics-MASTER.md").read_text(encoding="utf-8"))
    return "\n".join(chunks)


def bench_ids() -> set[str]:
    text = BENCH_PATH.read_text(encoding="utf-8")
    ids = set(re.findall(r"metadata\(\s*\"([^\"]+)\"", text))
    ids.update(re.findall(r"bench_expr\([^,]+,\s*\"([^\"]+)\"", text))
    ids.update(re.findall(r"bench_run_workflow\([^,]+,\s*\"([^\"]+)\"", text))
    return ids


def jsonl_rows(path: Path) -> list[dict[str, object]]:
    if not path.exists():
        return []
    rows: list[dict[str, object]] = []
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw_line.strip()
        if not line:
            continue
        try:
            parsed = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_number}: invalid JSONL: {error}") from error
        if isinstance(parsed, dict):
            rows.append(parsed)
    return rows


def metadata_field_status(field: str, rows_by_file: dict[str, list[dict[str, object]]]) -> bool:
    keys_by_field = {
        "git commit": (("section39", "commit"), ("benchmark", "commit")),
        "rustc version": (("section39", "rustc"),),
        "nightly date": (("section39", "rustc_commit"),),
        "CPU model": (("benchmark", "cpu_model"),),
        "CPU governor": (("section39", "cpu_governor"),),
        "kernel version": (("section39", "kernel"), ("benchmark", "kernel_release")),
        "build profile": (("bench_const", "profile=bench"),),
        "RUSTFLAGS": (("section39", "rustflags"),),
        "benchmark tool and version": (("bench_const", "tool=criterion-0.8"), ("benchmark", "tool_version")),
        "sample count or instruction count": (("benchmark", "sample_count"), ("instruction", "value")),
        "input fixture digest": (("section39", "fixture_digest"), ("benchmark", "fixture_digest")),
        "durability profile": (("section39", "durability_mode"), ("bench_const", "durability=mixed")),
        "execution mode": (("section39", "execution_mode"), ("benchmark", "mode")),
        "p50/p95/p99 latency": (("benchmark", "p50_latency_ns"), ("latency", "p99_latency_ns")),
        "instruction counts": (("benchmark", "instructions_count"), ("instruction", "value")),
        "allocation count": (("alloc", "alloc_count"),),
        "bytes allocated": (("alloc", "bytes_allocated"),),
        "Fjall write latency": (("benchmark", "fjall_write_latency"),),
        "direct API latency": (("benchmark", "direct_api_latency"),),
        "IPC latency": (("benchmark", "ipc_latency"),),
    }
    bench_text = BENCH_PATH.read_text(encoding="utf-8")
    for source, key in keys_by_field[field]:
        if source == "bench_const" and key in bench_text:
            return True
        for row in rows_by_file.get(source, []):
            if key in row and row[key] not in (None, ""):
                return True
    return False


def emit_audit(strict_complete: bool) -> int:
    corpus = rust_text()
    ids = bench_ids()
    gaps = 0

    print("section36-39-audit: Section 36 mandatory test categories")
    for expectation in SECTION36_EXPECTATIONS:
        missing = [token for token in expectation.tokens if token not in corpus]
        status = "COVERED" if not missing else "GAP"
        if missing:
            gaps += 1
        print(f"  {status}: {expectation.requirement}")
        if missing:
            print(f"    missing_tokens={missing}")

    print("section36-39-audit: Section 39 benchmark areas")
    missing_bench_requirements: list[str] = []
    for requirement in SECTION39_BENCH_REQUIREMENTS:
        matched = sorted(set(requirement.accepted_ids) & ids)
        status = "COVERED" if matched else "GAP"
        if not matched:
            gaps += 1
            missing_bench_requirements.append(f"{requirement.area}: {requirement.requirement}")
        print(f"  {status}: {requirement.area} / {requirement.requirement}")
        if matched:
            print(f"    bench_ids={matched}")
        else:
            print(f"    expected_any={list(requirement.accepted_ids)}")

    rows_by_file = {
        "section39": jsonl_rows(METADATA_PATH),
        "benchmark": jsonl_rows(BENCH_EVIDENCE_PATH),
        "latency": jsonl_rows(LATENCY_PATH),
        "instruction": jsonl_rows(INSTRUCTION_PATH),
        "alloc": jsonl_rows(ALLOC_PATH),
    }
    print("section36-39-audit: Section 39 metadata envelope")
    missing_metadata = [
        field for field in MASTER_METADATA_FIELDS if not metadata_field_status(field, rows_by_file)
    ]
    if missing_metadata:
        gaps += 1
        print("  GAP: metadata fields")
        print(f"    missing_fields={missing_metadata}")
    else:
        print("  COVERED: metadata fields")

    print(
        "section36-39-audit: summary "
        f"bench_ids={len(ids)} missing_bench_requirements={len(missing_bench_requirements)} gaps={gaps}"
    )
    if strict_complete and gaps:
        print("section36-39-audit: ERROR strict completeness failed", file=sys.stderr)
        return 1
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strict-complete", action="store_true")
    args = parser.parse_args(argv)
    try:
        return emit_audit(args.strict_complete)
    except OSError as error:
        print(f"section36-39-audit: ERROR {error}", file=sys.stderr)
        return 2
    except ValueError as error:
        print(f"section36-39-audit: ERROR {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
