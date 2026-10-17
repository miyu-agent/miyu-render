//! 画到页面上：字形照透明度叠上去（设计 36 第四节第 2 条：不盖像素），编成 PNG。

use cosmic_text::{Buffer, FontSystem, SwashCache};
use image::{ImageEncoder, Rgba, RgbaImage};

/// 把排好的 `buffer` 画到 `image` 上，左上角在 `(left, top)`；出了画布的不画。
pub fn text(
    image: &mut RgbaImage,
    system: &mut FontSystem,
    cache: &mut SwashCache,
    buffer: &Buffer,
    (left, top): (u32, u32),
) {
    let default = cosmic_text::Color::rgba(0, 0, 0, 255);
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            let base = glyph.color_opt.unwrap_or(default);
            cache.with_pixels(system, physical.cache_key, base, |x, y, color| {
                let x = i64::from(left) + i64::from(physical.x) + i64::from(x);
                let y = i64::from(top)
                    + super::units::round(run.line_y)
                    + i64::from(physical.y)
                    + i64::from(y);
                let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
                    return;
                };
                if let Some(pixel) = image.get_pixel_mut_checked(x, y) {
                    over(pixel, color.as_rgba());
                }
            });
        }
    }
}

/// 把 `color` 照它的不透明度叠到不透明的页面像素 `pixel` 上：页面一直是不透明的，叠完也是（image 库自带的叠法把不透明度
/// 算成 254，叠过字的地方留下一点透明，深色的聊天界面里看得出来）。
pub fn over(pixel: &mut Rgba<u8>, [r, g, b, a]: [u8; 4]) {
    let alpha = u32::from(a);
    for (slot, source) in pixel.0.iter_mut().zip([r, g, b]) {
        let mixed = (u32::from(source) * alpha + u32::from(*slot) * (255 - alpha) + 127) / 255;
        *slot = u8::try_from(mixed).unwrap_or(u8::MAX);
    }
    pixel.0[3] = 255;
}

/// 编成 PNG。编不出来的是空的（写进内存不会失败）。
pub fn png(image: &RgbaImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    if encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .is_err()
    {
        bytes.clear();
    }
    bytes
}
