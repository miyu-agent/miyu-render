//! 配色：三套照名字认得出，别的不认；每个颜色和底分得开，链接和正文、淡字不撞。

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
