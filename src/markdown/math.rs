//! 行内公式 `$…$` 换成 Unicode 写法（设计 36 第四节第 4 条，照旧版终端的退路 `render/math/unicode.rs`）：排字库没法在一行字中间
//! 插一张图，`\alpha` 换成 `α`，上下标换成上下标的字（没有对应的写成 `^(…)`），`\frac{a}{b}` 换成 `a/b`（不是一个字的加括号），
//! `\sqrt{x}` 换成 `√x`。独占一行的 `$$…$$` 画成图（`render::formula`）。
//!
//! 公式是模型写的：嵌套深过 [`MAX_NESTING`] 的照原文（递归没有上限就是一个爆栈的口子）。

mod symbols;

use symbols::{SUBSCRIPTS, SUPERSCRIPTS, symbol};

/// 嵌套最多几层：括号和命令取参数都算一层（`\sqrt\sqrt x` 两层，审计第四轮 R-4：原来只数 `{`）。
pub const MAX_NESTING: usize = 64;

/// 换成 Unicode 写法；嵌套太深的照原文。多的空白并成一个。
pub fn unicode(tex: &str) -> String {
    let chars: Vec<char> = tex.chars().collect();
    let mut reader = Reader {
        chars: &chars,
        cursor: 0,
        level: 0,
        too_deep: false,
    };
    let out = reader.sequence(None);
    if reader.too_deep {
        return tex.to_string();
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 读到哪了、递归到第几层；深过 [`MAX_NESTING`] 的记下、不再往下读。
struct Reader<'a> {
    chars: &'a [char],
    cursor: usize,
    level: usize,
    too_deep: bool,
}

impl Reader<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.cursor).copied()
    }

    /// 往下一层：深过上限的交回 `false`，整个公式照原文。
    fn enter(&mut self) -> bool {
        self.level += 1;
        if self.level > MAX_NESTING {
            self.too_deep = true;
        }
        !self.too_deep
    }

    fn leave(&mut self) {
        self.level -= 1;
    }

    /// 一个组 `{…}` 的里面：`{` 已经读过了。
    fn braced(&mut self) -> String {
        let out = if self.enter() {
            self.sequence(Some('}'))
        } else {
            String::new()
        };
        self.leave();
        out
    }

    /// 一串，读到 `stop`（组的 `}`）为止。
    fn sequence(&mut self, stop: Option<char>) -> String {
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if self.too_deep {
                break;
            }
            self.cursor += 1;
            if Some(ch) == stop {
                return out;
            }
            match ch {
                '\\' => out.push_str(&self.command()),
                '^' => out.push_str(&script(&self.group(), SUPERSCRIPTS, '^')),
                '_' => out.push_str(&script(&self.group(), SUBSCRIPTS, '_')),
                '{' => out.push_str(&self.braced()),
                '\'' => out.push('′'),
                '~' => out.push(' '),
                _ => out.push(ch),
            }
        }
        out
    }

    /// 一个参数：`{…}`、一个命令或者一个字。
    fn group(&mut self) -> String {
        let Some(ch) = self.peek() else {
            return String::new();
        };
        self.cursor += 1;
        match ch {
            '{' => self.braced(),
            '\\' => self.command(),
            _ => ch.to_string(),
        }
    }

    /// 反斜杠后面的一个命令：取参数的往下一层。
    fn command(&mut self) -> String {
        let out = if self.enter() {
            self.named()
        } else {
            String::new()
        };
        self.leave();
        out
    }

    fn named(&mut self) -> String {
        if let Some(ch) = self.peek()
            && !ch.is_ascii_alphabetic()
        {
            self.cursor += 1;
            return if matches!(ch, ',' | ';' | ':' | ' ' | '!') {
                " ".to_string()
            } else {
                ch.to_string()
            };
        }
        let start = self.cursor;
        while self.peek().is_some_and(|ch| ch.is_ascii_alphabetic()) {
            self.cursor += 1;
        }
        let name: String = self.chars[start..self.cursor].iter().collect();
        // 命令后面的一个空格是 TeX 的分隔，吃掉。
        if self.peek() == Some(' ') {
            self.cursor += 1;
        }
        match name.as_str() {
            "frac" | "dfrac" | "tfrac" => {
                let top = self.group();
                let bottom = self.group();
                format!("{}/{}", wrap(&top), wrap(&bottom))
            }
            "sqrt" => format!("√{}", wrap(&self.group())),
            "operatorname" | "text" | "mathrm" | "mathbf" | "mathit" | "textbf" | "textit"
            | "mathsf" | "mathcal" => self.group(),
            "binom" | "dbinom" | "tbinom" => {
                let upper = self.group();
                let lower = self.group();
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
                format!("{}{mark}", self.group())
            }
            "left" | "right" | "big" | "Big" | "bigg" | "Bigg" | "displaystyle" | "textstyle"
            | "limits" | "nolimits" => String::new(),
            "quad" | "qquad" => " ".to_string(),
            other => symbol(other).map_or_else(|| format!("\\{other}"), str::to_string),
        }
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
