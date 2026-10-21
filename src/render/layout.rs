//! 一块排成字（设计 36 第四节，数照旧版 `renderer/layout.rs`）：字号、行高、缩进、上下的空、每一段字的字体和颜色。排好的
//! [`Laid`] 只量高，怎么画在 `draw`。
//!
//! - 字号：正文照配置，代码是正文的 30/36（旧版 36、30），标题 1 到 4 级放大 1.55、1.35、1.2、1.1 倍；行高 1.42 倍。
//! - 颜色从六个颜色来：正文、标题、粗体都是字的颜色（粗体、标题用真的粗字形，不再像旧版那样靠变色），链接是强调色，删除线是
//!   淡字，代码的字是字的颜色。
//! - 行内代码的字形带一个记号（`INLINE_CODE`），画的时候在下面垫一块代码的底。
//! - 每个字用哪一类字体（代码里的中日韩字、表情）在 `script`。

use std::sync::Arc;

use cosmic_text::fontdb::{Database, Family};
use cosmic_text::{
    Align, Attrs, Buffer, Color, FontSystem, Metrics, Shaping, Style as FontStyle, Weight, Wrap,
};
use image::RgbaImage;

use super::script::{self, Script};
use super::table::{self, Grid};
use super::{COLUMN, units};
use super::{diagram, formula, highlight};
use crate::markdown::{Block, Kind, Span, Style, Token};
use crate::theme::Palette;

/// 行内代码的字形上带的记号。
pub const INLINE_CODE: usize = 1;

/// 行高是字号的几倍（旧版 1.42）。
const LINE_HEIGHT: f32 = 1.42;

/// 代码的字号是正文的几分之几（旧版 30 / 36）。
const CODE_SCALE: f32 = 30.0 / 36.0;

/// 表格的字号是正文的几倍（旧版 0.92）。
const TABLE_SCALE: f32 = 0.92;

/// 代码块左右、上下留的空。
pub const CODE_PAD: (u32, u32) = (32, 24);

/// 引用的竖条多宽、字离左边多远、上下留的空。
pub const QUOTE: (u32, u32, u32) = (6, 32, 14);

/// 列表每深一层往里缩多少。
const LIST_INDENT: u32 = 36;

/// 怎么排：配色、正文字号、两个字体族。
#[derive(Debug, Clone, Copy)]
pub struct Look<'a> {
    pub palette: Palette,
    pub size: f32,
    pub body: &'a str,
    pub code: &'a str,
    /// 代码字体有没有中文：有的代码里的中日韩字用它，没有的用正文字体。
    pub code_chinese: bool,
    /// 彩色表情字体，字体库里没有的是空的（照正文字体排）。
    pub emoji: Option<&'a str>,
    /// 字体库（画 mermaid 图的字用，R-3）。
    pub fonts: &'a Arc<Database>,
}

/// 排好的一块。
pub struct Laid {
    /// 是什么。
    pub kind: Kind,
    /// 排好的字；分割线没有。
    pub buffer: Option<Buffer>,
    /// 字离这一块左边、上边多远。
    pub inset: (u32, u32),
    /// 整块多高（连上下留的空）。
    pub height: u32,
    /// 和上一块之间、和下一块之间空多少。
    pub margin: (u32, u32),
    /// 任务项的勾选框：做没做完、左上角、边长。
    pub task: Option<(bool, (u32, u32), u32)>,
    /// 表格排好的格子（R-2 中）。
    pub grid: Option<Grid>,
    /// 能从哪里断开（分栏，R-2 下）：每一行字、表格每一行的底离这一块顶多远，照先后，最后一个就是整块的高。
    pub breaks: Vec<u32>,
    /// mermaid 画好的图（R-3）：透明的，叠到页面上。
    pub image: Option<RgbaImage>,
}

/// 排一块。mermaid 图先画成图，画不出来的照代码块排（源码照样看得到）。
pub fn lay(system: &mut FontSystem, block: &Block, look: &Look) -> Laid {
    if matches!(block.kind, Kind::Diagram | Kind::Formula) {
        return figure(block, look).unwrap_or_else(|| lay(system, &fallback(block), look));
    }
    let size = look.size;
    let small = units::ceil((size / 4.0).max(6.0));
    let font = units::ceil(size);
    let (margin, inset, pad_bottom, right) = match block.kind {
        Kind::Heading(1) => ((font, font / 2), (0, 0), 0, 0),
        Kind::Heading(_) => ((font / 2, small), (0, 0), 0, 0),
        Kind::Code => (
            (font / 2, font / 2),
            (CODE_PAD.0, CODE_PAD.1),
            CODE_PAD.1,
            CODE_PAD.0,
        ),
        Kind::Table | Kind::Rule | Kind::Diagram | Kind::Formula => {
            ((font / 2, font / 2), (0, 0), 0, 0)
        }
        Kind::Quote => ((small, small), (QUOTE.1, QUOTE.2), QUOTE.2, QUOTE.1 / 2),
        Kind::Item { depth } => (
            (small / 2, small / 2),
            (u32::from(depth.saturating_sub(1)) * LIST_INDENT, 0),
            0,
            0,
        ),
        Kind::Paragraph => ((small, small), (0, 0), 0, 0),
    };
    if let Some(grid) = block
        .table
        .as_ref()
        .and_then(|found| table::lay(system, found, look))
    {
        return gridded(block.kind, grid, margin);
    }
    if block.kind == Kind::Rule {
        return Laid {
            kind: block.kind,
            buffer: None,
            inset,
            height: font,
            margin,
            task: None,
            grid: None,
            breaks: vec![font],
            image: None,
        };
    }
    let line = units::ceil(size * LINE_HEIGHT);
    let (task, inset) = match block.task {
        Some(done) => {
            let edge = units::ceil(size * 0.72);
            let top = inset.1 + (line.saturating_sub(edge)) / 2;
            (
                Some((done, (inset.0, top), edge)),
                (inset.0 + edge + units::ceil(size * 0.4), inset.1),
            )
        }
        None => (None, inset),
    };
    let width = COLUMN.saturating_sub(inset.0 + right);
    let buffer = shape(system, block, look, width);
    let mut breaks: Vec<u32> = Vec::new();
    for run in buffer.layout_runs() {
        let at = inset.1 + units::ceil(run.line_top + run.line_height);
        if breaks.last().is_none_or(|last| *last < at) {
            breaks.push(at);
        }
    }
    let text_height = breaks.last().map_or(0, |last| last - inset.1);
    let height = inset.1 + text_height.max(line) + pad_bottom;
    breaks.pop();
    breaks.push(height);
    Laid {
        kind: block.kind,
        buffer: Some(buffer),
        inset,
        height,
        margin,
        task,
        grid: None,
        breaks,
        image: None,
    }
}

/// 排好的表格：照行断，最后一行的底换成整张表的高（连最下面一条线）。
fn gridded(kind: Kind, grid: Grid, margin: (u32, u32)) -> Laid {
    let mut breaks: Vec<u32> = grid.rows.iter().map(|row| row.top + row.height).collect();
    if let Some(last) = breaks.last_mut() {
        *last = grid.height;
    }
    Laid {
        kind,
        buffer: None,
        inset: (0, 0),
        height: grid.height,
        margin,
        task: None,
        breaks,
        grid: Some(grid),
        image: None,
    }
}

/// 画不出来的图换成什么排：mermaid 照代码块（源码照样看得到），公式照 Unicode 写法的段落。
fn fallback(block: &Block) -> Block {
    if block.kind == Kind::Formula {
        let tex: String = block.spans.iter().map(|span| span.text.as_str()).collect();
        let span = Span {
            text: crate::markdown::math::unicode(&tex),
            style: Style::default(),
        };
        return Block {
            kind: Kind::Paragraph,
            spans: vec![span],
            ..block.clone()
        };
    }
    Block {
        kind: Kind::Code,
        ..block.clone()
    }
}

/// mermaid 图、公式画成图：画不出来的交回空的。
fn figure(block: &Block, look: &Look) -> Option<Laid> {
    let source: String = block.spans.iter().map(|span| span.text.as_str()).collect();
    let image = if block.kind == Kind::Formula {
        formula::draw(&source, look.palette.text, look.size, COLUMN)?
    } else {
        diagram::draw(&source, &look.palette, look.body, look.fonts, COLUMN)?
    };
    let (font, height) = (units::ceil(look.size), image.height());
    Some(Laid {
        kind: block.kind,
        buffer: None,
        inset: (0, 0),
        height,
        margin: (font / 2, font / 2),
        task: None,
        grid: None,
        breaks: vec![height],
        image: Some(image),
    })
}

/// 排字：每一段字照它的样子挑字体、颜色、字号。
fn shape(system: &mut FontSystem, block: &Block, look: &Look, width: u32) -> Buffer {
    // 代码块认得出语言的先切词着色（R-5），认不出的照单色。
    let coloured = (block.kind == Kind::Code)
        .then_some(block.language.as_deref())
        .flatten()
        .and_then(|language| {
            let source: String = block.spans.iter().map(|span| span.text.as_str()).collect();
            highlight::tokens(&source, language)
        });
    rich(
        system,
        coloured.as_deref().unwrap_or(&block.spans),
        block.kind,
        look,
        Some(width),
        None,
    )
}

/// 一串字排成一块（表格的格子也用它）：`width` 是空的不折行（量一列不折行要多宽），`align` 是这一块怎么对齐。
pub fn rich(
    system: &mut FontSystem,
    spans: &[Span],
    kind: Kind,
    look: &Look,
    width: Option<u32>,
    align: Option<Align>,
) -> Buffer {
    let base = metrics(kind, Style::default(), look.size);
    let mut buffer = Buffer::new(system, base);
    buffer.set_size(width.map(units::float), None);
    buffer.set_wrap(if width.is_some() {
        Wrap::WordOrGlyph
    } else {
        Wrap::None
    });
    let pieces = script::split(spans, kind == Kind::Code);
    let spans: Vec<(&str, Attrs)> = pieces
        .iter()
        .map(|(text, style, script)| (text.as_str(), attrs(kind, *style, *script, look)))
        .collect();
    let default = attrs(kind, Style::default(), Script::Plain, look);
    buffer.set_rich_text(spans, &default, Shaping::Advanced, align);
    buffer.shape_until_scroll(system, true);
    buffer
}

/// 这一种块、这一段字的字号和行高。
fn metrics(kind: Kind, style: Style, size: f32) -> Metrics {
    let size = match kind {
        Kind::Heading(level) => size * heading_scale(level),
        Kind::Code => size * CODE_SCALE,
        _ if style.code => size * CODE_SCALE,
        Kind::Table => size * TABLE_SCALE,
        _ => size,
    };
    Metrics::new(size, (size * LINE_HEIGHT).ceil())
}

fn heading_scale(level: u8) -> f32 {
    match level {
        1 => 1.55,
        2 => 1.35,
        3 => 1.2,
        4 => 1.1,
        _ => 1.0,
    }
}

/// 一段字的字体、颜色、粗细、斜不斜、字号；行内代码带记号（代码里的中日韩字、表情也带，底照样垫上）。表情用彩色表情字体、
/// 常规粗细、不斜（[`script`]）。
fn attrs<'a>(kind: Kind, style: Style, script: Script, look: &Look<'a>) -> Attrs<'a> {
    let code = style.code || kind == Kind::Code;
    let emoji = script == Script::Emoji;
    let family = match script {
        Script::Emoji => look.emoji.unwrap_or(look.body),
        Script::Plain if code => look.code,
        Script::Cjk if look.code_chinese => look.code,
        Script::Cjk | Script::Plain => look.body,
    };
    let colour = match style.token {
        Some(Token::Keyword) => look.palette.accent,
        Some(Token::String) => look.palette.code_string(),
        Some(Token::Constant) => look.palette.code_constant(),
        Some(Token::Function) => look.palette.code_function(),
        Some(Token::Comment) => look.palette.muted,
        None if style.link => look.palette.accent,
        None if style.muted => look.palette.muted,
        None => look.palette.text,
    };
    let [r, g, b, a] = colour;
    let mut attrs = Attrs::new()
        .family(Family::Name(family))
        .color(Color::rgba(r, g, b, a))
        .metrics(metrics(kind, style, look.size));
    if !emoji && (style.bold || matches!(kind, Kind::Heading(_))) {
        attrs = attrs.weight(Weight::BOLD);
    }
    if !emoji && (style.italic || style.token == Some(Token::Comment)) {
        attrs = attrs.style(FontStyle::Italic);
    }
    if style.code && kind != Kind::Code {
        attrs = attrs.metadata(INLINE_CODE);
    }
    attrs
}

#[cfg(test)]
mod tests;
