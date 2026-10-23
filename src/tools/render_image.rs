//! `render_image`：画好的图存进核心给的 `pictures`，没给的存这一轮的工作区（`rendered/`），文件名 `render-<哈希前 12 位>-<页>.png`，交回每一页的路径和长宽，不交给核心、
//! 不发给谁（2026-10-17 项目主人定：发不发、怎么发是她的事）。同一份字、同样的配置画过的不重画。
//!
//! 这个进程不在沙盒里，场所会话只碰得到自己的工作区：读的 `.md` 和存图的文件夹都照真实的位置认，要在工作区里。

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::saved;
use crate::methods::{self, MAX_CHARS, State};
use crate::settings::Settings;

/// 读的 `.md` 最大多少字节：每个字最多 4 个字节，再大的一定超过 [`MAX_CHARS`]。
const MAX_BYTES: u64 = MAX_CHARS as u64 * 4;

/// 办一次：交回给她看的结果（一页一行：路径、长宽），失败的交回为什么。
pub fn run(state: &State, params: &Value) -> Result<String, String> {
    let cwd = params["cwd"]
        .as_str()
        .filter(|cwd| !cwd.is_empty())
        .ok_or("No workspace for this turn.")?;
    let workspace =
        fs::canonicalize(cwd).map_err(|error| format!("Cannot open the workspace: {error}."))?;
    let text = source(&workspace, &params["args"])?;
    let text = methods::checked(Some(&text)).map_err(|why| format!("{}.", capital(&why)))?;
    let settings = methods::settings(state);
    let key = key(&settings, text);
    let (folder, shown) = match params["pictures"].as_str().filter(|path| !path.is_empty()) {
        Some(pictures) => (
            saved::pictures(Path::new(pictures))?,
            PathBuf::from(pictures),
        ),
        None => (
            saved::folder(&workspace)?,
            Path::new(cwd).join(saved::FOLDER),
        ),
    };
    let found = if let Some(found) = saved::find(&folder, &key) {
        found
    } else {
        let pages = methods::pages(state, text, &settings, settings.theme)
            .map_err(|why| format!("{}.", capital(&why)))?;
        saved::store(&folder, &key, &pages)?
    };
    Ok(found
        .iter()
        .map(|page| {
            format!(
                "{} {}x{}",
                shown.join(&page.name).display(),
                page.width,
                page.height
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// 要画的字：`markdown` 和 `path` 二选一。
fn source(workspace: &Path, args: &Value) -> Result<String, String> {
    let given = |key: &str| args[key].as_str().filter(|value| !value.trim().is_empty());
    match (given("markdown"), given("path")) {
        (Some(_), Some(_)) => Err("Give markdown or path, not both.".into()),
        (Some(markdown), None) => Ok(markdown.to_string()),
        (None, Some(path)) => read(workspace, path),
        (None, None) => Err("Give markdown or path.".into()),
    }
}

/// 读工作区里的 `.md`、`.markdown`：照真实的位置认（不走链接绕出去）。
fn read(workspace: &Path, path: &str) -> Result<String, String> {
    let real: PathBuf = fs::canonicalize(workspace.join(path))
        .map_err(|error| format!("Cannot read {path}: {error}."))?;
    if !real.starts_with(workspace) {
        return Err(format!("{path} is outside the workspace."));
    }
    let markdown = real
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        });
    if !markdown || !real.is_file() {
        return Err(format!("{path} is not a .md or .markdown file."));
    }
    let size = fs::metadata(&real)
        .map_err(|error| format!("Cannot read {path}: {error}."))?
        .len();
    if size > MAX_BYTES {
        return Err(format!("{path} is longer than {MAX_CHARS} characters."));
    }
    fs::read_to_string(&real).map_err(|error| format!("Cannot read {path}: {error}."))
}

/// 文件名里的哈希：程序的版本、配色、字体、字号、字一样的才算画过（换了配置、升了级的重画）。
fn key(settings: &Settings, text: &str) -> String {
    let mut hasher = Sha256::new();
    for part in [
        env!("CARGO_PKG_VERSION"),
        settings.theme.name(),
        settings.font.as_deref().unwrap_or_default(),
        settings.code_font.as_deref().unwrap_or_default(),
        &settings.font_size.to_string(),
        text,
    ] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hasher
        .finalize()
        .iter()
        .take(6)
        .fold(String::new(), |mut hex, byte| {
            // 往 String 里写不会失败。
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// 头一个字母大写（给她看的报错是句子）。
fn capital(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
