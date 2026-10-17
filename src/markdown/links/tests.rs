//! 裸地址：句尾标点、配不上对的括号不算；中文标点截断；粘连的不认；光有协议头不算；行内代码里的不碰。

use super::*;

fn url(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    (0..chars.len()).find_map(|index| bare_url_at(&chars, index))
}

#[test]
fn tails_and_brackets() {
    assert_eq!(url("见 https://a.com。").as_deref(), Some("https://a.com"));
    assert_eq!(
        url("see https://a.com/x.").as_deref(),
        Some("https://a.com/x"),
        "英文句尾的标点也不算"
    );
    assert_eq!(
        url("(see https://a.com/x)").as_deref(),
        Some("https://a.com/x"),
        "配不上对的右括号不算"
    );
    assert_eq!(url("(https://a.com/x)").as_deref(), Some("https://a.com/x"));
    assert_eq!(
        url("https://en.wikipedia.org/wiki/Rust_(language)").as_deref(),
        Some("https://en.wikipedia.org/wiki/Rust_(language)")
    );
    assert_eq!(
        url("https://archlinux.org、AUR").as_deref(),
        Some("https://archlinux.org")
    );
    assert_eq!(url("xhttps://a.com"), None);
    assert_eq!(url("https://"), None);
    assert_eq!(url("HTTPS://A.COM").as_deref(), Some("HTTPS://A.COM"));
}

#[test]
fn only_the_address_is_marked_and_code_is_left_alone() {
    let mut spans = vec![
        Span {
            text: "看 https://a.com 和 ".into(),
            style: Style::default(),
        },
        Span {
            text: "https://b.com".into(),
            style: Style {
                code: true,
                ..Style::default()
            },
        },
    ];
    mark(&mut spans);
    let marked: Vec<(&str, bool, bool)> = spans
        .iter()
        .map(|span| (span.text.as_str(), span.style.link, span.style.code))
        .collect();
    assert_eq!(
        marked,
        [
            ("看 ", false, false),
            ("https://a.com", true, false),
            (" 和 ", false, false),
            ("https://b.com", false, true)
        ]
    );
}
