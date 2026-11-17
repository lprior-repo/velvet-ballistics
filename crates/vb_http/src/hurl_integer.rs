pub(super) fn parse(value: &str) -> Option<i64> {
    let digits = value.strip_prefix('-').map_or(value, |digits| digits);
    if digits.is_empty()
        || (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value.parse().ok()
}
