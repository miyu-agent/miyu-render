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

    /// 表头的底：比引用再深一档。
    pub fn header(&self) -> Rgba {
        mix(self.background, self.text, 8)
    }
}

impl Palette {
    /// 代码里字符串的颜色：强调色转 130°（蓝的强调色转出来偏绿），亮度、饱和度照强调色。
    pub fn code_string(&self) -> Rgba {
        hue(self.accent, 130.0)
    }

    /// 代码里数、常量的颜色：强调色转 200°（偏橙）。
    pub fn code_constant(&self) -> Rgba {
        hue(self.accent, 200.0)
    }

    /// 代码里函数名、类型名的颜色：强调色转 -60°（偏紫）。
    pub fn code_function(&self) -> Rgba {
        hue(self.accent, -60.0)
    }
}

/// 色相转 `degrees` 度，亮度、饱和度不变（HSL）。
pub fn hue([red, green, blue, alpha]: Rgba, degrees: f32) -> Rgba {
    let (red, green, blue) = (
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
    );
    let (max, min) = (red.max(green).max(blue), red.min(green).min(blue));
    let delta = max - min;
    if delta <= f32::EPSILON {
        return [channel(red), channel(green), channel(blue), alpha];
    }
    let light = f32::midpoint(max, min);
    let base = if (max - red).abs() <= f32::EPSILON {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if (max - green).abs() <= f32::EPSILON {
        60.0 * ((blue - red) / delta + 2.0)
    } else {
        60.0 * ((red - green) / delta + 4.0)
    };
    // HSL 里色度就是 max - min，转色相不变。
    let chroma = delta;
    let turned = (base + degrees).rem_euclid(360.0) / 60.0;
    let second = chroma * (1.0 - (turned.rem_euclid(2.0) - 1.0).abs());
    let (one, two, three) = match turned {
        sector if sector < 1.0 => (chroma, second, 0.0),
        sector if sector < 2.0 => (second, chroma, 0.0),
        sector if sector < 3.0 => (0.0, chroma, second),
        sector if sector < 4.0 => (0.0, second, chroma),
        sector if sector < 5.0 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let lift = light - chroma / 2.0;
    [
        channel(one + lift),
        channel(two + lift),
        channel(three + lift),
        alpha,
    ]
}

/// 0 到 1 换成 0 到 255（四舍五入，先夹在范围里）。
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "先夹在 0 到 255 之间再换"
)]
fn channel(value: f32) -> u8 {
    (value * 255.0).round().clamp(0.0, 255.0) as u8
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
