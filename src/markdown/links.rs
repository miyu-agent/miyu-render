//! 正文里直接贴的地址（旧版 `renderer/links.rs`、`render/link.rs`，09-23）：上链接色，读的人分得出哪里是链接。只给地址本身上色；
//! 句尾标点不算地址、配不上对的右括号不算；行内代码里的是给人抄的字，不碰；已经是链接的不重复。认法和终端、网页同一份规矩。

use super::{Span, Style};

/// 认得的协议。
const SCHEMES: [&str; 3] = ["https://", "http://", "file://"];

/// 跟在地址后面的句尾标点：一律不算地址。
const TAIL_TRIM: &str = "。，、；：！？…～\"'`,.;:!?’”»›|*_~";

/// 给 `spans` 里的裸地址打上链接色。
pub fn mark(spans: &mut Vec<Span>) {
    if !spans.iter().any(|span| span.text.contains("://")) {
        return;
    }
    let mut chars = Vec::new();
    let mut styles = Vec::new();
    for span in spans.iter() {
        for ch in span.text.chars() {
            chars.push(ch);
            styles.push(span.style);
        }
    }
    let mut index = 0;
    while index < chars.len() {
        if styles[index].link {
            index += 1;
            continue;
        }
        let Some(url) = bare_url_at(&chars, index) else {
            index += 1;
            continue;
        };
        let end = index + url.chars().count();
        if styles[index..end].iter().any(|style| style.code) {
            index += 1;
            continue;
        }
        for style in &mut styles[index..end] {
            style.link = true;
        }
        index = end;
    }
    *spans = regroup(&chars, &styles);
}

/// `chars[index]` 是不是一个裸地址的开头：是的交回地址原文。前一个字是字母、数字的不认（`xhttps://`）；光有协议头的不算。
pub fn bare_url_at(chars: &[char], index: usize) -> Option<String> {
    if index > 0 && chars[index - 1].is_ascii_alphanumeric() {
        return None;
    }
    let width = scheme_at(chars, index)?;
    let mut end = index + width;
    while end < chars.len() && is_url_body(chars[end]) {
        end += 1;
    }
    let raw: String = chars[index..end].iter().collect();
    let trimmed = trim_tail(&raw);
    (trimmed.chars().count() > width).then_some(trimmed)
}

fn scheme_at(chars: &[char], index: usize) -> Option<usize> {
    SCHEMES.iter().find_map(|scheme| {
        let end = index.checked_add(scheme.len())?;
        let head = chars.get(index..end)?;
        head.iter()
            .zip(scheme.chars())
            .all(|(left, right)| left.eq_ignore_ascii_case(&right))
            .then_some(scheme.len())
    })
}

/// 地址正文能有的字：空白、引号、尖括号不行，中日韩的标点一个都不行（「…archlinux.org、AUR」的顿号后面还跟着字母）。
fn is_url_body(ch: char) -> bool {
    if ch.is_whitespace() || matches!(ch, '<' | '>' | '"' | '\'' | '`' | '\u{00a0}') {
        return false;
    }
    !matches!(ch as u32, 0x2000..=0x206f | 0x3000..=0x303f | 0xff00..=0xffef)
}

/// 成对的收尾符号：在地址里配不上开头的才算句读（维基、GitHub 的地址自己带括号）。
fn opener(ch: char) -> Option<char> {
    match ch {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        '》' => Some('《'),
        '」' => Some('「'),
        '』' => Some('『'),
        '】' => Some('【'),
        _ => None,
    }
}

fn trim_tail(raw: &str) -> String {
    let mut value: Vec<char> = raw.chars().collect();
    while let Some(&last) = value.last() {
        if let Some(open) = opener(last) {
            let opens = value.iter().filter(|ch| **ch == open).count();
            let closes = value.iter().filter(|ch| **ch == last).count();
            if closes <= opens {
                break;
            }
        } else if !TAIL_TRIM.contains(last) {
            break;
        }
        value.pop();
    }
    value.into_iter().collect()
}

fn regroup(chars: &[char], styles: &[Style]) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    for (ch, style) in chars.iter().zip(styles) {
        match spans.last_mut() {
            Some(last) if last.style == *style => last.text.push(*ch),
            _ => spans.push(Span {
                text: ch.to_string(),
                style: *style,
            }),
        }
    }
    spans
}

#[cfg(test)]
mod tests;
