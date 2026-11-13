pub(crate) const FAMILIES: [&str; 6] = [
    "repeat", "for_each", "collect", "reduce", "together", "choose",
];

pub(crate) fn indent(text: &str, count: usize) -> String {
    text.lines()
        .map(|line| format!("{}{line}\n", " ".repeat(count)))
        .collect()
}

pub(crate) fn compound(family: &str, name: &str, body: &str) -> Result<String, String> {
    let primitive = match family {
        "repeat" => format!("repeat:\n  max_attempts: 2\n  steps:\n{}", indent(body, 4)),
        "for_each" => format!(
            "for_each:\n  variable: item_{name}\n  input: $input.items\n  at_once: 1\n  steps:\n{}",
            indent(body, 4)
        ),
        "collect" => format!(
            "collect:\n  variable: page_{name}\n  source: $input.items\n  pages: 2\n  items: 1\n  steps:\n{}",
            indent(body, 4)
        ),
        "reduce" => format!(
            "reduce:\n  variable: accumulator_{name}\n  input: $input.items\n  initial: '0'\n  steps:\n{}",
            indent(body, 4)
        ),
        "together" => format!(
            "together:\n  branches:\n    - label: first_{name}\n      steps:\n{}    - label: second_{name}\n      steps:\n        - id: other_{name}\n          set: {{output: other_{name}, value: '17'}}\n",
            indent(body, 8)
        ),
        "choose" => format!(
            "choose:\n  branches:\n    - when: 'true'\n      steps:\n{}",
            indent(body, 8)
        ),
        _ => return Err(format!("unknown family {family}")),
    };
    Ok(format!("- id: {name}\n{}", indent(&primitive, 2)))
}
