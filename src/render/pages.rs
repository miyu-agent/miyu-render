//! 分页（设计 36 第四节第 6 条，数照旧版）：分好的栏装进页面。一页最高 [`MAX_HEIGHT`]、最多 [`MAX_PIXELS`] 像素，几页加起来最多
//! [`MAX_TOTAL`] 像素；超了的交回空的（太长，画不了）。一页装几栏照像素上限算，栏太多的接着放下一页。

use super::columns::{self, Column, GAP, Shape};
use super::{COLUMN, MIN_HEIGHT, PADDING};

/// 一页最高（同旧版）。
pub const MAX_HEIGHT: u32 = 5000;

/// 一页最多多少像素（同旧版）。
pub const MAX_PIXELS: u64 = 20_000_000;

/// 几页加起来最多多少像素（同旧版）。
pub const MAX_TOTAL: u64 = 48_000_000;

/// `count` 栏、栏里最高用了 `used` 的一页多宽多高。
pub fn size(count: usize, used: u32) -> (u32, u32) {
    let count = u32::try_from(count).unwrap_or(u32::MAX);
    let width = PADDING * 2 + COLUMN * count + GAP * count.saturating_sub(1);
    (width, (used + PADDING * 2).clamp(MIN_HEIGHT, MAX_HEIGHT))
}

/// 分栏、分页：交回每一页的栏。有一行字比一页还高、总像素超了的交回空的。
pub fn plan(shapes: &[Shape]) -> Option<Vec<Vec<Column>>> {
    let max = MAX_HEIGHT - PADDING * 2;
    let floor = MIN_HEIGHT - PADDING * 2;
    let columns = columns::balanced(shapes, max, floor, size, MAX_PIXELS)?;
    let used = columns.iter().map(|column| column.used).max().unwrap_or(0);
    let per_page = (1..=columns.len().max(1))
        .rev()
        .find(|count| {
            let (width, height) = size(*count, used);
            u64::from(width) * u64::from(height) <= MAX_PIXELS
        })
        .unwrap_or(1);
    let pages: Vec<Vec<Column>> = columns.chunks(per_page).map(<[Column]>::to_vec).collect();
    let total: u64 = pages
        .iter()
        .map(|page| {
            let (width, height) = size(
                page.len(),
                page.iter().map(|column| column.used).max().unwrap_or(0),
            );
            u64::from(width) * u64::from(height)
        })
        .sum();
    (total <= MAX_TOTAL).then_some(pages)
}

#[cfg(test)]
mod tests;
