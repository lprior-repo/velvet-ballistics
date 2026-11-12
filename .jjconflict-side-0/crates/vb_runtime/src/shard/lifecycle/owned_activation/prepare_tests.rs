use super::*;
use vb_core::value_store::{InputSymbolBindingError, ValueStoreCheckpointV1};
use vb_core::{CoreError, ListId, ObjectId, SymbolId};

#[path = "prepare_fixtures.rs"]
mod fixtures;
use fixtures::*;

#[test]
fn bound_symbol_runtime_owned_preparation_seeds_remapped_roots_and_whole_store() -> TestResult {
    let workflow = workflow(Some(&["key", "compiler"]), 8, 3)?;
    let checkpoint = original_store();
    let bundle = OwnedRootBundle::new(original_roots(), checkpoint.clone(), &workflow)?;
    let mut shard = shard();
    let state = shard.prepare_owned_state(RunId::new(501), accepted(workflow, bundle))?;
    assert_eq!(state.store.checkpoint_v1(), expected_store());
    assert_eq!(state.frame, expected_frame()?);
    assert_eq!(state.admission, Some(admission(state.workflow.digest())));
    let prefix = [Box::<str>::from("key"), "compiler".into()];
    assert_eq!(state.workflow.symbol_payloads(), Some(prefix.as_slice()));
    assert_eq!(checkpoint, original_store());
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters.snapshot().runs_submitted, 0);
    Ok(())
}

#[test]
fn bound_symbol_runtime_owned_preparation_rejects_capacity_before_reservations() -> TestResult {
    let (accepted, checkpoint) = capacity_fixture()?;
    let before = checkpoint.clone();
    let mut shard = shard();
    let result = shard.prepare_owned_state(RunId::new(501), accepted);
    assert_eq!(
        result,
        Err(crate::RuntimeError::from(InputSymbolBindingError::Core(
            CoreError::BudgetExceeded {
                budget: "max_slots",
                limit: 3
            }
        )))
    );
    assert!(shard.frame_pools.is_empty());
    assert!(shard.runs.is_empty());
    assert_eq!(shard.runs.capacity(), 0);
    assert_eq!(shard.journal_sequences.capacity(), 0);
    assert_eq!(shard.counters.snapshot().runs_submitted, 0);
    assert_eq!(checkpoint, before);
    Ok(())
}

#[test]
fn bound_symbol_runtime_owned_v1_retains_original_table_and_roots() -> TestResult {
    let workflow = workflow(None, 8, 3)?;
    let bundle = OwnedRootBundle::new(original_roots(), original_store(), &workflow)?;
    let roots_before = bundle.roots().to_vec();
    let (store, roots) = prepare_inputs(&workflow, &bundle)?;
    assert_eq!(store.checkpoint_v1(), original_store());
    assert_eq!(roots.as_ref(), original_roots().as_ref());
    assert!(matches!(roots, Cow::Borrowed(_)));
    assert_eq!(bundle.checkpoint(), &original_store());
    assert_eq!(bundle.roots(), roots_before);
    Ok(())
}
