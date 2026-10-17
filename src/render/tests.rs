//! 画一页：用仓库里的测试字体（AGENTS.md 第 4 条）。挑到的西文字体不含中文的换成有中文的；画出来的宽高、底色、字的颜色在
//! 页面上、整页不透明（叠上去，不盖）；代码块、引用、分割线、勾选框各用各的颜色；没有粗体的字体假装加粗；表格有表头、隔行的底和格子线；段和段之间有空；长的分栏、第二栏有字；字多了页面变高；三套主题的底不一样；字体库里没有中文的报错。

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
        emoji: Vec::new(),
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

/// 画成一页（这些测试的字都短）。
fn one(engine: &mut Engine, markdown: &str, style: &Style) -> Page {
    let mut pages = engine.render(markdown, style).expect("画得了");
    assert_eq!(pages.len(), 1);
    pages.remove(0)
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
    let page = one(&mut engine, "你好世界\n\nhello 测试", &style(Theme::Paper));
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
    let page = one(&mut engine, &long, &style(Theme::Paper));
    assert!(page.height > MIN_HEIGHT * 2, "{}", page.height);
    let short = one(&mut engine, "这个群里说话", &style(Theme::Paper));
    assert!(page.height > short.height);
}

#[test]
fn each_theme_paints_its_own_background() {
    let mut engine = engine();
    for theme in [Theme::Paper, Theme::Light, Theme::Dark] {
        let image = decoded(&one(&mut engine, "你好", &style(theme)));
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

/// 和 `color` 一模一样（三个通道）的像素有几个。
fn exactly(image: &image::RgbaImage, color: [u8; 4]) -> usize {
    image
        .pixels()
        .filter(|pixel| pixel.0[..3] == color[..3])
        .count()
}

#[test]
fn decorations_take_their_colours() {
    let mut engine = engine();
    let palette = Theme::Paper.palette();
    let plain = decoded(&one(&mut engine, "你好", &style(Theme::Paper)));
    let chip = decoded(&one(&mut engine, "你 `code` 好", &style(Theme::Paper)));
    assert!(exactly(&chip, palette.code) > 500, "行内代码的底");
    let got = decoded(&one(
        &mut engine,
        "```\ncode\n```\n\n> 引用\n\n---\n\n- [x] 好\n",
        &style(Theme::Paper),
    ));
    assert!(exactly(&got, palette.code) > 960 * 60, "代码块的底");
    assert!(exactly(&got, palette.quote()) > 960 * 30, "引用的底");
    assert!(exactly(&got, palette.muted) > 6 * 40, "引用的竖条");
    assert!(exactly(&got, palette.line) >= 960 * 2, "分割线");
    assert!(exactly(&got, palette.accent) > 100, "做完的勾选框");
    for colour in [palette.code, palette.quote(), palette.accent] {
        assert!(
            exactly(&plain, colour) < 50,
            "只有字的页面几乎没有这些颜色（字边上偶尔撞上一两个）：{colour:?}"
        );
    }
}

/// 不是底色的像素有几个（墨）。
fn ink(image: &image::RgbaImage, background: [u8; 4]) -> usize {
    image
        .pixels()
        .filter(|pixel| pixel.0[..3] != background[..3])
        .count()
}

#[test]
fn bold_without_a_bold_face_is_drawn_heavier() {
    // 测试字体只有常规体：粗体错开一点再画一遍。
    let mut engine = engine();
    let palette = Theme::Paper.palette();
    let regular = ink(
        &decoded(&one(&mut engine, "这个群里说话", &style(Theme::Paper))),
        palette.background,
    );
    let bold = ink(
        &decoded(&one(&mut engine, "**这个群里说话**", &style(Theme::Paper))),
        palette.background,
    );
    assert!(bold * 10 > regular * 11, "{bold} {regular}");
}

#[test]
fn paragraphs_are_set_apart() {
    let mut engine = engine();
    let lines = "这\n个\n群\n里\n说\n话\n的".to_string();
    let apart = "这\n\n个\n\n群\n\n里\n\n说\n\n话\n\n的".to_string();
    let joined = one(&mut engine, &lines, &style(Theme::Paper)).height;
    let split = one(&mut engine, &apart, &style(Theme::Paper)).height;
    assert!(split > joined + 6 * 6, "{split} {joined}");
}

#[test]
fn a_table_has_a_header_band_and_lines() {
    let mut engine = engine();
    let palette = Theme::Paper.palette();
    let got = decoded(&one(
        &mut engine,
        "| 名 | 数 |\n|---|---|\n| 你 | 1 |\n| 好 | 2 |\n",
        &style(Theme::Paper),
    ));
    assert!(exactly(&got, palette.header()) > 960 * 30, "表头的底");
    assert!(exactly(&got, palette.quote()) > 960 * 30, "第二行的底");
    assert!(exactly(&got, palette.line) > 960 * 3, "格子线");
}

#[test]
fn a_long_reply_is_drawn_in_columns() {
    let mut engine = engine();
    let palette = Theme::Paper.palette();
    let long = "这个群里说话的时候长回复会转成图。\n\n".repeat(60);
    let page = one(&mut engine, &long, &style(Theme::Paper));
    assert!(
        page.width >= PADDING * 2 + COLUMN * 2,
        "分了栏：{}x{}",
        page.width,
        page.height
    );
    let image = decoded(&page);
    let second = PADDING + COLUMN + 32;
    let ink = (second..second + COLUMN)
        .flat_map(|x| (PADDING..page.height - PADDING).map(move |y| (x, y)))
        .filter(|(x, y)| image.get_pixel(*x, *y).0[..3] != palette.background[..3])
        .count();
    assert!(ink > 1000, "第二栏有字：{ink}");
}

#[test]
fn a_split_paragraph_continues_at_the_top_of_the_next_column() {
    let mut engine = engine();
    let palette = Theme::Paper.palette();
    let long = "这个群里说话的时候长回复会转成图。".repeat(400);
    let page = one(&mut engine, &long, &style(Theme::Paper));
    let image = decoded(&page);
    let second = PADDING + COLUMN + 32;
    let ink = (second..second + COLUMN)
        .flat_map(|x| (PADDING..PADDING + 60).map(move |y| (x, y)))
        .filter(|(x, y)| image.get_pixel(*x, *y).0[..3] != palette.background[..3])
        .count();
    assert!(ink > 300, "第二栏一开头就接着前一栏的字：{ink}");
}

#[test]
fn a_diagram_sits_on_the_page_and_a_broken_one_shows_its_source() {
    let mut engine = engine();
    let palette = Theme::Dark.palette();
    let image = decoded(&one(
        &mut engine,
        "```mermaid\ngraph TD\n  A[你好] --> B[世界]\n```",
        &style(Theme::Dark),
    ));
    assert!(exactly(&image, palette.quote()) > 1000, "节点的底");
    assert!(image.pixels().all(|pixel| pixel.0[3] == 255), "整页不透明");
    assert!(
        image
            .pixels()
            .filter(|pixel| pixel.0[..3].iter().all(|channel| *channel > 240))
            .count()
            == 0,
        "深色页面上没有白块"
    );
    let broken = decoded(&one(
        &mut engine,
        "```mermaid\ngraph TD\n  A -->\n```",
        &style(Theme::Dark),
    ));
    assert!(
        exactly(&broken, palette.code) > 960 * 40,
        "画不出来的照代码块画"
    );
}
