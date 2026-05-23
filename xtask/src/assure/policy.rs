//! Static policy: AST-grep rules, source scanner, derive/serde policy.

use super::error::AssurecError;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PolicyViolation {
    pub rule: String,
    pub file: String,
    pub line: u32,
    pub message: String,
}

impl PolicyViolation {
    pub fn new(rule: impl Into<String>, file: impl Into<String>, line: u32, message: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            file: file.into(),
            line,
            message: message.into(),
        }
    }
}

pub fn check_forbidden_derives(source: &str, forbidden: &[&str]) -> Vec<PolicyViolation> {
    let mut violations = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        for derive in forbidden {
            if line.contains(&format!("#[derive({})]", derive))
                || line.contains(&format!("derive({})", derive))
            {
                violations.push(PolicyViolation::new(
                    "forbidden_derive",
                    "<source>",
                    line_num as u32 + 1,
                    format!("forbidden derive: {}", derive),
                ));
            }
        }
    }

    violations
}

pub fn check_direct_construction(source: &str, witness_types: &[&str]) -> Vec<PolicyViolation> {
    let mut violations = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        for wt in witness_types {
            if line.contains(&format!("{}::new(", wt))
                || line.contains(&format!("{} {{", wt))
                || line.contains(&format!("let {} = {}", wt, wt))
            {
                violations.push(PolicyViolation::new(
                    "direct_witness_construction",
                    "<source>",
                    line_num as u32 + 1,
                    format!("direct construction of witness type: {}", wt),
                ));
            }
        }
    }

    violations
}

pub fn check_witness_policy(source: &str, allowed: &[&str]) -> Vec<PolicyViolation> {
    let mut violations = Vec::new();
    let forbidden = vec!["Default", "Copy", "Serialize", "Deserialize", "From", "Into", "Deref", "DerefMut"];

    for (line_num, line) in source.lines().enumerate() {
        if line.contains("#[derive(") {
            let derives = line.split("derive(").nth(1)
                .and_then(|s| s.split(')').next())
                .unwrap_or("");

            for d in &forbidden {
                if derives.contains(d) && !allowed.contains(&d) {
                    violations.push(PolicyViolation::new(
                        "forbidden_witness_trait",
                        "<source>",
                        line_num as u32 + 1,
                        format!("forbidden trait on witness: {}", d),
                    ));
                }
            }
        }
    }

    violations
}

pub fn check_deserialization_bypass(source: &str) -> Vec<PolicyViolation> {
    let mut violations = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        if line.contains("impl Deserialize")
            && line.contains("for TenantAccess")
        {
            violations.push(PolicyViolation::new(
                "witness_deserialization",
                "<source>",
                line_num as u32 + 1,
                "witness type has Deserialize impl - forbidden in v1",
            ));
        }
    }

    violations
}

pub fn generate_ast_grep_rules(bead_id: &str) -> String {
    format!(
        r#"# Generated AST-grep rules for {}
# DO NOT EDIT - content addressed

rules:
  - id: no_direct_tenant_access
    pattern: |
      struct TenantAccess
    message: TenantAccess must not be constructed directly

  - id: no_default_tenant_access
    pattern: |
      impl Default for TenantAccess
    message: Default impl on witness type forbidden in v1

  - id: no_copy_tenant_access
    pattern: |
      impl Copy for TenantAccess
    message: Copy impl on witness type forbidden in v1

  - id: no_serde_tenant_access
    pattern: |
      impl Serialize for TenantAccess
      impl Deserialize for TenantAccess
    message: Serde impl on witness type forbidden in v1
"#,
        bead_id
    )
}

pub fn check_generated_drift(
    generated_dir: &Path,
    manifest: &std::collections::BTreeMap<String, String>,
) -> Result<Vec<PolicyViolation>, AssurecError> {
    let mut violations = Vec::new();

    for (path, expected_digest) in manifest {
        let full_path = generated_dir.join(path);
        if !full_path.exists() {
            violations.push(PolicyViolation::new(
                "generated_file_missing",
                path,
                0,
                format!("generated file missing: {}", path),
            ));
            continue;
        }

        let content = std::fs::read_to_string(&full_path)?;
        let actual_digest = format!("{}", blake3::hash(content.as_bytes()));

        if actual_digest != *expected_digest {
            violations.push(PolicyViolation::new(
                "generated_drift",
                path,
                0,
                "generated file has been modified - drift detected",
            ));
        }
    }

    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_forbidden_derives_finds_default() {
        let source = r#"
            #[derive(Default)]
            pub struct TenantAccess();
        "#;
        let violations = check_forbidden_derives(source, &["Default"]);
        assert!(!violations.is_empty());
    }

    #[test]
    fn check_forbidden_derives_allows_when_not_present() {
        let source = r#"
            #[derive(Debug, Clone)]
            pub struct TenantAccess();
        "#;
        let violations = check_forbidden_derives(source, &["Default"]);
        assert!(violations.is_empty());
    }
}
