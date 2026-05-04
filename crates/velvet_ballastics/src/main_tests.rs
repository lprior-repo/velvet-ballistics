//! Tests for velvet_ballastics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]

#[cfg(test)]
mod tests {
    use super::{
        Command, DurabilityMode, INPUT_MAPPING_DECODE_FAILED_MESSAGE,
        INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE, ParseError, StepTarget, StorageWorkflowResolver,
        build_step_frame, decode_step_inputs, execute_step_isolated,
        map_runtime_inputs, node_kind_name, parse_args, run_compiled_workflow, signal_name,
        write_step_inputs,
    };
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::Arc;
    use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::ConstValue;
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };
    use vb_storage::{CompiledIrRecord, EventSeq, JournalEvent};

    fn args(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(|part| OsString::from(*part)).collect()
    }

    fn finish_workflow() -> Option<CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("finish"),
            digest: WorkflowDigest::from_bytes([9; 32]),
            nodes: Box::from([set_const, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(true)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::default(),
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    #[test]
    fn parse_run_accepts_db_for_journaled_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "journaled",
            "--db",
            "journal-db",
        ]));

        assert!(
            matches!(parsed, Ok(Command::Run { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            ..
        }) = parsed
        {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(input_bin, PathBuf::from("input.bin"));
            assert_eq!(durability, DurabilityMode::Journaled);
            assert_eq!(db, Some(PathBuf::from("journal-db")));
        }
    }

    #[test]
    fn parse_run_compiled_requires_db_for_strict_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run-compiled",
            "workflow.vbir",
            "--input-bin",
            "input.bin",
            "--durability",
            "strict",
        ]));

        assert!(
            matches!(parsed, Err(ParseError::MissingArgument("--db"))),
            "unexpected parse result: {parsed:?}"
        );
    }

    #[test]
    fn parse_run_none_mode_keeps_db_optional() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
        ]));

        assert!(
            matches!(parsed, Ok(Command::Run { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Run { durability, db, .. }) = parsed {
            assert_eq!(durability, DurabilityMode::None);
            assert_eq!(db, None);
        }
    }

    #[test]
    fn input_mapping_failure_message_uses_stable_code() {
        assert_eq!(
            INPUT_MAPPING_DECODE_FAILED_MESSAGE,
            "INPUT_MAPPING_FAILED: input-bin decode failed"
        );
        assert_eq!(
            INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
            "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count"
        );
    }

    #[test]
    fn map_runtime_inputs_decodes_slot_values() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let values: Box<[vb_core::SlotValue]> = Box::from([vb_core::SlotValue::Bool(true)]);
            let payload = postcard::to_allocvec(&values);
            assert!(payload.is_ok(), "test payload should encode: {payload:?}");
            let Ok(payload) = payload else {
                return;
            };
            let mapped = map_runtime_inputs(&compiled, &payload);
            assert!(mapped.is_ok(), "input mapping should decode: {mapped:?}");
            if let Ok(mapped) = mapped {
                assert_eq!(
                    mapped.as_ref(),
                    &[(vb_core::SlotIdx::ZERO, vb_core::SlotValue::Bool(true))]
                );
            }
        }
    }

    #[test]
    fn map_runtime_inputs_rejects_malformed_input_bin() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let mapped = map_runtime_inputs(&compiled, b"not-postcard");
            assert_eq!(
                mapped.map(|_| ()),
                Err(super::InputMappingError::DecodeFailed)
            );
        }
    }

    #[test]
    fn journaled_run_writes_storage_events() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        let dir = tempfile::tempdir();
        assert!(dir.is_ok(), "test directory should be available: {dir:?}");

        if let (Some(compiled), Ok(dir)) = (compiled, dir) {
            let code = run_compiled_workflow(
                &compiled,
                Box::from([]),
                DurabilityMode::Journaled,
                Some(dir.path()),
            );
            assert_eq!(code, std::process::ExitCode::SUCCESS);

            let journal = vb_storage::FjallJournal::open(dir.path(), None);
            assert!(journal.is_ok(), "journal should reopen");
            if let Ok(journal) = journal {
                let run = vb_core::RunId::new(1);
                let events = journal.events_for_run(run);
                assert!(events.is_ok(), "events should be readable: {events:?}");

                if let Ok(events) = events {
                    assert!(events.contains(&JournalEvent::RunAccepted {
                        run,
                        seq: EventSeq::new(0),
                        workflow: WorkflowDigest::from_bytes([9; 32]),
                    }));
                    assert!(events.iter().any(|e| matches!(
                        e,
                        JournalEvent::RunFinished {
                            run: r,
                            result: SlotIdx::ZERO,
                            ..
                        } if *r == run
                    )));
                }
            }
        }
    }

    #[test]
    fn ipc_storage_resolver_loads_compiled_ir_from_journal() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        let dir = tempfile::tempdir();
        assert!(dir.is_ok(), "test directory should be available: {dir:?}");

        if let (Some(compiled), Ok(dir)) = (compiled, dir) {
            let journal = vb_storage::FjallJournal::open(dir.path(), None);
            assert!(journal.is_ok(), "journal should open");
            let Ok(journal) = journal else {
                return;
            };
            let parts = compiled.to_parts();
            let ir = postcard::to_allocvec(&parts);
            assert!(ir.is_ok(), "workflow parts should encode: {ir:?}");
            let Ok(ir) = ir else {
                return;
            };
            let record = CompiledIrRecord {
                digest: compiled.digest(),
                ir,
            };
            assert!(
                journal.put_compiled_ir(&record).is_ok(),
                "put_compiled_ir must succeed"
            );
            let mut resolver = StorageWorkflowResolver {
                journal: Arc::new(journal),
            };

            let resolved = vb_ipc::server::WorkflowResolver::resolve_workflow(
                &mut resolver,
                compiled.digest(),
            );

            assert!(resolved.is_ok(), "resolver should load compiled IR");
            let Ok(resolved) = resolved else {
                return;
            };
            assert_eq!(resolved.digest(), compiled.digest());
        }
    }

    #[test]
    fn ipc_storage_resolver_returns_not_found_for_missing_digest() {
        let dir = tempfile::tempdir();
        assert!(dir.is_ok(), "test directory should be available: {dir:?}");
        if let Ok(dir) = dir {
            let journal = vb_storage::FjallJournal::open(dir.path(), None);
            assert!(journal.is_ok(), "journal should open");
            let Ok(journal) = journal else {
                return;
            };
            let mut resolver = StorageWorkflowResolver {
                journal: Arc::new(journal),
            };

            let result = vb_ipc::server::WorkflowResolver::resolve_workflow(
                &mut resolver,
                WorkflowDigest::from_bytes([5; 32]),
            );

            assert!(
                matches!(
                    result,
                    Err(vb_ipc::server::WorkflowResolutionError::NotFound)
                ),
                "missing digest should return NotFound"
            );
        }
    }

    #[test]
    fn parse_run_without_step_flags_produces_none_step() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Run { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Run { step, .. }) = parsed {
            assert!(step.is_none());
        }
    }

    #[test]
    fn parse_run_step_requires_step_input() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
            "--step",
            "0",
        ]));
        assert!(
            matches!(parsed, Err(ParseError::MissingArgument("--step-input"))),
            "unexpected parse result: {parsed:?}"
        );
    }

    #[test]
    fn step_target_holds_step_id_and_path() {
        let target = StepTarget {
            step_id: 5,
            step_input: PathBuf::from("data.bin"),
        };
        assert_eq!(target.step_id, 5);
        assert_eq!(target.step_input, PathBuf::from("data.bin"));
    }

    #[test]
    fn node_kind_name_returns_correct_labels() {
        assert_eq!(node_kind_name(&CompiledNodeKind::Nop), "Nop");
        assert_eq!(
            node_kind_name(&CompiledNodeKind::SetConst {
                value: ConstIdx::new(0)
            }),
            "SetConst"
        );
        assert_eq!(
            node_kind_name(&CompiledNodeKind::Finish {
                result: SlotIdx::ZERO
            }),
            "Finish"
        );
    }

    #[test]
    fn signal_name_returns_correct_labels() {
        assert_eq!(signal_name(&vb_core::EngineSignal::Continue), "Continue");
        assert_eq!(
            signal_name(&vb_core::EngineSignal::Finished(
                vb_core::SlotValue::Bool(true),
                vb_core::Taint::Clean
            )),
            "Finished"
        );
    }

    #[test]
    fn decode_step_inputs_empty_data_returns_empty() {
        let result = decode_step_inputs(b"");
        assert!(result.is_ok());
        let values = result.expect("ok");
        assert!(values.is_empty());
    }

    #[test]
    fn decode_step_inputs_invalid_data_returns_error() {
        let result = decode_step_inputs(b"garbage");
        assert!(result.is_err());
    }

    #[test]
    fn write_step_inputs_populates_frame_slots() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let mut frame = build_step_frame(&compiled, StepIdx::ZERO).expect("frame should build");
            let inputs: Box<[vb_core::SlotValue]> = Box::from([vb_core::SlotValue::I64(42)]);
            write_step_inputs(&mut frame, &inputs);
            assert_eq!(
                frame.read_slot(SlotIdx::ZERO),
                Ok(&vb_core::SlotValue::I64(42))
            );
        }
    }

    #[test]
    fn execute_step_isolated_set_const_step_succeeds() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let node = compiled.node(StepIdx::ZERO).expect("step 0 must exist");
            let inputs: Box<[vb_core::SlotValue]> = Box::from([]);
            let code = execute_step_isolated(&compiled, StepIdx::ZERO, node, &inputs);
            assert_eq!(code, std::process::ExitCode::SUCCESS);
        }
    }

    #[test]
    fn build_step_frame_out_of_range_returns_error() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let result = build_step_frame(&compiled, StepIdx::new(99));
            assert!(result.is_err());
        }
    }
}