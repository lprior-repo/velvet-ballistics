//! Bounded subprocess execution for tooling fuzz targets.
//!
//! Tooling targets spawn repo scripts once per fuzzer input to fuzz their
//! argument handling. Argument handling is millisecond-scale, but
//! adversarial arguments can steer the scripts into real work (cargo
//! builds, workspace scans) that outlasts the smoke lane's per-target
//! `timeout 30s` budget and fails the lane under `set -e`. Every spawn
//! here carries a hard wall-clock cap so argument handling stays fuzzable
//! while unbounded downstream work cannot hang the lane.
//!
//! The cap is enforced with the `timeout(1)` wrapper, matching the lane's
//! Linux execution environment.

/// Smoke cap shared by tooling fuzz targets: comfortably above
/// millisecond-scale argument handling, far below the lane's 30s slot.
pub const SMOKE_COMMAND_TIMEOUT_SECS: u64 = 5;

/// Runs `program` with `args` and `env`, killing the child when it exceeds
/// `timeout_secs`. The child's own exit status (any value) and output are
/// discarded: these targets fuzz argument handling for panics, not script
/// results.
pub fn run_bounded(program: &str, args: &[&str], env: &[(&str, &str)], timeout_secs: u64) {
    let timeout_arg = std::format!("{timeout_secs}s");
    let mut command = std::process::Command::new("timeout");
    command.arg(timeout_arg).arg(program).args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    match command.output() {
        Ok(_) | Err(_) => {}
    }
}

/// Runs with the shared smoke cap.
pub fn run_smoke_bounded(program: &str, args: &[&str], env: &[(&str, &str)]) {
    run_bounded(program, args, env, SMOKE_COMMAND_TIMEOUT_SECS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_command_returns_within_cap() {
        let start = std::time::Instant::now();
        run_bounded("true", &[], &[], 5);
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "true must return at once"
        );
    }

    #[test]
    fn slow_command_is_killed_at_cap() {
        let start = std::time::Instant::now();
        run_bounded("sleep", &["30"], &[], 1);
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(10),
            "sleep 30 must die near the 1s cap, took {elapsed:?}"
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(500),
            "cap must actually wait out the second, took {elapsed:?}"
        );
    }

    #[test]
    fn missing_program_does_not_panic() {
        run_bounded("definitely-not-a-real-program-xyz", &[], &[], 5);
    }
}
