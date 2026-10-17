//! 公式：画成透明的图、颜色是给的颜色、角上透明；写错的、空的、太宽的画不出来。

use super::*;

#[test]
fn a_formula_is_drawn_clear_in_the_given_colour() {
    let image = draw(
        r"E = mc^2 \quad \frac{a+b}{\sqrt{x}}",
        [200, 30, 40, 255],
        36.0,
        960,
    )
    .expect("画得出");
    assert_eq!(image.get_pixel(0, 0).0[3], 0, "角上透明");
    let solid: Vec<[u8; 4]> = image
        .pixels()
        .filter(|pixel| pixel.0[3] == 255)
        .map(|pixel| pixel.0)
        .collect();
    assert!(solid.len() > 100);
    assert!(
        solid
            .iter()
            .all(|pixel| pixel[0].abs_diff(200) <= 2 && pixel[1].abs_diff(30) <= 2),
        "字是给的颜色"
    );
}

#[test]
fn broken_empty_or_too_wide_are_not_drawn() {
    assert!(draw("   ", [0, 0, 0, 255], 36.0, 960).is_none());
    assert!(draw(r"\frac{a}{", [0, 0, 0, 255], 36.0, 960).is_none());
    assert!(
        draw(r"a+b+c+d+e+f+g+h+i+j+k+l+m+n", [0, 0, 0, 255], 36.0, 50).is_none(),
        "太宽"
    );
}
