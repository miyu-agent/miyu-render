//! 一段字里每个字用哪一类字体（设计 36 第四节第 3 条）：
//!
//! - 代码里的中日韩字用正文的字体：等宽字体里的中文一个字占两格，看着字距拉开（网页 2026-09-30 同一条，字的范围同 `base.css`）。
//! - 表情用彩色表情字体，常规粗细、不斜：不交出去的话，粗体的标题里会挑到黑白的表情字体（R-2 下实测）。范围照旧版
//!   `grapheme_is_emoji`；零宽连接符、变体选择符、组合用的符号跟着前面的表情走。

use crate::markdown::{Span, Style};

/// 一个字用哪一类字体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    /// 照这一段的样子：代码用等宽，别的用正文。
    Plain,
    /// 代码里的中日韩字：用正文的字体。
    Cjk,
    /// 表情：用彩色表情字体。
    Emoji,
}

/// 把几段字照「样子、字体类别」切开：交回字、样子、类别，相邻同样的并在一起。`code_block` 是代码块（整段都是代码）。
pub fn split(spans: &[Span], code_block: bool) -> Vec<(String, Style, Script)> {
    let mut pieces: Vec<(String, Style, Script)> = Vec::new();
    for span in spans {
        let code = code_block || span.style.code;
        for ch in span.text.chars() {
            let previous = pieces.last().map(|(_, _, script)| *script);
            let script = if is_emoji(ch) || (joins(ch) && previous == Some(Script::Emoji)) {
                Script::Emoji
            } else if code && is_cjk(ch) {
                Script::Cjk
            } else {
                Script::Plain
            };
            match pieces.last_mut() {
                Some((text, style, last)) if *style == span.style && *last == script => {
                    text.push(ch);
                }
                _ => pieces.push((ch.to_string(), span.style, script)),
            }
        }
    }
    pieces
}

/// 中日韩的字（同网页 `base.css` 的 `unicode-range`）。
fn is_cjk(ch: char) -> bool {
    matches!(
        u32::from(ch),
        0x2E80..=0x2FDF | 0x3000..=0x303F | 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFFEF
    )
}

/// 表情（同旧版的范围）。
fn is_emoji(ch: char) -> bool {
    matches!(u32::from(ch), 0x1F000..=0x1FAFF | 0x2300..=0x23FF | 0x2600..=0x27BF | 0x2B00..=0x2BFF)
}

/// 跟着前面的表情走的：零宽连接符、变体选择符、组合用的方框、旗子的标签。
fn joins(ch: char) -> bool {
    matches!(u32::from(ch), 0x200D | 0xFE0F | 0x20E3 | 0xE0020..=0xE007F)
}

#[cfg(test)]
mod tests;
