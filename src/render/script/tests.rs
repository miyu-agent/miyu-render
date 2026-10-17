//! 字体类别：代码里的中日韩字单独切出来（正文里的不切）；表情单独切出来，连接符、变体选择符跟着走；相邻同样的并在一起。

use super::*;

fn spans(list: &[(&str, bool)]) -> Vec<Span> {
    list.iter()
        .map(|(text, code)| Span {
            text: (*text).to_string(),
            style: Style {
                code: *code,
                ..Style::default()
            },
        })
        .collect()
}

fn kinds(got: &[(String, Style, Script)]) -> Vec<(&str, Script)> {
    got.iter()
        .map(|(text, _, script)| (text.as_str(), *script))
        .collect()
}

#[test]
fn cjk_is_split_only_inside_code() {
    let got = split(&spans(&[("你好 ab", false), ("代码 cd", true)]), false);
    assert_eq!(
        kinds(&got),
        [
            ("你好 ab", Script::Plain),
            ("代码", Script::Cjk),
            (" cd", Script::Plain)
        ]
    );
    let got = split(&spans(&[("你 x", false)]), true);
    assert_eq!(
        kinds(&got),
        [("你", Script::Cjk), (" x", Script::Plain)],
        "代码块整段都是代码"
    );
}

#[test]
fn emoji_and_their_joiners_go_together() {
    let got = split(
        &spans(&[("好🎉的👨\u{200D}👩\u{200D}👧☀\u{FE0F}", false)]),
        false,
    );
    assert_eq!(
        kinds(&got),
        [
            ("好", Script::Plain),
            ("🎉", Script::Emoji),
            ("的", Script::Plain),
            ("👨\u{200D}👩\u{200D}👧☀\u{FE0F}", Script::Emoji)
        ]
    );
    let got = split(&spans(&[("a\u{FE0F}", false)]), false);
    assert_eq!(
        kinds(&got),
        [("a\u{FE0F}", Script::Plain)],
        "前面不是表情的变体选择符照常"
    );
}
