//! 画（设计 36 第四节）：Markdown 读成块、排字、画到页面上、编成 PNG。R-1 只有段落和换行（软换行照旧版也换行），别的
//! Markdown 照它的字画成段落，R-2 起一样一样补。
//!
//! - 字体库、字形的缓存放在 [`Engine`] 里，一次只画一张（调的一方拿锁）。字体库第一次画的时候才读（包常开，闲着不占）。
//! - 字形照透明度叠到页面上（[`paint`]），不盖像素：mermaid、公式以后也照这一条叠（设计 36 第四节第 2 条）。

mod blocks;
mod paint;
mod units;

use std::cell::RefCell;

use cosmic_text::fontdb::{Database, Family, Query};
use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap};
use image::RgbaImage;

use crate::fonts::{self, Catalog, Families, FontError, Generic};
use crate::theme::Palette;

/// 一栏多宽（同旧版）。
pub const COLUMN: u32 = 960;

/// 页边（同旧版的出厂值）。
pub const PADDING: u32 = 64;

/// 一页最矮（同旧版）。
pub const MIN_HEIGHT: u32 = 360;

/// 行高是字号的几倍。
const LINE_HEIGHT: f32 = 1.6;

/// 段和段之间空多少，字号的几倍。
const PARAGRAPH_GAP: f32 = 0.6;

/// 字形的缓存最多留多少个，多了清空（同旧版）。
const MAX_CACHED_GLYPHS: usize = 2048;

/// 怎么画：配色、字号。
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// 六个颜色。
    pub palette: Palette,
    /// 正文的字号，像素。
    pub font_size: u32,
}

/// 画好的一页。
#[derive(Debug, Clone)]
pub struct Page {
    /// PNG 的字节。
    pub png: Vec<u8>,
    /// 宽，像素。
    pub width: u32,
    /// 高，像素。
    pub height: u32,
}

/// 字体库、字形的缓存、挑好的字体。
pub struct Engine {
    system: FontSystem,
    cache: SwashCache,
    families: Families,
}

impl Engine {
    /// 照字体库 `database` 开一个：正文、代码照配置写的 `body`、`code` 和系统的通用名 `generic` 挑（[`fonts::resolve`]）。
    ///
    /// # Errors
    ///
    /// 字体库里一个有中文的都没有。
    pub fn new(
        database: Database,
        (body, code): (Option<&str>, Option<&str>),
        generic: &Generic,
    ) -> Result<Engine, FontError> {
        let mut system = FontSystem::new_with_locale_and_db("zh-CN".to_string(), database);
        let families = fonts::resolve(&Live(RefCell::new(&mut system)), body, code, generic)?;
        Ok(Engine {
            system,
            cache: SwashCache::new(),
            families,
        })
    }

    /// 挑好的字体。
    pub fn families(&self) -> &Families {
        &self.families
    }

    /// 把 `markdown` 照 `style` 画成一页。
    pub fn render(&mut self, markdown: &str, style: &Style) -> Page {
        if self.cache.image_cache.len() > MAX_CACHED_GLYPHS {
            self.cache.image_cache.clear();
        }
        let size = units::float(style.font_size);
        let metrics = Metrics::new(size, (size * LINE_HEIGHT).round());
        let attrs = Attrs::new()
            .family(Family::Name(&self.families.body))
            .color(color(style.palette.text));
        let mut laid = Vec::new();
        for paragraph in blocks::paragraphs(markdown) {
            let mut buffer = Buffer::new(&mut self.system, metrics);
            buffer.set_size(Some(units::float(COLUMN)), None);
            buffer.set_wrap(Wrap::WordOrGlyph);
            buffer.set_text(&paragraph, &attrs, Shaping::Advanced, None);
            buffer.shape_until_scroll(&mut self.system, true);
            let height = buffer
                .layout_runs()
                .map(|run| run.line_top + run.line_height)
                .fold(0.0_f32, f32::max);
            laid.push((buffer, units::ceil(height)));
        }
        let gap = units::ceil(size * PARAGRAPH_GAP);
        let content = laid.iter().map(|(_, height)| height).sum::<u32>()
            + gap * u32::try_from(laid.len().saturating_sub(1)).unwrap_or(0);
        let width = PADDING * 2 + COLUMN;
        let height = (PADDING * 2 + content).max(MIN_HEIGHT);
        let mut image = RgbaImage::from_pixel(width, height, image::Rgba(style.palette.background));
        let mut top = PADDING;
        for (buffer, height) in &laid {
            paint::text(
                &mut image,
                &mut self.system,
                &mut self.cache,
                buffer,
                (PADDING, top),
            );
            top += height + gap;
        }
        Page {
            png: paint::png(&image),
            width,
            height,
        }
    }
}

/// 颜色换成 cosmic-text 的。
fn color([r, g, b, a]: [u8; 4]) -> Color {
    Color::rgba(r, g, b, a)
}

/// 照字体库答 [`Catalog`]：有没有中文要读字体，读要可变的字体库，所以包一层。
struct Live<'a>(RefCell<&'a mut FontSystem>);

impl Catalog for Live<'_> {
    fn families(&self) -> Vec<String> {
        let system = self.0.borrow();
        let mut names: Vec<String> = Vec::new();
        for face in system.db().faces() {
            if let Some((name, _)) = face.families.first()
                && !names.contains(name)
            {
                names.push(name.clone());
            }
        }
        names
    }

    fn covers_chinese(&self, family: &str) -> bool {
        let mut system = self.0.borrow_mut();
        let families = [Family::Name(family)];
        let query = Query {
            families: &families,
            ..Query::default()
        };
        let Some(id) = system.db().query(&query) else {
            return false;
        };
        system
            .get_font(id, Weight::NORMAL)
            .is_some_and(|font| font.as_swash().charmap().map('中') != 0)
    }
}

#[cfg(test)]
mod tests;
