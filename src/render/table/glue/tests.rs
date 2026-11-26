//! 粘：数连单位不断；表头连着的中日韩字之间垫连接符、跨段也垫；正文的中文不垫；别的空格不动。

use super::glue;
use crate::markdown::{Span, Style};

fn plain(text: &str) -> Vec<Span> {
    vec![Span {
        text: text.to_string(),
        style: Style::default(),
    }]
}

fn joined(spans: &[Span]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

#[test]
fn a_number_keeps_its_unit() {
    assert_eq!(joined(&glue(&plain("125.7 ms"), false)), "125.7\u{a0}ms");
    assert_eq!(joined(&glue(&plain("3.6 MiB"), false)), "3.6\u{a0}MiB");
    assert_eq!(joined(&glue(&plain("12 %"), false)), "12\u{a0}%");
    // 不是数后面的空格、单位不是字母的不动。
    assert_eq!(joined(&glue(&plain("Claude Code"), false)), "Claude Code");
    assert_eq!(joined(&glue(&plain("3 / 4"), false)), "3 / 4");
}

#[test]
fn header_cjk_runs_stay_whole() {
    let got = joined(&glue(&plain("空闲 CPU / 分钟"), true));
    assert_eq!(got, "空\u{2060}闲 CPU / 分\u{2060}钟");
    let got = joined(&glue(&plain("启动中位数"), true));
    assert_eq!(got.matches('\u{2060}').count(), 4, "{got:?}");
}

#[test]
fn header_glue_crosses_spans() {
    let spans = vec![
        Span {
            text: "空".to_string(),
            style: Style {
                bold: true,
                ..Style::default()
            },
        },
        Span {
            text: "闲".to_string(),
            style: Style::default(),
        },
    ];
    let got = glue(&spans, true);
    assert_eq!(got[0].text, "空");
    assert_eq!(got[1].text, "\u{2060}闲");
    assert!(got[0].style.bold && !got[1].style.bold);
}

#[test]
fn body_cjk_still_folds() {
    assert_eq!(joined(&glue(&plain("长长的一句话"), false)), "长长的一句话");
}
