//! 配置项（设计 36 第七节）：清单 `[settings]` 的四项，握手交来的、推来的照换。键是 `miyu-render.<名字>`。

use serde_json::{Map, Value};

use crate::theme::Theme;

/// 包的编号：配置项的键以它开头。
const PACKAGE: &str = "miyu-render";

/// 字号的出厂值和范围（同旧版）。
pub const FONT_SIZE: (u32, u32, u32) = (36, 14, 56);

/// 四项这一刻的值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// 默认用哪一套配色。
    pub theme: Theme,
    /// 正文的字体族名，没写的照系统的无衬线。
    pub font: Option<String>,
    /// 代码的字体族名，没写的照系统的等宽。
    pub code_font: Option<String>,
    /// 正文的字号，像素。
    pub font_size: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            theme: Theme::default(),
            font: None,
            code_font: None,
            font_size: FONT_SIZE.0,
        }
    }
}

impl Settings {
    /// 照 `keys` 换：认得的换上，`null`、写错的回出厂值，别的不理。交回字体换没换（换了要重新挑）。
    pub fn apply(&mut self, keys: &Map<String, Value>) -> bool {
        let fonts = (self.font.clone(), self.code_font.clone());
        let fallback = Settings::default();
        for (key, value) in keys {
            let Some(name) = key
                .strip_prefix(PACKAGE)
                .and_then(|rest| rest.strip_prefix('.'))
            else {
                continue;
            };
            match name {
                "theme" => {
                    self.theme = value
                        .as_str()
                        .and_then(Theme::parse)
                        .unwrap_or(fallback.theme);
                }
                "font" => self.font = text(value),
                "code_font" => self.code_font = text(value),
                "font_size" => {
                    self.font_size = value
                        .as_u64()
                        .and_then(|size| u32::try_from(size).ok())
                        .map_or(fallback.font_size, |size| {
                            size.clamp(FONT_SIZE.1, FONT_SIZE.2)
                        });
                }
                _ => {}
            }
        }
        (self.font.clone(), self.code_font.clone()) != fonts
    }
}

/// 一段字：空白的、不是字的当没写。
fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests;
