/// Default first-party crates to scan when no globs are provided.
pub(crate) const DEFAULT_FIRST_PARTY_CRATES: &[&str] = &[
    "vb_compile",
    "vb_core",
    "vb_ipc",
    "vb_queue_semantics",
    "vb_runtime",
    "vb_storage",
    "vb_validate",
    "vb_cli",
    "workspace_tests",
];
/// Forbidden patterns: (pattern, description)
pub(crate) const FORBIDDEN_PATTERNS: &[(&str, &str)] = &[
    (r"Arc::unwrap\b", "Arc::unwrap call"),
    (r"Arc::expect\b", "Arc::expect call"),
    (r"\b\.unwrap\(\)", "arbitrary .unwrap() call"),
    (r"\b\.expect\(", "arbitrary .expect() call"),
    (r"\bpanic!\s*\(", "panic! macro invocation"),
    (r"\btodo!\s*\(", "todo! macro invocation"),
    (r"\bunimplemented!\s*\(", "unimplemented! macro invocation"),
];
/// Unsafe block pattern (to detect unsafe { } blocks).
pub(crate) const UNSAFE_BLOCK_PATTERN: &str = r"\bunsafe\s*\{";
pub(crate) fn regex_contains(haystack: &str, pattern: &str) -> bool {
    // Use simple substring matching for known patterns to avoid regex crate dependency
    // For complex patterns, fall back to regex crate
    match pattern {
        r"Arc::unwrap\b" => haystack.contains("Arc::unwrap"),
        r"Arc::expect\b" => haystack.contains("Arc::expect"),
        r"\b\.unwrap\(\)" => {
            // Match .unwrap() but not dbg!.unwrap()
            haystack.contains(".unwrap()") && !haystack.contains("dbg!.unwrap()")
        }
        r"\b\.expect\(" => haystack.contains(".expect(") && !haystack.contains("dbg!.expect("),
        r"\bpanic!\s*\(" => haystack.contains("panic!"),
        r"\btodo!\s*\(" => haystack.contains("todo!"),
        r"\bunimplemented!\s*\(" => haystack.contains("unimplemented!"),
        r"\bunsafe\s*\{" => haystack.contains("unsafe {"),
        _ => {
            // Fall back to regex for complex patterns
            use regex::Regex;
            let re = match Regex::new(pattern) {
                Ok(r) => r,
                Err(_) => return haystack.contains(pattern),
            };
            re.is_match(haystack)
        }
    }
}
