//! Source payloads must fit the same finite arena used by public execution.
use std::{num::NonZeroUsize, sync::Arc};
use vb_compile::{CompileError, compile_workflow};
use vb_core::workflow::{ContractResource, WorkflowError};
use vb_core::{CompiledWorkflow, RunId, ValueStore};
use vb_runtime::{journal::VolatileRuntimeJournal, runtime::Runtime, shard::ShardConfig};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn owned_payload_and_defaults_obey_actual_admission_capacity() -> TestResult {
    use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
    use vb_core::{SlotValue, Taint};
    for supplied in [true, false] {
        for symbols in [1020_usize, 1021, 1022] {
            let texts = (0..symbols)
                .map(|index| format!("user_{index}"))
                .collect::<Vec<_>>();
            let default = if supplied {
                String::new()
            } else {
                format!(", default: [{}]", texts.join(", "))
            };
            let source = format!(
                "version: velvet-ballistics/v1\nname: owned_headroom\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: text, max: 1024{default}}}}}\nvars: {{config: {{key: value}}}}\nsteps:\n  - id: done\n    finish: {{result: '$vars.config'}}\n"
            );
            let workflow = compile_workflow(source.as_bytes())?;
            let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
            let roots = if supplied {
                let values = texts
                    .into_iter()
                    .map(|text| store.insert_symbol(text).map(SlotValue::Symbol))
                    .collect::<Result<Vec<_>, _>>()?;
                let list = store.insert_list(values.into_boxed_slice())?;
                let input = workflow.input_slots().first().ok_or("input")?.slot;
                vec![(input, SlotValue::List(list), Taint::Clean)].into_boxed_slice()
            } else {
                Box::default()
            };
            let result = OwnedRootBundle::new(roots, store.checkpoint_v1(), &workflow);
            if symbols < 1022 {
                let bundle = result?;
                assert_eq!(
                    bundle.checkpoint().symbols.len(),
                    symbols.checked_add(2).ok_or("count overflow")?
                );
                assert_eq!(bundle.checkpoint().lists.len(), 1);
            } else {
                let error = match result {
                    Err(error) => error,
                    Ok(_) => return Err("oversized owned payload admitted".into()),
                };
                if supplied {
                    let OwnedRootBundleError::ArenaLimit { count, limit } = error else {
                        return Err(format!("supplied capacity refusal: {error:?}").into());
                    };
                    assert_eq!(count, 1025);
                    assert_eq!(limit, 1024);
                } else {
                    let OwnedRootBundleError::Checkpoint(error) = error else {
                        return Err(format!("default capacity refusal: {error:?}").into());
                    };
                    assert_eq!(
                        error,
                        vb_core::CoreError::ResourceLimitExceeded {
                            resource: "input defaults"
                        }
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn nonallocating_numeric_repeat_keeps_declared_arena_bound() -> TestResult {
    let source = b"version: velvet-ballistics/v1\nname: numeric_repeat\nwhen: {manual: {}}\nvars: {marker: source_payload}\nsteps:\n  - id: repeated\n    repeat:\n      max_attempts: 10000\n      steps:\n        - id: leaf\n          set: {output: leaf, value: '9'}\n  - id: done\n    finish: {result: '$step.repeated'}\n";
    let workflow = compile_workflow(source)?;
    assert_eq!(workflow.resource_contract().max_slots, 1024);
    assert_eq!(
        workflow
            .source_metadata()
            .ok_or("metadata")?
            .symbols_count(),
        1
    );
    Ok(())
}

fn source_fields(count: usize) -> String {
    let fields = (0..count)
        .map(|i| format!("key_{i}: value_{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "version: velvet-ballistics/v1\nname: source_capacity\nwhen: {{manual: {{}}}}\nvars:\n  config: {{{fields}}}\nsteps:\n  - id: calculate\n    set: {{output: answer, value: '42'}}\n  - id: done\n    finish: {{result: answer}}\n"
    )
}

#[test]
fn prefix_and_object_fit_bounded_public_runtime() -> TestResult {
    let workflow = compile_workflow(source_fields(511).as_bytes())?;
    let metadata = workflow
        .source_metadata()
        .ok_or("missing source metadata")?;
    assert_eq!(metadata.symbols_count(), 1022);
    assert_eq!(workflow.resource_contract().max_slots, 1024);
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: vb_core::RuntimePolicy::Relaxed,
            step_budget_per_tick: 1024,
            ..ShardConfig::default()
        },
        Arc::new(VolatileRuntimeJournal::new()),
    );
    runtime.submit_compiled(RunId::new(1), workflow)?;
    assert!(runtime.tick_all()?);
    Ok(())
}

#[test]
fn oversized_prefix_is_rejected_before_execution() -> TestResult {
    for fields in [513, 600] {
        let Err(errors) = compile_workflow(source_fields(fields).as_bytes()) else {
            return Err("compiler emitted an oversized source prefix".into());
        };
        assert_eq!(
            errors.0.as_slice(),
            &[CompileError::Workflow(
                WorkflowError::ResourceContractExceeded {
                    resource: ContractResource::Slots
                }
            )]
        );
    }
    Ok(())
}

#[test]
fn full_prefix_and_numeric_loop_refuse_runtime_growth_at_capacity() -> TestResult {
    use vb_core::{CapabilitySet, CoreError, StepBudget, new_run_frame};
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;
    let numeric = "version: velvet-ballistics/v1\nname: numeric_capacity\nwhen: {manual: {}}\nvars: {xs: [1, 2]}\nsteps:\n  - id: loop\n    repeat:\n      max_attempts: 1024\n      steps:\n        - id: work\n          set: {output: result, value: 'unique($vars.xs)'}\n  - id: done\n    finish: {result: '$vars.xs'}\n";
    for (source, expected) in [
        (
            source_fields(512),
            CoreError::BudgetExceeded {
                budget: "max_slots",
                limit: 1024,
            },
        ),
        (numeric.to_owned(), CoreError::AllocationFailed),
    ] {
        let workflow = compile_workflow(source.as_bytes())?;
        let mut frame = new_run_frame(RunId::new(1), &workflow)?;
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        if let Some(metadata) = workflow.source_metadata() {
            vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
        }
        let mut evidence = EvidenceCollector::new();
        let mut collect = CollectStates::new();
        let mut terminal = None;
        for _ in 0..8192 {
            let signal = drive_deterministic_full(
                &workflow,
                &mut frame,
                &mut StepBudget::new(1),
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut collect,
                &CapabilitySet::empty(),
                1,
            );
            let _events = evidence.drain();
            if signal != Ok(RuntimeSignal::StepBudgetExhausted) {
                terminal = Some(signal);
                break;
            }
        }
        assert_eq!(terminal, Some(Err(RuntimeEngineError::Core(expected))));
        assert_eq!(store.total_arena_count(), 1024);
        let before = store.checkpoint_v1();
        assert_eq!(
            store.insert_list(Box::default()),
            Err(CoreError::BudgetExceeded {
                budget: "max_slots",
                limit: 1024
            })
        );
        assert_eq!(store.checkpoint_v1(), before);
    }
    Ok(())
}

#[test]
fn constructor_rejects_tampered_contract_and_accepts_exact_capacity() -> TestResult {
    let original = compile_workflow(source_fields(511).as_bytes())?;
    let mut parts = original.to_parts();
    parts.resource_contract.max_slots = 1023;
    let exact = CompiledWorkflow::try_from_parts(parts.clone())?;
    let mut store = ValueStore::with_max_slots(exact.resource_contract().max_slots);
    vb_core::workflow::source_metadata::seed_source_symbols(
        exact.source_metadata().ok_or("metadata")?,
        &mut store,
    )?;
    store.insert_object(Vec::new().into_boxed_slice())?;
    assert_eq!(store.total_arena_count(), 1023);
    parts.resource_contract.max_slots = 1022;
    let prefix_only = CompiledWorkflow::try_from_parts(parts.clone())?;
    let mut full = ValueStore::with_max_slots(prefix_only.resource_contract().max_slots);
    vb_core::workflow::source_metadata::seed_source_symbols(
        prefix_only.source_metadata().ok_or("metadata")?,
        &mut full,
    )?;
    assert_eq!(full.total_arena_count(), 1022);
    assert_eq!(
        full.insert_object(Box::default()),
        Err(vb_core::CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1022
        })
    );
    assert_eq!(full.total_arena_count(), 1022);
    parts.resource_contract.max_slots = 1021;
    let Err(error) = CompiledWorkflow::try_from_parts(parts) else {
        return Err("tampered contract accepted".into());
    };
    assert_eq!(
        error,
        WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Slots
        }
    );
    Ok(())
}

#[test]
fn actual_source_symbol_bytes_reach_existing_boundary() -> TestResult {
    let limit = vb_core::limits::MAX_SYMBOL_BYTES_PER_VALUE;
    for bytes in [limit, limit.checked_add(1).ok_or("byte boundary overflow")?] {
        let text = "x".repeat(bytes);
        let source = format!(
            "version: velvet-ballistics/v1\nname: symbol_bytes\nwhen: {{manual: {{}}}}\nvars: {{text: '{text}'}}\nsteps:\n  - id: done\n    finish: {{result: '$vars.text'}}\n"
        );
        if bytes == limit {
            let workflow = compile_workflow(source.as_bytes())?;
            let metadata = workflow.source_metadata().ok_or("missing metadata")?;
            assert_eq!(metadata.symbol_bytes(), u64::try_from(limit)?);
            let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
            vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
            let symbol = metadata.symbols().first().ok_or("missing symbol")?;
            assert_eq!(symbol.len(), limit);
            assert_eq!(store.symbol(vb_core::SymbolId::new(0))?, text);
        } else {
            let Err(errors) = compile_workflow(source.as_bytes()) else {
                return Err("oversized source symbol accepted".into());
            };
            assert_eq!(errors.0.len(), 1);
            let Some(CompileError::ExpressionLoweringUnsupported { feature }) = errors.0.first()
            else {
                return Err(format!("unexpected byte rejection: {errors:?}").into());
            };
            assert_eq!(
                feature.as_ref(),
                format!("symbol byte length {bytes} exceeds {limit}")
            );
        }
    }
    Ok(())
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(32))]
    #[test]
    fn real_prefix_and_container_obey_exact_contract(fields in 1_usize..64) {
        let workflow = compile_workflow(source_fields(fields).as_bytes())?;
        let mut parts = workflow.to_parts();
        let demand = u16::try_from(fields.checked_mul(2).and_then(|n| n.checked_add(1)).ok_or_else(|| proptest::test_runner::TestCaseError::fail("count overflow"))?)?;
        parts.resource_contract.max_slots = demand.max(parts.slot_count);
        let accepted = CompiledWorkflow::try_from_parts(parts.clone())?;
        let mut store = ValueStore::with_max_slots(accepted.resource_contract().max_slots);
        vb_core::workflow::source_metadata::seed_source_symbols(accepted.source_metadata().ok_or_else(|| proptest::test_runner::TestCaseError::fail("metadata"))?, &mut store)?;
        store.insert_object(Vec::new().into_boxed_slice())?;
        proptest::prop_assert_eq!(store.total_arena_count(), u64::from(demand));
        if demand > parts.slot_count {
            parts.resource_contract.max_slots = demand.checked_sub(1).ok_or_else(|| proptest::test_runner::TestCaseError::fail("underflow"))?;
            let prefix_only = CompiledWorkflow::try_from_parts(parts.clone())?;
            let mut full = ValueStore::with_max_slots(prefix_only.resource_contract().max_slots);
            vb_core::workflow::source_metadata::seed_source_symbols(prefix_only.source_metadata().ok_or_else(|| proptest::test_runner::TestCaseError::fail("metadata"))?, &mut full)?;
            proptest::prop_assert_eq!(full.total_arena_count(), u64::from(parts.resource_contract.max_slots));
            proptest::prop_assert_eq!(full.insert_object(Box::default()), Err(vb_core::CoreError::BudgetExceeded {budget: "max_slots", limit: u64::from(parts.resource_contract.max_slots)}));
            proptest::prop_assert_eq!(full.total_arena_count(), u64::from(parts.resource_contract.max_slots));
            parts.resource_contract.max_slots = parts.resource_contract.max_slots.checked_sub(1).ok_or_else(|| proptest::test_runner::TestCaseError::fail("underflow"))?;
            let Err(error) = CompiledWorkflow::try_from_parts(parts) else { return Err(proptest::test_runner::TestCaseError::fail("oversized prefix accepted")); };
            proptest::prop_assert_eq!(error, WorkflowError::ResourceContractExceeded { resource: ContractResource::Slots });
        }
    }
}
