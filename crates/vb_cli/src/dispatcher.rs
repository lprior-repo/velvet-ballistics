pub(crate) fn run_from_env() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let requested_output = output_format_from_args(&args);
    let parsed = parse_args(&args);

    match parsed {
        Ok(Command::Help) => exit_from_io(&write_help_stdout(), ExitCode::SUCCESS),
        Ok(Command::Version) => exit_from_io(&write_version_stdout(), ExitCode::SUCCESS),
        Ok(Command::AgentContext { deliver }) => cmd_agent_context(deliver.as_deref()),
        Ok(Command::AiContext { run_id, db, output }) => {
            commands_ai_context::handle(&run_id, &db, output)
        }
        Ok(Command::Status { options, output }) => cmd_status(options, output),
        Ok(Command::SystemStatus { options, output }) => cmd_system_status(options, output),
        Ok(Command::ActionList { output, registry }) => cmd_action_list(output, registry),
        Ok(Command::ActionInspect {
            action_name,
            output,
            registry,
        }) => cmd_action_inspect(action_name, output, registry),
        Ok(Command::Verify {
            workflow,
            profile,
            output,
        }) => cmd_verify(&workflow, profile, output),
        Ok(Command::Validate { workflow, output }) => cmd_validate(&workflow, output),
        Ok(Command::Explain { workflow, output }) => cmd_explain(&workflow, output),
        Ok(Command::Compile {
            workflow,
            emit,
            out,
            output,
        }) => cmd_compile(&workflow, emit, &out, output),
        Ok(Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            step,
            output,
        }) => match step {
            Some(target) => cmd_run_step(&workflow, durability, &target, output),
            None => cmd_run(&workflow, &input_bin, durability, db.as_deref(), output),
        },
        Ok(Command::RunCompiled {
            workflow,
            input_bin,
            durability,
            db,
            output,
        }) => cmd_run_compiled(&workflow, &input_bin, durability, db.as_deref(), output),
        Ok(Command::IpcServe { socket, db }) => cmd_ipc_serve(&socket, &db),
        Ok(Command::Inspect { run_id, db, output }) => cmd_inspect(&run_id, &db, output),
        Ok(Command::Events {
            run_id,
            db,
            output,
            status,
            limit,
        }) => cmd_events(&run_id, &db, output, status, limit),
        Ok(Command::Replay { run_id, db, output }) => cmd_replay(&run_id, &db, output),
        Ok(Command::Trace {
            run_id,
            db,
            output,
            filters,
        }) => cmd_trace(&run_id, &db, output, filters),
        Ok(Command::Retry { run_id, db, output }) => cmd_retry(&run_id, &db, output),
        Ok(Command::Resume { run_id, db, output }) => cmd_resume(&run_id, &db, output),
        Ok(Command::BenchRun { workflow, output }) => cmd_bench_run(&workflow, output),
        Ok(Command::Doctor { db, output }) => cmd_doctor(db.as_deref(), output),
        Ok(Command::Answer {
            run_id,
            step,
            value_file,
            db,
            output,
        }) => cmd_answer(&run_id, step, &value_file, &db, output),
        Ok(Command::Graph { workflow, output }) => cmd_graph(&workflow, output),
        Ok(Command::Diff {
            run_a,
            run_b,
            db,
            output,
        }) => cmd_diff(&run_a, &run_b, &db, output),
        Ok(Command::Incident { run_id, db, output }) => cmd_incident(&run_id, &db, output),
        Ok(Command::Submit {
            workflow,
            input_bin,
            db,
            durability,
            output,
        }) => cmd_submit(&workflow, &input_bin, &db, durability, output),
        Ok(Command::Simulate { workflow, output }) => cmd_simulate(&workflow, output),
        Ok(Command::Cancel {
            run_id,
            db,
            reason,
            output,
        }) => cmd_cancel(&run_id, &db, reason, output),
        Err(e) => exit_from_io(
            &write_parse_error_stderr(&e, requested_output),
            CliExitCode::ValidationFailed.into(),
        ),
    }
}
