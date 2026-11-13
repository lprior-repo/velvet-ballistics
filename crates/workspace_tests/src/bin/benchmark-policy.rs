#![forbid(unsafe_code)]
#[path = "../benchmark_policy/mod.rs"]
mod benchmark_policy;
fn main() -> std::process::ExitCode {
    benchmark_policy::run_cli()
}
