use super::*;

pub(crate) fn closing_brace(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.text.as_str() {
            "{" => depth = depth.saturating_add(1),
            "}" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn leading_file_cfg_test(lexed: &[Token]) -> bool {
    let mut index = 0;
    while index < lexed.len() {
        let text = &lexed[index].text;
        if text.starts_with("//") || text.starts_with("/*") {
            index = index.saturating_add(1);
            continue;
        }
        break;
    }
    if index.saturating_add(8) > lexed.len() {
        return false;
    }
    lexed[index..index + 8]
        .iter()
        .zip(["#", "!", "[", "cfg", "(", "test", ")", "]"].iter())
        .all(|(actual, expected)| actual.text == **expected)
}
