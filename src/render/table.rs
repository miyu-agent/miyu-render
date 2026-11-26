//! 表格画成格子（设计 36 第四节，照旧版 `renderer/layout.rs` 的 `layout_table`、`table_width.rs`）。
//!
//! - 列宽照内容分（[`widths()`]）：每一列量三个宽（拆不开的、正文不折的、连表头都不折的，施工 R-14），装得下照宽的给，装不下先让
//!   表头折，再让正文折，每列不少于拆不开的宽；削的时候只削最宽的几列（先削到一条线上，线以下的不动），窄的序号列、数字列不受
//!   牵连；加起来恰好一栏宽。等分会让九位的 QQ 号被逐字劈成两行（旧版实测）。
//! - 哪里不许断（`glue`，施工 R-14）：数连单位不断，表头里连着的中文只在空格处折。
//! - 正文不折要多宽（[`needed`]）：整张图的栏宽照它放宽（`crate::render` 的 `column_for`，施工 R-14）。
//! - 一行多高照这一行最高的格子；表头的底比页面深两档，正文隔一行深一档；格子之间一像素的线。
//! - 每一列照表头那一行的对齐（左、中、右）；格子里的 `<br>` 换行照换。
//! - 列多到每列给字的宽不够一个字时（审计 #137，原来给的是 0、字挤出格子），格子左右留的空先跟着收窄；连不留空都不够一个字
//!   的，整张表的字跟着缩小到一个字放得下。总宽照旧一栏宽。

mod glue;
mod widths;

use cosmic_text::{Align as TextAlign, Buffer, FontSystem};

use super::layout::{Look, TABLE_SCALE, narrowest, rich};
use super::units;
use crate::markdown::{Align, Kind, Span, Table};

pub use widths::{Need, widths};

/// 格子左右留的空（列不多的时候；列多的收窄，见 [`Grid::pad`]）。
pub const PAD: u32 = 14;

/// 格子上下留的空。
pub const PAD_Y: u32 = 10;

/// 一行。
pub struct Row {
    /// 这一行的顶离表格顶多远。
    pub top: u32,
    /// 多高。
    pub height: u32,
    /// 是不是表头。
    pub header: bool,
    /// 隔行的底色（正文的第 2、4、6 行）。
    pub stripe: bool,
    /// 每一格排好的字；空的、这一行少写了的格子是空的。
    pub cells: Vec<Option<Buffer>>,
}

/// 排好的表格。
pub struct Grid {
    /// 每一列多宽，加起来是一栏宽。
    pub widths: Vec<u32>,
    /// 格子左右留的空：列不多的是 [`PAD`]，列多的收窄。
    pub pad: u32,
    /// 每一行，照先后：有表头的第一行是表头。
    pub rows: Vec<Row>,
    /// 整张表多高（连最下面一条线）。
    pub height: u32,
}

/// 排一张表。没有一列的交回空的（照说不会：读的时候空表不收）。
pub fn lay(system: &mut FontSystem, table: &Table, look: &Look) -> Option<Grid> {
    let count = columns(table);
    if count == 0 {
        return None;
    }
    let (pad, size) = fit(
        look.size,
        u32::try_from(count).unwrap_or(u32::MAX),
        look.column,
    );
    let look = &Look { size, ..*look };
    let needs = measure(system, table, look, count);
    let min_content = units::ceil(look.size * 2.0);
    let widths = widths(&needs, look.column, pad, min_content);
    let aligns: Vec<Option<TextAlign>> = (0..count)
        .map(|column| align(table.aligns.get(column).copied().unwrap_or_default()))
        .collect();
    let mut rows = Vec::new();
    let mut top = 0;
    let header = (!table.header.is_empty()).then_some(&table.header);
    for (index, (cells, is_header)) in header
        .map(|row| (row, true))
        .into_iter()
        .chain(table.rows.iter().map(|row| (row, false)))
        .enumerate()
    {
        let body_index = if header.is_some() {
            index.saturating_sub(1)
        } else {
            index
        };
        let mut height = 0;
        let mut laid = Vec::with_capacity(count);
        for (column, width) in widths.iter().enumerate() {
            let buffer = cells
                .get(column)
                .filter(|cell| !cell.is_empty())
                .map(|cell| {
                    rich(
                        system,
                        &glue::glue(cell, is_header),
                        Kind::Table,
                        look,
                        Some(width.saturating_sub(pad * 2)),
                        aligns[column],
                    )
                });
            let text = buffer.as_ref().map_or(0, text_height);
            height = height.max(text);
            laid.push(buffer);
        }
        let line = units::ceil(look.size * TABLE_SCALE * 1.42);
        let height = height.max(line) + PAD_Y * 2;
        rows.push(Row {
            top,
            height,
            header: is_header,
            stripe: !is_header && body_index % 2 == 1,
            cells: laid,
        });
        top += height;
    }
    Some(Grid {
        widths,
        pad,
        rows,
        height: top + 1,
    })
}

/// 列数是 `count` 时格子左右留多少空、表格照多大的正文字号排：每列至少分到一栏宽的 `1 / count`（[`widths()`]），从里面
/// 先给字留一个字宽（表格字号的一个字），剩下的才是两边的空，最多 [`PAD`]；连一个字宽都分不到的不留空，字缩小到一个字正好放得下。
fn fit(size: f32, count: u32, column: u32) -> (u32, f32) {
    let share = column / count.max(1);
    let em = units::ceil(size * TABLE_SCALE);
    if share >= em {
        (PAD.min((share - em) / 2), size)
    } else {
        (0, (size * units::float(share) / units::float(em)).max(1.0))
    }
}

/// 这张表正文不折行要多宽（连格子留的空），照 `look` 的正文字号、栏宽 `look.column` 时的留空量。整张图的栏宽照它放宽（施工 R-14）。
/// 没有一列的是 0。
pub fn needed(system: &mut FontSystem, table: &Table, look: &Look) -> u32 {
    let count = columns(table);
    if count == 0 {
        return 0;
    }
    let (pad, size) = fit(
        look.size,
        u32::try_from(count).unwrap_or(u32::MAX),
        look.column,
    );
    let look = &Look { size, ..*look };
    measure(system, table, look, count)
        .iter()
        .map(|need| need.body.max(need.least) + pad * 2)
        .sum()
}

/// 一共几列。
fn columns(table: &Table) -> usize {
    table
        .aligns
        .len()
        .max(table.header.len())
        .max(table.rows.iter().map(Vec::len).max().unwrap_or(0))
}

/// 每一列的三个宽（[`Need`]）：表头、正文各照粘好的字量（`glue`）。
fn measure(system: &mut FontSystem, table: &Table, look: &Look, count: usize) -> Vec<Need> {
    let mut needs = vec![Need::default(); count];
    for (need, cell) in needs.iter_mut().zip(&table.header) {
        let glued = glue::glue(cell, true);
        let least = narrowest(system, &glued, Kind::Table, look);
        need.least = need.least.max(least);
        need.body = need.body.max(least);
        need.full = need.full.max(width_of(system, &glued, look));
    }
    for row in &table.rows {
        for (need, cell) in needs.iter_mut().zip(row) {
            let glued = glue::glue(cell, false);
            let natural = width_of(system, &glued, look);
            need.least = need.least.max(narrowest(system, &glued, Kind::Table, look));
            need.body = need.body.max(natural);
            need.full = need.full.max(natural);
        }
    }
    needs
}

/// 一格不折行要多宽。
fn width_of(system: &mut FontSystem, cell: &[Span], look: &Look) -> u32 {
    if cell.is_empty() {
        return 0;
    }
    let buffer = rich(system, cell, Kind::Table, look, None, None);
    buffer
        .layout_runs()
        .map(|run| units::ceil(run.line_w))
        .max()
        .unwrap_or(0)
}

/// 排好的字多高。
fn text_height(buffer: &Buffer) -> u32 {
    buffer
        .layout_runs()
        .map(|run| units::ceil(run.line_top + run.line_height))
        .max()
        .unwrap_or(0)
}

/// 一列怎么对齐：没写的靠左。
fn align(align: Align) -> Option<TextAlign> {
    match align {
        Align::None | Align::Left => None,
        Align::Center => Some(TextAlign::Center),
        Align::Right => Some(TextAlign::Right),
    }
}

#[cfg(test)]
mod tests;
