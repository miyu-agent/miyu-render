//! 代码着色（设计 36 第四节第 5 条，2026-10-17 项目主人定补上）：syntect 认语言、切词（纯 Rust 的正则），每一段标一个类别
//! （`markdown::Token`），颜色在排字时照页面的配色调：关键字强调色、字符串和数和函数名是强调色转出来的颜色、注释淡字加斜体。
//! 不用 syntect 自带的主题。认不出的语言、切词出错的交回空的，照单色画。
//!
//! 语言表第一次用的时候才读（几十毫秒），之后留着。

use std::sync::OnceLock;

use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

use crate::markdown::{Span, Style, Token};

/// 一个代码块最多着色多少个字：太长的照单色画（切词是逐行跑正则，不让一大块拖慢）。
const MAX_CHARS: usize = 20_000;

/// 语言表。
fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// 把 `source` 照 `language` 切成带类别的几段（样子都是代码）；认不出、出错、太长的交回空的。
pub fn tokens(source: &str, language: &str) -> Option<Vec<Span>> {
    if source.chars().count() > MAX_CHARS {
        return None;
    }
    let set = syntaxes();
    let syntax = set.find_syntax_by_token(language)?;
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut spans: Vec<Span> = Vec::new();
    for line in source.split_inclusive('\n') {
        let ops = state.parse_line(line, set).ok()?;
        let mut at = 0;
        for (index, op) in ops {
            push(&mut spans, &line[at..index], classify(&stack));
            stack.apply(&op).ok()?;
            at = index;
        }
        push(&mut spans, &line[at..], classify(&stack));
    }
    Some(spans)
}

/// 接一段：和上一段同类的并在一起。
fn push(spans: &mut Vec<Span>, text: &str, token: Option<Token>) {
    if text.is_empty() {
        return;
    }
    match spans.last_mut() {
        Some(last) if last.style.token == token => last.text.push_str(text),
        _ => spans.push(Span {
            text: text.to_string(),
            style: Style {
                code: true,
                token,
                ..Style::default()
            },
        }),
    }
}

/// 照最里面认得的那一层作用域定类别。
fn classify(stack: &ScopeStack) -> Option<Token> {
    stack.as_slice().iter().rev().find_map(|scope| {
        let name = scope.build_string();
        let starts = |prefix: &str| name == prefix || name.starts_with(&format!("{prefix}."));
        if starts("comment") {
            Some(Token::Comment)
        } else if starts("string") {
            Some(Token::String)
        } else if starts("constant.numeric")
            || starts("constant.language")
            || starts("constant.character")
        {
            Some(Token::Constant)
        } else if starts("keyword") || starts("storage") {
            Some(Token::Keyword)
        } else if starts("entity.name.function")
            || starts("support.function")
            || starts("entity.name.type")
            || starts("support.type")
        {
            Some(Token::Function)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests;
