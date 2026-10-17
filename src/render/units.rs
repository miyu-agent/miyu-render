//! 排字用浮点数，画布用整数：互换只走这里，先夹在范围里再换。页面最大几千像素，字号几十，都远在范围里。

/// 往上取整成像素：负数是 0。
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "先夹在 0 到 1e9 之间再换，页面不会有这么大"
)]
pub fn ceil(value: f32) -> u32 {
    value.ceil().clamp(0.0, 1e9) as u32
}

/// 取整成可正可负的像素（字形的偏移、行的基线）。
#[allow(
    clippy::cast_possible_truncation,
    reason = "先夹在正负 1e9 之间再换，页面不会有这么大"
)]
pub fn round(value: f32) -> i64 {
    value.round().clamp(-1e9, 1e9) as i64
}

/// 像素换成排字用的浮点数：超过 65535 的照 65535（字号、栏宽都远小于它）。
pub fn float(value: u32) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}
