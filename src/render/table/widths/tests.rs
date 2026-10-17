//! 列宽：和恒等于总宽；装得下按字多少分；装不下只削最宽的；列太多退回等分；空表；全空的格子均分。

use super::widths;

const TOTAL: u32 = 960;
const PAD: u32 = 14;
const MIN: u32 = 60;

fn sum(list: &[u32]) -> u32 {
    list.iter().sum()
}

#[test]
fn widths_always_add_up() {
    for natural in [
        vec![10],
        vec![10, 900, 30],
        vec![7, 11, 13],
        vec![1, 2, 4, 8, 16, 32, 64],
        vec![2000, 2000, 2000, 2000, 2000],
        vec![0, 0],
        vec![5; 40],
    ] {
        let got = widths(&natural, TOTAL, PAD, MIN);
        assert_eq!(got.len(), natural.len());
        assert_eq!(sum(&got), TOTAL, "{natural:?} -> {got:?}");
    }
    assert!(widths(&[], TOTAL, PAD, MIN).is_empty());
}

#[test]
fn surplus_follows_the_text() {
    let got = widths(&[20, 200], TOTAL, PAD, MIN);
    assert!(got[1] > got[0] * 3, "{got:?}");
}

#[test]
fn only_the_widest_columns_are_cut() {
    // 一列长句子、四列短数字（九位的号要装得下）：只削那一列。
    let natural = [3000, 40, 40, 140, 40];
    let got = widths(&natural, TOTAL, PAD, MIN);
    for (index, width) in got.iter().enumerate().skip(1) {
        assert!(
            *width >= natural[index] + PAD * 2,
            "第 {index} 列被削了：{got:?}"
        );
    }
    assert_eq!(sum(&got), TOTAL);
}

#[test]
fn too_many_columns_fall_back_to_even() {
    let got = widths(&[100; 40], TOTAL, PAD, MIN);
    let even = TOTAL / 40;
    assert!(
        got.iter()
            .all(|width| width.abs_diff(even) <= 1 || *width >= even),
        "{got:?}"
    );
}
