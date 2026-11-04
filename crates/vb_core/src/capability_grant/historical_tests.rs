use super::{
    CapabilityGrant, GrantIdentity, HistoricalGrantSignature, MAX_GRANT_CAPABILITIES,
    MAX_GRANT_RESOURCES, MAX_GRANT_SIGNATURE_LEN, MAX_GRANT_TEXT_LEN, ResourceScope,
};
use crate::capability::Capability;
use crate::capability_grant::{
    GrantId, GrantSignature as ScopedGrantSignature, RevocationEpoch, ScopedGrant,
};
use crate::ids::{ActionId, RunId, WorkflowDigest};
use crate::manual_gate::GateDigest;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const _: fn(&ScopedGrant) -> &Option<ScopedGrantSignature> = |grant| &grant.signature;

// Manually encoded from the fifteen-field Serde layout at ee72229531fdeb052acacc1071055ff3d9607385.
// No historical Rust producer was executed to obtain these source-derived bytes.
const GOLDEN_WITH_SIGNATURE: &[u8] = &[
    0x81, 0x02, 0x06, 0x69, 0x73, 0x73, 0x75, 0x65, 0x72, 0x07, 0x73, 0x75, 0x62, 0x6a, 0x65, 0x63,
    0x74, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
    0x1f, 0x83, 0x84, 0x04, 0x81, 0x04, 0x02, 0x04, 0x72, 0x65, 0x61, 0x64, 0x81, 0x04, 0x05, 0x77,
    0x72, 0x69, 0x74, 0x65, 0x81, 0x04, 0x02, 0x06, 0x72, 0x65, 0x70, 0x6f, 0x2d, 0x61, 0x07, 0x76,
    0x61, 0x75, 0x6c, 0x74, 0x2d, 0x62, 0x81, 0x01, 0x81, 0x80, 0x01, 0x01, 0x82, 0x02, 0x00, 0x01,
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
    0x01, 0x03, 0xa1, 0xb2, 0xc3,
];
const GOLDEN_WITHOUT_OPTIONALS: &[u8] = &[
    0x81, 0x02, 0x06, 0x69, 0x73, 0x73, 0x75, 0x65, 0x72, 0x07, 0x73, 0x75, 0x62, 0x6a, 0x65, 0x63,
    0x74, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
    0x1f, 0x83, 0x84, 0x04, 0x81, 0x04, 0x02, 0x04, 0x72, 0x65, 0x61, 0x64, 0x81, 0x04, 0x05, 0x77,
    0x72, 0x69, 0x74, 0x65, 0x81, 0x04, 0x02, 0x06, 0x72, 0x65, 0x70, 0x6f, 0x2d, 0x61, 0x07, 0x76,
    0x61, 0x75, 0x6c, 0x74, 0x2d, 0x62, 0x81, 0x01, 0x81, 0x80, 0x01, 0x01, 0x82, 0x02, 0x00, 0x00,
    0x00,
];
const SCOPED_SIGNATURE_BYTES: [u8; 64] = [
    0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19,
    0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19,
    0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7,
    0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7, 0xe7,
];

fn capability(name: &str, action: u16) -> Capability {
    Capability::new(name.into(), ActionId::new(action))
}

fn historical_record() -> TestResult<CapabilityGrant> {
    Ok(CapabilityGrant {
        id: GrantId::new(257),
        issuer: GrantIdentity::new("issuer".to_string()).ok_or("invalid fixture issuer")?,
        subject: GrantIdentity::new("subject".to_string()).ok_or("invalid fixture subject")?,
        artifact_digest: WorkflowDigest::from_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ]),
        run: RunId::new(66_051),
        action: ActionId::new(513),
        capabilities: Box::new([capability("read", 513), capability("write", 513)]),
        resources: ResourceScope {
            patterns: Box::new(["repo-a".into(), "vault-b".into()]),
        },
        issued_at_ms: 129,
        expires_at_ms: 16_385,
        delegable: true,
        revocation_epoch: RevocationEpoch::new(258),
        break_glass: false,
        approval_receipt_digest: Some(GateDigest::from_bytes([
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d,
            0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b,
            0x3c, 0x3d, 0x3e, 0x3f,
        ])),
        signature: Some(
            HistoricalGrantSignature::new(Box::new([0xa1, 0xb2, 0xc3]))
                .ok_or("invalid fixture signature")?,
        ),
    })
}

#[test]
fn historical_signed_record_matches_independent_source_layout() -> TestResult {
    let expected = historical_record()?;
    assert!(expected.is_valid());
    let (decoded, remainder) = postcard::take_from_bytes::<CapabilityGrant>(GOLDEN_WITH_SIGNATURE)?;
    assert!(remainder.is_empty());
    assert_eq!(decoded, expected);
    assert_eq!(
        postcard::to_allocvec(&expected)?.as_slice(),
        GOLDEN_WITH_SIGNATURE
    );
    assert_eq!(
        decoded
            .signature
            .as_ref()
            .ok_or("signature missing")?
            .bytes
            .len(),
        3
    );
    Ok(())
}

#[test]
fn historical_absent_optionals_keep_their_wire_positions() -> TestResult {
    let mut expected = historical_record()?;
    expected.approval_receipt_digest = None;
    expected.signature = None;
    assert!(expected.is_valid());
    let (decoded, remainder) =
        postcard::take_from_bytes::<CapabilityGrant>(GOLDEN_WITHOUT_OPTIONALS)?;
    assert!(remainder.is_empty());
    assert_eq!(decoded, expected);
    assert_eq!(
        postcard::to_allocvec(&expected)?.as_slice(),
        GOLDEN_WITHOUT_OPTIONALS
    );
    Ok(())
}

#[test]
fn constructors_keep_historical_byte_and_signature_bounds() -> TestResult {
    assert_eq!(MAX_GRANT_TEXT_LEN, 128);
    assert_eq!(MAX_GRANT_CAPABILITIES, 256);
    assert_eq!(MAX_GRANT_RESOURCES, 256);
    assert_eq!(MAX_GRANT_SIGNATURE_LEN, 1024);
    for text in [String::new(), "x".repeat(129), "é".repeat(65)] {
        assert!(GrantIdentity::new(text).is_none());
    }
    for text in ["x".to_string(), "x".repeat(128), "é".repeat(64)] {
        let identity = GrantIdentity::new(text.clone()).ok_or("valid boundary identity refused")?;
        assert!(identity.is_valid());
        assert_eq!(identity.value(), text);
    }
    for length in [0, 1025] {
        assert!(HistoricalGrantSignature::new(vec![7; length].into_boxed_slice()).is_none());
    }
    for length in [1, 64, 1024] {
        let signature = HistoricalGrantSignature::new(vec![7; length].into_boxed_slice())
            .ok_or("valid boundary signature refused")?;
        assert!(signature.is_valid());
        assert_eq!(signature.bytes.len(), length);
    }
    Ok(())
}

#[test]
fn structural_validation_checks_each_independent_invalid_record() -> TestResult {
    type Mutation = fn(&mut CapabilityGrant);
    let cases: [(&str, Mutation); 15] = [
        ("empty issuer", |grant| grant.issuer.value.clear()),
        ("empty subject", |grant| grant.subject.value.clear()),
        ("overlong issuer", |grant| {
            grant.issuer.value = "x".repeat(129)
        }),
        ("overlong UTF-8 subject", |grant| {
            grant.subject.value = "é".repeat(65)
        }),
        ("empty capabilities", |grant| {
            grant.capabilities = Box::new([])
        }),
        ("too many capabilities", |grant| {
            grant.capabilities = vec![capability("read", 513); 257].into_boxed_slice();
        }),
        ("last capability empty", |grant| {
            grant.capabilities = Box::new([capability("read", 513), capability("", 513)]);
        }),
        ("too many resources", |grant| {
            grant.resources.patterns = vec![Box::<str>::from("r"); 257].into_boxed_slice();
        }),
        ("last resource empty", |grant| {
            grant.resources.patterns = Box::new(["valid".into(), "".into()]);
        }),
        ("last resource overlong", |grant| {
            grant.resources.patterns = Box::new(["valid".into(), "x".repeat(129).into()]);
        }),
        ("last resource overlong UTF-8", |grant| {
            grant.resources.patterns = Box::new(["valid".into(), "é".repeat(65).into()]);
        }),
        ("empty recorded signature", |grant| {
            grant.signature = Some(HistoricalGrantSignature {
                bytes: Box::new([]),
            });
        }),
        ("overlong recorded signature", |grant| {
            grant.signature = Some(HistoricalGrantSignature {
                bytes: vec![7; 1025].into_boxed_slice(),
            });
        }),
        ("equal expiry and issue", |grant| {
            grant.expires_at_ms = grant.issued_at_ms
        }),
        ("expiry before issue", |grant| grant.expires_at_ms = 0),
    ];
    for (label, mutate) in cases {
        let mut record = historical_record()?;
        assert!(record.is_valid());
        mutate(&mut record);
        assert!(!record.is_valid(), "{label}");
    }
    Ok(())
}

#[test]
fn historical_data_membership_and_valid_edges_stay_exact() -> TestResult {
    let record = historical_record()?;
    assert!(record.grants(&capability("read", 513)));
    assert!(!record.grants(&capability("read", 514)));
    assert!(!record.grants(&capability("rea", 513)));
    assert!(record.resources.covers("repo-a"));
    assert!(!record.resources.covers("repo"));
    assert!(!record.resources.covers("repo-a/child"));
    let wildcard_text = ResourceScope {
        patterns: Box::new(["repo-*".into()]),
    };
    assert!(wildcard_text.is_valid());
    assert!(wildcard_text.covers("repo-*"));
    assert!(!wildcard_text.covers("repo-a"));

    let mut empty_resources = historical_record()?;
    empty_resources.resources.patterns = Box::new([]);
    assert!(empty_resources.resources.is_valid());
    assert!(empty_resources.is_valid());
    assert!(!empty_resources.resources.covers("repo-a"));

    let long_name = "é".repeat(65);
    let mut long_capability = historical_record()?;
    long_capability.capabilities = Box::new([capability(&long_name, 513)]);
    assert!(long_capability.is_valid());
    assert!(long_capability.grants(&capability(&long_name, 513)));

    let mut maximums = historical_record()?;
    maximums.issuer.value = "x".repeat(128);
    maximums.subject.value = "é".repeat(64);
    maximums.capabilities = vec![capability("read", 513); 256].into_boxed_slice();
    maximums.resources.patterns = vec![Box::<str>::from("é".repeat(64)); 256].into_boxed_slice();
    maximums.signature = Some(HistoricalGrantSignature {
        bytes: vec![7; 1024].into_boxed_slice(),
    });
    assert!(maximums.is_valid());

    let mut oversized_membership = historical_record()?;
    oversized_membership.capabilities = vec![capability("read", 513); 257].into_boxed_slice();
    assert!(!oversized_membership.grants(&capability("read", 513)));
    let oversized_resources = ResourceScope {
        patterns: vec![Box::<str>::from("repo-a"); 257].into_boxed_slice(),
    };
    assert!(!oversized_resources.covers("repo-a"));
    Ok(())
}

#[test]
fn scoped_fixed_signature_keeps_its_separate_type_and_wire() -> TestResult {
    let scoped = ScopedGrantSignature::from_bytes(SCOPED_SIGNATURE_BYTES);
    assert_eq!(scoped.as_bytes(), SCOPED_SIGNATURE_BYTES);
    assert_eq!(
        postcard::to_allocvec(&scoped)?.as_slice(),
        SCOPED_SIGNATURE_BYTES.as_slice()
    );
    let (decoded, remainder) =
        postcard::take_from_bytes::<ScopedGrantSignature>(&SCOPED_SIGNATURE_BYTES)?;
    assert!(remainder.is_empty());
    assert_eq!(decoded.as_bytes(), SCOPED_SIGNATURE_BYTES);
    let historical = HistoricalGrantSignature::new(Box::new([0xa1, 0xb2, 0xc3]))
        .ok_or("historical signature fixture invalid")?;
    assert_eq!(postcard::to_allocvec(&historical)?, [3, 0xa1, 0xb2, 0xc3]);
    Ok(())
}
