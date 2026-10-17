//! 分栏：放得下的不拆；放不下的拆进剩下的地方、两截各至少两行；拆不开的整块挪；表格续栏先重复表头、这一截至少一行正文；
//! 标题跟下文；两块之间的空取大的、栏顶不空；挑最接近方图的栏数、比例差不多的偏向少栏；一行比一栏还高的交回空的。

use super::*;

/// 一段 `lines` 行、每行 `line` 高的文字，前后各空 `margin`。
fn text(lines: u32, line: u32, margin: u32) -> Shape {
    Shape {
        breaks: (1..=lines).map(|n| n * line).collect(),
        margin: (margin, margin),
        heading: false,
        header: 0,
        table: false,
    }
}

/// 一个标题（一行）。
fn heading(line: u32) -> Shape {
    Shape {
        heading: true,
        ..text(1, line, 10)
    }
}

/// 一张表：表头 `header` 高，`rows` 行正文各 `row` 高。
fn table(header: u32, rows: u32, row: u32) -> Shape {
    let mut breaks = vec![header];
    breaks.extend((1..=rows).map(|n| header + n * row));
    Shape {
        breaks,
        margin: (10, 10),
        heading: false,
        header,
        table: true,
    }
}

/// 每一栏的每一截：（第几块，从，到，y）。
fn plan(columns: &[Column]) -> Vec<Vec<(usize, u32, u32, u32)>> {
    columns
        .iter()
        .map(|column| {
            column
                .pieces
                .iter()
                .map(|piece| (piece.block, piece.from, piece.to, piece.y))
                .collect()
        })
        .collect()
}

#[test]
fn what_fits_stays_whole_and_gaps_take_the_larger() {
    let shapes = [
        Shape {
            margin: (5, 15),
            ..text(2, 10, 0)
        },
        Shape {
            margin: (12, 3),
            ..text(1, 10, 0)
        },
    ];
    let got = fill(&shapes, 100).expect("放得下");
    assert_eq!(
        plan(&got),
        [vec![(0, 0, 20, 0), (1, 0, 10, 35)]],
        "栏顶不空；两块之间取前一块之后的 15、这一块之前的 12 里大的"
    );
}

#[test]
fn a_long_block_splits_into_the_room_left_with_two_lines_each() {
    let shapes = [text(3, 10, 0), text(6, 10, 0)];
    let got = fill(&shapes, 60).expect("放得下");
    assert_eq!(
        plan(&got),
        [vec![(0, 0, 30, 0), (1, 0, 30, 30)], vec![(1, 30, 60, 0)]]
    );
    // 剩下的地方只够一行：不拆出孤零零的一行，整块挪到下一栏。
    let shapes = [text(5, 10, 0), text(4, 10, 0)];
    let got = fill(&shapes, 60).expect("放得下");
    assert_eq!(plan(&got), [vec![(0, 0, 50, 0)], vec![(1, 0, 40, 0)]]);
}

#[test]
fn an_unsplittable_block_moves_whole() {
    let shapes = [text(5, 10, 0), text(1, 30, 0)];
    let got = fill(&shapes, 60).expect("放得下");
    assert_eq!(plan(&got), [vec![(0, 0, 50, 0)], vec![(1, 0, 30, 0)]]);
}

#[test]
fn tables_repeat_their_header_and_keep_one_row() {
    let shapes = [table(20, 6, 10)];
    let got = fill(&shapes, 50).expect("放得下");
    assert_eq!(
        plan(&got),
        [vec![(0, 0, 50, 0)], vec![(0, 0, 20, 0), (0, 50, 80, 20)]]
    );
}

#[test]
fn a_heading_goes_with_what_follows() {
    let shapes = [text(4, 10, 0), heading(10), text(4, 10, 0)];
    let got = fill(&shapes, 60).expect("放得下");
    assert_eq!(got[0].pieces.len(), 1, "标题不留在栏底：{:?}", plan(&got));
    assert_eq!(got[1].pieces[0].block, 1);
}

#[test]
fn a_line_taller_than_a_column_cannot_be_placed() {
    assert_eq!(fill(&[text(1, 100, 0)], 50), None);
}

#[test]
fn the_column_count_aims_for_a_square_page() {
    // 每栏 100 宽、页边 0；内容 400 高。一栏 100x400、两栏 200x200（方图）、四栏 400x100：挑两栏。
    let shapes: Vec<Shape> = (0..40).map(|_| text(1, 10, 0)).collect();
    let size = |count: usize, used: u32| (u32::try_from(count).unwrap_or(0) * 100, used);
    let got = balanced(&shapes, 1000, 10, size, u64::MAX).expect("放得下");
    assert_eq!(got.len(), 2);
    assert!(got.iter().all(|column| column.used == 200));
    // 像素有上限：方图超了，交回最少栏数的那一种（太大的再分页）。
    let got = balanced(&shapes, 1000, 10, size, 200 * 200 - 1).expect("放得下");
    assert_eq!(got.len(), 1);
}

#[test]
fn a_table_may_leave_just_one_row_behind() {
    // 前面垫一段字（表格前面再空 10），剩下的地方放得下表头和一行正文：表格这一截至少一行就够，拆开（文字要两行）。
    let shapes = [text(1, 10, 0), table(20, 2, 10)];
    let got = fill(&shapes, 55).expect("放得下");
    assert_eq!(
        plan(&got),
        [
            vec![(0, 0, 10, 0), (1, 0, 30, 20)],
            vec![(1, 0, 20, 0), (1, 30, 40, 20)]
        ]
    );
}
