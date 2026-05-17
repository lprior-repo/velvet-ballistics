#![forbid(unsafe_code)]

use core::fmt;

#[derive(Clone, PartialEq, Eq)]
pub struct RedactionViolation {
    pub code: &'static str,
    pub secret_class: &'static str,
    pub redacted_sample: &'static str,
}

impl fmt::Debug for RedactionViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedactionViolation")
            .field("code", &self.code)
            .field("secret_class", &self.secret_class)
            .field("redacted_sample", &self.redacted_sample)
            .finish()
    }
}

impl fmt::Display for RedactionViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {}",
            self.code, self.secret_class, self.redacted_sample
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RedactionViolation {}

const DENIED: [(&str, &str, &str); 6] = [
    ("vb_nf2u_secret_sentinel", "sentinel", "[REDACTED:sentinel]"),
    (
        "sk_test_vb_nf2u_raw_secret",
        "api_key",
        "[REDACTED:api_key]",
    ),
    ("Bearer vb_nf2u_token", "token", "[REDACTED:token]"),
    ("password=hunter2", "password", "[REDACTED:password]"),
    (
        "Idempotency-Key: idem_vb_nf2u_secret",
        "idempotency_key",
        "[REDACTED:idempotency_key]",
    ),
    (
        "tainted_fixture_value_vb_nf2u",
        "tainted_fixture_value",
        "[REDACTED:tainted_fixture_value]",
    ),
];

pub fn scan_release_artifact(artifact: &str) -> Result<(), RedactionViolation> {
    for (raw, secret_class, redacted_sample) in DENIED {
        if artifact.contains(raw) {
            return Err(RedactionViolation {
                code: "redaction_violation",
                secret_class,
                redacted_sample,
            });
        }
    }
    Ok(())
}
