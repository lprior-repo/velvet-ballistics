//! Fuzz target: step_budget_new (FUZZ-001)
!
//! Specifically targets StepBudget::new clamping boundary at MAX_STEP_BUDGET.
//! Verifies that StepBudget::new never panics for any u64 input and always
//! produces a remaining value in [0, MAX_STEP_BUDGET].
//!
//! Obligation: FUZZ-001 (vb-qi37.2.5)
//! Command: cargo fuzz run step_budget_new -- -runs=10000

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    run_with_stdin(fuzz_lib::fuzz_step_budget_new)
}

#[cfg(feature = "fuzz")]
fn run_with_stdin(target: fn(&[u8])) -> std::process::ExitCode {
    let mut input = Vec::new();
    match std::io::Read::read_to_end(&mut std::io::stdin(), &mut input) {
        Ok(_) => {
            target(&input);
            std::process::ExitCode::SUCCESS
        }
        Err(error) => write_stderr(error),
    }
}

#[cfg(feature = "fuzz")]
fn write_stderr(error: std::io::Error) -> std::process::ExitCode {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    match std::io::Write::write_fmt(&mut handle, format_args!("stdin read error: {error}\n")) {
        Ok(()) | Err(_) => {}
    }
    std::process::ExitCode::FAILURE
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
