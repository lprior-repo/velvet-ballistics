use super::*;
use crate::{DecodeLimits, HttpLimits, RepositoryRef};
use std::net::SocketAddr;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn action(
    repository: &str,
    peer: u8,
    bytes: usize,
) -> Result<WorkflowCountAction, Box<dyn std::error::Error>> {
    Ok(WorkflowCountAction::new(
        &RepositoryRef::new("owner", repository)?,
        SocketAddr::from(([1, 1, 1, peer], 443)),
        HttpLimits::new(bytes, 1024, Duration::from_secs(1))?,
        DecodeLimits::new(1024, 8, 128, 128)?,
    )?)
}

fn fixture()
-> Result<(WorkflowCountAction, HttpDeclarationV1, HttpRequestPolicy), Box<dyn std::error::Error>> {
    let action = action("repo", 1, 1024)?;
    let declaration = HttpDeclarationV1 {
        version: 1,
        id: GrantId::new(7),
        artifact: WorkflowDigest::from_bytes([1; 32]),
        action: ActionId::new(3),
        request: action.request_digest()?,
        issuer: ApproverId::new("issuer".to_owned()).ok_or("invalid issuer")?,
        subject: ApproverId::new("subject".to_owned()).ok_or("invalid subject")?,
        issued_at: GrantTimeMs::new(100),
        expires_at: GrantTimeMs::new(200),
        revocation_epoch: RevocationEpoch::new(2),
    };
    let policy = HttpRequestPolicy {
        artifact: declaration.artifact,
        action: declaration.action,
        request: declaration.request,
        expected_issuer: declaration.issuer.clone(),
        expected_subject: declaration.subject.clone(),
        now: GrantTimeMs::new(100),
        min_epoch: RevocationEpoch::new(2),
        max_lifetime_ms: NonZeroU64::new(100).ok_or("zero lifetime")?,
    };
    Ok((action, declaration, policy))
}

#[test]
fn exact_declaration_is_checked_only_at_supplied_time() -> TestResult {
    let (action, declaration, mut policy) = fixture()?;
    for now in [100, 199] {
        policy.now = GrantTimeMs::new(now);
        let checked = ValidatedHttpDeclaration::check(declaration.clone(), &policy, &action, &[])?;
        assert_eq!(checked.declaration(), &declaration);
        assert_eq!(checked.checked_at(), policy.now);
    }
    for now in [99, 200, u64::MAX] {
        policy.now = GrantTimeMs::new(now);
        assert_eq!(
            ValidatedHttpDeclaration::check(declaration.clone(), &policy, &action, &[]).err(),
            Some(DeclarationRejection::Time)
        );
    }
    Ok(())
}

#[test]
fn exact_identity_version_and_request_are_required() -> TestResult {
    let (action, declaration, policy) = fixture()?;
    let mut changed = declaration.clone();
    changed.version = 2;
    assert_eq!(
        ValidatedHttpDeclaration::check(changed, &policy, &action, &[]).err(),
        Some(DeclarationRejection::Version)
    );
    let mut changed = declaration.clone();
    changed.artifact = WorkflowDigest::from_bytes([9; 32]);
    assert_eq!(
        ValidatedHttpDeclaration::check(changed, &policy, &action, &[]).err(),
        Some(DeclarationRejection::Invocation)
    );
    let mut changed = declaration.clone();
    changed.action = ActionId::new(4);
    assert_eq!(
        ValidatedHttpDeclaration::check(changed, &policy, &action, &[]).err(),
        Some(DeclarationRejection::Invocation)
    );
    let mut changed = declaration.clone();
    changed.issuer.value = "foreign".to_owned();
    assert_eq!(
        ValidatedHttpDeclaration::check(changed, &policy, &action, &[]).err(),
        Some(DeclarationRejection::Identity)
    );
    let mut changed = declaration.clone();
    changed.subject.value = "foreign".to_owned();
    assert_eq!(
        ValidatedHttpDeclaration::check(changed, &policy, &action, &[]).err(),
        Some(DeclarationRejection::Identity)
    );
    let mut changed = declaration;
    changed.request = RequestDigest::from_digest(WorkflowDigest::from_bytes([9; 32]));
    assert_eq!(
        ValidatedHttpDeclaration::check(changed, &policy, &action, &[]).err(),
        Some(DeclarationRejection::Request)
    );
    Ok(())
}

#[test]
fn matching_malformed_identities_do_not_validate() -> TestResult {
    let (action, mut declaration, mut policy) = fixture()?;
    declaration.issuer.value.clear();
    policy.expected_issuer.value.clear();
    assert_eq!(
        ValidatedHttpDeclaration::check(declaration, &policy, &action, &[]).err(),
        Some(DeclarationRejection::Identity)
    );
    let (action, mut declaration, mut policy) = fixture()?;
    declaration.subject.value = "x".repeat(4096);
    policy.expected_subject = declaration.subject.clone();
    assert_eq!(
        ValidatedHttpDeclaration::check(declaration, &policy, &action, &[]).err(),
        Some(DeclarationRejection::Identity)
    );
    Ok(())
}

#[test]
fn finite_window_and_current_revocation_are_required() -> TestResult {
    let (action, declaration, mut policy) = fixture()?;
    for expires in [99, 100, 201, u64::MAX] {
        let mut changed = declaration.clone();
        changed.expires_at = GrantTimeMs::new(expires);
        assert_eq!(
            ValidatedHttpDeclaration::check(changed, &policy, &action, &[]).err(),
            Some(DeclarationRejection::Window)
        );
    }
    policy.min_epoch = RevocationEpoch::new(3);
    assert_eq!(
        ValidatedHttpDeclaration::check(declaration.clone(), &policy, &action, &[]).err(),
        Some(DeclarationRejection::Epoch)
    );
    policy.min_epoch = RevocationEpoch::new(2);
    assert_eq!(
        ValidatedHttpDeclaration::check(declaration.clone(), &policy, &action, &[declaration.id])
            .err(),
        Some(DeclarationRejection::Revoked)
    );
    let full = vec![GrantId::new(8); MAX_REVOKED_IDS];
    assert!(ValidatedHttpDeclaration::check(declaration.clone(), &policy, &action, &full).is_ok());
    let mut over = full;
    over.push(GrantId::new(8));
    assert_eq!(
        ValidatedHttpDeclaration::check(declaration, &policy, &action, &over).err(),
        Some(DeclarationRejection::RevocationLimit)
    );
    Ok(())
}

#[test]
fn actual_request_must_match_even_when_declaration_matches_policy() -> TestResult {
    let (_, declaration, policy) = fixture()?;
    for changed in [
        action("other", 1, 1024)?,
        action("repo", 2, 1024)?,
        action("repo", 1, 2048)?,
    ] {
        assert_ne!(changed.request_digest()?, declaration.request);
        assert_eq!(
            ValidatedHttpDeclaration::check(declaration.clone(), &policy, &changed, &[]).err(),
            Some(DeclarationRejection::Request)
        );
    }
    Ok(())
}

#[test]
fn decode_and_header_deadline_limits_are_inherited_from_request_digest() -> TestResult {
    let (_, declaration, policy) = fixture()?;
    let repository = RepositoryRef::new("owner", "repo")?;
    let peer = SocketAddr::from(([1, 1, 1, 1], 443));
    let changed_decode = WorkflowCountAction::new(
        &repository,
        peer,
        HttpLimits::new(1024, 1024, Duration::from_secs(1))?,
        DecodeLimits::new(1024, 8, 64, 128)?,
    )?;
    let changed_headers = WorkflowCountAction::new(
        &repository,
        peer,
        HttpLimits::new(1024, 2048, Duration::from_secs(1))?,
        DecodeLimits::new(1024, 8, 128, 128)?,
    )?;
    let changed_deadline = WorkflowCountAction::new(
        &repository,
        peer,
        HttpLimits::new(1024, 1024, Duration::from_secs(2))?,
        DecodeLimits::new(1024, 8, 128, 128)?,
    )?;
    for changed in [changed_decode, changed_headers, changed_deadline] {
        assert_ne!(changed.request_digest()?, declaration.request);
        assert_eq!(
            ValidatedHttpDeclaration::check(declaration.clone(), &policy, &changed, &[]).err(),
            Some(DeclarationRejection::Request)
        );
    }
    Ok(())
}
