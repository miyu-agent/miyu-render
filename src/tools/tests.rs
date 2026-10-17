//! 登记：一件，名字、说明最多三句的英文、访问类别、三种场所、显示名；不认识的工具名、没有工作区的回失败。

use serde_json::json;

use super::*;
use crate::test_support::setup;

#[test]
fn one_tool_with_a_short_english_description() {
    let provided = provide();
    let tools = provided["tools"].as_array().expect("是数组");
    assert_eq!(tools.len(), 1);
    let tool = &tools[0];
    assert_eq!(tool["name"], "render_image");
    let description = tool["description"].as_str().expect("有说明");
    assert!(
        description.matches(". ").count() < 3,
        "最多三句：{description}"
    );
    assert!(description.is_ascii(), "给模型看的写英文");
    assert!(tool["input_schema"].to_string().is_ascii());
    assert_eq!(tool["access"], "write", "往工作区写文件");
    assert_eq!(tool["venues"], json!(["local", "private", "group"]));
    assert_eq!(tool["display_name"]["zh"], "渲染图片");
}

#[test]
fn unknown_tools_and_missing_workspaces_fail() {
    let (state, _) = setup(false, &["cjk.otf"]);
    let got = call(&state, &json!({"tool": "nope", "args": {}, "cwd": "/"}));
    assert_eq!(got["error"], true);
    assert_eq!(got["blocks"][0]["text"], "No tool named nope.");
    let got = call(
        &state,
        &json!({"tool": "render_image", "args": {"markdown": "a"}}),
    );
    assert_eq!(got["error"], true);
    assert_eq!(got["blocks"][0]["text"], "No workspace for this turn.");
    let got = call(
        &state,
        &json!({"tool": "render_image", "args": {"markdown": "a"}, "cwd": ""}),
    );
    assert_eq!(
        got["blocks"][0]["text"], "No workspace for this turn.",
        "空的也是没有"
    );
}
