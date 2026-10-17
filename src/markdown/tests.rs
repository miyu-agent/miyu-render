//! 读成块：段落和换行、标题、列表的记号和层、任务项、引用、代码、分割线、表格（表头加粗、对齐、格子里的 `<br>`）、链接补地址、
//! 自动链接不补、图只剩说明和地址、删除线是淡字、裸地址上色、空的不要。

use super::*;

/// 一块的字连起来。
fn text(block: &Block) -> String {
    block.spans.iter().map(|span| span.text.as_str()).collect()
}

/// 每一块的种类和字。
fn shape(markdown: &str) -> Vec<(Kind, String)> {
    blocks(markdown)
        .iter()
        .map(|block| (block.kind, text(block)))
        .collect()
}

#[test]
fn paragraphs_keep_their_line_breaks() {
    assert_eq!(
        shape("一行\n二行\n\n第二段"),
        [
            (Kind::Paragraph, "一行\n二行".into()),
            (Kind::Paragraph, "第二段".into())
        ]
    );
}

#[test]
fn headings_lists_quotes_code_and_rules() {
    let got = shape(
        "# 标题\n\n1. 一\n2. 二\n   - 里面\n\n   续段\n\n> 引\n\n```rust\nfn main() {}\n```\n\n---\n",
    );
    assert_eq!(
        got,
        [
            (Kind::Heading(1), "标题".into()),
            (Kind::Item { depth: 1 }, "1. 一".into()),
            (Kind::Item { depth: 1 }, "2. 二".into()),
            (Kind::Item { depth: 2 }, "• 里面".into()),
            (Kind::Item { depth: 1 }, "    续段".into()),
            (Kind::Quote, "引".into()),
            (Kind::Code, "fn main() {}\n".into()),
            (Kind::Rule, String::new()),
        ]
    );
    assert!(blocks("```\nx\n```")[0].spans[0].style.code, "代码块是等宽");
}

#[test]
fn tasks_drop_the_bullet_and_remember_done() {
    let got = blocks("- [x] 做完了\n- [ ] 没做\n- [ ]\n");
    let tasks: Vec<(String, Option<bool>)> =
        got.iter().map(|block| (text(block), block.task)).collect();
    assert_eq!(
        tasks,
        [
            ("做完了".into(), Some(true)),
            ("没做".into(), Some(false)),
            (String::new(), Some(false))
        ]
    );
}

#[test]
fn inline_styles() {
    let got = blocks("**粗** *斜* `码` ~~删~~ <b>b</b>");
    let styles: Vec<(&str, Style)> = got[0]
        .spans
        .iter()
        .map(|span| (span.text.as_str(), span.style))
        .collect();
    let plain = Style::default();
    assert_eq!(
        styles,
        [
            (
                "粗",
                Style {
                    bold: true,
                    ..plain
                }
            ),
            (" ", plain),
            (
                "斜",
                Style {
                    italic: true,
                    ..plain
                }
            ),
            (" ", plain),
            (
                "码",
                Style {
                    code: true,
                    ..plain
                }
            ),
            (" ", plain),
            (
                "删",
                Style {
                    muted: true,
                    ..plain
                }
            ),
            (" ", plain),
            (
                "b",
                Style {
                    bold: true,
                    ..plain
                }
            ),
        ]
    );
}

#[test]
fn links_show_their_address_once() {
    let got = blocks(
        "[文档](https://x.invalid/doc) <https://y.invalid> ![图](https://z.invalid/a.png) ![](https://w.invalid)",
    );
    assert_eq!(
        text(&got[0]),
        "文档 (https://x.invalid/doc) https://y.invalid 图 (https://z.invalid/a.png) (https://w.invalid)"
    );
    let linked: Vec<&str> = got[0]
        .spans
        .iter()
        .filter(|span| span.style.link)
        .map(|span| span.text.as_str())
        .collect();
    assert_eq!(
        linked,
        [
            "https://x.invalid/doc",
            "https://y.invalid",
            "https://z.invalid/a.png",
            "https://w.invalid"
        ],
        "只有地址上链接色"
    );
    let bare = blocks("看 https://a.invalid。");
    assert!(
        bare[0]
            .spans
            .iter()
            .any(|span| span.style.link && span.text == "https://a.invalid")
    );
}

#[test]
fn tables_have_a_bold_header_aligns_and_cell_breaks() {
    let got = blocks("| 名 | 数 |\n|:--|--:|\n| 甲<br>乙 | 1 |\n");
    assert_eq!(got.len(), 1);
    let table = got[0].table.as_ref().expect("是表格");
    assert_eq!(table.aligns, [Align::Left, Align::Right]);
    assert!(
        table.header[0].iter().all(|span| span.style.bold),
        "表头加粗"
    );
    let cell: String = table.rows[0][0]
        .iter()
        .map(|span| span.text.as_str())
        .collect();
    assert_eq!(cell, "甲\n乙");
    assert!(!table.rows[0][0][0].style.bold, "正文不加粗");
}

#[test]
fn nothing_to_draw_is_no_block() {
    assert!(blocks("").is_empty());
    assert!(blocks("\n\n  \n").is_empty());
    assert!(blocks("<!-- 只有注释 -->").is_empty());
}

#[test]
fn mermaid_fences_are_diagrams() {
    let got = shape(
        "```mermaid\ngraph TD\nA-->B\n```\n\n```Mermaid title\nx\n```\n\n```rust\nfn x() {}\n```",
    );
    assert_eq!(
        got,
        [
            (Kind::Diagram, "graph TD\nA-->B\n".into()),
            (Kind::Diagram, "x\n".into()),
            (Kind::Code, "fn x() {}\n".into())
        ]
    );
}

#[test]
fn formulas_stand_alone_or_turn_into_unicode_inline() {
    let got = shape("能量 $E=mc^2$ 守恒\n\n$$\\frac{a}{b}$$\n\n下一段");
    assert_eq!(
        got,
        [
            (Kind::Paragraph, "能量 E=mc² 守恒".into()),
            (Kind::Formula, "\\frac{a}{b}".into()),
            (Kind::Paragraph, "下一段".into()),
        ]
    );
}
