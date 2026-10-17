//! 配色：三套照名字认得出，别的不认；每个颜色和底分得开，链接和正文、淡字不撞；挪颜色的两头、中间、四舍五入；引用、表头的底一档比一档深；转色相的数对、亮度不变，代码的几种颜色两两分得开。

use super::*;

/// 两个颜色差多少：三个通道差的和。
fn distance(a: Rgba, b: Rgba) -> u32 {
    a.iter()
        .zip(b.iter())
        .take(3)
        .map(|(x, y)| u32::from(x.abs_diff(*y)))
        .sum()
}

#[test]
fn themes_are_read_by_name_only() {
    assert_eq!(Theme::parse("paper"), Some(Theme::Paper));
    assert_eq!(Theme::parse("light"), Some(Theme::Light));
    assert_eq!(Theme::parse("dark"), Some(Theme::Dark));
    for other in ["Paper", "", "sepia", " dark"] {
        assert_eq!(Theme::parse(other), None, "{other:?}");
    }
    assert_eq!(Theme::default(), Theme::Paper);
}

#[test]
fn the_colours_stand_apart_from_the_page_in_every_theme() {
    for theme in [Theme::Paper, Theme::Light, Theme::Dark] {
        let p = theme.palette();
        // 每个颜色和底分得开（线、代码底都是「比底深一点」，它俩挨得近不要紧）。
        for colour in [p.text, p.muted, p.accent, p.line, p.code] {
            assert!(
                distance(colour, p.background) >= 30,
                "{theme:?}: {colour:?}"
            );
        }
        // 字在底上看得清；链接和正文、淡字分得开。
        assert!(distance(p.text, p.background) >= 400, "{theme:?}");
        assert!(distance(p.accent, p.text) >= 100, "{theme:?}");
        assert!(distance(p.accent, p.muted) >= 60, "{theme:?}");
    }
}

#[test]
fn mixing_moves_part_of_the_way_and_keeps_the_page_opaque() {
    let (black, white) = ([0, 0, 0, 255], [255, 255, 255, 0]);
    assert_eq!(mix(black, white, 0), black);
    assert_eq!(
        mix(black, white, 100),
        [255, 255, 255, 255],
        "不透明度照 from"
    );
    assert_eq!(mix(black, white, 200), [255, 255, 255, 255], "超了照 100");
    assert_eq!(mix(black, white, 50), [128, 128, 128, 255]);
    assert_eq!(
        mix([10, 0, 0, 255], [11, 0, 0, 255], 50),
        [11, 0, 0, 255],
        "四舍五入"
    );
    for theme in [Theme::Paper, Theme::Light, Theme::Dark] {
        let p = theme.palette();
        assert_ne!(p.quote(), p.background, "{theme:?}: 引用比页面深一档");
        assert_ne!(p.header(), p.quote(), "{theme:?}: 表头再深一档");
    }
}

#[test]
fn turning_the_hue_keeps_lightness_and_lands_on_the_right_colour() {
    let red = [255, 0, 0, 255];
    assert_eq!(hue(red, 120.0), [0, 255, 0, 255]);
    assert_eq!(hue(red, 240.0), [0, 0, 255, 255]);
    assert_eq!(hue(red, 60.0), [255, 255, 0, 255]);
    assert_eq!(hue(red, -60.0), [255, 0, 255, 255], "往回转");
    assert_eq!(hue(red, 30.0), [255, 128, 0, 255], "扇区里的中间");
    assert_eq!(hue(red, 90.0), [128, 255, 0, 255]);
    let accent = [37, 99, 235, 200];
    assert_eq!(hue(accent, 360.0), accent, "转一圈回原样");
    assert_eq!(
        hue([90, 90, 90, 255], 130.0),
        [90, 90, 90, 255],
        "灰的没有色相"
    );
    let lightness = |[r, g, b, _]: Rgba| u16::from(r.max(g).max(b)) + u16::from(r.min(g).min(b));
    assert!(lightness(hue(accent, 130.0)).abs_diff(lightness(accent)) <= 2);
    for theme in [Theme::Paper, Theme::Light, Theme::Dark] {
        let p = theme.palette();
        let colours = [
            p.accent,
            p.code_string(),
            p.code_constant(),
            p.code_function(),
            p.muted,
            p.text,
        ];
        for (i, a) in colours.iter().enumerate() {
            for b in &colours[i + 1..] {
                assert!(distance(*a, *b) >= 60, "{theme:?}: {a:?} 和 {b:?} 太像");
            }
            assert!(
                distance(*a, p.code) >= 150,
                "{theme:?}: {a:?} 在代码底上看得清"
            );
        }
    }
}
