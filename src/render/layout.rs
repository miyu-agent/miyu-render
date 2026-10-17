//! 一块排成字（设计 36 第四节，数照旧版 `renderer/layout.rs`）：字号、行高、缩进、上下的空、每一段字的字体和颜色。排好的
//! [`Laid`] 只量高，怎么画在 `draw`。
//!
//! - 字号：正文照配置，代码是正文的 30/36（旧版 36、30），标题 1 到 4 级放大 1.55、1.35、1.2、1.1 倍；行高 1.42 倍。
//! - 颜色从六个颜色来：正文、标题、粗体都是字的颜色（粗体、标题用真的粗字形，不再像旧版那样靠变色），链接是强调色，删除线是
//!   淡字，代码的字是字的颜色。
//! - 行内代码的字形带一个记号（`INLINE_CODE`），画的时候在下面垫一块代码的底。

use cosmic_text::fontdb::Family;
use cosmic_text::{
    Align, Attrs, Buffer, Color, FontSystem, Metrics, Shaping, Style as FontStyle, Weight, Wrap,
};

use super::table::{self, Grid};
use super::{COLUMN, units};
use crate::markdown::{Block, Kind, Span, Style};
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
}

/// 排一块。
pub fn lay(system: &mut FontSystem, block: &Block, look: &Look) -> Laid {
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
        Kind::Table | Kind::Rule => ((font / 2, font / 2), (0, 0), 0, 0),
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
        return Laid {
            kind: block.kind,
            buffer: None,
            inset,
            height: grid.height,
            margin,
            task: None,
            grid: Some(grid),
        };
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
    let text_height = buffer
        .layout_runs()
        .map(|run| units::ceil(run.line_top + run.line_height))
        .max()
        .unwrap_or(0);
    let height = inset.1 + text_height.max(line) + pad_bottom;
    Laid {
        kind: block.kind,
        buffer: Some(buffer),
        inset,
        height,
        margin,
        task,
        grid: None,
    }
}

/// 排字：每一段字照它的样子挑字体、颜色、字号。
fn shape(system: &mut FontSystem, block: &Block, look: &Look, width: u32) -> Buffer {
    rich(system, &block.spans, block.kind, look, Some(width), None)
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
    let pieces = cjk_apart(spans, kind == Kind::Code);
    let spans: Vec<(&str, Attrs)> = pieces
        .iter()
        .map(|(text, style, cjk)| (text.as_str(), attrs(kind, *style, *cjk, look)))
        .collect();
    let default = attrs(kind, Style::default(), false, look);
    buffer.set_rich_text(spans, &default, Shaping::Advanced, align);
    buffer.shape_until_scroll(system, true);
    buffer
}

/// 代码里的中日韩字和别的字分开（交回字、样子、是不是中日韩字）：中日韩字用正文的字体。等宽字体里的中文一个字占两格，看着字距
/// 拉开（网页 2026-09-30 同一条：`base.css` 的「Miyu CJK Sans」，字的范围照它）。不是代码的整段照原样。
fn cjk_apart(spans: &[Span], code_block: bool) -> Vec<(String, Style, bool)> {
    let mut pieces: Vec<(String, Style, bool)> = Vec::new();
    for span in spans {
        let code = code_block || span.style.code;
        for ch in span.text.chars() {
            let cjk = code && is_cjk(ch);
            match pieces.last_mut() {
                Some((text, style, last)) if *style == span.style && *last == cjk => text.push(ch),
                _ => pieces.push((ch.to_string(), span.style, cjk)),
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

/// 一段字的字体、颜色、粗细、斜不斜、字号；行内代码带记号（代码里的中日韩字也带，底照样垫上）。
fn attrs<'a>(kind: Kind, style: Style, cjk: bool, look: &Look<'a>) -> Attrs<'a> {
    let code = style.code || kind == Kind::Code;
    let family = if code && !cjk { look.code } else { look.body };
    let colour = if style.link {
        look.palette.accent
    } else if style.muted {
        look.palette.muted
    } else {
        look.palette.text
    };
    let [r, g, b, a] = colour;
    let mut attrs = Attrs::new()
        .family(Family::Name(family))
        .color(Color::rgba(r, g, b, a))
        .metrics(metrics(kind, style, look.size));
    if style.bold || matches!(kind, Kind::Heading(_)) {
        attrs = attrs.weight(Weight::BOLD);
    }
    if style.italic {
        attrs = attrs.style(FontStyle::Italic);
    }
    if style.code && kind != Kind::Code {
        attrs = attrs.metadata(INLINE_CODE);
    }
    attrs
}

#[cfg(test)]
mod tests;
