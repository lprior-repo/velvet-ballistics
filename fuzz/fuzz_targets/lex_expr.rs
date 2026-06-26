#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let result = vb_compile::lexer::lex_expr(text);
    #[allow(clippy::let_underscore_must_use)]
    let _ = result
        .map(|tokens| {
            for token in tokens {
                let _ = token;
            }
        })
        .ok();
});
