//! 配置项：认得的换上、`null` 和写错的回出厂值、别的包的不理；字号夹在范围里；交回字体换没换。

use serde_json::json;

use super::*;

fn keys(value: &serde_json::Value) -> Map<String, Value> {
    value.as_object().expect("对象").clone()
}

#[test]
fn known_keys_change_and_the_rest_are_ignored() {
    let mut settings = Settings::default();
    let moved = settings.apply(&keys(&json!({
        "miyu-render.theme": "dark",
        "miyu-render.font": " Noto Sans CJK SC ",
        "miyu-render.font_size": 40,
        "miyu-memes.font": "x",
        "miyu-render.unknown": 1,
    })));
    assert!(moved, "字体换了");
    assert_eq!(settings.theme, Theme::Dark);
    assert_eq!(settings.font.as_deref(), Some("Noto Sans CJK SC"));
    assert_eq!(settings.code_font, None);
    assert_eq!(settings.font_size, 40);
    assert!(
        !settings.apply(&keys(&json!({"miyu-render.theme": "light"}))),
        "只换主题不算字体换了"
    );
}

#[test]
fn null_and_wrong_values_fall_back() {
    let mut settings = Settings::default();
    settings.apply(&keys(
        &json!({"miyu-render.theme": "dark", "miyu-render.font": "A", "miyu-render.font_size": 50}),
    ));
    let moved = settings.apply(&keys(&json!({"miyu-render.theme": "sepia", "miyu-render.font": null, "miyu-render.font_size": "big"})));
    assert!(moved);
    assert_eq!(settings, Settings::default());
    settings.apply(&keys(&json!({"miyu-render.font_size": 5})));
    assert_eq!(settings.font_size, 14, "夹在范围里");
    settings.apply(&keys(
        &json!({"miyu-render.font_size": 500, "miyu-render.code_font": "  "}),
    ));
    assert_eq!((settings.font_size, settings.code_font), (56, None));
}

/// 清单里一节 `[settings.<名字>]` 的一个键的那一行（只认 `键 = 值` 写在一行的）。
fn manifest_line(setting: &str, key: &str) -> Option<String> {
    let manifest = include_str!("../../package/package.toml");
    let header = format!("[settings.{setting}]");
    manifest
        .lines()
        .skip_while(|line| line.trim() != header)
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .find(|line| {
            line.split('=')
                .next()
                .is_some_and(|name| name.trim() == key)
        })
        .map(str::to_string)
}

#[test]
fn the_theme_setting_lists_its_choices_the_way_the_core_reads_them() {
    // 核心的清单只认 `choices`（`miyu_config::package`）；以前写成 `options`，装包时整个清单读不进（R-9）。
    assert!(manifest_line("theme", "options").is_none(), "不认 options");
    let line = manifest_line("theme", "choices").expect("theme 写了 choices");
    for theme in [Theme::Paper, Theme::Light, Theme::Dark] {
        assert!(
            line.contains(&format!("\"{}\"", theme.name())),
            "{line} 少了 {}",
            theme.name()
        );
    }
}
