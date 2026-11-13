use crate::benchmark_policy::error::PolicyError;

fn flag_name(flag: &str) -> String {
    if let Some(stripped) = flag.strip_prefix("--") {
        format!("--{stripped}")
    } else {
        flag.to_string()
    }
}

fn metavariable(flag: &str) -> String {
    if let Some(stripped) = flag.strip_prefix("--") {
        stripped.to_ascii_uppercase()
    } else {
        "VALUE".to_string()
    }
}

fn usage_line(selected: &[&str]) -> String {
    let flags = if selected.is_empty() {
        String::from("[OPTIONS]")
    } else {
        selected
            .iter()
            .map(|name| match *name {
                "--budget" => "--budget <BUDGET>",
                "--evidence" => "--evidence <EVIDENCE>",
                _ => "--help",
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    format!("Usage: benchmark-policy {flags} [ROOT]")
}

pub(super) fn missing(flag: &str) -> PolicyError {
    let name = flag_name(flag);
    let meta = metavariable(flag);
    let msg = format!(
        "error: a value is required for '{name} <{meta}>' but none was supplied\n\nFor more information, try '--help'.\n"
    );
    PolicyError::Usage(msg)
}

pub(super) fn extra_root(argument: &str) -> PolicyError {
    PolicyError::Usage(format!(
        "error: unexpected argument '{argument}' found\n\n{}\n\nFor more information, try '--help'.\n",
        usage_line(&[])
    ))
}

pub(super) fn argument_encoding() -> PolicyError {
    PolicyError::Usage(format!(
        "error: arguments must be valid UTF-8\n\n{}\n\nFor more information, try '--help'.\n",
        usage_line(&[])
    ))
}

pub(super) fn unknown(argument: &str, selected: &[&str]) -> PolicyError {
    let clean = argument.split_once('=').map_or(argument, |(name, _)| name);
    let mut selected = selected.to_vec();
    let tip = match super::suggestions::suggest(clean) {
        Some(name) => {
            let flag = match name {
                "budget" => "--budget",
                "evidence" => "--evidence",
                _ => "--help",
            };
            if !selected.contains(&flag) {
                selected.push(flag);
            }
            format!("  tip: a similar argument exists: '{flag}'\n\n")
        }
        None => String::new(),
    };
    PolicyError::Usage(format!(
        "error: unexpected argument '{clean}' found\n\n{tip}{}\n\nFor more information, try '--help'.\n",
        usage_line(&selected)
    ))
}
