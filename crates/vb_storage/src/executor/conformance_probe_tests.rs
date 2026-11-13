//! Observes the trusted suite's changed-scope and changed-pack probes.
//! Deleting either probe call makes this test fail; no literal case list is asserted.

use super::conformance::{execute_suite, probe_observations, reset_probe_observations};

#[test]
fn trusted_suite_executes_changed_scope_and_pack_probes() -> Result<(), Box<dyn std::error::Error>>
{
    reset_probe_observations();
    let suite = execute_suite()?;
    let observed = probe_observations();
    assert_eq!(observed & 0b01, 0b01, "changed-scope probe did not execute");
    assert_eq!(observed & 0b10, 0b10, "changed-pack probe did not execute");
    assert_eq!(suite.report().effects_before_reopen, 1);
    assert_eq!(suite.report().effects_after_reopen, 1);
    assert_eq!(suite.report().target_after_reopen, 42);
    Ok(())
}
