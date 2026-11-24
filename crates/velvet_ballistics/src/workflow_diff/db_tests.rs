//! `--db` baseline selection and artifact decoding.

use std::path::Path;

use vb_core::action::{ActionContract, ActionName};
use vb_core::{
    ActionId, CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow,
    InputSlotKind, ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
};

use super::baseline;
use super::execute;
use super::model::digest_hex;
use super::tests::{TestResult, messages, write};

const PLAIN: &str = "version: velvet-ballistics/v1
name: plain
when: {manual: {}}
steps:
  - id: seed
    set: {output: base, value: '1'}
  - id: done
    finish: {result: '$base'}
";

const CONCURRENT: &str = "version: velvet-ballistics/v1
name: fanout
when: {manual: {}}
inputs:
  items: {is: list, of: any, max: 4}
steps:
  - id: seed
    set: {output: base, value: '1'}
  - id: each
    for_each:
      input: '$input.items'
      variable: entry
      at_once: 2
      steps:
        - id: leaf
          set: {output: picked, value: '$entry'}
  - id: done
    finish: {result: '$seed'}
";

/// Stores `yaml` as an accepted artifact and returns its stored digest.
fn store(db: &Path, yaml: &str) -> Result<String, Box<dyn std::error::Error>> {
    let compiled = vb_compile::compile_workflow(yaml.as_bytes())?;
    let journal = vb_storage::FjallJournal::open(db, None)?;
    let accepted =
        vb_storage::submit_artifact(&journal, &compiled, vb_core::RuntimePolicy::Journaled)?;
    Ok(digest_hex(accepted.digest))
}

fn variant(value: &str) -> String {
    PLAIN.replace("value: '1'", &format!("value: '{value}'"))
}

fn matched(report: &super::report::DiffReport, key: &str) -> String {
    report.baseline[key].as_str().unwrap_or_default().to_owned()
}

#[test]
fn stored_artifact_with_same_digest_means_no_change() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    store(&db, PLAIN)?;
    let new_path = write(dir.path(), "new.yaml", PLAIN)?;
    let report = execute(&new_path, None, Some(&db))?;
    assert!(!report.has_semantic_changes(), "{:?}", report.changes);
    assert_eq!(report.old_digest, report.new_digest);
    assert_eq!(matched(&report, "matched_by"), "workflow_digest");
    assert!(report.text.is_none(), "DB artifacts carry no YAML source");
    Ok(())
}

#[test]
fn stored_artifact_of_older_version_is_found_by_workflow_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    let stored = store(&db, PLAIN)?;
    let grown = PLAIN.replace(
        "  - id: done",
        "  - id: more\n    set: {output: extra, value: '2'}\n  - id: done",
    );
    let new_path = write(dir.path(), "new.yaml", &grown)?;
    let report = execute(&new_path, None, Some(&db))?;
    assert_eq!(matched(&report, "matched_by"), "workflow_name");
    assert_eq!(matched(&report, "stored_artifact_digest"), stored);
    assert_eq!((report.old_steps, report.new_steps), (2, 3));
    assert_eq!(
        messages(&report, "step_count"),
        ["step count changed: 2 -> 3"]
    );
    assert_eq!(messages(&report, "steps"), ["step added: more:set_const"]);
    Ok(())
}

#[test]
fn empty_db_has_no_baseline_and_fails() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    drop(vb_storage::FjallJournal::open(&db, None)?);
    let new_path = write(dir.path(), "new.yaml", PLAIN)?;
    let error = execute(&new_path, None, Some(&db))
        .err()
        .ok_or("empty DB must fail")?;
    assert!(
        error.contains("no stored compiled artifact for workflow 'plain'"),
        "{error}"
    );
    Ok(())
}

#[test]
fn db_holding_only_unrelated_workflows_has_no_baseline() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    store(&db, &PLAIN.replace("name: plain", "name: other_a"))?;
    store(&db, &PLAIN.replace("name: plain", "name: other_b"))?;
    let new_path = write(dir.path(), "new.yaml", PLAIN)?;
    let error = execute(&new_path, None, Some(&db))
        .err()
        .ok_or("no match must fail")?;
    assert!(
        error.contains("no stored compiled artifact for workflow 'plain'"),
        "{error}"
    );
    Ok(())
}

#[test]
fn unrelated_workflows_do_not_make_a_single_same_name_artifact_ambiguous() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    store(&db, &PLAIN.replace("name: plain", "name: other_a"))?;
    let wanted = store(&db, PLAIN)?;
    store(&db, &PLAIN.replace("name: plain", "name: other_b"))?;
    let new_path = write(dir.path(), "new.yaml", &variant("9"))?;
    let report = execute(&new_path, None, Some(&db))?;
    assert_eq!(matched(&report, "matched_by"), "workflow_name");
    assert_eq!(matched(&report, "stored_artifact_digest"), wanted);
    assert_eq!(report.old_steps, 2);
    Ok(())
}

#[test]
fn exact_digest_match_wins_over_same_name_ambiguity() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    store(&db, &variant("1"))?;
    let exact = store(&db, &variant("2"))?;
    let new_path = write(dir.path(), "new.yaml", &variant("2"))?;
    let report = execute(&new_path, None, Some(&db))?;
    assert_eq!(matched(&report, "matched_by"), "workflow_digest");
    assert_eq!(matched(&report, "stored_artifact_digest"), exact);
    assert!(!report.has_semantic_changes(), "{:?}", report.changes);
    Ok(())
}

#[test]
fn two_stored_versions_are_ambiguous_and_fail() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    let first = store(&db, &variant("1"))?;
    let second = store(&db, &variant("2"))?;
    let new_path = write(dir.path(), "new.yaml", &variant("3"))?;
    let error = execute(&new_path, None, Some(&db))
        .err()
        .ok_or("ambiguity must fail")?;
    assert!(
        error.contains("2 stored artifacts share workflow name 'plain'"),
        "{error}"
    );
    assert!(error.contains(&first) && error.contains(&second), "{error}");
    Ok(())
}

#[test]
fn ambiguity_diagnostic_stays_bounded_for_many_versions() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    for n in 0..12 {
        store(&db, &variant(&n.to_string()))?;
    }
    let new_path = write(dir.path(), "new.yaml", &variant("99"))?;
    let error = execute(&new_path, None, Some(&db))
        .err()
        .ok_or("ambiguity must fail")?;
    assert!(
        error.contains("12 stored artifacts share workflow name 'plain'"),
        "{error}"
    );
    assert_eq!(error.matches("share").count(), 1);
    assert!(error.contains(", ..."), "{error}");
    let listed = error
        .split(',')
        .filter(|part| part.trim_start().len() >= 64)
        .count();
    assert!(listed <= 8, "listed {listed} digests: {error}");
    Ok(())
}

#[test]
fn missing_db_directory_is_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let new_path = write(dir.path(), "new.yaml", PLAIN)?;
    let error = execute(&new_path, None, Some(&dir.path().join("nope")))
        .err()
        .ok_or("missing DB must fail")?;
    assert!(
        error.contains("journal directory does not exist"),
        "{error}"
    );
    Ok(())
}

#[test]
fn stored_concurrent_workflow_diffs_against_itself_cleanly() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    let stored = store(&db, CONCURRENT)?;
    let new_path = write(dir.path(), "new.yaml", CONCURRENT)?;
    let report = execute(&new_path, None, Some(&db))?;
    assert_eq!(matched(&report, "stored_artifact_digest"), stored);
    assert!(!report.has_semantic_changes(), "{:?}", report.changes);
    let wider = CONCURRENT.replace("at_once: 2", "at_once: 3");
    let wider_path = write(dir.path(), "wider.yaml", &wider)?;
    let changed = execute(&wider_path, None, Some(&db))?;
    assert!(changed.has_semantic_changes());
    assert_eq!(matched(&changed, "matched_by"), "workflow_name");
    Ok(())
}

fn action_workflow() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let node = |id, kind, output, next| vb_core::node(StepIdx::new(id), kind, output, next);
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("stored-contract"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            node(
                0,
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::ZERO,
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(1)),
            ),
            node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::Any,
            constraint: CompiledInputConstraint::required(),
        }]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let mut canonical = workflow.to_parts();
    canonical.digest = WorkflowDigest::from_bytes([0; 32]);
    let bytes = postcard::to_allocvec(&canonical)?;
    canonical.digest = WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes());
    Ok(CompiledWorkflow::try_from_parts(canonical)?)
}

fn contract(timeout_ms: u64) -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("stored-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}

#[test]
fn stored_action_contract_is_compared_against_the_current_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    let workflow = action_workflow()?;
    let journal = vb_storage::FjallJournal::open(&db, None)?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[contract(1_000)?],
    )?;
    drop(journal);
    let old = baseline::resolve(None, Some(&db), &workflow, &[])?;
    assert_eq!(
        old.loaded.contracts,
        vec![contract(1_000)?],
        "persisted contracts are kept"
    );
    let unchanged = super::Loaded {
        compiled: workflow.clone(),
        source: None,
        contracts: vec![contract(1_000)?],
    };
    assert!(!super::diff_loaded("new", &old, &unchanged)?.has_semantic_changes());
    let loosened = super::Loaded {
        contracts: vec![contract(2_000)?],
        ..unchanged
    };
    let report = super::diff_loaded("new", &old, &loosened)?;
    let changes = messages(&report, "action_contract");
    assert_eq!(changes.len(), 1, "{:?}", report.changes);
    let change = changes.first().copied().ok_or("action contract change")?;
    let (before, after) = change.split_once(" -> ").ok_or("old -> new")?;
    assert!(before.contains("timeout_ms=1000,"), "{change}");
    assert!(after.contains("timeout_ms=2000,"), "{change}");
    assert!(before.contains("stored-action (id 0)"), "{change}");
    Ok(())
}

fn with_capabilities(names: &[&str]) -> Result<ActionContract, Box<dyn std::error::Error>> {
    let mut action = contract(1_000)?;
    action.required_capabilities = names
        .iter()
        .map(|name| vb_core::Capability::new(Box::from(*name), ActionId::new(0)))
        .collect();
    Ok(action)
}

#[test]
fn capability_only_contract_difference_is_one_action_contract_change() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    let workflow = action_workflow()?;
    let journal = vb_storage::FjallJournal::open(&db, None)?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[with_capabilities(&["net.read"])?],
    )?;
    drop(journal);
    let old = baseline::resolve(None, Some(&db), &workflow, &[])?;
    let current = |names: &[&str]| -> Result<super::Loaded, Box<dyn std::error::Error>> {
        Ok(super::Loaded {
            compiled: workflow.clone(),
            source: None,
            contracts: vec![with_capabilities(names)?],
        })
    };
    let same = super::diff_loaded("new", &old, &current(&["net.read"])?)?;
    assert!(!same.has_semantic_changes(), "{:?}", same.changes);
    let widened = super::diff_loaded("new", &old, &current(&["net.read", "fs.write"])?)?;
    let changes = messages(&widened, "action_contract");
    assert_eq!(changes.len(), 1, "{:?}", widened.changes);
    let change = changes.first().copied().ok_or("action contract change")?;
    assert!(
        change.contains(r#"required_capabilities={("net.read", 0)}"#),
        "{change}"
    );
    assert!(
        change.contains(r#"required_capabilities={("fs.write", 0), ("net.read", 0)}"#),
        "{change}"
    );
    let reordered = super::diff_loaded("new", &old, &current(&["fs.write", "net.read"])?)?;
    assert_eq!(messages(&reordered, "action_contract"), changes);
    Ok(())
}
