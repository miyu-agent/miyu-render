//! 挑字体（设计 36 第四节第 3 条，2026-10-17 项目主人定：先用系统的，能指定）：正文、代码各一个字体族。
//!
//! - 配置里写了的照写的；没写的照系统的通用名（[`system`]：Linux 问 fontconfig，macOS、Windows 照默认的名字）。
//! - 字体库里没有的名字跳过，照下一个；都没有的：正文另找一个有中文的，代码照正文。
//! - 正文挑到的不含中文的，换成字体库里一个有中文的（名字带 `Sans`、`Hei` 的优先）；一个都没有的报错，说清楚该装什么，不下载。
//! - 记下代码字体有没有中文：有的代码里的中文也用它（2026-10-21 项目主人定），没有的回退正文。
//!
//! 挑法是纯逻辑，字体库经 [`Catalog`] 问：测试换成测试字体，不靠跑测试的机器装了什么。

pub mod system;

/// 一个字体库能答的：有哪些字体族，某个族有没有中文。
pub trait Catalog {
    /// 有哪些字体族，照字体库的先后，不重复。
    fn families(&self) -> Vec<String>;
    /// 名字是 `family` 的族有没有中文（照「中」字认）。
    fn covers_chinese(&self, family: &str) -> bool;
}

/// 系统的通用名换成的几个候选，照先后试。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Generic {
    /// 无衬线（`sans-serif`）。
    pub sans: Vec<String>,
    /// 等宽（`monospace`）。
    pub mono: Vec<String>,
    /// 彩色表情。
    pub emoji: Vec<String>,
}

/// 挑好的两个族。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Families {
    /// 正文。
    pub body: String,
    /// 代码。
    pub code: String,
    /// 代码字体有没有中文：有的代码里的中日韩字也用它，和英文按格对齐；没有的回退正文。
    pub code_chinese: bool,
    /// 彩色表情：字体库里有的话。表情字明确交给它，不让排字自己挑（粗体的标题里会挑到黑白的表情字体）。
    pub emoji: Option<String>,
    /// 挑的时候换过的一句（写进运行日志）：正文想用的不含中文、换成了有中文的。
    pub note: Option<String>,
}

/// 挑不出来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontError {
    /// 字体库里一个有中文的都没有。
    NoChinese,
}

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontError::NoChinese => write!(
                f,
                "no font with Chinese characters is installed; {}",
                system::INSTALL_HINT
            ),
        }
    }
}

/// 挑字体：配置里写的 `body`、`code`（空的当没写），系统的通用名 `generic`，字体库 `catalog`。
///
/// # Errors
///
/// 字体库里一个有中文的都没有：[`FontError::NoChinese`]。
pub fn resolve(
    catalog: &dyn Catalog,
    body: Option<&str>,
    code: Option<&str>,
    generic: &Generic,
) -> Result<Families, FontError> {
    let known = catalog.families();
    let present = |name: &str| {
        known
            .iter()
            .find(|family| family.eq_ignore_ascii_case(name))
            .cloned()
    };
    let first = |wanted: Option<&str>, candidates: &[String]| {
        wanted
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .into_iter()
            .chain(candidates.iter().map(String::as_str))
            .find_map(present)
    };
    let chinese = || {
        let mut with: Vec<&String> = known
            .iter()
            .filter(|family| catalog.covers_chinese(family))
            .collect();
        // 名字带 Sans、Hei 的排前面（黑体、无衬线），别的照字体库的先后。
        with.sort_by_key(|family| !(family.contains("Sans") || family.contains("Hei")));
        with.first().map(|family| (*family).clone())
    };
    let (body, note) = match first(body, &generic.sans) {
        Some(chosen) if catalog.covers_chinese(&chosen) => (chosen, None),
        Some(chosen) => {
            let swapped = chinese().ok_or(FontError::NoChinese)?;
            let note = format!("font {chosen} has no Chinese characters; using {swapped}");
            (swapped, Some(note))
        }
        None => (chinese().ok_or(FontError::NoChinese)?, None),
    };
    let code = first(code, &generic.mono).unwrap_or_else(|| body.clone());
    let code_chinese = catalog.covers_chinese(&code);
    let emoji = first(None, &generic.emoji);
    Ok(Families {
        body,
        code,
        code_chinese,
        emoji,
        note,
    })
}

#[cfg(test)]
mod tests;
