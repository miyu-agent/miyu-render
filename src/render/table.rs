//! 表格画成格子（设计 36 第四节，照旧版 `renderer/layout.rs` 的 `layout_table`、`table_width.rs`）。
//!
//! - 列宽照内容分（[`widths()`]）：先量每一列不折行要多宽，装得下按各列的字多少分富余，装不下只削最宽的几列（先削到一条线上，
//!   线以下的不动），窄的序号列、数字列不受牵连；加起来恰好一栏宽。等分会让九位的 QQ 号被逐字劈成两行（旧版实测）。
//! - 一行多高照这一行最高的格子；表头的底比页面深两档，正文隔一行深一档；格子之间一像素的线。
//! - 每一列照表头那一行的对齐（左、中、右）；格子里的 `<br>` 换行照换。

mod widths;

use cosmic_text::{Align as TextAlign, Buffer, FontSystem};

use super::layout::{Look, rich};
use super::{COLUMN, units};
use crate::markdown::{Align, Kind, Span, Table};

pub use widths::widths;

/// 格子左右留的空。
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
    /// 每一行，照先后：有表头的第一行是表头。
    pub rows: Vec<Row>,
    /// 整张表多高（连最下面一条线）。
    pub height: u32,
}

/// 排一张表。没有一列的交回空的（照说不会：读的时候空表不收）。
pub fn lay(system: &mut FontSystem, table: &Table, look: &Look) -> Option<Grid> {
    let count = table
        .aligns
        .len()
        .max(table.header.len())
        .max(table.rows.iter().map(Vec::len).max().unwrap_or(0));
    if count == 0 {
        return None;
    }
    let mut natural = vec![0; count];
    for row in std::iter::once(&table.header).chain(table.rows.iter()) {
        for (slot, cell) in natural.iter_mut().zip(row) {
            *slot = (*slot).max(width_of(system, cell, look));
        }
    }
    let min_content = units::ceil(look.size * 2.0);
    let widths = widths(&natural, COLUMN, PAD, min_content);
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
                        cell,
                        Kind::Table,
                        look,
                        Some(width.saturating_sub(PAD * 2)),
                        aligns[column],
                    )
                });
            let text = buffer.as_ref().map_or(0, text_height);
            height = height.max(text);
            laid.push(buffer);
        }
        let line = units::ceil(look.size * 0.92 * 1.42);
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
        rows,
        height: top + 1,
    })
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
