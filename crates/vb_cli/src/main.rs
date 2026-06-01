//! Velvet Ballistics binary entrypoint.
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    vb_cli::run_from_env()
}
