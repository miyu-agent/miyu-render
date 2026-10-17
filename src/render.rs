//! 画（设计 36 第四节）：Markdown 读成块（`crate::markdown`）、一块一块排字（[`layout`]）、画到页面上（[`draw`]）、编成 PNG。
//! R-2 上一栏往下排；分栏、分页随 R-2 下。
//!
//! - 字体库、字形的缓存放在 [`Engine`] 里，一次只画一张（调的一方拿锁）。字体库第一次画的时候才读（包常开，闲着不占）。
//! - 字形照透明度叠到页面上（[`paint`]），不盖像素：mermaid、公式以后也照这一条叠（设计 36 第四节第 2 条）。

mod draw;
mod layout;
mod paint;
mod units;

use std::cell::RefCell;

use cosmic_text::fontdb::{Database, Family, Query};
use cosmic_text::{FontSystem, SwashCache, Weight};
use image::RgbaImage;

use crate::fonts::{self, Catalog, Families, FontError, Generic};
use crate::theme::Palette;

/// 一栏多宽（同旧版）。
pub const COLUMN: u32 = 960;

/// 页边（同旧版的出厂值）。
pub const PADDING: u32 = 64;

/// 一页最矮（同旧版）。
pub const MIN_HEIGHT: u32 = 360;

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
        let look = layout::Look {
            palette: style.palette,
            size: units::float(style.font_size),
            body: &self.families.body,
            code: &self.families.code,
        };
        let laid: Vec<layout::Laid> = crate::markdown::blocks(markdown)
            .iter()
            .map(|block| layout::lay(&mut self.system, block, &look))
            .collect();
        let mut tops = Vec::with_capacity(laid.len());
        let mut bottom = 0;
        for (index, block) in laid.iter().enumerate() {
            let gap = if index == 0 {
                0
            } else {
                block.margin.0.max(laid[index - 1].margin.1)
            };
            tops.push(bottom + gap);
            bottom += gap + block.height;
        }
        let width = PADDING * 2 + COLUMN;
        let height = (PADDING * 2 + bottom).max(MIN_HEIGHT);
        let mut image = RgbaImage::from_pixel(width, height, image::Rgba(style.palette.background));
        for (block, top) in laid.iter().zip(tops) {
            draw::block(
                &mut image,
                &mut self.system,
                &mut self.cache,
                block,
                (PADDING, PADDING + top),
                &style.palette,
            );
        }
        Page {
            png: paint::png(&image),
            width,
            height,
        }
    }
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
