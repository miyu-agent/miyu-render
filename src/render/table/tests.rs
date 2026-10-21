//! 排表格：列宽加起来一栏宽；表头在第一行、正文隔行标底色；一行多高照最高的格子（格子里的 `<br>` 换行）；少写的格子空着；
//! 右对齐的字贴着右边；十一位的号一行装得下。

use std::path::Path;

use cosmic_text::FontSystem;
use cosmic_text::fontdb::Database;

use super::*;

/// 画 mermaid 图用的字体库：这里的测试不画图，空的就够。
static FONTS: std::sync::LazyLock<std::sync::Arc<Database>> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(Database::new()));
use crate::markdown::blocks;
use crate::theme::Theme;

fn system() -> FontSystem {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts");
    let mut database = Database::new();
    for file in ["cjk.otf", "mono.ttf"] {
        database.load_font_file(dir.join(file)).expect("读得出");
    }
    FontSystem::new_with_locale_and_db("zh-CN".into(), database)
}

fn grid(markdown: &str) -> Grid {
    let look = Look {
        palette: Theme::Paper.palette(),
        size: 36.0,
        body: "Miyu Test CJK",
        code: "Miyu Test Mono",
        code_chinese: false,
        emoji: None,
        fonts: &FONTS,
    };
    let block = blocks(markdown).remove(0);
    lay(&mut system(), block.table.as_ref().expect("是表格"), &look).expect("排得出")
}

#[test]
fn rows_header_stripes_and_widths() {
    let got = grid("| 名 | 数 |\n|---|--:|\n| 你 | 1 |\n| 好 | 2 |\n| 是 | 3 |\n");
    assert_eq!(got.widths.iter().sum::<u32>(), COLUMN);
    let flags: Vec<(bool, bool)> = got
        .rows
        .iter()
        .map(|row| (row.header, row.stripe))
        .collect();
    assert_eq!(
        flags,
        [(true, false), (false, false), (false, true), (false, false)]
    );
    let mut top = 0;
    for row in &got.rows {
        assert_eq!(row.top, top);
        assert!(row.height > PAD_Y * 2);
        top += row.height;
    }
    assert_eq!(got.height, top + 1, "最下面一条线");
}

#[test]
fn a_break_in_a_cell_makes_the_row_taller() {
    let got = grid("| 名 |\n|---|\n| 你 |\n| 你<br>好 |\n");
    assert!(
        got.rows[2].height > got.rows[1].height,
        "{} {}",
        got.rows[2].height,
        got.rows[1].height
    );
}

#[test]
fn missing_cells_stay_empty() {
    let got = grid("| a | b | c |\n|---|---|---|\n| 1 |\n");
    assert_eq!(
        got.rows[1]
            .cells
            .iter()
            .map(Option::is_some)
            .collect::<Vec<_>>(),
        [true, false, false]
    );
}

#[test]
fn right_aligned_text_sits_at_the_right() {
    let got = grid("| 名 | 数 |\n|---|--:|\n| 你 | 1 |\n");
    let buffer = got.rows[1].cells[1].as_ref().expect("有字");
    let glyph = buffer.layout_runs().next().expect("一行").glyphs[0].clone();
    let inner = got.widths[1] - PAD * 2;
    assert!(
        glyph.x + glyph.w > units::float(inner) - 2.0,
        "{} {} {inner}",
        glyph.x,
        glyph.w
    );
}

#[test]
fn a_long_number_stays_on_one_line() {
    // 五列：等分一列 192，十一位的号连留空装不下；照内容分，长说明那一列让出来。
    let got = grid(
        "| 一段很长很长的说明文字一段很长很长的说明文字一段很长很长的说明文字一段很长很长的说明文字 | a | b | c | 号 |\n|---|---|---|---|---|\n| 你 | 1 | 2 | 3 | 12345678901 |\n",
    );
    let buffer = got.rows[1].cells[4].as_ref().expect("有字");
    assert_eq!(
        buffer.layout_runs().count(),
        1,
        "十一位的号不劈开：{:?}",
        got.widths
    );
}
