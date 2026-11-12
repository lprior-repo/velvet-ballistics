//! End-to-end workflow YAML generator + property sweep.
//! Bead: vb-ngv7u
//! PO: PO-SWEEP-001 (typed compile outcomes), PO-SWEEP-002 (compile
//! determinism), PO-SWEEP-003 (bounded execution outcomes), PO-SWEEP-004
//! (hostile-input typed rejection).
//!
//! Production bindings:
//! - `vb_compile::compile_workflow` in crates/vb_compile/src/mod_compile_core.rs
//! - `drive_deterministic_full` in crates/vb_runtime/src/engine/drive/loop_step.rs
//! - `RuntimeSignal` in crates/vb_runtime/src/engine/types.rs
//!
//! Strategy: a deterministic seeded generator emits YAML workflows spanning
//! the step alphabet (set/save/do/choose/for_each/wait/ask/repeat/finish)
//! crossed with header, input, literal, and reference mutations. Every
//! emitted source must produce a typed outcome (never a panic), compile
//! deterministically, and — when input-free and compilable — drive to a
//! typed terminal signal under a fixed step budget.
//!
//! Budget: `VB_SWEEP_N` sets generated cases (default 256, cap 8192).
//! Deep sweep: `VB_SWEEP_N=4096 cargo test -p
//! velvet-ballistics-workspace-tests --test e2e_yaml_sweep -- --test-threads=1`.

#![forbid(unsafe_code)]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use vb_compile::compile_workflow;
use vb_core::{CapabilitySet, CompiledWorkflow, RunId, StepBudget, ValueStore, new_run_frame};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

const DEFAULT_SWEEP_N: usize = 256;
const MAX_SWEEP_N: usize = 8192;
const DRIVE_BUDGET: u64 = 512;
const SHAPE_COUNT: usize = 9;

const SEEDS: &[&str] = &[
    // set + finish baseline.
    "version: velvet-ballistics/v1\nname: sweep_set\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    // Legacy save + finish baseline.
    "version: velvet-ballistics/v1\nname: sweep_save\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n",
    // choose with branches + otherwise.
    "version: velvet-ballistics/v1\nname: sweep_choose\nwhen:\n  manual: {}\nsteps:\n  - id: flag\n    set:\n      output: flag_value\n      value: \"true\"\n  - id: route\n    choose:\n      branches:\n        - when: \"0\"\n          steps:\n            - id: body_set\n              set:\n                output: branch_value\n                value: \"7\"\n      otherwise: done\n  - id: done\n    finish:\n      result: 0\n",
    // Bounded for_each over a declared list input.
    "version: velvet-ballistics/v1\nname: sweep_foreach\nwhen:\n  manual: {}\ninputs:\n  entries: {is: list, of: any, max: 4}\nsteps:\n  - id: seed\n    set:\n      output: anchor\n      value: '42'\n  - id: bounded\n    for_each:\n      input: '$input.entries'\n      variable: entry\n      at_once: 1\n      steps:\n        - id: copy_first\n          set:\n            output: copied\n            value: '$entry'\n  - id: done\n    finish:\n      result: anchor\n",
    // do action boundary (suspends awaiting action).
    "version: velvet-ballistics/v1\nname: sweep_do\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: act\n    do: { action: \"1\", input: \"$seed\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    // wait boundary.
    "version: velvet-ballistics/v1\nname: sweep_wait\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: w\n    wait: { event: \"1\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    // ask boundary.
    "version: velvet-ballistics/v1\nname: sweep_ask\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: q\n    ask: { prompt: \"1\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    // Bounded repeat with do body.
    "version: velvet-ballistics/v1\nname: sweep_repeat\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: inner\n          do: { action: \"1\", input: \"$seed\" }\n  - id: end\n    finish: { result: \"$seed\" }",
];

const HOSTILE: &[&str] = &[
    "",
    "\0\0\0",
    "not: [valid yaml\n  broken indent:::\n",
    "version: velvet-ballistics/v1\nname: dup\nwhen:\n  manual: {}\nsteps:\n  - id: a\n    set: { output: x, value: \"1\" }\n  - id: a\n    set: { output: x, value: \"2\" }\n  - id: end\n    finish: { result: \"$a\" }",
    "version: velvet-ballistics/v1\nname: okcontrol\nwhen:\n  manual: {}\nsteps:\n  - id: s0\n    set: { output: o0, value: \"1\" }\n  - id: end\n    finish: { result: \"$s0\" }",
];

fn sweep_count() -> usize {
    let from_env = std::env::var("VB_SWEEP_N")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok());
    match from_env {
        Some(n) if n > MAX_SWEEP_N => MAX_SWEEP_N,
        Some(n) if n > 0 => n,
        _ => DEFAULT_SWEEP_N,
    }
}

fn rng_for(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed ^ 0x9E37_79B9_7F4A_7C15)
}

fn pick<'a>(rng: &mut StdRng, options: &'a [&'a str]) -> &'a str {
    let idx = rng.random_range(0..options.len());
    match options.get(idx) {
        Some(value) => value,
        None => match options.first() {
            Some(first) => first,
            None => "",
        },
    }
}

const LITERALS: &[&str] = &["'42'", "\"true\"", "\"1\"", "0", "'7'", "\"0\""];

fn value_ref(rng: &mut StdRng, ids: &[String], outputs: &[String]) -> String {
    if ids.is_empty() || outputs.is_empty() {
        return String::from(pick(rng, LITERALS));
    }
    let mode = rng.random_range(0..3);
    if mode == 0 {
        String::from(pick(rng, LITERALS))
    } else if mode == 1 {
        match ids.get(rng.random_range(0..ids.len())) {
            Some(id) => format!("\"${id}\""),
            None => String::from(pick(rng, LITERALS)),
        }
    } else {
        match outputs.get(rng.random_range(0..outputs.len())) {
            Some(output) => output.clone(),
            None => String::from(pick(rng, LITERALS)),
        }
    }
}

fn emit_step(
    rng: &mut StdRng,
    shape: usize,
    id: &str,
    ids: &[String],
    outputs: &[String],
) -> (String, Option<String>) {
    let out = format!("o{}_{}", ids.len(), rng.random_range(0..64u32));
    let val = value_ref(rng, ids, outputs);
    if shape == 0 {
        (
            format!("  - id: {id}\n    set: {{ output: {out}, value: {val} }}"),
            Some(out),
        )
    } else if shape == 1 {
        (
            format!("  - id: {id}\n    save:\n      output: {out}\n      value: {val}"),
            Some(out),
        )
    } else if shape == 2 {
        (
            format!("  - id: {id}\n    do: {{ action: \"1\", input: {val} }}"),
            None,
        )
    } else if shape == 3 {
        let branch_out = format!("{out}_b");
        (
            format!(
                "  - id: {id}\n    choose:\n      branches:\n        - when: \"0\"\n          steps:\n            - id: {id}_t\n              set:\n                output: {branch_out}\n                value: {val}\n      otherwise: end"
            ),
            Some(branch_out),
        )
    } else if shape == 4 {
        (
            format!(
                "  - id: {id}\n    for_each:\n      input: '$input.entries'\n      variable: entry\n      at_once: 1\n      steps:\n        - id: {id}_each\n          set:\n            output: {out}\n            value: '$entry'"
            ),
            None,
        )
    } else if shape == 5 {
        (format!("  - id: {id}\n    wait: {{ event: {val} }}"), None)
    } else if shape == 6 {
        (format!("  - id: {id}\n    ask: {{ prompt: {val} }}"), None)
    } else if shape == 7 {
        (
            format!(
                "  - id: {id}\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: {id}_try\n          set:\n            output: {out}\n            value: {val}"
            ),
            None,
        )
    } else {
        (
            format!("  - id: {id}\n    set: {{ output: {out}, value: {val} }}"),
            Some(out),
        )
    }
}

fn build_source(seed: u64, variant: u8) -> String {
    let mut rng = rng_for(seed);
    let header = rng.random_range(0..8);
    let version = if header == 0 {
        "velvet-ballistics/v0"
    } else {
        "velvet-ballistics/v1"
    };
    let mut source = String::from("version: ");
    source.push_str(version);
    if header != 1 {
        source.push_str("\nname: sweep_");
        source.push_str(&seed.to_string());
    }
    if header != 2 {
        source.push_str("\nwhen:\n  manual: {}");
    }
    let with_inputs = variant >= 4 || rng.random_bool(0.25);
    if with_inputs {
        source.push_str("\ninputs:\n  entries: {is: list, of: any, max: 4}");
    }
    source.push_str("\nsteps:");
    let mut ids: Vec<String> = Vec::new();
    let mut outputs: Vec<String> = Vec::new();
    let steps = rng.random_range(1..6);
    for n in 0..steps {
        let shape = if variant < 4 {
            rng.random_range(0..SHAPE_COUNT)
        } else {
            usize::from(variant) % SHAPE_COUNT
        };
        let dup = rng.random_bool(0.05);
        let id = if dup && !ids.is_empty() {
            match ids.first() {
                Some(first) => first.clone(),
                None => format!("s{n}"),
            }
        } else {
            format!("s{n}")
        };
        let (step, declared) = emit_step(&mut rng, shape, &id, &ids, &outputs);
        source.push('\n');
        source.push_str(&step);
        ids.push(id);
        if let Some(output) = declared {
            outputs.push(output);
        }
    }
    if rng.random_bool(0.85) {
        let result = value_ref(&mut rng, &ids, &outputs);
        source.push_str("\n  - id: end\n    finish: { result: ");
        source.push_str(&result);
        source.push_str(" }");
    }
    source.push('\n');
    source
}

/// PO-SWEEP-001: every source maps to a typed outcome. Ok carries at least
/// one node; Err carries at least one diagnostic.
fn assert_typed_outcome(source: &[u8]) -> Option<CompiledWorkflow> {
    match compile_workflow(source) {
        Ok(workflow) => {
            assert!(
                workflow.node_count() > 0,
                "Ok workflow must carry nodes: {}",
                String::from_utf8_lossy(source)
            );
            Some(workflow)
        }
        Err(errors) => {
            assert!(
                !errors.is_empty(),
                "Err outcome must carry diagnostics: {}",
                String::from_utf8_lossy(source)
            );
            None
        }
    }
}

/// PO-SWEEP-002: compilation is deterministic for identical bytes.
fn outcome_disc<T, E>(result: &Result<T, E>) -> u8 {
    match result {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

fn assert_deterministic(source: &[u8]) {
    let first = compile_workflow(source);
    let second = compile_workflow(source);
    match (first, second) {
        (Ok(a), Ok(b)) => {
            assert_eq!(
                a.node_count(),
                b.node_count(),
                "node count drifted: {}",
                String::from_utf8_lossy(source)
            );
            assert_eq!(
                a.to_parts(),
                b.to_parts(),
                "compiled parts drifted: {}",
                String::from_utf8_lossy(source)
            );
        }
        (Err(a), Err(b)) => {
            assert_eq!(
                a.is_empty(),
                b.is_empty(),
                "error-shape drifted: {}",
                String::from_utf8_lossy(source)
            );
        }
        (first, second) => {
            assert_eq!(
                outcome_disc(&first),
                outcome_disc(&second),
                "compile discriminant drifted: {}",
                String::from_utf8_lossy(source)
            );
        }
    }
}

fn signal_kind(signal: &RuntimeSignal) -> u8 {
    match signal {
        RuntimeSignal::Continue => 0,
        RuntimeSignal::Finished(_) => 1,
        RuntimeSignal::StepBudgetExhausted => 2,
        RuntimeSignal::AwaitingAction(_) => 3,
        RuntimeSignal::AwaitingWait => 4,
        RuntimeSignal::AwaitingAsk => 5,
        _ => 6,
    }
}

/// PO-SWEEP-003: input-free workflows drive to a typed signal under budget,
/// deterministically across fresh frames.
fn assert_bounded_drive(workflow: &CompiledWorkflow, source: &[u8]) {
    if !workflow.input_slots().is_empty() {
        return;
    }
    let mut first_frame = match new_run_frame(RunId::new(1), workflow) {
        Ok(frame) => frame,
        Err(_) => return,
    };
    let mut first_store = ValueStore::new();
    let mut first_evidence = EvidenceCollector::new();
    let mut first_collect = CollectStates::new();
    let mut first_budget = StepBudget::new(DRIVE_BUDGET);
    let first = drive_deterministic_full(
        workflow,
        &mut first_frame,
        &mut first_budget,
        &mut first_store,
        &[],
        RetryPolicy::NEVER,
        &mut first_evidence,
        &mut first_collect,
        &CapabilitySet::empty(),
        1,
    );
    let mut second_frame = match new_run_frame(RunId::new(1), workflow) {
        Ok(frame) => frame,
        Err(_) => return,
    };
    let mut second_store = ValueStore::new();
    let mut second_evidence = EvidenceCollector::new();
    let mut second_collect = CollectStates::new();
    let mut second_budget = StepBudget::new(DRIVE_BUDGET);
    let second = drive_deterministic_full(
        workflow,
        &mut second_frame,
        &mut second_budget,
        &mut second_store,
        &[],
        RetryPolicy::NEVER,
        &mut second_evidence,
        &mut second_collect,
        &CapabilitySet::empty(),
        1,
    );
    match (first, second) {
        (Ok(a), Ok(b)) => {
            assert_eq!(
                signal_kind(&a),
                signal_kind(&b),
                "drive signal drifted: {}",
                String::from_utf8_lossy(source)
            );
        }
        (Err(_), Err(_)) => {}
        (first, second) => {
            assert_eq!(
                outcome_disc(&first),
                outcome_disc(&second),
                "drive discriminant drifted: {}",
                String::from_utf8_lossy(source)
            );
        }
    }
}

fn assert_sweep_case(source: &str) {
    let bytes = source.as_bytes();
    let compiled = assert_typed_outcome(bytes);
    assert_deterministic(bytes);
    if let Some(workflow) = compiled {
        assert_bounded_drive(&workflow, bytes);
    }
}

#[test]
fn sweep_seeds_are_typed_and_deterministic() {
    for seed in SEEDS {
        assert_sweep_case(seed);
    }
}

/// PO-SWEEP-004: hostile bytes are rejected with typed diagnostics.
#[test]
fn sweep_hostile_inputs_rejected_typed() {
    for hostile in HOSTILE {
        assert_sweep_case(hostile);
    }
    let oversized = format!(
        "version: velvet-ballistics/v1\nname: big\nwhen:\n  manual: {{}}\nsteps:\n  - id: s0\n    set: {{ output: o, value: '{}' }}\n  - id: end\n    finish: {{ result: \"$s0\" }}\n",
        "9".repeat(1_000_000)
    );
    assert_sweep_case(&oversized);
    let mut nested = String::from(
        "version: velvet-ballistics/v1\nname: deep\nwhen:\n  manual: {}\nsteps:\n  - id: s0\n    repeat:\n      max_attempts: 2\n      steps:\n",
    );
    for depth in 0..24 {
        nested.push_str(&format!(
            "        - id: d{depth}\n          repeat:\n            max_attempts: 2\n            steps:\n"
        ));
    }
    nested.push_str(
        "              - id: leaf\n                set: { output: o, value: \"1\" }\n  - id: end\n    finish: { result: \"$s0\" }\n",
    );
    assert_sweep_case(&nested);
}

#[test]
fn sweep_generated_sources_are_typed_and_deterministic() {
    let total = sweep_count();
    for case in 0..total {
        let variant = u8::try_from(case % 8).unwrap_or_default();
        let seed = u64::try_from(case).unwrap_or_default();
        let source = build_source(seed.wrapping_mul(0xD1B5_4CB3_457E_9F1C), variant);
        assert_sweep_case(&source);
    }
}

/// Outcome census for deep runs. Silent unless `VB_SWEEP_STATS` is set.
/// Prints Ok/Err totals, per-variant Ok counts, and drive-signal mix so a
/// deep sweep reports what it actually covered.
#[test]
fn sweep_outcome_stats() {
    if std::env::var("VB_SWEEP_STATS").is_err() {
        return;
    }
    let total = sweep_count();
    let mut ok_total = 0usize;
    let mut err_total = 0usize;
    let mut ok_per_variant = [0usize; 8];
    let mut drive_eligible = 0usize;
    let mut err_buckets: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for case in 0..total {
        let variant_usize = case % 8;
        let variant = u8::try_from(variant_usize).unwrap_or_default();
        let seed = u64::try_from(case).unwrap_or_default();
        let source = build_source(seed.wrapping_mul(0xD1B5_4CB3_457E_9F1C), variant);
        match compile_workflow(source.as_bytes()) {
            Ok(workflow) => {
                ok_total = ok_total.saturating_add(1);
                if let Some(slot) = ok_per_variant.get_mut(variant_usize) {
                    *slot = slot.saturating_add(1);
                }
                if workflow.input_slots().is_empty() {
                    drive_eligible = drive_eligible.saturating_add(1);
                }
            }
            Err(errors) => {
                err_total = err_total.saturating_add(1);
                for err in errors.0.iter() {
                    let debug = format!("{err:?}");
                    let name: String = debug
                        .chars()
                        .take_while(|c| *c != '(' && *c != '{')
                        .collect();
                    match err_buckets.get_mut(&name) {
                        Some(slot) => *slot = slot.saturating_add(1),
                        None => {
                            err_buckets.insert(name, 1);
                        }
                    }
                }
            }
        }
    }
    let mut buckets: Vec<(&String, &usize)> = err_buckets.iter().collect();
    buckets.sort_by(|a, b| b.1.cmp(a.1));
    let top: Vec<(&String, &usize)> = buckets.into_iter().take(8).collect();
    println!(
        "sweep_stats total={total} ok={ok_total} err={err_total} drive_eligible={drive_eligible} per_variant={ok_per_variant:?} top_err={top:?}"
    );
}

#[cfg(test)]
mod sweep_props {
    use super::{assert_sweep_case, build_source};
    use proptest::prelude::*;

    proptest! {
        /// Generated sources across all shape variants stay typed.
        #[test]
        fn prop_generated_source_typed(seed in any::<u64>(), variant in 0..8u8) {
            let source = build_source(seed, variant);
            assert_sweep_case(&source);
        }
    }
}
