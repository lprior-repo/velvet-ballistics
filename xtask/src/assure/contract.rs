//! TenantAccess contract fixture for v1.

use super::schema::*;
use super::error::AssurecError;

pub fn tenant_claim_rel_domain() -> DomainType {
    DomainType::new(
        "TenantClaimRel",
        vec!["Missing", "MatchesRequested", "DiffersFromRequested"],
    )
}

pub fn membership_fact_domain() -> DomainType {
    DomainType::new("MembershipFact", vec!["Exists", "NotExists", "LookupFailed"])
}

pub fn required_error_codes() -> Vec<String> {
    vec![
        "MissingTenantClaim".to_string(),
        "TenantMismatch".to_string(),
        "MembershipLookupFailed".to_string(),
        "NoMembership".to_string(),
    ]
}

pub fn v1_contract() -> Vec<ContractClause> {
    vec![
        ContractClause::new(
            "missing_claim",
            vec![TypedExpr::eq(
                TypedExpr::var("tenant_claim_rel"),
                TypedExpr::var("Missing"),
            )],
            EffectOutcome::err("MissingTenantClaim"),
        ),
        ContractClause::new(
            "mismatch",
            vec![TypedExpr::eq(
                TypedExpr::var("tenant_claim_rel"),
                TypedExpr::var("DiffersFromRequested"),
            )],
            EffectOutcome::err("TenantMismatch"),
        ),
        ContractClause::new(
            "lookup_failed",
            vec![TypedExpr::eq(
                TypedExpr::var("membership_fact"),
                TypedExpr::var("LookupFailed"),
            )],
            EffectOutcome::err("MembershipLookupFailed"),
        ),
        ContractClause::new(
            "no_membership",
            vec![TypedExpr::eq(
                TypedExpr::var("membership_fact"),
                TypedExpr::var("NotExists"),
            )],
            EffectOutcome::err("NoMembership"),
        ),
        ContractClause::new(
            "grant",
            vec![
                TypedExpr::eq(
                    TypedExpr::var("tenant_claim_rel"),
                    TypedExpr::var("MatchesRequested"),
                ),
                TypedExpr::eq(
                    TypedExpr::var("membership_fact"),
                    TypedExpr::var("Exists"),
                ),
            ],
            PathOutcome::Grant.into(),
        ),
    ]
}

impl From<PathOutcome> for EffectOutcome {
    fn from(outcome: PathOutcome) -> Self {
        match outcome {
            PathOutcome::Grant => EffectOutcome { err: String::new() },
            PathOutcome::Error { code } => EffectOutcome::err(code),
        }
    }
}

impl From<EffectOutcome> for PathOutcome {
    fn from(outcome: EffectOutcome) -> Self {
        if outcome.err.is_empty() {
            PathOutcome::Grant
        } else {
            PathOutcome::Error { code: outcome.err }
        }
    }
}

pub fn v1_claim_ceiling() -> ClaimCeiling {
    ClaimCeiling {
        id: "v1_auth_ceiling".to_string(),
        blocked_claims: vec![
            "JWT signature verification correctness".to_string(),
            "JWT expiry verification correctness".to_string(),
            "JWT parser correctness".to_string(),
            "Database truthfulness".to_string(),
            "Revocation correctness".to_string(),
            "Distributed/session correctness".to_string(),
        ],
        reason: "V1 scope is TenantAccess decision table only. Upstream beads must close these claims.".to_string(),
    }
}

pub fn trusted_jwt_verified_oracle() -> OracleRecord {
    OracleRecord {
        id: "oracle_jwt_verified_assumption".to_string(),
        source_kind: OracleProvenanceKind::VcsPreexisting,
        provenance: OracleProvenance {
            commit: "0000000000000000000000000000000000000000".to_string(),
            present_in_merge_base: true,
            signature_verified: true,
        },
        claim: "JwtVerified is assumed from upstream JWT validation".to_string(),
        generated: false,
    }
}

pub fn validate_contract(clauses: &[ContractClause]) -> Result<(), AssurecError> {
    for clause in clauses {
        if clause.when.is_empty() {
            return Err(AssurecError::Contract(format!(
                "clause {} has empty when condition",
                clause.id
            )));
        }
    }

    let ids: Vec<_> = clauses.iter().map(|c| c.id.as_str()).collect();
    for id in &ids {
        if ids.iter().filter(|&x| x == id).count() > 1 {
            return Err(AssurecError::Contract(format!("duplicate clause id: {}", id)));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_contract_has_5_paths() {
        let clauses = v1_contract();
        assert_eq!(clauses.len(), 5);
    }

    #[test]
    fn tenant_claim_rel_has_3_variants() {
        let domain = tenant_claim_rel_domain();
        assert_eq!(domain.variants.len(), 3);
    }

    #[test]
    fn validate_contract_rejects_empty_when() {
        let mut clauses = v1_contract();
        clauses.push(ContractClause::new("empty_when", vec![], EffectOutcome::err("Err")));
        let result = validate_contract(&clauses);
        assert!(result.is_err());
    }

    #[test]
    fn validate_contract_rejects_duplicate_ids() {
        let mut clauses = v1_contract();
        clauses.push(ContractClause::new(
            "missing_claim",
            vec![],
            EffectOutcome::err("Err"),
        ));
        let result = validate_contract(&clauses);
        assert!(result.is_err());
    }
}
