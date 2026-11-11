use super::{goldens, support};
use support::TestResult;
use vb_core::{ObjectId, SlotValue};

#[test]
fn bound_symbol_runtime_fresh_literal_has_present() -> TestResult {
    run("key", &["key", "noise"], 0, true)
}

#[test]
fn bound_symbol_runtime_fresh_literal_has_missing() -> TestResult {
    run("absent", &["key", "noise", "absent"], 2, false)
}

fn run(key: &str, symbols: &[&str], expected_key: u32, expected: bool) -> TestResult {
    cut(key, symbols, expected_key, expected, 1)?;
    cut(key, symbols, expected_key, expected, 2)?;
    terminal(key, expected)
}

fn slots(value: Option<bool>) -> Vec<Option<SlotValue>> {
    vec![
        Some(SlotValue::I64(0)),
        value.map(SlotValue::Bool),
        Some(SlotValue::Object(ObjectId::new(0))),
    ]
}

fn cut(key: &str, symbols: &[&str], expected_key: u32, expected: bool, budget: u64) -> TestResult {
    let case = goldens::fresh_case(key)?;
    let journal = case.open()?;
    let retained = case.retained_bytes(&journal)?;
    let before = support::journal_snapshot(&journal, case.run)?;
    let mut actor = support::runtime(journal.clone(), support::config(budget))?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(support::journal_snapshot(&journal, case.run)?, before);
    assert_eq!(actor.tick_all(), Ok(true));
    let frame = cut_frame(&case, budget, expected)?;
    goldens::assert_state(
        &case,
        &actor,
        &goldens::store(&case, symbols, expected_key),
        frame,
    )?;
    assert_eq!(case.retained_bytes(&journal)?, retained);
    Ok(())
}

fn cut_frame(
    case: &support::Case,
    budget: u64,
    expected: bool,
) -> TestResult<vb_core::frame::RunFrameCheckpointV1> {
    match budget {
        1 => Ok(goldens::frame(case, 1, 1, slots(None))),
        2 => Ok(goldens::frame(case, 2, 2, slots(Some(expected)))),
        _ => Err("unexpected cut budget".into()),
    }
}

fn terminal(key: &str, expected: bool) -> TestResult {
    let case = goldens::fresh_case(key)?;
    let journal = case.open()?;
    let retained = case.retained_bytes(&journal)?;
    let mut actor = support::runtime(journal.clone(), support::config(3))?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(actor.tick_all(), Ok(true));
    support::finished_has(&case, &mut actor, &journal, expected)?;
    assert_eq!(case.retained_bytes(&journal)?, retained);
    Ok(())
}

#[test]
fn bound_symbol_runtime_direct_prefix_and_bound_empty() -> TestResult {
    direct_scalar(&["key", "compiler"])?;
    direct_scalar(&[])
}

pub(super) fn direct_scalar(prefix: &[&str]) -> TestResult {
    let case = super::legacy_authority::owned_case(prefix, 8)?;
    assert_eq!(
        case.workflow.symbol_payloads(),
        Some(
            prefix
                .iter()
                .map(|text| Box::<str>::from(*text))
                .collect::<Vec<_>>()
                .as_slice()
        )
    );
    let journal = case.open()?;
    let retained = case.retained_bytes(&journal)?;
    direct_cut(&case, journal.clone(), prefix)?;
    direct_terminal(&case, journal.clone())?;
    assert_eq!(case.retained_bytes(&journal)?, retained);
    Ok(())
}

fn direct_terminal(
    case: &support::Case,
    journal: std::sync::Arc<vb_storage::FjallJournal>,
) -> TestResult {
    let mut actor = support::runtime(journal.clone(), support::config(2))?;
    submit_scalar(&actor, case, 20)?;
    assert_eq!(actor.tick_all(), Ok(true));
    support::assert_terminal(
        &journal,
        vb_core::RunId::new(20),
        vb_core::SlotIdx::new(0),
        SlotValue::I64(7),
    )?;
    let before = support::journal_snapshot(&journal, vb_core::RunId::new(20))?;
    assert_eq!(actor.tick_all(), Ok(true));
    assert_eq!(
        support::journal_snapshot(&journal, vb_core::RunId::new(20))?,
        before
    );
    Ok(())
}

fn submit_scalar(
    actor: &vb_runtime::runtime::Runtime,
    case: &support::Case,
    run: u64,
) -> TestResult {
    assert_eq!(
        actor.submit_direct_with_inputs_grants_and_contracts(
            vb_core::RunId::new(run),
            case.workflow.clone(),
            Box::from([(vb_core::SlotIdx::new(1), SlotValue::I64(7))]),
            vb_core::CapabilitySet::empty(),
            Box::default()
        ),
        Ok(())
    );
    Ok(())
}

fn direct_cut(
    case: &support::Case,
    journal: std::sync::Arc<vb_storage::FjallJournal>,
    prefix: &[&str],
) -> TestResult {
    let mut actor = support::runtime(journal, support::config(1))?;
    submit_scalar(&actor, case, 19)?;
    assert_eq!(actor.tick_all(), Ok(true));
    let actual = actor.inspect_run_state(vb_core::RunId::new(19))?;
    let mut store = vb_core::ValueStore::with_max_slots(8).checkpoint_v1();
    store.symbols = prefix.iter().map(|text| Box::<str>::from(*text)).collect();
    assert_eq!(actual.store.checkpoint_v1(), store);
    assert_eq!(actual.frame.checkpoint_v1(), direct_frame());
    Ok(())
}

fn direct_frame() -> vb_core::frame::RunFrameCheckpointV1 {
    vb_core::frame::RunFrameCheckpointV1 {
        run_id: vb_core::RunId::new(19),
        pc: vb_core::StepIdx::new(1),
        executed: 1,
        max_parallel_in_flight: 0,
        parallel_in_flight: 0,
        states: vec![
            vb_core::frame::CheckpointStepState::Succeeded,
            vb_core::frame::CheckpointStepState::Pending,
        ],
        slots: vec![Some(SlotValue::I64(7)), Some(SlotValue::I64(7))],
        taint: vec![vb_core::Taint::Clean, vb_core::Taint::Clean],
    }
}

#[test]
fn bound_symbol_runtime_preserves_nested_arenas_and_taints() -> TestResult {
    let case = nested_case()?;
    let journal = case.open()?;
    let retained = case.retained_bytes(&journal)?;
    let before = support::journal_snapshot(&journal, case.run)?;
    let mut actor = support::runtime(journal.clone(), support::config(1))?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(support::journal_snapshot(&journal, case.run)?, before);
    assert_eq!(actor.tick_all(), Ok(true));
    assert_nested(&actor, &case)?;
    assert_nested_lookup(&actor, &case)?;
    assert_eq!(case.retained_bytes(&journal)?, retained);
    Ok(())
}

fn assert_nested_lookup(actor: &vb_runtime::runtime::Runtime, case: &support::Case) -> TestResult {
    let state = actor.inspect_run_state(case.run)?;
    assert_eq!(
        state
            .store
            .object_field_with_taint(ObjectId::new(0), vb_core::SymbolId::new(2)),
        Ok((
            SlotValue::Symbol(vb_core::SymbolId::new(0)),
            vb_core::Taint::Secret
        ))
    );
    Ok(())
}

fn nested_case() -> TestResult<support::Case> {
    let workflow = super::legacy_authority::plan(&["k", "p"], 7)?;
    let roots = Box::from([(
        vb_core::SlotIdx::new(1),
        SlotValue::List(vb_core::ListId::new(0)),
        vb_core::Taint::Secret,
    )]);
    let bundle =
        vb_core::root_bundle::OwnedRootBundle::new(roots, support::nested_store(false), &workflow)?;
    support::Case::persist(
        workflow,
        "nested public runtime control",
        bundle,
        vb_core::RunId::new(18),
        &[],
    )
}

fn assert_nested(actor: &vb_runtime::runtime::Runtime, case: &support::Case) -> TestResult {
    let state = actor.inspect_run_state(case.run)?;
    assert_eq!(state.store.checkpoint_v1(), support::nested_store(true));
    let mut frame = direct_frame();
    frame.run_id = vb_core::RunId::new(18);
    frame.slots = vec![
        Some(SlotValue::I64(7)),
        Some(SlotValue::List(vb_core::ListId::new(0))),
    ];
    frame.taint = vec![vb_core::Taint::Clean, vb_core::Taint::Secret];
    assert_eq!(state.frame.checkpoint_v1(), frame);
    assert_eq!(state.workflow, case.workflow);
    assert_eq!(state.run_attempt.get(), 1);
    assert_eq!(state.action_contracts, case.artifact.action_contracts);
    assert_eq!(state.action_attempts.as_ref(), &[0, 0]);
    assert_eq!(
        state
            .admission
            .as_ref()
            .ok_or("missing admission")?
            .policy_pin(),
        vb_storage::RunPolicyPin::Pinned(case.artifact.policy_digest)
    );
    Ok(())
}
