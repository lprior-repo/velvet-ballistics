pub(crate) fn hot_function_source(name: &str, logical_lines: usize) -> String {
    let mut text = String::from(
        "// fixture prelude 1\n// fixture prelude 2\n// fixture prelude 3\n// fixture prelude 4\n",
    );
    text.push_str(&format!("pub fn {name}() -> u8 {{\n"));
    for number in 0..logical_lines.saturating_sub(2) {
        text.push_str(&format!("    let value_{number} = {number};\n"));
    }
    text.push_str("    0\n}\n");
    text
}

pub(crate) fn unsafe_hot_function_source(name: &str, logical_lines: usize) -> String {
    let mut text = String::from(
        "// fixture prelude 1\n// fixture prelude 2\n// fixture prelude 3\n// fixture prelude 4\n",
    );
    text.push_str(&format!("pub unsafe fn {name}() -> u8 {{\n"));
    for number in 0..logical_lines.saturating_sub(2) {
        text.push_str(&format!("    let value_{number} = {number};\n"));
    }
    text.push_str("    0\n}\n");
    text
}

pub(crate) fn adversarial_hot_function_source() -> String {
    let mut text = String::from(
        "// fixture prelude 1\n// fixture prelude 2\n// fixture prelude 3\n// fixture prelude 4\n",
    );
    text.push_str("pub fn hostile() -> u8 {\n");
    text.push_str("    let quoted = \"unterminated { { {;\n");
    text.push_str("    let charish = '}';\n");
    text.push_str("    /* comment opens with {\n");
    text.push_str("       still comment }\n");
    text.push_str("    */\n");
    for number in 0..23 {
        text.push_str(&format!("    let value_{number} = {number};\n"));
    }
    text.push_str("    0\n}\n");
    text
}

pub(crate) fn long_file_source(lines: u16) -> String {
    let mut text = String::new();
    for line in 1..=lines {
        text.push_str(&format!("// line {line}\n"));
    }
    text
}

pub(crate) fn non_utf8_source_bytes() -> Vec<u8> {
    let mut bytes = b"// fixture prelude\npub fn invalid_utf8() -> u8 {\n    0\n}\n".to_vec();
    bytes.extend([0xff, 0xfe, b'\n']);
    bytes
}
