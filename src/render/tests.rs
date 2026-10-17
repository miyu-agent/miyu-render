//! 画一页：用仓库里的测试字体（AGENTS.md 第 4 条）。挑到的西文字体不含中文的换成有中文的；画出来的宽高、底色、字的颜色在
//! 页面上、整页不透明（叠上去，不盖）；字多了页面变高；三套主题的底不一样；字体库里没有中文的报错。

use std::path::Path;

use cosmic_text::fontdb::Database;

use super::*;
use crate::fonts::Generic;
use crate::theme::Theme;

/// 只装了测试字体的字体库：`files` 是 `tests/fonts/` 里的文件名。
fn database(files: &[&str]) -> Database {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts");
    let mut database = Database::new();
    for file in files {
        database
            .load_font_file(dir.join(file))
            .expect("读得出测试字体");
    }
    database
}

/// 西文、等宽是系统的通用名的一台机器。
fn generic() -> Generic {
    Generic {
        sans: vec!["Miyu Test Latin".into()],
        mono: vec!["Miyu Test Mono".into()],
    }
}

fn engine() -> Engine {
    Engine::new(
        database(&["latin.ttf", "cjk.otf", "mono.ttf"]),
        (None, None),
        &generic(),
    )
    .expect("挑得出字体")
}

fn style(theme: Theme) -> Style {
    Style {
        palette: theme.palette(),
        font_size: 36,
    }
}

/// 解开 PNG。
fn decoded(page: &Page) -> image::RgbaImage {
    image::load_from_memory_with_format(&page.png, image::ImageFormat::Png)
        .expect("是 PNG")
        .to_rgba8()
}

/// 和 `color` 差不多（三个通道各差不过 8）的像素有几个。
fn near(image: &image::RgbaImage, color: [u8; 4]) -> usize {
    image
        .pixels()
        .filter(|pixel| {
            pixel
                .0
                .iter()
                .zip(color)
                .take(3)
                .all(|(a, b)| a.abs_diff(b) <= 8)
        })
        .count()
}

#[test]
fn a_latin_font_is_swapped_for_one_with_chinese() {
    let engine = engine();
    assert_eq!(engine.families().body, "Miyu Test CJK");
    assert_eq!(engine.families().code, "Miyu Test Mono");
    assert!(
        engine
            .families()
            .note
            .as_deref()
            .is_some_and(|note| note.contains("Miyu Test Latin"))
    );
}

#[test]
fn a_page_has_the_column_width_the_background_and_the_ink() {
    let mut engine = engine();
    let page = engine.render("你好世界\n\nhello 测试", &style(Theme::Paper));
    let image = decoded(&page);
    assert_eq!((image.width(), image.height()), (page.width, page.height));
    assert_eq!(page.width, PADDING * 2 + COLUMN);
    assert_eq!(page.height, MIN_HEIGHT, "两行字不到最矮的高度");
    let palette = Theme::Paper.palette();
    assert_eq!(image.get_pixel(0, 0).0, palette.background);
    assert_eq!(
        image.get_pixel(page.width - 1, page.height - 1).0,
        palette.background
    );
    assert!(near(&image, palette.text) > 200, "字照正文的颜色画上去了");
    // 字形照透明度叠上去，不盖像素：整页没有透明的洞，字边上是底和字之间的颜色。
    assert!(image.pixels().all(|pixel| pixel.0[3] == 255), "整页不透明");
    // 页边里没有字。
    for x in 0..PADDING {
        assert_eq!(image.get_pixel(x, page.height / 2).0, palette.background);
    }
}

#[test]
fn more_text_makes_a_taller_page() {
    let mut engine = engine();
    let long = "这个群里说话\n\n".repeat(20);
    let page = engine.render(&long, &style(Theme::Paper));
    assert!(page.height > MIN_HEIGHT * 2, "{}", page.height);
    let short = engine.render("这个群里说话", &style(Theme::Paper));
    assert!(page.height > short.height);
}

#[test]
fn each_theme_paints_its_own_background() {
    let mut engine = engine();
    for theme in [Theme::Paper, Theme::Light, Theme::Dark] {
        let image = decoded(&engine.render("你好", &style(theme)));
        assert_eq!(
            image.get_pixel(1, 1).0,
            theme.palette().background,
            "{theme:?}"
        );
        assert!(near(&image, theme.palette().text) > 20, "{theme:?}");
    }
}

#[test]
fn without_a_chinese_font_it_says_what_to_install() {
    let got = Engine::new(
        database(&["latin.ttf", "mono.ttf"]),
        (None, None),
        &generic(),
    );
    assert!(matches!(got, Err(FontError::NoChinese)));
}

#[test]
fn colour_is_laid_over_the_page_by_its_opacity() {
    let mut pixel = image::Rgba([200, 100, 0, 255]);
    paint::over(&mut pixel, [0, 200, 255, 0]);
    assert_eq!(pixel.0, [200, 100, 0, 255], "全透明的不改");
    paint::over(&mut pixel, [0, 200, 255, 255]);
    assert_eq!(pixel.0, [0, 200, 255, 255], "不透明的盖上");
    let mut pixel = image::Rgba([200, 100, 0, 255]);
    paint::over(&mut pixel, [0, 200, 255, 128]);
    assert_eq!(pixel.0, [100, 150, 128, 255], "一半一半，叠完不透明");
}
