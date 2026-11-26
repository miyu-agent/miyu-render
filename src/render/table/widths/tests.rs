//! 列宽：和恒等于总宽；装得下按字多少分；装不下只削最宽的；列太多退回等分；空表；全空的格子均分。三档（施工 R-14）：表头能折
//! 时先折表头、正文不折；正文也放不下时每列不少于拆不开的宽；数字列不受长句子列牵连。

use super::{Need, widths};

const TOTAL: u32 = 960;
const PAD: u32 = 14;
const MIN: u32 = 60;

fn sum(list: &[u32]) -> u32 {
    list.iter().sum()
}

/// 只有一种宽的列（表头、正文一样宽，拆不开的当 0）：旧的几条照它测。
fn plain(natural: &[u32]) -> Vec<Need> {
    natural
        .iter()
        .map(|width| Need {
            least: 0,
            body: *width,
            full: *width,
        })
        .collect()
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
        let got = widths(&plain(&natural), TOTAL, PAD, MIN);
        assert_eq!(got.len(), natural.len());
        assert_eq!(sum(&got), TOTAL, "{natural:?} -> {got:?}");
    }
    assert!(widths(&[], TOTAL, PAD, MIN).is_empty());
}

#[test]
fn surplus_follows_the_text() {
    let got = widths(&plain(&[20, 200]), TOTAL, PAD, MIN);
    assert!(got[1] > got[0] * 3, "{got:?}");
}

#[test]
fn only_the_widest_columns_are_cut() {
    // 一列长句子、四列短数字（九位的号要装得下）：只削那一列。
    let natural = [3000, 40, 40, 140, 40];
    let got = widths(&plain(&natural), TOTAL, PAD, MIN);
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
    let got = widths(&plain(&[100; 40]), TOTAL, PAD, MIN);
    let even = TOTAL / 40;
    assert!(
        got.iter()
            .all(|width| width.abs_diff(even) <= 1 || *width >= even),
        "{got:?}"
    );
}

#[test]
fn headers_fold_before_the_body_does() {
    // 2026-11-26 那张六列的表：表头整行比正文宽，连表头都不折放不下，正文不折放得下。每列给的都不少于正文不折的宽，表头去折。
    let needs = [
        Need {
            least: 110,
            body: 110,
            full: 110,
        },
        Need {
            least: 140,
            body: 135,
            full: 150,
        },
        Need {
            least: 120,
            body: 130,
            full: 120,
        },
        Need {
            least: 120,
            body: 130,
            full: 120,
        },
        Need {
            least: 120,
            body: 135,
            full: 200,
        },
        Need {
            least: 110,
            body: 125,
            full: 120,
        },
    ];
    let got = widths(&needs, TOTAL, PAD, MIN);
    assert_eq!(sum(&got), TOTAL);
    for (index, (width, need)) in got.iter().zip(&needs).enumerate() {
        assert!(
            *width >= need.body.max(need.least) + PAD * 2,
            "第 {index} 列正文要折了：{got:?}"
        );
    }
}

#[test]
fn every_column_keeps_its_unbreakable_width() {
    // 正文也放不下：长句子那一列让出来，别的列不少于拆不开的宽（一个数连单位不劈开）。
    let needs = [
        Need {
            least: 80,
            body: 2000,
            full: 2000,
        },
        Need {
            least: 150,
            body: 150,
            full: 150,
        },
        Need {
            least: 160,
            body: 160,
            full: 300,
        },
    ];
    let got = widths(&needs, TOTAL, PAD, MIN);
    assert_eq!(sum(&got), TOTAL);
    assert!(got[1] >= 150 + PAD * 2, "{got:?}");
    assert!(got[2] >= 160 + PAD * 2, "{got:?}");
    assert!(got[0] >= 80 + PAD * 2, "{got:?}");
}

#[test]
fn a_roomy_table_never_folds() {
    let needs = [
        Need {
            least: 40,
            body: 60,
            full: 200,
        },
        Need {
            least: 40,
            body: 300,
            full: 300,
        },
    ];
    let got = widths(&needs, TOTAL, PAD, MIN);
    assert!(
        got[0] >= 200 + PAD * 2 && got[1] >= 300 + PAD * 2,
        "{got:?}"
    );
    assert_eq!(sum(&got), TOTAL);
}
