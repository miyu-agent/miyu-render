//! 排一块：标题比正文高、代码块和引用留空、列表一层层往里缩、任务项有勾选框且字让开、分割线、行内代码带记号、粗体要粗字形、
//! 链接是强调色、删除线是淡字；着色的代码照配色、注释是淡字加斜体。

use std::path::Path;

use cosmic_text::FontSystem;
use cosmic_text::fontdb::Database;

use super::*;

/// 画 mermaid 图用的字体库：这里的测试不画图，空的就够。
static FONTS: std::sync::LazyLock<std::sync::Arc<Database>> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(Database::new()));
use crate::markdown::blocks;
use crate::render::COLUMN;
use crate::theme::Theme;

fn system() -> FontSystem {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts");
    let mut database = Database::new();
    for file in ["cjk.otf", "mono.ttf", "mono-cjk.otf"] {
        database.load_font_file(dir.join(file)).expect("读得出");
    }
    FontSystem::new_with_locale_and_db("zh-CN".into(), database)
}

fn look() -> Look<'static> {
    Look {
        palette: Theme::Paper.palette(),
        size: 36.0,
        body: "Miyu Test CJK",
        code: "Miyu Test Mono",
        code_chinese: false,
        emoji: None,
        fonts: &FONTS,
        column: COLUMN,
    }
}

/// 排 `markdown` 的每一块。
fn laid(markdown: &str) -> Vec<Laid> {
    let mut system = system();
    blocks(markdown)
        .iter()
        .map(|block| lay(&mut system, block, &look()))
        .collect()
}

/// 一块里所有字形。
fn glyphs(laid: &Laid) -> Vec<cosmic_text::LayoutGlyph> {
    laid.buffer
        .iter()
        .flat_map(|buffer| buffer.layout_runs().flat_map(|run| run.glyphs.to_vec()))
        .collect()
}

#[test]
fn headings_are_taller_and_bold() {
    let got = laid("# 你好\n\n你好");
    assert!(
        got[0].height > got[1].height,
        "{} {}",
        got[0].height,
        got[1].height
    );
    assert!(
        glyphs(&got[0])
            .iter()
            .all(|glyph| glyph.font_weight == Weight::BOLD)
    );
    assert!(got[0].margin.0 > got[1].margin.0, "一级标题上面空得多");
}

#[test]
fn code_and_quotes_leave_room_around_their_text() {
    let got = laid("```\ncode\n```\n\n> 你好\n\n你好");
    let (code, quote, plain) = (&got[0], &got[1], &got[2]);
    assert_eq!(code.inset, CODE_PAD);
    assert!(code.height >= CODE_PAD.1 * 2 + 30, "{}", code.height);
    assert_eq!(quote.inset, (QUOTE.1, QUOTE.2));
    assert_eq!(quote.height, plain.height + QUOTE.2 * 2);
}

#[test]
fn lists_indent_by_depth_and_tasks_get_a_box() {
    let got = laid("- 一\n  - 二\n- [x] 好了\n");
    assert_eq!(got[0].inset.0, 0);
    assert_eq!(got[1].inset.0, 36);
    let (done, (x, _), side) = got[2].task.expect("有勾选框");
    assert!(done);
    assert_eq!(x, 0);
    assert!(got[2].inset.0 > side, "字让开勾选框");
    assert!(got[0].task.is_none());
}

#[test]
fn a_rule_has_no_text() {
    let got = laid("---");
    assert_eq!(
        (got[0].kind, got[0].buffer.is_none(), got[0].height),
        (Kind::Rule, true, 36)
    );
}

#[test]
fn inline_styles_reach_the_glyphs() {
    let got = laid("你 `code` **好** [是](https://a.invalid) ~~的~~");
    let all = glyphs(&got[0]);
    let palette = Theme::Paper.palette();
    let colour = |rgba: [u8; 4]| cosmic_text::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]);
    assert_eq!(
        all.iter()
            .filter(|glyph| glyph.metadata == INLINE_CODE)
            .count(),
        4,
        "行内代码四个字形带记号"
    );
    assert!(
        all.iter().any(|glyph| glyph.font_weight == Weight::BOLD),
        "粗体"
    );
    assert!(
        all.iter()
            .any(|glyph| glyph.color_opt == Some(colour(palette.accent))),
        "链接是强调色"
    );
    assert!(
        all.iter()
            .any(|glyph| glyph.color_opt == Some(colour(palette.muted))),
        "删除线是淡字"
    );
}

/// 排 `markdown`，交回每个不是空白的字用了哪个字体族、带不带行内代码的记号。
fn families_of(markdown: &str, look: &Look) -> Vec<(char, String, usize)> {
    let mut system = system();
    let got: Vec<Laid> = blocks(markdown)
        .iter()
        .map(|block| lay(&mut system, block, look))
        .collect();
    let mut seen = Vec::new();
    for laid in &got {
        for run in laid.buffer.as_ref().expect("有字").layout_runs() {
            for glyph in run.glyphs {
                let ch = run.text[glyph.start..glyph.end]
                    .chars()
                    .next()
                    .expect("有字");
                if ch.is_whitespace() {
                    continue;
                }
                let family = system
                    .db()
                    .face(glyph.font_id)
                    .map(|face| face.families[0].0.clone())
                    .unwrap_or_default();
                seen.push((ch, family, glyph.metadata));
            }
        }
    }
    seen
}

#[test]
fn chinese_in_code_uses_the_code_font_when_it_has_chinese() {
    // 等宽字体自带中文（这台机器上的 JetBrains Maple Mono 就是）：代码里的中文也用它，和英文按格对齐（2026-10-21 项目主人定，
    // 改掉 R-2 上「用正文字体」那一条）。
    let look = Look {
        code: "Miyu Test Mono CJK",
        code_chinese: true,
        ..look()
    };
    let mono = "Miyu Test Mono CJK".to_string();
    assert_eq!(
        families_of("`代码 code`\n\n```\n你 x\n```", &look),
        [
            ('代', mono.clone(), INLINE_CODE),
            ('码', mono.clone(), INLINE_CODE),
            ('c', mono.clone(), INLINE_CODE),
            ('o', mono.clone(), INLINE_CODE),
            ('d', mono.clone(), INLINE_CODE),
            ('e', mono.clone(), INLINE_CODE),
            ('你', mono.clone(), 0),
            ('x', mono, 0),
        ],
        "中日韩字用代码字体、照样垫代码的底；代码块不垫"
    );
}

#[test]
fn chinese_in_code_falls_back_to_the_body_font_without_chinese() {
    let cjk = "Miyu Test CJK".to_string();
    let mono = "Miyu Test Mono".to_string();
    assert_eq!(
        families_of("`代 c`\n\n```\n你 x\n```", &look()),
        [
            ('代', cjk.clone(), INLINE_CODE),
            ('c', mono.clone(), INLINE_CODE),
            ('你', cjk, 0),
            ('x', mono, 0),
        ],
        "代码字体没有中文的，中日韩字回退正文字体、照样垫代码的底"
    );
}

#[test]
fn emoji_in_a_heading_stay_regular() {
    // 彩色表情字体只有常规体：要粗的会挑到黑白的表情字体（R-2 下实测）。
    let got = laid("# 好🎉");
    let weights: Vec<Weight> = glyphs(&got[0])
        .iter()
        .map(|glyph| glyph.font_weight)
        .collect();
    assert_eq!(weights.first(), Some(&Weight::BOLD));
    assert_eq!(weights.last(), Some(&Weight::NORMAL), "{weights:?}");
}

#[test]
fn a_formula_that_cannot_be_drawn_falls_back_to_unicode_text() {
    let got = laid("$$x^2 + \\frac{a}$$");
    assert_eq!(got[0].kind, Kind::Paragraph);
    let text: String = got[0]
        .buffer
        .as_ref()
        .expect("有字")
        .lines
        .iter()
        .map(|line| line.text().to_string())
        .collect();
    assert_eq!(text, crate::markdown::math::unicode("x^2 + \\frac{a}"));
    assert!(text.starts_with("x²"), "{text}");
}

/// 画不出来的公式退回 Unicode 写法时也去掉 `%` 注释（审计 #136 收尾）：注释的字不显示，注释后面那几行照样在。
#[test]
fn audit_136_a_formula_falling_back_to_text_drops_its_comments() {
    let got = laid("$$\nx % note\n= y + \\frac{a}\n$$");
    assert_eq!(got[0].kind, Kind::Paragraph, "分式缺分母，画不出来");
    let text: String = got[0]
        .buffer
        .as_ref()
        .expect("有字")
        .lines
        .iter()
        .map(|line| line.text().to_string())
        .collect();
    assert!(!text.contains("note") && !text.contains('%'), "{text}");
    assert!(text.contains("= y"), "{text}");
}

#[test]
fn highlighted_code_takes_palette_colours_and_comments_slant() {
    let got = laid("```rust\nlet n = 1; // hi\n```");
    let all = glyphs(&got[0]);
    let palette = Theme::Paper.palette();
    let colour = |rgba: [u8; 4]| Some(cosmic_text::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]));
    let slanted = |glyph: &&cosmic_text::LayoutGlyph| {
        glyph
            .cache_key_flags
            .contains(cosmic_text::CacheKeyFlags::FAKE_ITALIC)
    };
    let comment: Vec<_> = all
        .iter()
        .filter(|glyph| glyph.color_opt == colour(palette.muted))
        .collect();
    assert_eq!(comment.len(), "// hi".len(), "注释是淡字");
    assert!(comment.iter().all(slanted), "注释斜");
    let keyword: Vec<_> = all
        .iter()
        .filter(|glyph| glyph.color_opt == colour(palette.accent))
        .collect();
    assert!(keyword.len() >= "let".len(), "关键字是强调色");
    assert!(!keyword.iter().any(slanted), "别的不斜");
    assert!(
        all.iter()
            .any(|glyph| glyph.color_opt == colour(palette.code_constant())),
        "数"
    );
}

/// 审计 #137：列表每深一层缩 36 像素、不封顶，第 28 层起缩进超过一栏宽（960），字的宽归零、起点在画布外。
#[test]
fn audit_137_a_very_deep_list_item_still_has_its_text_inside_the_column() {
    let mut markdown = String::new();
    for level in 0..30 {
        markdown.push_str(&"  ".repeat(level));
        markdown.push_str("- 第");
        markdown.push_str(&(level + 1).to_string());
        markdown.push_str("层\n");
    }
    let got = laid(&markdown);
    let deepest = got.last().expect("有块");
    assert_eq!(
        deepest.kind,
        Kind::Item { depth: 30 },
        "测试要真排到第 30 层"
    );
    let found = glyphs(deepest);
    assert!(!found.is_empty(), "最深那项要有字");
    #[expect(clippy::cast_precision_loss, reason = "几千像素以内")]
    let (left, column) = (deepest.inset.0 as f32, COLUMN as f32);
    for glyph in &found {
        assert!(
            left + glyph.x + glyph.w <= column,
            "最深那项的字落在栏外：缩进 {left}，字在 {}..{}，栏宽 {column}",
            glyph.x,
            glyph.x + glyph.w
        );
    }
}

/// 审计 #137：表格列多到 34 列以上时每列分到的宽不够两边留空（14 × 2），给字的宽是 0，字挤出自己的格子、压到旁边。
/// 画的时候字从格子左边 `pad` 处开始（`draw.rs`，列多的表留空跟着收窄），所以每格的字要落在 `pad..宽-pad` 里，整张表不超过一栏宽。
#[test]
fn audit_137_a_table_with_many_columns_keeps_each_text_inside_its_cell() {
    let count = 40;
    let header = vec!["h"; count].join(" | ");
    let rule = vec!["-"; count].join(" | ");
    let row = vec!["1"; count].join(" | ");
    let got = laid(&format!("| {header} |\n| {rule} |\n| {row} |\n"));
    let grid = got[0].grid.as_ref().expect("排成表格");
    assert_eq!(grid.widths.len(), count, "测试要真排出 {count} 列");
    assert!(
        grid.widths.iter().sum::<u32>() <= COLUMN,
        "整张表不超过一栏宽"
    );
    let mut seen = 0;
    for row in &grid.rows {
        for (width, cell) in grid.widths.iter().zip(&row.cells) {
            let Some(buffer) = cell else { continue };
            #[expect(clippy::cast_precision_loss, reason = "几千像素以内")]
            let room = width.saturating_sub(grid.pad * 2) as f32;
            for run in buffer.layout_runs() {
                for glyph in run.glyphs {
                    seen += 1;
                    assert!(
                        glyph.x >= 0.0 && glyph.x + glyph.w <= room,
                        "列宽 {width}，给字的宽 {room}，字在 {}..{}，挤出了格子",
                        glyph.x,
                        glyph.x + glyph.w
                    );
                }
            }
        }
    }
    assert!(seen >= count * 2, "每格的字都要排出来：{seen}");
}

/// 列表缩进封顶（审计 #137）：头 14 层照旧每层缩 36，再深的不再往右缩，给字的宽总有半栏多。
#[test]
fn list_indent_stops_growing_past_the_cap() {
    let mut markdown = String::new();
    for level in 0..20 {
        markdown.push_str(&"  ".repeat(level));
        markdown.push_str("- 字\n");
    }
    let insets: Vec<u32> = laid(&markdown).iter().map(|laid| laid.inset.0).collect();
    let expected: Vec<u32> = (0..20u32).map(|level| level.min(13) * 36).collect();
    assert_eq!(insets, expected);
    assert!(COLUMN - insets[19] >= COLUMN / 2, "给字的宽至少半栏");
}
