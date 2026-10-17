//! 段落：空行分段；软换行、硬换行都换行；行内代码、强调照字；标题、列表项、代码块各算一段；空白的段不要。

use super::paragraphs;

#[test]
fn blank_lines_split_and_line_breaks_stay() {
    assert_eq!(
        paragraphs("第一行\n第二行\n\n第二段  \n硬换行"),
        ["第一行\n第二行", "第二段\n硬换行"]
    );
}

#[test]
fn inline_marks_keep_their_text() {
    assert_eq!(
        paragraphs("有 **粗体**、*斜体*、`code` 和 [链接](https://x.invalid)"),
        ["有 粗体、斜体、code 和 链接"]
    );
}

#[test]
fn headings_items_and_code_are_paragraphs_of_their_own() {
    let got = paragraphs("# 标题\n正文\n\n- 一\n- 二\n\n```\nfn main() {}\n```\n");
    assert_eq!(got, ["标题", "正文", "一", "二", "fn main() {}"]);
}

#[test]
fn nothing_to_draw_is_empty() {
    assert!(paragraphs("").is_empty());
    assert!(paragraphs("\n\n   \n").is_empty());
    assert!(paragraphs("---").is_empty());
}
