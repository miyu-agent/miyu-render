//! Markdown 读成要画的块（设计 36 第四节）。读法照旧版 `renderer/markdown.rs` 推（它踩过的坑都留着）：
//!
//! - 段落里的单个换行照样换行：终端、网页、QQ 的短回复都是一行一行显示的，转成图不该挤成一坨（旧版 09-23）。
//! - `[标题](地址)`、`![说明](地址)`：画成图以后地址点不了，标题后面补 ` (地址)`，只有地址上链接色；标题本身就是地址的不补
//!   （[`collect`]）。正文里直接贴的地址也上链接色，行内代码里的不碰（[`links`]）。
//! - 夹着的 HTML：`<br>` 换行，`<b>`、`<i>`、`<s>`、`<code>` 换成样式，别的标签只留字，注释不要（[`html`]）。
//! - 删除线画成淡字（旧版同）。表头一律加粗。
//!
//! 只读出要画什么，怎么画在 `render` 里。

mod collect;
mod html;
mod links;

/// 一块是什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 段落。
    Paragraph,
    /// 标题，1 到 6 级。
    Heading(u8),
    /// 列表的一项，`depth` 从 1 数；开头的记号（`• `、`3. `）已经写进字里。
    Item {
        /// 第几层。
        depth: u8,
    },
    /// 引用。
    Quote,
    /// 代码块：字原样。
    Code,
    /// 表格：字在 [`Block::table`] 里。
    Table,
    /// 分割线。
    Rule,
}

/// 一段字的样子。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "几样各自开关的标记，同 Markdown 的写法"
)]
pub struct Style {
    /// 粗体。
    pub bold: bool,
    /// 斜体。
    pub italic: bool,
    /// 行内代码、代码块：等宽字体。
    pub code: bool,
    /// 链接（只有地址那一截）。
    pub link: bool,
    /// 淡字：删除线。
    pub muted: bool,
}

/// 一段同样子的字。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// 字。
    pub text: String,
    /// 样子。
    pub style: Style,
}

/// 表格。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    /// 表头，一格一串。
    pub header: Vec<Vec<Span>>,
    /// 正文，一行一串格子。
    pub rows: Vec<Vec<Vec<Span>>>,
    /// 每一列怎么对齐。
    pub aligns: Vec<Align>,
}

/// 表格一列怎么对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// 没写：靠左。
    #[default]
    None,
    /// 靠左。
    Left,
    /// 居中。
    Center,
    /// 靠右。
    Right,
}

/// 一块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// 是什么。
    pub kind: Kind,
    /// 字：表格的在 `table` 里，这里是空的。
    pub spans: Vec<Span>,
    /// 表格。
    pub table: Option<Table>,
    /// 任务列表的一项：做没做完。
    pub task: Option<bool>,
}

impl Block {
    fn new(kind: Kind) -> Block {
        Block {
            kind,
            spans: Vec::new(),
            table: None,
            task: None,
        }
    }

    /// 接一段字：和上一段样子一样的并成一段。
    fn push(&mut self, text: &str, style: Style) {
        match self.spans.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            _ => self.spans.push(Span {
                text: text.to_string(),
                style,
            }),
        }
    }

    /// 有没有要画的：分割线、有字的表格、不全是空白的字，任务项光有勾选框也算。
    fn has_content(&self) -> bool {
        match self.kind {
            Kind::Rule => true,
            Kind::Table => self
                .table
                .as_ref()
                .is_some_and(|table| !table.header.is_empty() || !table.rows.is_empty()),
            _ => self.task.is_some() || self.spans.iter().any(|span| !span.text.trim().is_empty()),
        }
    }
}

/// 读 `markdown`：交回要画的块，照先后。
pub fn blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = collect::collect(markdown);
    for block in &mut blocks {
        if block.kind == Kind::Code {
            continue;
        }
        links::mark(&mut block.spans);
        if let Some(table) = block.table.as_mut() {
            for cell in table
                .header
                .iter_mut()
                .chain(table.rows.iter_mut().flatten())
            {
                links::mark(cell);
            }
        }
    }
    blocks
}

#[cfg(test)]
mod tests;
