//! 画（设计 36 第四节）：Markdown 读成块（`crate::markdown`）、一块一块排字（[`layout`]）、分栏分页（[`columns`]、[`pages`]）、
//! 画到页面上（[`draw`]）、编成 PNG。
//!
//! - 字体库、字形的缓存放在 [`Engine`] 里，一次只画一张（调的一方拿锁）。字体库第一次画的时候才读（包常开，闲着不占）。
//! - 字形照透明度叠到页面上（[`paint`]），不盖像素：mermaid、公式以后也照这一条叠（设计 36 第四节第 2 条）。

mod columns;
mod diagram;
mod draw;
mod formula;
mod layout;
mod pages;
mod paint;
mod script;
mod table;
mod units;

use std::cell::RefCell;
use std::sync::Arc;

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
    /// 字体库的一份（mermaid 图的字用：resvg 和排字是同一个 fontdb）。
    fonts: Arc<Database>,
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
        let fonts = Arc::new(system.db().clone());
        Ok(Engine {
            system,
            cache: SwashCache::new(),
            families,
            fonts,
        })
    }

    /// 挑好的字体。
    pub fn families(&self) -> &Families {
        &self.families
    }

    /// 把 `markdown` 照 `style` 画成一页或几页。
    ///
    /// # Errors
    ///
    /// 太长，超了像素的上限，或者有一行比一页还高：[`TooLong`]。
    pub fn render(&mut self, markdown: &str, style: &Style) -> Result<Vec<Page>, TooLong> {
        if self.cache.image_cache.len() > MAX_CACHED_GLYPHS {
            self.cache.image_cache.clear();
        }
        let look = layout::Look {
            palette: style.palette,
            size: units::float(style.font_size),
            body: &self.families.body,
            code: &self.families.code,
            emoji: self.families.emoji.as_deref(),
            fonts: &self.fonts,
        };
        let laid: Vec<layout::Laid> = crate::markdown::blocks(markdown)
            .iter()
            .map(|block| layout::lay(&mut self.system, block, &look))
            .collect();
        let shapes: Vec<columns::Shape> = laid.iter().map(shape).collect();
        let plan = pages::plan(&shapes).ok_or(TooLong)?;
        let background = image::Rgba(style.palette.background);
        // 一块先画成一条，再把分到的几行拷上去；同一块连着用（拆开的）不重画。
        let mut strip: Option<(usize, RgbaImage)> = None;
        let mut out = Vec::with_capacity(plan.len());
        for page in &plan {
            let used = page.iter().map(|column| column.used).max().unwrap_or(0);
            let (width, height) = pages::size(page.len(), used);
            let mut image = RgbaImage::from_pixel(width, height, background);
            for (index, column) in page.iter().enumerate() {
                let left = PADDING + u32::try_from(index).unwrap_or(0) * (COLUMN + columns::GAP);
                for piece in &column.pieces {
                    if strip
                        .as_ref()
                        .is_none_or(|(block, _)| *block != piece.block)
                    {
                        let block = &laid[piece.block];
                        let mut drawn =
                            RgbaImage::from_pixel(COLUMN, block.height.max(1), background);
                        draw::block(
                            &mut drawn,
                            &mut self.system,
                            &mut self.cache,
                            block,
                            (0, 0),
                            &style.palette,
                        );
                        strip = Some((piece.block, drawn));
                    }
                    if let Some((_, drawn)) = strip.as_ref() {
                        copy(
                            &mut image,
                            drawn,
                            (piece.from, piece.to),
                            (left, PADDING + piece.y),
                        );
                    }
                }
            }
            out.push(Page {
                png: paint::png(&image),
                width,
                height,
            });
        }
        Ok(out)
    }
}

/// 太长，画不了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLong;

impl std::fmt::Display for TooLong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "too long to draw")
    }
}

/// 排好的一块的形状（分栏只看它）。
fn shape(laid: &layout::Laid) -> columns::Shape {
    let header = laid
        .grid
        .as_ref()
        .and_then(|grid| grid.rows.first())
        .filter(|row| row.header)
        .map_or(0, |row| row.height);
    columns::Shape {
        breaks: laid.breaks.clone(),
        margin: laid.margin,
        heading: matches!(laid.kind, crate::markdown::Kind::Heading(_)),
        header,
        table: laid.grid.is_some(),
    }
}

/// 把 `strip` 里 `[from, to)` 那几行拷到 `image` 的 `(left, top)`。
fn copy(image: &mut RgbaImage, strip: &RgbaImage, (from, to): (u32, u32), (left, top): (u32, u32)) {
    for row in from..to.min(strip.height()) {
        let y = top + row - from;
        if y >= image.height() {
            break;
        }
        for x in 0..strip.width().min(image.width().saturating_sub(left)) {
            image.put_pixel(left + x, y, *strip.get_pixel(x, row));
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
