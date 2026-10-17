//! 画一块（设计 36 第四节，样子照旧版 `renderer/paint.rs`）：先画底下的（代码块的圆角底、引用的底和竖条、分割线、一级标题下面的
//! 线、行内代码的底、勾选框），再画字。都照透明度叠（`paint::over`），页面一直不透明。
//!
//! 粗体：字体族里有粗体的用真的粗字形；没有的（挑到的那个字形不粗）往右错开一点再画一遍，假装加粗。

use cosmic_text::{Buffer, FontSystem, LayoutRun, SwashCache};
use image::RgbaImage;

use super::layout::{INLINE_CODE, Laid, QUOTE};
use super::table::{Grid, PAD as TABLE_PAD, PAD_Y as TABLE_PAD_Y};
use super::{COLUMN, paint, units};
use crate::markdown::Kind;
use crate::theme::{Palette, Rgba};

/// 代码块底的圆角半径。
const CODE_RADIUS: u32 = 12;

/// 行内代码的底左右多出去多少、上下往里收几成。
const CHIP: (f32, f32) = (5.0, 0.1);

/// 粗细到多少算粗（字形的字重）。
const BOLD: u16 = 600;

/// 把排好的 `laid` 画在 `image` 上，左上角在 `(left, top)`。
pub fn block(
    image: &mut RgbaImage,
    system: &mut FontSystem,
    cache: &mut SwashCache,
    laid: &Laid,
    (left, top): (u32, u32),
    palette: &Palette,
) {
    match laid.kind {
        Kind::Code => rounded(
            image,
            (left, top, COLUMN, laid.height),
            CODE_RADIUS,
            palette.code,
        ),
        Kind::Quote => {
            rect(image, (left, top, COLUMN, laid.height), palette.quote());
            rect(image, (left, top, QUOTE.0, laid.height), palette.muted);
        }
        Kind::Rule => rect(
            image,
            (left, top + laid.height / 2, COLUMN, 2),
            palette.line,
        ),
        Kind::Heading(1) => rect(
            image,
            (left, top + laid.height.saturating_sub(2), COLUMN, 2),
            palette.line,
        ),
        _ => {}
    }
    if let Some(grid) = laid.grid.as_ref() {
        table(image, system, cache, grid, (left, top), palette);
    }
    if let Some((done, (x, y), side)) = laid.task {
        checkbox(image, (left + x, top + y), side, done, palette);
    }
    let Some(buffer) = laid.buffer.as_ref() else {
        return;
    };
    let origin = (left + laid.inset.0, top + laid.inset.1);
    for run in buffer.layout_runs() {
        chips(image, &run, origin, palette.code);
    }
    text(image, system, cache, buffer, origin);
}

/// 表格：先铺表头、隔行的底，再画每一格的字（行内代码垫底），最后画一像素的格子线。
fn table(
    image: &mut RgbaImage,
    system: &mut FontSystem,
    cache: &mut SwashCache,
    grid: &Grid,
    (left, top): (u32, u32),
    palette: &Palette,
) {
    for row in &grid.rows {
        let colour = if row.header {
            Some(palette.header())
        } else if row.stripe {
            Some(palette.quote())
        } else {
            None
        };
        if let Some(colour) = colour {
            rect(image, (left, top + row.top, COLUMN, row.height), colour);
        }
        let mut x = left;
        for (width, cell) in grid.widths.iter().zip(&row.cells) {
            if let Some(buffer) = cell {
                let origin = (x + TABLE_PAD, top + row.top + TABLE_PAD_Y);
                for run in buffer.layout_runs() {
                    chips(image, &run, origin, palette.code);
                }
                text(image, system, cache, buffer, origin);
            }
            x += width;
        }
    }
    let bottom = grid.rows.last().map_or(0, |row| row.top + row.height);
    for row in &grid.rows {
        rect(image, (left, top + row.top, COLUMN, 1), palette.line);
    }
    rect(image, (left, top + bottom, COLUMN, 1), palette.line);
    let mut x = left;
    for width in &grid.widths {
        rect(image, (x, top, 1, bottom + 1), palette.line);
        x += width;
    }
    rect(image, (left + COLUMN - 1, top, 1, bottom + 1), palette.line);
}

/// 行内代码的底：一行里挨着的代码字形连成一块。
fn chips(image: &mut RgbaImage, run: &LayoutRun, (left, top): (u32, u32), colour: Rgba) {
    let mut ranges: Vec<(f32, f32)> = Vec::new();
    for glyph in run
        .glyphs
        .iter()
        .filter(|glyph| glyph.metadata == INLINE_CODE)
    {
        let (start, end) = (glyph.x, glyph.x + glyph.w);
        match ranges.last_mut() {
            Some((_, last)) if start - *last <= 0.5 => *last = end.max(*last),
            _ => ranges.push((start, end)),
        }
    }
    let inset = (run.line_height * CHIP.1).max(2.0);
    for (start, end) in ranges {
        let x = left + units::ceil(start - CHIP.0);
        let width = units::ceil(end - start + CHIP.0 * 2.0);
        let y = top + units::ceil(run.line_top + inset);
        let height = units::ceil(run.line_height - inset * 2.0);
        rounded(image, (x, y, width, height), 6, colour);
    }
}

/// 画字：字形照透明度叠上去；要粗、挑到的字形不粗的再错开画一遍。
fn text(
    image: &mut RgbaImage,
    system: &mut FontSystem,
    cache: &mut SwashCache,
    buffer: &Buffer,
    (left, top): (u32, u32),
) {
    let default = cosmic_text::Color::rgba(0, 0, 0, 255);
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            let base = glyph.color_opt.unwrap_or(default);
            let thin = system
                .db()
                .face(glyph.font_id)
                .is_some_and(|face| face.weight.0 < BOLD);
            let fake = glyph.font_weight.0 >= BOLD && thin;
            let strokes: &[i64] = if fake { &[0, 1] } else { &[0] };
            cache.with_pixels(system, physical.cache_key, base, |x, y, colour| {
                for shift in strokes {
                    let x = i64::from(left) + i64::from(physical.x) + i64::from(x) + shift;
                    let y = i64::from(top)
                        + units::round(run.line_y)
                        + i64::from(physical.y)
                        + i64::from(y);
                    let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
                        continue;
                    };
                    if let Some(pixel) = image.get_pixel_mut_checked(x, y) {
                        paint::over(pixel, colour.as_rgba());
                    }
                }
            });
        }
    }
}

/// 勾选框：方框是线的颜色；做完了的填强调色、画一个勾。
fn checkbox(
    image: &mut RgbaImage,
    (left, top): (u32, u32),
    side: u32,
    done: bool,
    palette: &Palette,
) {
    let stroke = (side / 10).max(2);
    if done {
        rounded(image, (left, top, side, side), side / 5, palette.accent);
        // 勾：左下一短笔、右上一长笔。
        let start = (left + side / 4, top + side / 2);
        let corner = (left + side * 2 / 5, top + side * 7 / 10);
        let end = (left + side * 3 / 4, top + side * 3 / 10);
        line(image, start, corner, stroke, palette.background);
        line(image, corner, end, stroke, palette.background);
    } else {
        rounded(image, (left, top, side, side), side / 5, palette.line);
        let inner = (
            left + stroke,
            top + stroke,
            side - stroke * 2,
            side - stroke * 2,
        );
        rounded(
            image,
            inner,
            (side / 5).saturating_sub(stroke),
            palette.background,
        );
    }
}

/// 一条粗 `width` 的直线（勾的两笔）。
fn line(image: &mut RgbaImage, from: (u32, u32), to: (u32, u32), width: u32, colour: Rgba) {
    let steps = from.0.abs_diff(to.0).max(from.1.abs_diff(to.1)).max(1);
    for step in 0..=steps {
        let at = |a: u32, b: u32| {
            let a = i64::from(a);
            u32::try_from(a + (i64::from(b) - a) * i64::from(step) / i64::from(steps)).unwrap_or(0)
        };
        rect(
            image,
            (at(from.0, to.0), at(from.1, to.1), width, width),
            colour,
        );
    }
}

/// 填一个方块。
pub fn rect(image: &mut RgbaImage, (x, y, width, height): (u32, u32, u32, u32), colour: Rgba) {
    for row in y..y.saturating_add(height).min(image.height()) {
        for column in x..x.saturating_add(width).min(image.width()) {
            paint::over(image.get_pixel_mut(column, row), colour);
        }
    }
}

/// 填一个圆角方块：角上照离圆心多远算盖住几成，边缘不起锯齿。
pub fn rounded(
    image: &mut RgbaImage,
    (x, y, width, height): (u32, u32, u32, u32),
    radius: u32,
    colour: Rgba,
) {
    let radius = radius.min(width / 2).min(height / 2);
    let r = units::float(radius);
    for row in 0..height {
        for column in 0..width {
            let (px, py) = (x + column, y + row);
            if px >= image.width() || py >= image.height() {
                continue;
            }
            let dx = if column < radius {
                r - units::float(column) - 0.5
            } else if column >= width - radius {
                units::float(column) + 0.5 - units::float(width - radius)
            } else {
                0.0
            };
            let dy = if row < radius {
                r - units::float(row) - 0.5
            } else if row >= height - radius {
                units::float(row) + 0.5 - units::float(height - radius)
            } else {
                0.0
            };
            let coverage = if dx > 0.0 && dy > 0.0 {
                (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let [cr, cg, cb, ca] = colour;
            let alpha = units::ceil(f32::from(ca) * coverage).min(255);
            paint::over(
                image.get_pixel_mut(px, py),
                [cr, cg, cb, u8::try_from(alpha).unwrap_or(u8::MAX)],
            );
        }
    }
}
