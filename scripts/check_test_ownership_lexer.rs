#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub(crate) text: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(super) fn tokens(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(current) = bytes.get(cursor).copied() else {
            break;
        };
        if current.is_ascii_whitespace() {
            cursor = cursor.saturating_add(1);
        } else if bytes.get(cursor..cursor.saturating_add(2)) == Some(b"//") {
            cursor = cursor.saturating_add(2);
            while cursor < bytes.len() && bytes.get(cursor).is_some_and(|byte| *byte != b'\n') {
                cursor = cursor.saturating_add(1);
            }
        } else if bytes.get(cursor..cursor.saturating_add(2)) == Some(b"/*") {
            cursor = block_comment_end(bytes, cursor);
        } else if let Some(end) = raw_string_end(bytes, cursor) {
            out.push(Token {
                text: source
                    .get(cursor..end)
                    .map_or_else(String::new, str::to_owned),
                start: cursor,
                end,
            });
            cursor = end;
        } else if current == b'"' {
            let end = quoted_end(bytes, cursor, b'"');
            out.push(Token {
                text: source
                    .get(cursor..end)
                    .map_or_else(String::new, str::to_owned),
                start: cursor,
                end,
            });
            cursor = end;
        } else if current == b'\'' && is_character_literal(bytes, cursor) {
            cursor = quoted_end(bytes, cursor, b'\'');
        } else if current.is_ascii_alphanumeric() || current == b'_' {
            let start = cursor;
            cursor = cursor.saturating_add(1);
            while cursor < bytes.len()
                && bytes
                    .get(cursor)
                    .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            {
                cursor = cursor.saturating_add(1);
            }
            out.push(Token {
                text: source
                    .get(start..cursor)
                    .map_or_else(String::new, str::to_owned),
                start,
                end: cursor,
            });
        } else {
            let start = cursor;
            let width = source
                .get(cursor..)
                .and_then(|tail| tail.chars().next())
                .map_or(1, char::len_utf8);
            cursor = cursor.saturating_add(width);
            out.push(Token {
                text: source
                    .get(start..cursor)
                    .map_or_else(String::new, str::to_owned),
                start,
                end: cursor,
            });
        }
    }
    out
}

fn is_character_literal(bytes: &[u8], start: usize) -> bool {
    let content = start.saturating_add(1);
    let mut cursor;
    if bytes.get(content) == Some(&b'\\') {
        cursor = content.saturating_add(1);
        match bytes.get(cursor) {
            Some(b'u') if bytes.get(cursor.saturating_add(1)) == Some(&b'{') => {
                cursor = cursor.saturating_add(2);
                while cursor < bytes.len() && bytes.get(cursor) != Some(&b'}') {
                    cursor = cursor.saturating_add(1);
                }
                cursor = cursor.saturating_add(1);
            }
            Some(b'x') => cursor = cursor.saturating_add(3),
            Some(b'n' | b'r' | b't' | b'0' | b'\\' | b'\'' | b'"') => {
                cursor = cursor.saturating_add(1);
            }
            _ => return false,
        }
    } else {
        let width = bytes.get(content).map_or(0, |byte| match byte {
            0x00..=0x7f => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => 0,
        });
        cursor = content.saturating_add(width);
    }
    bytes.get(cursor) == Some(&b'\'')
}

fn quoted_end(bytes: &[u8], start: usize, delimiter: u8) -> usize {
    let mut cursor = start.saturating_add(1);
    while cursor < bytes.len() {
        if bytes.get(cursor) == Some(&b'\\') {
            cursor = cursor.saturating_add(2);
        } else if bytes.get(cursor) == Some(&delimiter) {
            return cursor.saturating_add(1).min(bytes.len());
        } else {
            cursor = cursor.saturating_add(1);
        }
    }
    bytes.len()
}

fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start.saturating_add(2);
    let mut depth = 1usize;
    while cursor < bytes.len() && depth > 0 {
        if bytes.get(cursor..cursor.saturating_add(2)) == Some(b"/*") {
            depth = depth.saturating_add(1);
            cursor = cursor.saturating_add(2);
        } else if bytes.get(cursor..cursor.saturating_add(2)) == Some(b"*/") {
            depth = depth.saturating_sub(1);
            cursor = cursor.saturating_add(2);
        } else {
            cursor = cursor.saturating_add(1);
        }
    }
    cursor
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut quote = start;
    if bytes.get(quote) == Some(&b'b') {
        quote = quote.saturating_add(1);
    }
    if bytes.get(quote) != Some(&b'r') {
        return None;
    }
    quote = quote.saturating_add(1);
    let hashes_start = quote;
    while bytes.get(quote) == Some(&b'#') {
        quote = quote.saturating_add(1);
    }
    if bytes.get(quote) != Some(&b'"') {
        return None;
    }
    let hashes = quote.saturating_sub(hashes_start);
    let mut cursor = quote.saturating_add(1);
    while cursor < bytes.len() {
        if bytes.get(cursor) == Some(&b'"')
            && bytes
                .get(cursor.saturating_add(1)..cursor.saturating_add(1usize.saturating_add(hashes)))
                == bytes.get(hashes_start..hashes_start.saturating_add(hashes))
        {
            return Some(cursor.saturating_add(1 + hashes).min(bytes.len()));
        }
        cursor = cursor.saturating_add(1);
    }
    Some(bytes.len())
}
