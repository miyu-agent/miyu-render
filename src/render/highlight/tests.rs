//! 切词：认得的语言标出关键字、字符串、数、函数名、注释，字连起来和原文一样；认不出的语言交回空的。

use super::*;

/// 每一段的字和类别，去掉只有空白的段。
fn pieces(spans: &[Span]) -> Vec<(String, Option<Token>)> {
    spans
        .iter()
        .filter(|span| !span.text.trim().is_empty())
        .map(|span| (span.text.trim().to_string(), span.style.token))
        .collect()
}

#[test]
fn rust_words_get_their_kinds() {
    let source = "fn main() {\n    // 说话\n    let n = 42;\n    println!(\"你好\");\n}\n";
    let got = tokens(source, "rust").expect("认得 rust");
    assert_eq!(
        got.iter()
            .map(|span| span.text.as_str())
            .collect::<String>(),
        source,
        "字一个不少"
    );
    assert!(got.iter().all(|span| span.style.code));
    let kinds = pieces(&got);
    let find = |text: &str| {
        kinds
            .iter()
            .find(|(piece, _)| piece.contains(text))
            .map(|(_, token)| *token)
    };
    assert_eq!(find("fn"), Some(Some(Token::Keyword)));
    assert_eq!(find("main"), Some(Some(Token::Function)));
    assert_eq!(find("// 说话"), Some(Some(Token::Comment)));
    assert_eq!(find("let"), Some(Some(Token::Keyword)));
    assert_eq!(find("42"), Some(Some(Token::Constant)));
    assert_eq!(find("你好"), Some(Some(Token::String)));
}

#[test]
fn the_innermost_known_scope_wins() {
    // 字符串里的转义：外层是字符串，里层是字符常量，照里层的。
    let got = tokens("let s = \"a\\nb\";\n", "rust").expect("认得 rust");
    let kinds = pieces(&got);
    assert!(
        kinds.contains(&("\\n".to_string(), Some(Token::Constant))),
        "{kinds:?}"
    );
    assert!(
        kinds
            .iter()
            .any(|(text, token)| text.contains('a') && *token == Some(Token::String)),
        "{kinds:?}"
    );
}

#[test]
fn common_aliases_work_and_unknown_languages_do_not() {
    for language in ["py", "python", "js", "sh", "json", "c", "go"] {
        assert!(tokens("x = 1\n", language).is_some(), "{language}");
    }
    assert!(tokens("x", "not-a-language").is_none());
    assert!(
        tokens(&"a".repeat(MAX_CHARS + 1), "rust").is_none(),
        "太长的照单色"
    );
}
