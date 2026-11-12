use super::*;
use crate::journal::VolatileRuntimeJournal;
use crate::shard::{Shard, ShardConfig};
use std::sync::Arc;
use vb_core::workflow::source_metadata::{
    CompiledSecretRequirement, WorkflowSourceMetadata, seed_source_symbols,
};
use vb_core::{
    CompiledInputConstraint, CompiledInputSlot, CoreError, InputDefaultValue, InputSlotKind,
    ListId, RuntimePolicy, SymbolId,
};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SLOT_ZERO: SlotIdx = SlotIdx::ZERO;
const SYMBOL_ID_0: SymbolId = SymbolId::new(0);
const SYMBOL_ID_1: SymbolId = SymbolId::new(1);
const LIST_ID_0: ListId = ListId::new(0);

fn test_result<T>(r: RuntimeResult<T>) -> Result<T, Box<dyn std::error::Error>> {
    r.map_err(|e| e.into())
}

fn make_input_slot() -> CompiledInputSlot {
    CompiledInputSlot {
        slot: SLOT_ZERO,
        kind: InputSlotKind::Symbol,
        constraint: CompiledInputConstraint {
            secretness: true.into(),
            default_cold: Some(InputDefaultValue::Text("TOKEN".into())),
            ..CompiledInputConstraint::required()
        },
    }
}

fn make_secret_requirement() -> CompiledSecretRequirement {
    CompiledSecretRequirement::with_slot(SYMBOL_ID_0, SYMBOL_ID_1, SLOT_ZERO)
}

fn build_fixture_workflow() -> TestResult<CompiledWorkflow> {
    let payloads: Box<[Box<str>]> = Box::from([Box::from("token"), Box::from("TOKEN")]);
    let mut parts =
        crate::run_checkpoint::symbol_prefix_fixture::prefix_workflow(payloads.to_vec())?
            .to_parts();
    let metadata =
        WorkflowSourceMetadata::try_new(payloads.clone(), Box::from([make_secret_requirement()]))?;
    parts.source_metadata = Some(Box::new(metadata));
    parts.input_slots = Box::from([make_input_slot()]);
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        parts, payloads,
    )?)
}

fn authorized_shard() -> TestResult<Shard> {
    let config = ShardConfig {
        policy: RuntimePolicy::Relaxed,
        ..Default::default()
    };
    let mut shard = Shard::new_with_journal(config, Arc::new(VolatileRuntimeJournal::new()));
    shard.set_available_secret_names(Arc::from([Box::<str>::from("TOKEN")]));
    Ok(shard)
}

fn unavailable_shard() -> TestResult<Shard> {
    let config = ShardConfig {
        policy: RuntimePolicy::Relaxed,
        ..Default::default()
    };
    Ok(Shard::new_with_journal(
        config,
        Arc::new(VolatileRuntimeJournal::new()),
    ))
}

fn make_carrier(
    workflow: &CompiledWorkflow,
    roots: Vec<(SlotIdx, SlotValue, Taint)>,
) -> TestResult<RecoveredRetainedInputs> {
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    seed_source_symbols(workflow.source_metadata().ok_or("metadata")?, &mut store)?;
    assert_eq!(store.insert_text("retired")?, SymbolId::new(2));
    assert_eq!(
        store.insert_list(Box::from([SlotValue::I64(7)]))?,
        LIST_ID_0
    );
    Ok(RecoveredRetainedInputs {
        initial_inputs: roots,
        store: Some(store),
    })
}

fn assert_rejected(
    actual: RuntimeResult<RecoveredRetainedInputs>,
    expected: RuntimeError,
) -> TestResult<()> {
    match actual {
        Err(e) => {
            if e != expected {
                return Err(format!("expected {:?}, got {:?}", expected, e).into());
            }
        }
        Ok(_) => return Err("unexpected recovery success".into()),
    }
    Ok(())
}

#[test]
fn rejects_clean_tainted_root() -> TestResult<()> {
    let workflow = build_fixture_workflow()?;
    let carrier = make_carrier(
        &workflow,
        vec![(SLOT_ZERO, SlotValue::Symbol(SYMBOL_ID_1), Taint::Clean)],
    )?;
    let shard = authorized_shard()?;
    let result = carrier.with_secret_inputs(&shard, &workflow);
    assert_rejected(
        result,
        RuntimeError::from(CoreError::InternalInvariantViolation {
            reason: "secret_input_binding".into(),
        }),
    )
}

#[test]
fn rejects_wrong_symbol_id() -> TestResult<()> {
    let workflow = build_fixture_workflow()?;
    let carrier = make_carrier(
        &workflow,
        vec![(SLOT_ZERO, SlotValue::Symbol(SYMBOL_ID_0), Taint::Secret)],
    )?;
    let shard = authorized_shard()?;
    let result = carrier.with_secret_inputs(&shard, &workflow);
    assert_rejected(
        result,
        RuntimeError::from(CoreError::InternalInvariantViolation {
            reason: "secret_input_binding".into(),
        }),
    )
}

#[test]
fn rejects_unavailable_secret_host() -> TestResult<()> {
    let workflow = build_fixture_workflow()?;
    let carrier = make_carrier(
        &workflow,
        vec![(SLOT_ZERO, SlotValue::Symbol(SYMBOL_ID_1), Taint::Secret)],
    )?;
    let shard = unavailable_shard()?;
    let result = carrier.with_secret_inputs(&shard, &workflow);
    assert_rejected(result, RuntimeError::SecretUnavailable)
}

#[test]
fn fills_missing_secret_root() -> TestResult<()> {
    let workflow = build_fixture_workflow()?;
    let carrier = make_carrier(&workflow, vec![])?;
    let before = carrier.store_checkpoint().ok_or("store")?;
    let shard = authorized_shard()?;
    let recovered = test_result(carrier.with_secret_inputs(&shard, &workflow))?;
    assert_eq!(
        recovered.initial_inputs.as_slice(),
        &[(SLOT_ZERO, SlotValue::Symbol(SYMBOL_ID_1), Taint::Secret)]
    );
    assert_eq!(recovered.store_checkpoint(), Some(before));
    Ok(())
}

#[test]
fn preserves_existing_secret_root_and_store() -> TestResult<()> {
    let workflow = build_fixture_workflow()?;
    let roots = vec![(SLOT_ZERO, SlotValue::Symbol(SYMBOL_ID_1), Taint::Secret)];
    let carrier = make_carrier(&workflow, roots.clone())?;
    let before = carrier.store_checkpoint().ok_or("store")?;
    let shard = authorized_shard()?;
    let recovered = test_result(carrier.with_secret_inputs(&shard, &workflow))?;
    assert_eq!(
        recovered.initial_inputs.as_slice(),
        &[(SLOT_ZERO, SlotValue::Symbol(SYMBOL_ID_1), Taint::Secret)]
    );
    let after = recovered.store_checkpoint().ok_or("store")?;
    assert_eq!(before, after);
    Ok(())
}
