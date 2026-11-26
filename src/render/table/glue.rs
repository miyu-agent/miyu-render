//! 格子里哪里不许断（施工 R-14，2026-11-26 项目主人看了一张六列的表：「启动中位数」断成「启动中 / 位数」、「分钟」断成「分 /
//! 钟」、「125.7 ms」的单位掉到下一行）。排字照 Unicode 的断行规则，中日韩的字之间、空格处都能断；这里把不该断的地方粘上：
//!
//! - 一个数和它的单位：数字后面的空格、下一个字是字母或者 `%` 的，换成不断行的空格（U+00A0），「125.7 ms」「3.6 MiB」整个当一段。
//! - 表头：连着的中日韩字之间垫一个不断行的连接符（U+2060，不占宽），表头只在空格处折：「空闲 CPU / 分钟」折成「空闲 CPU」和
//!   「/ 分钟」，不会把「分钟」劈开。一段比整列还宽的，排字时照旧逐字劈（`WordOrGlyph`），字不会挤出格子。
//!
//! 只改交给排字的字，不改读出来的表格。

use super::super::script::is_cjk;
use crate::markdown::Span;

/// 不断行的空格。
const NO_BREAK_SPACE: char = '\u{00A0}';

/// 不断行的连接符（不占宽）。
const WORD_JOINER: char = '\u{2060}';

/// 一格的几段字粘好不该断的地方。`header` 是表头的格子。
pub fn glue(spans: &[Span], header: bool) -> Vec<Span> {
    let chars: Vec<char> = spans.iter().flat_map(|span| span.text.chars()).collect();
    let mut at: usize = 0;
    spans
        .iter()
        .map(|span| {
            let mut text = String::with_capacity(span.text.len());
            for ch in span.text.chars() {
                let previous = at.checked_sub(1).and_then(|index| chars.get(index));
                let next = chars.get(at + 1);
                // 跨段（「**空闲**CPU」这类粗细换了的）也粘：连接符放在后一段的开头。
                if header && previous.is_some_and(|previous| is_cjk(*previous)) && is_cjk(ch) {
                    text.push(WORD_JOINER);
                }
                if ch == ' '
                    && previous.is_some_and(char::is_ascii_digit)
                    && next.is_some_and(|next| next.is_alphabetic() || *next == '%')
                {
                    text.push(NO_BREAK_SPACE);
                } else {
                    text.push(ch);
                }
                at += 1;
            }
            Span {
                text,
                style: span.style,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
