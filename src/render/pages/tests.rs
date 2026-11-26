//! 分页：页的宽高；短的一页一栏；长的分几栏、一页装不下的接着下一页、每页不超像素上限；太长的交回空的。

use super::*;
use crate::render::COLUMN;

/// `count` 段、每段一行 `line` 高。
fn lines(count: u32, line: u32) -> Vec<Shape> {
    (0..count)
        .map(|_| Shape {
            breaks: vec![line],
            margin: (0, 0),
            heading: false,
            header: 0,
            table: false,
        })
        .collect()
}

#[test]
fn page_sizes() {
    assert_eq!(size(1, 0, COLUMN), (PADDING * 2 + COLUMN, MIN_HEIGHT));
    // 宽表放宽了栏（施工 R-14）：页宽跟着栏宽走。
    assert_eq!(size(2, 0, 1200).0, PADDING * 2 + 1200 * 2 + GAP);
    assert_eq!(
        size(2, 1000, COLUMN),
        (PADDING * 2 + COLUMN * 2 + GAP, 1000 + PADDING * 2)
    );
    assert_eq!(size(1, 99_999, COLUMN), (PADDING * 2 + COLUMN, MAX_HEIGHT));
}

#[test]
fn short_replies_are_one_page_one_column() {
    let got = plan(&lines(5, 50), COLUMN).expect("画得了");
    assert_eq!((got.len(), got[0].len()), (1, 1));
}

#[test]
fn long_replies_spread_over_columns_and_pages_within_the_limits() {
    let got = plan(&lines(800, 50), COLUMN).expect("画得了");
    assert!(got.len() >= 2, "{} 页", got.len());
    for page in &got {
        let (width, height) = size(
            page.len(),
            page.iter().map(|column| column.used).max().unwrap_or(0),
            COLUMN,
        );
        assert!(u64::from(width) * u64::from(height) <= MAX_PIXELS);
        assert!(height <= MAX_HEIGHT);
    }
    let placed: usize = got.iter().flatten().map(|column| column.pieces.len()).sum();
    assert_eq!(placed, 800, "一块不少");
}

#[test]
fn too_long_is_refused() {
    assert!(plan(&lines(3000, 50), COLUMN).is_none());
    assert!(
        plan(&lines(1, MAX_HEIGHT), COLUMN).is_none(),
        "一行比一页还高"
    );
}
