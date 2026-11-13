use super::*;
use vb_runtime::RuntimeError;

type RawRows = Vec<(String, Vec<u8>, Vec<u8>)>;

fn raw_rows(path: &std::path::Path) -> TestResult<RawRows> {
    let db = fjall::Database::builder(path)
        .open()
        .map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for name in FjallJournal::declared_keyspaces() {
        let keyspace = db
            .keyspace(name, fjall::KeyspaceCreateOptions::default)
            .map_err(|e| e.to_string())?;
        for entry in keyspace.iter() {
            let (key, value) = entry.into_inner().map_err(|e| e.to_string())?;
            rows.push((name.to_owned(), key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}

fn cut() -> TestResult<Vec<JournalEvent>> {
    let case = case()?;
    let journal = open_journal(&case)?;
    let mut runtime = make_runtime(journal.clone())?;
    runtime
        .activate_owned_submission(RUN, case.source_digest)
        .map_err(|e| format!("{e:?}"))?;
    for _ in 0..32 {
        runtime.tick_all().map_err(|e| format!("{e:?}"))?;
        let events = journal
            .events_for_run_full(RUN)
            .map_err(|e| e.to_string())?;
        if runtime
            .inspect_run_state(RUN)
            .map_err(|e| format!("{e:?}"))?
            .frame
            .pc()
            == case.finish
        {
            return Ok(events);
        }
    }
    Err("live lane did not complete".into())
}

#[derive(Debug, Clone, Copy)]
enum Corruption {
    Cursor,
    Attempt,
    ForeignStep,
    Taint,
}

fn mutate(
    events: &mut [JournalEvent],
    case: &Case,
    corruption: Corruption,
) -> TestResult<RuntimeError> {
    let workflow = super::super::compiled_source()?;
    let owner = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| {
            matches!(
                node.kind,
                vb_core::CompiledNodeKind::ForEachConcurrentStart { .. }
            )
        })
        .cloned()
        .ok_or("owner missing")?;
    let vb_core::CompiledNodeKind::ForEachConcurrentStart { join, .. } = owner.kind else {
        return Err("wrong owner".into());
    };
    let cursor = workflow
        .node(join)
        .and_then(|node| node.output.slot())
        .ok_or("cursor output missing")?;
    for event in events {
        match (corruption, event) {
            (Corruption::Cursor, JournalEvent::SlotWrittenEvent { slot, value, .. })
                if *slot == cursor =>
            {
                *value = Some(
                    postcard::to_allocvec(&SlotValue::List(vb_core::ListId::new(u32::MAX)))
                        .map_err(|e| e.to_string())?,
                );
                return Ok(divergence(join, "capture differs from artifact replay"));
            }
            (Corruption::Taint, JournalEvent::SlotWrittenEvent { slot, extra, .. })
                if *slot == case.output =>
            {
                *extra = Some(
                    vb_storage::slot_extra::encode_slot_written_extra(Taint::Clean, None)
                        .map_err(|e| format!("{e:?}"))?,
                );
                return Ok(divergence(
                    case.body,
                    "capture differs from artifact replay",
                ));
            }
            (Corruption::Attempt, JournalEvent::StepStarted { step, attempt, .. })
                if *step == case.body =>
            {
                *attempt = 2;
                return Ok(RuntimeError::RecoveryCannotResume {
                    reason: "run_attempt_authority_mismatch".into(),
                });
            }
            (Corruption::ForeignStep, JournalEvent::StepStarted { step, .. })
                if *step == case.body =>
            {
                *step = case.finish;
                return Ok(divergence(
                    case.finish,
                    "StepStarted differs from replay control edge",
                ));
            }
            _ => {}
        }
    }
    Err(format!("mutation target missing: {corruption:?}"))
}

fn divergence(step: vb_core::StepIdx, detail: &str) -> RuntimeError {
    RuntimeError::Recovery {
        error: vb_storage::recovery::RecoveryError::ReplayDivergence {
            step,
            detail: detail.into(),
        }
        .to_string(),
    }
}

#[test]
fn cold_recovery_rejects_encoded_cursor_attempt_foreign_step_and_taint_without_mutation()
-> TestResult {
    let original = cut()?;
    for corruption in [
        Corruption::Cursor,
        Corruption::Attempt,
        Corruption::ForeignStep,
        Corruption::Taint,
    ] {
        let case = case()?;
        let mut events = original.clone();
        let expected = mutate(&mut events, &case, corruption)?;
        let journal = open_journal(&case)?;
        let prefix_len = journal
            .events_for_run_full(RUN)
            .map_err(|e| e.to_string())?
            .len();
        journal
            .append_strict_batch(events.get(prefix_len..).ok_or("bad prefix")?)
            .map_err(|e| e.to_string())?;
        drop(journal);
        let before = raw_rows(case.directory.path())?;
        let journal = open_journal(&case)?;
        let runtime = make_runtime(journal.clone())?;
        let pending = runtime.pending_boundary_snapshot(64);
        assert_eq!(
            runtime.recover_product(RUN),
            Err(expected.clone()),
            "{corruption:?}"
        );
        assert_eq!(
            runtime.recover_and_resume(RUN),
            Err(expected),
            "{corruption:?}"
        );
        assert_eq!(runtime.pending_boundary_snapshot(64), pending);
        assert!(matches!(
            runtime.inspect_run_state(RUN),
            Err(RuntimeError::RunNotFound)
        ));
        assert_eq!(
            journal
                .events_for_run_full(RUN)
                .map_err(|e| e.to_string())?,
            events
        );
        drop(runtime);
        drop(journal);
        assert_eq!(
            raw_rows(case.directory.path())?,
            before,
            "raw durable rows changed: {corruption:?}"
        );
    }
    Ok(())
}
