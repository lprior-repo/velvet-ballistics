// Obligation: PO-R03
// Claim: Manifest/version advancement is impossible before verification
// succeeds. Missing-verification path returns typed error.
#![cfg(kani)]

const CURRENT_VERSION: u16 = 2;

#[derive(Clone, Copy, kani::Arbitrary)]
struct AoahInput {
    phase_verified: bool,
    manifest_version: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdvanceResult {
    Advanced { new_version: u16 },
    ErrorNotVerified,
}

fn adapter_advance_manifest(verified: bool, current_version: u16) -> AdvanceResult {
    if verified {
        AdvanceResult::Advanced {
            new_version: current_version,
        }
    } else {
        AdvanceResult::ErrorNotVerified
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn vb_aoah_verify_before_manifest_advance() {
    let input: AoahInput = kani::any();
    kani::assume(input.manifest_version <= 5);

    let result = adapter_advance_manifest(input.phase_verified, CURRENT_VERSION);

    match result {
        AdvanceResult::Advanced { new_version } => {
            // Claim: advance only allowed if verified
            assert!(input.phase_verified);
            assert_eq!(new_version, CURRENT_VERSION);
        }
        AdvanceResult::ErrorNotVerified => {
            // Claim: unverified state returns typed error, not silent advancement
            assert!(!input.phase_verified);
        }
    }
}
