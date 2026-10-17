//! 叠色（设计 36 第四节第 2 条：照透明度叠、不盖像素）、编成 PNG。

use image::{ImageEncoder, Rgba, RgbaImage};

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
