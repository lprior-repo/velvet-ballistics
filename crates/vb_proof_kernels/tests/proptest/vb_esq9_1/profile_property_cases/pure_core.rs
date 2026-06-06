//! Idempotence properties for pure profile-contract functions.

use proptest::prelude::*;
use vb_proof_kernels::profile_contract::{
    MASTER_PROFILE_CONTRACT, resolve_inheritance, validate_against_governance,
    validate_against_master,
};

use super::strategies::arb_workspace_profile_set;

proptest! {
    #[test]
    fn prop_pure_core_functions_are_idempotent(ws in arb_workspace_profile_set()) {
        let gaps1 = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);
        let gaps2 = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);
        assert_eq!(gaps1, gaps2, "validate_against_master must be deterministic");

        let gov1 = validate_against_governance(&ws);
        let gov2 = validate_against_governance(&ws);
        assert_eq!(gov1, gov2, "validate_against_governance must be deterministic");

        for config in &ws.profiles {
            let r1 = resolve_inheritance(config, &ws);
            let r2 = resolve_inheritance(config, &ws);
            assert_eq!(r1, r2, "resolve_inheritance must be deterministic");
        }
    }
}
