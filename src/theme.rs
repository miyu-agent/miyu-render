//! 配色（设计 36 第四节第 1 条）：一套主题只定六个颜色，别的都从这六个调出来。出厂三套：`paper`（默认，旧版的纸色）、
//! `light`、`dark`，六个颜色照旧版 `paint.rs` 的同名主题取。

/// 一个颜色：红、绿、蓝、不透明度。
pub type Rgba = [u8; 4];

/// 出厂的三套。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// 纸色：旧版的默认，在 QQ 的浅色、深色界面里都不刺眼。
    #[default]
    Paper,
    /// 近白的底。
    Light,
    /// 深灰的底。
    Dark,
}

impl Theme {
    /// 照名字认：`paper`、`light`、`dark`，别的不认。
    pub fn parse(name: &str) -> Option<Theme> {
        match name {
            "paper" => Some(Theme::Paper),
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }

    /// 这一套的六个颜色。
    pub fn palette(self) -> Palette {
        match self {
            Theme::Paper => Palette {
                background: [244, 239, 229, 255],
                text: [48, 46, 41, 255],
                muted: [104, 98, 88, 255],
                accent: [45, 95, 125, 255],
                line: [211, 201, 184, 255],
                code: [225, 219, 208, 255],
            },
            Theme::Light => Palette {
                background: [250, 250, 248, 255],
                text: [30, 34, 40, 255],
                muted: [92, 96, 104, 255],
                accent: [48, 101, 190, 255],
                line: [218, 222, 230, 255],
                code: [226, 229, 235, 255],
            },
            Theme::Dark => Palette {
                background: [28, 29, 32, 255],
                text: [231, 232, 235, 255],
                muted: [164, 168, 176, 255],
                accent: [104, 179, 255, 255],
                line: [72, 76, 84, 255],
                code: [43, 45, 51, 255],
            },
        }
    }
}

/// 六个颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// 页面的底。
    pub background: Rgba,
    /// 正文的字。
    pub text: Rgba,
    /// 淡字：引用、说明、mermaid 的线。
    pub muted: Rgba,
    /// 强调：链接。和正文、标题分得开（旧版有测试守着）。
    pub accent: Rgba,
    /// 线：表格、分割线、mermaid 节点的边框。
    pub line: Rgba,
    /// 代码的底。
    pub code: Rgba,
}

impl Palette {
    /// 引用的底：页面的底往字挪一点（比页面深一档，同卡片）。
    pub fn quote(&self) -> Rgba {
        mix(self.background, self.text, 4)
    }
}

/// `from` 往 `to` 挪 `percent`（0 到 100，超了照 100）：每个通道各算，四舍五入，不透明度照 `from`。
pub fn mix(from: Rgba, to: Rgba, percent: u8) -> Rgba {
    let percent = u32::from(percent.min(100));
    let mut out = from;
    for (slot, (a, b)) in out.iter_mut().zip(from.iter().zip(to.iter())).take(3) {
        let value = (u32::from(*a) * (100 - percent) + u32::from(*b) * percent + 50) / 100;
        *slot = u8::try_from(value).unwrap_or(u8::MAX);
    }
    out
}

#[cfg(test)]
mod tests;
