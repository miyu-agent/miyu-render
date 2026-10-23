//! 给她用的工具（设计 36 第五节）：`render_image` 一件，经 `provide` 登记。说明、参数给模型看，写英文短句、最多三句；
//! 调用以后才用得上的写进结果里。字由这个包自己登记（`docs/prompt-ledger.md`），改了要重量 token。

mod render_image;
mod saved;

use serde_json::{Value, json};

use crate::methods::State;

/// 工具名。
pub const RENDER_IMAGE: &str = "render_image";

/// `provide` 的参数：给三种场所。往工作区写文件，访问类别是 `write`（只读开着时核心拦下）。
pub fn provide() -> Value {
    json!({"tools": [{
        "name": RENDER_IMAGE,
        "description": "Draw Markdown as PNG images, with tables, Mermaid diagrams, LaTeX formulas and highlighted code. The images are saved as files and nobody sees them until you show them.",
        "input_schema": {"type": "object", "properties": {
            "markdown": {"type": "string"},
            "path": {"type": "string", "description": "A .md file in the workspace, instead of markdown."}
        }},
        "access": "write",
        "venues": ["local", "private", "group"],
        "display_name": {"zh": "渲染图片", "en": "Render image", "ja": "画像にレンダリング"},
    }]})
}

/// 办一次 `tool.call`：`params` 是核心发来的那一份，交回工具结果。
pub fn call(state: &State, params: &Value) -> Value {
    let done = match params["tool"].as_str().unwrap_or_default() {
        RENDER_IMAGE => render_image::run(state, params),
        other => Err(format!("No tool named {other}.")),
    };
    match done {
        Ok(text) => json!({"blocks": [{"type": "text", "text": text}]}),
        Err(why) => json!({"blocks": [{"type": "text", "text": why}], "error": true}),
    }
}

#[cfg(test)]
mod tests;
