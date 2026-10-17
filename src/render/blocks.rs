//! Markdown 读成要画的块（设计 36 第四节）。R-1 只有段落：每一段的字连起来，软换行、硬换行都换行（旧版 0.7.0 起软换行照旧换行，
//! 人写几行就是几行）；标题、列表项、引用、代码块各自算一段，照它们的字画，样子 R-2 起补。

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// 一段一个字符串，照先后；空白的段不要。
pub fn paragraphs(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Text(text) | Event::Code(text) | Event::Html(text) | Event::InlineHtml(text) => {
                current.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => current.push('\n'),
            Event::Start(Tag::Item)
            | Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::CodeBlock,
            ) => {
                finish(&mut out, &mut current);
            }
            _ => {}
        }
    }
    finish(&mut out, &mut current);
    out
}

/// 手上的这一段收下（去掉头尾的空白行），开新的一段。
fn finish(out: &mut Vec<String>, current: &mut String) {
    let text = current.trim_matches('\n').trim_end().to_string();
    if !text.trim().is_empty() {
        out.push(text);
    }
    current.clear();
}

#[cfg(test)]
mod tests;
