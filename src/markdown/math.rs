//! 行内公式 `$…$` 换成 Unicode 写法（设计 36 第四节第 4 条，照旧版终端的退路 `render/math/unicode.rs`）：排字库没法在一行字中间
//! 插一张图，`\alpha` 换成 `α`，上下标换成上下标的字（没有对应的写成 `^(…)`），`\frac{a}{b}` 换成 `a/b`（不是一个字的加括号），
//! `\sqrt{x}` 换成 `√x`。独占一行的 `$$…$$` 画成图（`render::formula`）。
//!
//! 公式是模型写的：嵌套深过 [`MAX_NESTING`] 的照原文（递归没有上限就是一个爆栈的口子）。

mod symbols;

use symbols::{SUBSCRIPTS, SUPERSCRIPTS, symbol};

/// 括号嵌套最多几层。
pub const MAX_NESTING: usize = 64;

/// 换成 Unicode 写法；嵌套太深的照原文。多的空白并成一个。
pub fn unicode(tex: &str) -> String {
    let chars: Vec<char> = tex.chars().collect();
    if depth(&chars) > MAX_NESTING {
        return tex.to_string();
    }
    let mut cursor = 0;
    sequence(&chars, &mut cursor, None)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// 裸 `{` 最深几层（`\{` 不算）。
fn depth(chars: &[char]) -> usize {
    let (mut now, mut most, mut escaped) = (0usize, 0usize, false);
    for ch in chars {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '{' => {
                now += 1;
                most = most.max(now);
            }
            '}' => now = now.saturating_sub(1),
            _ => {}
        }
    }
    most
}

/// 一串，读到 `stop`（组的 `}`）为止。
fn sequence(chars: &[char], cursor: &mut usize, stop: Option<char>) -> String {
    let mut out = String::new();
    while let Some(&ch) = chars.get(*cursor) {
        *cursor += 1;
        if Some(ch) == stop {
            return out;
        }
        match ch {
            '\\' => out.push_str(&command(chars, cursor)),
            '^' => out.push_str(&script(&group(chars, cursor), SUPERSCRIPTS, '^')),
            '_' => out.push_str(&script(&group(chars, cursor), SUBSCRIPTS, '_')),
            '{' => out.push_str(&sequence(chars, cursor, Some('}'))),
            '\'' => out.push('′'),
            '~' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}

/// 一个参数：`{…}`、一个命令或者一个字。
fn group(chars: &[char], cursor: &mut usize) -> String {
    let Some(&ch) = chars.get(*cursor) else {
        return String::new();
    };
    *cursor += 1;
    match ch {
        '{' => sequence(chars, cursor, Some('}')),
        '\\' => command(chars, cursor),
        _ => ch.to_string(),
    }
}

/// 反斜杠后面的一个命令。
fn command(chars: &[char], cursor: &mut usize) -> String {
    if let Some(&ch) = chars.get(*cursor)
        && !ch.is_ascii_alphabetic()
    {
        *cursor += 1;
        return if matches!(ch, ',' | ';' | ':' | ' ' | '!') {
            " ".to_string()
        } else {
            ch.to_string()
        };
    }
    let start = *cursor;
    while chars.get(*cursor).is_some_and(char::is_ascii_alphabetic) {
        *cursor += 1;
    }
    let name: String = chars[start..*cursor].iter().collect();
    // 命令后面的一个空格是 TeX 的分隔，吃掉。
    if chars.get(*cursor) == Some(&' ') {
        *cursor += 1;
    }
    match name.as_str() {
        "frac" | "dfrac" | "tfrac" => {
            let top = group(chars, cursor);
            let bottom = group(chars, cursor);
            format!("{}/{}", wrap(&top), wrap(&bottom))
        }
        "sqrt" => format!("√{}", wrap(&group(chars, cursor))),
        "operatorname" | "text" | "mathrm" | "mathbf" | "mathit" | "textbf" | "textit"
        | "mathsf" | "mathcal" => group(chars, cursor),
        "binom" | "dbinom" | "tbinom" => {
            let upper = group(chars, cursor);
            let lower = group(chars, cursor);
            format!("C({},{})", upper.trim(), lower.trim())
        }
        "hat" | "bar" | "overline" | "vec" | "tilde" | "dot" | "ddot" | "check" | "breve" => {
            let mark = match name.as_str() {
                "hat" => '\u{0302}',
                "bar" | "overline" => '\u{0304}',
                "vec" => '\u{20d7}',
                "tilde" => '\u{0303}',
                "dot" => '\u{0307}',
                "ddot" => '\u{0308}',
                "check" => '\u{030c}',
                _ => '\u{0306}',
            };
            format!("{}{mark}", group(chars, cursor))
        }
        "left" | "right" | "big" | "Big" | "bigg" | "Bigg" | "displaystyle" | "textstyle"
        | "limits" | "nolimits" => String::new(),
        "quad" | "qquad" => " ".to_string(),
        other => symbol(other).map_or_else(|| format!("\\{other}"), str::to_string),
    }
}

/// 上下标：每个字都有对应的换成上下标的字，有一个没有的整段写成 `^(…)`、`_(…)`。
fn script(content: &str, table: &[(char, char)], marker: char) -> String {
    let trimmed = content.trim();
    let mut out = String::new();
    for ch in trimmed.chars() {
        match table.iter().find(|(from, _)| *from == ch) {
            Some((_, to)) => out.push(*to),
            None => return format!("{marker}{}", wrap(trimmed)),
        }
    }
    out
}

/// 不是一个字、一串字母数字、本来就整个括着的，加括号（`\frac{a+1}{2}` 换成 `(a+1)/2`，优先级才对）。
fn wrap(text: &str) -> String {
    let trimmed = text.trim();
    let simple = trimmed.chars().count() <= 1
        || trimmed
            .chars()
            .all(|ch| ch.is_alphanumeric() || ch == '.' || ch == '′')
        || enclosed(trimmed);
    if simple {
        trimmed.to_string()
    } else {
        format!("({trimmed})")
    }
}

/// 整个被同一对括号括着（`(a)+(b)` 不算：头一个括号中途就关了）。
fn enclosed(text: &str) -> bool {
    if !text.starts_with('(') || !text.ends_with(')') {
        return false;
    }
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return index + 1 == text.len();
                }
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests;
