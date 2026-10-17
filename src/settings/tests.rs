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
