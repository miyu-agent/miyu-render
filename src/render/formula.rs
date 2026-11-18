//! 独占一行的公式 `$$…$$`（设计 36 第四节第 4 条，2026-10-17 项目主人定补上）：RaTeX 画成透明的图（纯 Rust，旧版终端在用），颜色
//! 是正文的字，字号同正文，按透明度叠到页面上（同 mermaid），居中。写错了、画成空的交回空的，调的一方照 Unicode 写法排。

use image::RgbaImage;
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_parser::parser::parse;
use ratex_render::{RenderOptions, render_to_png};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;

use crate::theme::Rgba;

/// 公式的源码最长多少：模型写的，太长的多半是写坏了。
const MAX_SOURCE: usize = 4096;

/// 画一条公式：颜色 `colour`、字号 `size`，宽不超过 `max_width`（超了的不画，照 Unicode 写法排）。
pub fn draw(tex: &str, colour: Rgba, size: f32, max_width: u32) -> Option<RgbaImage> {
    let normalized = uncommented(tex)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() || normalized.len() > MAX_SOURCE {
        return None;
    }
    let ast = parse(&normalized).ok()?;
    let [r, g, b, _] = colour;
    let ink = Color {
        r: channel(r),
        g: channel(g),
        b: channel(b),
        a: 1.0,
    };
    let options = LayoutOptions::default()
        .with_style(MathStyle::Display)
        .with_color(ink);
    let list = to_display_list(&layout(&ast, &options));
    let render = RenderOptions {
        font_size: size,
        padding: 4.0,
        background_color: Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        },
        font_dir: String::new(),
        device_pixel_ratio: 1.0,
    };
    let png = render_to_png(&list, &render).ok()?;
    let image = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    (image.width() <= max_width && image.pixels().any(|pixel| pixel.0[3] > 0)).then_some(image)
}

/// 去掉 `%` 到行尾的注释（审计 #136）：先去再把换行压成空格，不然一个注释把后面所有行都吞了。`\%` 是字面的百分号，
/// `\\%` 是换行后面跟注释：反斜杠和它后面的一个字成对跳过。
fn uncommented(tex: &str) -> String {
    let mut out = String::with_capacity(tex.len());
    let mut chars = tex.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                out.push(ch);
                out.extend(chars.next());
            }
            '%' => {
                if chars.by_ref().any(|skipped| skipped == '\n') {
                    out.push('\n');
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// 0 到 255 换成 0 到 1。
fn channel(value: u8) -> f32 {
    f32::from(value) / 255.0
}

#[cfg(test)]
mod tests;
