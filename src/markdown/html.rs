//! Markdown 里夹着的 HTML（旧版 `renderer/html.rs`，09-24）：模型爱在表格格子里写 `<br>` 换行。不是浏览器，只认几样：`<br>`
//! 换行，`<b>`、`<strong>`、`<i>`、`<em>`、`<s>`、`<del>`、`<strike>`、`<code>` 换成样子，别的标签只去掉记号、留下字，注释整段
//! 不要；`a < b` 这种不像标签的照字。代码里的尖括号不走这里（pulldown-cmark 交成代码）。

use std::borrow::Cow;

/// 换成样子的几种标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Strike,
    Code,
}

/// 切出来的一段。
#[derive(Debug, PartialEq, Eq)]
pub enum Piece<'a> {
    Text(Cow<'a, str>),
    LineBreak,
    Open(Format),
    Close(Format),
}

/// 标签开了几层：和 Markdown 的 `**`、`*` 分开记，块和格子收尾时清零。
#[derive(Debug, Clone, Copy, Default)]
pub struct Depths {
    pub bold: usize,
    pub italic: usize,
    pub strike: usize,
    pub code: usize,
}

impl Depths {
    /// 开（`open`）或关一层 `format`。
    pub fn apply(&mut self, format: Format, open: bool) {
        let depth = match format {
            Format::Bold => &mut self.bold,
            Format::Italic => &mut self.italic,
            Format::Strike => &mut self.strike,
            Format::Code => &mut self.code,
        };
        *depth = if open {
            *depth + 1
        } else {
            depth.saturating_sub(1)
        };
    }
}

/// 一段 HTML 原文切成字和标签。`in_comment` 由调的一方跨行留着（注释可以跨几行）。
pub fn scan<'a>(html: &'a str, in_comment: &mut bool) -> Vec<Piece<'a>> {
    let mut pieces = Vec::new();
    let mut rest = html;
    loop {
        if *in_comment {
            let Some(end) = rest.find("-->") else {
                return pieces;
            };
            rest = &rest[end + 3..];
            *in_comment = false;
        }
        let Some(open) = rest.find('<') else {
            push_text(&mut pieces, rest);
            return pieces;
        };
        push_text(&mut pieces, &rest[..open]);
        let tail = &rest[open..];
        if let Some(comment) = tail.strip_prefix("<!--") {
            *in_comment = true;
            rest = comment;
            continue;
        }
        let found = tail.find('>').map(|close| (close, tag(&tail[1..close])));
        if let Some((close, read)) = found.filter(|(_, read)| *read != Tag::Text) {
            if let Tag::Piece(piece) = read {
                pieces.push(piece);
            }
            rest = &tail[close + 1..];
        } else {
            push_text(&mut pieces, "<");
            rest = &tail[1..];
        }
    }
}

/// `<` 和 `>` 之间读出来的。
#[derive(Debug, PartialEq, Eq)]
enum Tag {
    /// 不像标签：照字。
    Text,
    /// 是标签、不认得：去掉记号，什么都不画。
    Ignored,
    /// 认得的。
    Piece(Piece<'static>),
}

/// `<` 和 `>` 之间是什么。
fn tag(inner: &str) -> Tag {
    let (closing, body) = match inner.strip_prefix('/') {
        Some(body) => (true, body),
        None => (false, inner),
    };
    let name_end = body
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(body.len());
    let name = &body[..name_end];
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Tag::Text;
    }
    let after = body[name_end..].trim_end_matches('/');
    if !(after.is_empty() || after.starts_with(char::is_whitespace)) {
        return Tag::Text;
    }
    let format = match name.to_ascii_lowercase().as_str() {
        // `</br>` 不合规范，可模型真会这么写，浏览器也当换行。
        "br" => return Tag::Piece(Piece::LineBreak),
        "b" | "strong" => Format::Bold,
        "i" | "em" => Format::Italic,
        "s" | "del" | "strike" => Format::Strike,
        "code" => Format::Code,
        _ => return Tag::Ignored,
    };
    Tag::Piece(if closing {
        Piece::Close(format)
    } else {
        Piece::Open(format)
    })
}

fn push_text<'a>(pieces: &mut Vec<Piece<'a>>, text: &'a str) {
    if !text.is_empty() {
        pieces.push(Piece::Text(entities(text)));
    }
}

/// HTML 块里的字 pulldown-cmark 不解码实体：补上常见的几种，认不出的原样。
fn entities(text: &str) -> Cow<'_, str> {
    if !text.contains('&') {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let decoded = tail.find(';').filter(|end| *end <= 10).and_then(|end| {
            let ch = match &tail[1..end] {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" | "#39" => Some('\''),
                "nbsp" => Some(' '),
                _ => None,
            }?;
            Some((ch, end))
        });
        if let Some((ch, end)) = decoded {
            out.push(ch);
            rest = &tail[end + 1..];
        } else {
            out.push('&');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests;
