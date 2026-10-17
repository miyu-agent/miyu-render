//! 挑字体：写了的、系统的通用名照先后；没有的名字跳过；正文不含中文的换成有中文的（黑体、无衬线优先）；一个有中文的都没有报错；
//! 代码没有的照正文；大小写不论；空的名字当没写。字体库是假的（只说有哪些族、哪个有中文）。

use super::*;

/// 假的字体库：族名和有没有中文。
struct Fake(Vec<(&'static str, bool)>);

impl Catalog for Fake {
    fn families(&self) -> Vec<String> {
        self.0.iter().map(|(name, _)| (*name).to_string()).collect()
    }
    fn covers_chinese(&self, family: &str) -> bool {
        self.0
            .iter()
            .any(|(name, chinese)| *name == family && *chinese)
    }
}

fn generic(sans: &[&str], mono: &[&str]) -> Generic {
    let owned = |list: &[&str]| list.iter().map(|name| (*name).to_string()).collect();
    Generic {
        sans: owned(sans),
        mono: owned(mono),
        emoji: vec!["Noto Color Emoji".into()],
    }
}

fn machine() -> Fake {
    Fake(vec![
        ("DejaVu Sans", false),
        ("Noto Serif CJK SC", true),
        ("Noto Sans CJK SC", true),
        ("JetBrains Mono", false),
    ])
}

#[test]
fn written_names_win_then_the_system_ones() {
    let system = generic(&["Noto Sans CJK SC"], &["JetBrains Mono"]);
    let got = resolve(&machine(), None, None, &system).expect("挑得出");
    assert_eq!(
        (got.body.as_str(), got.code.as_str(), got.note),
        ("Noto Sans CJK SC", "JetBrains Mono", None)
    );
    let got = resolve(
        &machine(),
        Some("noto serif cjk sc"),
        Some("DejaVu Sans"),
        &system,
    )
    .expect("挑得出");
    assert_eq!(
        (got.body.as_str(), got.code.as_str()),
        ("Noto Serif CJK SC", "DejaVu Sans"),
        "大小写不论"
    );
    let got = resolve(&machine(), Some("  "), Some(""), &system).expect("挑得出");
    assert_eq!(got.body, "Noto Sans CJK SC", "空的当没写");
}

#[test]
fn missing_names_are_skipped() {
    let system = generic(
        &["PingFang SC", "Noto Sans CJK SC"],
        &["Menlo", "JetBrains Mono"],
    );
    let got = resolve(&machine(), Some("Nope"), Some("Nope Mono"), &system).expect("挑得出");
    assert_eq!(
        (got.body.as_str(), got.code.as_str()),
        ("Noto Sans CJK SC", "JetBrains Mono")
    );
}

#[test]
fn a_body_without_chinese_is_swapped_for_one_with_it() {
    let got = resolve(&machine(), None, None, &generic(&["DejaVu Sans"], &[])).expect("挑得出");
    assert_eq!(
        got.body, "Noto Sans CJK SC",
        "黑体、无衬线优先，排在前面的宋体不挑"
    );
    assert_eq!(
        got.note.as_deref(),
        Some("font DejaVu Sans has no Chinese characters; using Noto Sans CJK SC")
    );
    assert_eq!(got.code, "Noto Sans CJK SC", "代码没有的照正文");
    let got = resolve(&machine(), None, None, &generic(&[], &[])).expect("挑得出");
    assert_eq!(
        (got.body.as_str(), got.note),
        ("Noto Sans CJK SC", None),
        "都没有的另找一个有中文的"
    );
}

#[test]
fn no_chinese_font_at_all_is_an_error() {
    let latin = Fake(vec![("DejaVu Sans", false), ("DejaVu Sans Mono", false)]);
    let got = resolve(
        &latin,
        None,
        None,
        &generic(&["DejaVu Sans"], &["DejaVu Sans Mono"]),
    );
    assert_eq!(got, Err(FontError::NoChinese));
    assert!(
        FontError::NoChinese
            .to_string()
            .starts_with("no font with Chinese characters is installed; ")
    );
    assert_eq!(
        resolve(&Fake(vec![]), None, None, &Generic::default()),
        Err(FontError::NoChinese)
    );
}

#[test]
fn the_system_list_is_never_empty() {
    let found = system::generic();
    assert!(!found.sans.is_empty() && !found.mono.is_empty());
}

#[test]
fn the_colour_emoji_font_is_picked_when_installed() {
    let without =
        resolve(&machine(), None, None, &generic(&["Noto Sans CJK SC"], &[])).expect("挑得出");
    assert_eq!(without.emoji, None);
    let mut with = machine();
    with.0.push(("Noto Color Emoji", false));
    let got = resolve(&with, None, None, &generic(&["Noto Sans CJK SC"], &[])).expect("挑得出");
    assert_eq!(got.emoji.as_deref(), Some("Noto Color Emoji"));
}
