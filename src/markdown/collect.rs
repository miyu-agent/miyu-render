//! 照 pulldown-cmark 的事件一块一块收（旧版 `MarkdownCollector`）。列表项的记号写进字里（`• `、`3. `，同一项的第二段缩进四个
//! 空格）；任务项去掉 `• `、记下做没做完；引用、列表可以套几层，只记最里面那一种；独占一行的公式自成一块，行内的换成 Unicode 写法（R-4）。

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::html::{self, Piece};
use super::{Align, Block, Kind, Span, Style, Table};

/// 读 `markdown`，交回块（还没认裸地址）。
pub fn collect(markdown: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH;
    let mut collector = Collector::default();
    for event in Parser::new_ext(markdown, options) {
        collector.event(event);
    }
    collector.finish_current();
    collector.blocks
}

/// 一层列表。
struct List {
    /// 有序的下一个号；无序的是空的。
    next: Option<u64>,
    /// 正在这一层的某一项里。
    in_item: bool,
    /// 这一项的记号写过了（同一项的第二段缩进，不再写记号）。
    marked: bool,
}

/// 正在收的表格。
#[derive(Default)]
struct Building {
    table: Table,
    row: Vec<Vec<Span>>,
    cell: Vec<Span>,
}

#[derive(Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "读的时候正在哪：代码块、表头、注释、HTML 块各是一个标记"
)]
struct Collector {
    blocks: Vec<Block>,
    current: Option<Block>,
    lists: Vec<List>,
    quotes: usize,
    heading: Option<u8>,
    code: bool,
    table: Option<Building>,
    header: bool,
    strong: usize,
    emphasis: usize,
    strike: usize,
    /// 没收口的链接：地址和到这一刻为止看得见的字（可以套：图在链接里）。
    links: Vec<(String, String)>,
    html: html::Depths,
    /// 上一段 HTML 停在注释中间（注释可以跨行）。
    comment: bool,
    /// 这个 HTML 块已经有字了：下一行有字先换行。
    html_block_text: bool,
    /// `<br>` 欠下的换行：等下一段字来了才补（格子、段落末尾的 `<br>` 不多出空行）。
    breaks: usize,
}

impl Collector {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => {
                // 链接里的字只有「看得见的字本身就是地址」（`<https://x>`）才上链接色。
                let mut style = self.style();
                style.link = self
                    .links
                    .last()
                    .is_some_and(|(url, _)| same_target(url.trim(), text.trim()));
                self.push_text(&text, style);
            }
            Event::Code(text) => {
                let style = Style {
                    code: true,
                    ..self.style()
                };
                self.push_text(&text, style);
            }
            Event::InlineMath(text) => self.push_text(&super::math::unicode(&text), self.style()),
            Event::DisplayMath(text) => {
                // 独占一行的公式自成一块，后面的字接着开新的一段。
                self.finish_current();
                let mut block = Block::new(Kind::Formula);
                block.push(text.trim(), Style::default());
                if block.has_content() {
                    self.blocks.push(block);
                }
            }
            Event::SoftBreak | Event::HardBreak if self.breaks > 0 => {}
            Event::SoftBreak | Event::HardBreak => self.push_text("\n", self.style()),
            Event::Rule => {
                self.finish_current();
                self.blocks.push(Block::new(Kind::Rule));
            }
            Event::TaskListMarker(done) => self.mark_task(done),
            Event::FootnoteReference(label) => self.push_text(&format!("[{label}]"), self.style()),
            Event::Html(text) => {
                for line in text.lines() {
                    self.html_line(line.trim(), true);
                }
            }
            Event::InlineHtml(text) => self.html_line(&text, false),
        }
    }

    fn html_line(&mut self, raw: &str, block: bool) {
        let mut line_has_text = false;
        for piece in html::scan(raw, &mut self.comment) {
            match piece {
                Piece::Text(text) => {
                    if block && !line_has_text {
                        if self.html_block_text {
                            self.push_text("\n", self.style());
                        }
                        self.html_block_text = true;
                        line_has_text = true;
                    }
                    self.push_text(&text, self.style());
                }
                Piece::LineBreak => self.breaks += 1,
                Piece::Open(format) => self.html.apply(format, true),
                Piece::Close(format) => self.html.apply(format, false),
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.ensure_current(),
            Tag::Heading { level, .. } => {
                self.finish_current();
                let level = level as u8;
                self.heading = Some(level);
                self.current = Some(Block::new(Kind::Heading(level)));
            }
            Tag::BlockQuote(_) => {
                self.finish_current();
                self.quotes += 1;
            }
            Tag::CodeBlock(kind) => {
                self.finish_current();
                self.code = true;
                let language = match &kind {
                    CodeBlockKind::Fenced(language) => {
                        language.split_whitespace().next().map(str::to_string)
                    }
                    CodeBlockKind::Indented => None,
                };
                let diagram = language.as_deref().is_some_and(is_mermaid);
                let mut block = Block::new(if diagram { Kind::Diagram } else { Kind::Code });
                block.language = language;
                self.current = Some(block);
            }
            Tag::List(start) => {
                self.finish_current();
                self.lists.push(List {
                    next: start,
                    in_item: false,
                    marked: false,
                });
            }
            Tag::Item => {
                self.finish_current();
                if let Some(list) = self.lists.last_mut() {
                    list.in_item = true;
                    list.marked = false;
                }
                self.ensure_current();
            }
            Tag::Table(aligns) => {
                self.finish_current();
                let aligns = aligns.into_iter().map(align).collect();
                self.table = Some(Building {
                    table: Table {
                        aligns,
                        ..Table::default()
                    },
                    ..Building::default()
                });
                self.header = false;
            }
            Tag::TableHead => self.header = true,
            Tag::Strong => self.strong += 1,
            Tag::Emphasis => self.emphasis += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                self.links.push((dest_url.to_string(), String::new()));
            }
            Tag::HtmlBlock => {
                self.finish_current();
                self.html_block_text = false;
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph if !self.code && self.table.is_none() && self.heading.is_none() => {
                self.finish_current();
            }
            TagEnd::Heading(_) => {
                self.finish_current();
                self.heading = None;
            }
            TagEnd::BlockQuote(_) => {
                self.finish_current();
                self.quotes = self.quotes.saturating_sub(1);
            }
            TagEnd::CodeBlock => {
                self.finish_current();
                self.code = false;
            }
            TagEnd::List(_) => {
                self.finish_current();
                self.lists.pop();
            }
            TagEnd::Item => {
                self.finish_current();
                if let Some(list) = self.lists.last_mut() {
                    list.in_item = false;
                }
            }
            TagEnd::Table => {
                if let Some(building) = self.table.take() {
                    let mut block = Block::new(Kind::Table);
                    block.table = Some(building.table);
                    if block.has_content() {
                        self.blocks.push(block);
                    }
                }
                self.header = false;
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(building) = self.table.as_mut() {
                    let row = std::mem::take(&mut building.row);
                    if self.header {
                        building.table.header = row;
                    } else {
                        building.table.rows.push(row);
                    }
                }
                self.header = false;
            }
            TagEnd::TableCell => {
                if let Some(building) = self.table.as_mut() {
                    let cell = std::mem::take(&mut building.cell);
                    building.row.push(cell);
                }
                self.html = html::Depths::default();
                self.breaks = 0;
            }
            TagEnd::HtmlBlock => {
                self.finish_current();
                self.comment = false;
            }
            TagEnd::Strong => self.strong = self.strong.saturating_sub(1),
            TagEnd::Emphasis => self.emphasis = self.emphasis.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link | TagEnd::Image => {
                let (url, shown) = self.links.pop().unwrap_or_default();
                if let Some(lead) = suffix_lead(&url, &shown) {
                    let plain = self.style();
                    self.push_text(lead, plain);
                    self.push_text(
                        url.trim(),
                        Style {
                            link: true,
                            ..plain
                        },
                    );
                    self.push_text(")", plain);
                }
            }
            _ => {}
        }
    }

    /// 没有正在收的块就照这一刻在哪开一块：代码、标题、列表项（写上记号）、引用、段落；表格里不开（字进格子）。
    fn ensure_current(&mut self) {
        if self.current.is_some() || self.table.is_some() {
            return;
        }
        if self.code {
            self.current = Some(Block::new(Kind::Code));
            return;
        }
        if let Some(level) = self.heading {
            self.current = Some(Block::new(Kind::Heading(level)));
            return;
        }
        if let Some(index) = self.lists.iter().rposition(|list| list.in_item) {
            let depth = u8::try_from(index + 1).unwrap_or(u8::MAX);
            let list = &mut self.lists[index];
            let mark = if list.marked {
                "    ".to_string()
            } else {
                list.marked = true;
                match list.next.as_mut() {
                    Some(next) => {
                        let number = *next;
                        *next += 1;
                        format!("{number}. ")
                    }
                    None => "• ".to_string(),
                }
            };
            let mut block = Block::new(Kind::Item { depth });
            block.push(&mark, Style::default());
            self.current = Some(block);
        } else if self.quotes > 0 {
            self.current = Some(Block::new(Kind::Quote));
        } else {
            self.current = Some(Block::new(Kind::Paragraph));
        }
    }

    fn push_text(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        if self.breaks > 0 {
            let breaks = "\n".repeat(std::mem::take(&mut self.breaks));
            self.push_raw(&breaks, style);
        }
        self.push_raw(text, style);
    }

    fn push_raw(&mut self, text: &str, style: Style) {
        if let Some((_, shown)) = self.links.last_mut() {
            shown.push_str(text);
        }
        if let Some(building) = self.table.as_mut() {
            match building.cell.last_mut() {
                Some(last) if last.style == style => last.text.push_str(text),
                _ => building.cell.push(Span {
                    text: text.to_string(),
                    style,
                }),
            }
            return;
        }
        self.ensure_current();
        if let Some(block) = self.current.as_mut() {
            block.push(text, style);
        }
    }

    /// 任务项：去掉开头的 `• `，记下做没做完（画成勾选框）。
    fn mark_task(&mut self, done: bool) {
        self.ensure_current();
        let Some(block) = self.current.as_mut() else {
            return;
        };
        if let Some(first) = block.spans.first_mut()
            && let Some(rest) = first.text.strip_prefix("• ")
        {
            first.text = rest.to_string();
            if first.text.is_empty() {
                block.spans.remove(0);
            }
        }
        block.task = Some(done);
    }

    fn style(&self) -> Style {
        Style {
            bold: self.strong > 0 || self.html.bold > 0 || self.header,
            italic: self.emphasis > 0 || self.html.italic > 0,
            code: self.code || self.html.code > 0,
            link: false,
            muted: self.strike > 0 || self.html.strike > 0,
            token: None,
        }
    }

    /// 收下手上的这一块：没闭合的 `<b>` 只染到块尾，块尾欠着的 `<br>` 不补。
    fn finish_current(&mut self) {
        self.html = html::Depths::default();
        self.breaks = 0;
        if let Some(block) = self.current.take()
            && block.has_content()
        {
            self.blocks.push(block);
        }
    }
}

fn align(alignment: Alignment) -> Align {
    match alignment {
        Alignment::None => Align::None,
        Alignment::Left => Align::Left,
        Alignment::Center => Align::Center,
        Alignment::Right => Align::Right,
    }
}

/// 链接看得见的字后面要不要补地址：补的话交回开头（` (`，看得见的字是空的就是 `(`）。地址是空的、看得见的字就是地址的不补。
fn suffix_lead(url: &str, shown: &str) -> Option<&'static str> {
    let (url, shown) = (url.trim(), shown.trim());
    if url.is_empty() || same_target(url, shown) {
        return None;
    }
    Some(if shown.is_empty() { "(" } else { " (" })
}

/// 看得见的字就是这个地址（自动链接）：同一串不写两遍。
fn same_target(url: &str, shown: &str) -> bool {
    url == shown
        || ["https://", "http://", "mailto:"].iter().any(|scheme| {
            url.strip_prefix(scheme)
                .is_some_and(|rest| rest.trim_end_matches('/') == shown.trim_end_matches('/'))
        })
}

/// 围栏的语言是不是 mermaid：只看第一个词，大小写不论（```` ```mermaid title ````）。
fn is_mermaid(language: &str) -> bool {
    language
        .split_whitespace()
        .next()
        .is_some_and(|word| word.eq_ignore_ascii_case("mermaid"))
}
