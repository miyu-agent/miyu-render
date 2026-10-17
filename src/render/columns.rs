//! 分栏（设计 36 第四节第 6 条，照旧版 `renderer/columns.rs`，09-24 定的规矩）：排好的块从左往右一栏一栏放，长宽都跟着内容走。
//!
//! - 放不下的块先拆进这一栏剩下的地方，再续到下一栏；拆开的两截都得像样：文字各至少 [`MIN_SPLIT_LINES`] 行，表格这一截至少一行
//!   正文、续栏先重复表头。拆不开的（分割线、只有一行的）整块挪到下一栏。
//! - 标题跟下文的开头待在同一栏：这一栏放不下「标题和下文的头几行」就整组换栏。
//! - 几栏、多高（[`balanced`]）：从放得下的最少栏数往上试，每一种二分出能装下的最矮栏高，挑成图最接近方图的那一种（比例差不多的
//!   偏向少栏）。方图在手机竖屏、电脑横屏上都不吃亏：4:3 会让四五百字的回复摊成又矮又宽的两栏（旧版 09-24 改）。
//!
//! 只看块的形状（[`Shape`]）：多高、能在哪断、和前后空多少、是不是标题、表头多高，不碰字体，好测。

/// 拆开的文字块每一截至少几行。
pub const MIN_SPLIT_LINES: usize = 2;

/// 栏和栏之间空多宽。
pub const GAP: u32 = 32;

/// 栏数从最少的往上最多再试几种：过了方图就停，到不了这个数，只给退化的输入兜底。
const MAX_EXTRA_COLUMNS: usize = 16;

/// 一块的形状。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// 能从哪里断开，照先后，最后一个就是整块的高。
    pub breaks: Vec<u32>,
    /// 和上一块之间、和下一块之间空多少（两块之间取大的）。
    pub margin: (u32, u32),
    /// 是不是标题。
    pub heading: bool,
    /// 表格的表头多高；不是表格、没有表头的是 0。
    pub header: u32,
    /// 是不是表格（拆开的一截至少一行正文）。
    pub table: bool,
}

impl Shape {
    /// 整块多高。
    pub fn height(&self) -> u32 {
        self.breaks.last().copied().unwrap_or(0)
    }
}

/// 放进一栏的一截：第几块、块里从哪到哪、放在这一栏的哪个高度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub block: usize,
    pub from: u32,
    pub to: u32,
    pub y: u32,
}

/// 一栏。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Column {
    pub pieces: Vec<Piece>,
    /// 用了多高。
    pub used: u32,
    /// 最后放进来的那一块和下一块之间要空多少。
    after: u32,
}

/// 栏高固定 `height`，从左到右放。有一行字比一栏还高的交回空的。
pub fn fill(shapes: &[Shape], height: u32) -> Option<Vec<Column>> {
    let mut columns = vec![Column::default()];
    for (index, shape) in shapes.iter().enumerate() {
        if shape.heading {
            keep_with_next(&mut columns, shapes, index, height);
        }
        place(&mut columns, index, shape, height)?;
    }
    Some(columns)
}

/// 两块之间空多少：上一块的「之后」、这一块的「之前」取大的；栏顶不空。
fn gap(column: &Column, shape: &Shape) -> u32 {
    if column.used == 0 {
        0
    } else {
        column.after.max(shape.margin.0)
    }
}

/// 标题和下文的开头放不进这一栏剩下的地方、一整栏放得下，就先换栏。
fn keep_with_next(columns: &mut Vec<Column>, shapes: &[Shape], index: usize, height: u32) {
    let Some(column) = columns.last() else {
        return;
    };
    if column.used == 0 {
        return;
    }
    let mut needed = 0;
    let mut previous_after = column.after;
    for shape in &shapes[index..] {
        needed += previous_after.max(shape.margin.0);
        previous_after = shape.margin.1;
        if shape.heading {
            needed += shape.height();
            continue;
        }
        needed += first_piece(shape);
        break;
    }
    if needed > height.saturating_sub(column.used) && needed <= height {
        columns.push(Column::default());
    }
}

/// 这一块在一栏里至少要占多高：表格是表头加第一行正文，文字是头 [`MIN_SPLIT_LINES`] 行，拆不开的是整块。
fn first_piece(shape: &Shape) -> u32 {
    if shape.table {
        return shape
            .breaks
            .iter()
            .copied()
            .find(|at| *at > shape.header)
            .unwrap_or(shape.height());
    }
    shape
        .breaks
        .get(MIN_SPLIT_LINES - 1)
        .copied()
        .unwrap_or(shape.height())
        .min(shape.height())
}

/// 放一块：放不下的拆，拆不出像样的两截的换栏。
fn place(columns: &mut Vec<Column>, index: usize, shape: &Shape, height: u32) -> Option<()> {
    let total = shape.height();
    let mut from = 0;
    loop {
        let column = columns.last_mut()?;
        if from > 0 && shape.header > 0 && column.used == 0 {
            column.pieces.push(Piece {
                block: index,
                from: 0,
                to: shape.header,
                y: 0,
            });
            column.used = shape.header;
        }
        let margin = if from == 0 { gap(column, shape) } else { 0 };
        let available = height.saturating_sub(column.used + margin);
        let to = if total - from <= available {
            total
        } else if let Some(end) = split(shape, from, from + available) {
            end
        } else if column
            .pieces
            .iter()
            .any(|piece| !(from > 0 && piece.block == index && piece.from == 0))
        {
            columns.push(Column::default());
            continue;
        } else {
            // 空栏也凑不出像样的两截（栏很矮时）：能放多少放多少。
            shape
                .breaks
                .iter()
                .copied()
                .rfind(|at| *at > from && *at <= from + available)?
        };
        let y = column.used + margin;
        column.pieces.push(Piece {
            block: index,
            from,
            to,
            y,
        });
        column.used = y + (to - from);
        column.after = shape.margin.1;
        if to == total {
            return Some(());
        }
        from = to;
        columns.push(Column::default());
    }
}

/// 在 `(from, limit]` 里找最靠下、两截都像样的断点。
fn split(shape: &Shape, from: u32, limit: u32) -> Option<u32> {
    let breaks = &shape.breaks;
    let min_lines = if shape.table { 1 } else { MIN_SPLIT_LINES };
    let floor = if from == 0 { first_piece(shape) } else { 0 };
    let first = breaks.partition_point(|at| *at <= from);
    (first..breaks.len())
        .rev()
        .filter(|index| {
            breaks[*index] <= limit && breaks[*index] >= floor && breaks[*index] < shape.height()
        })
        .find(|index| index + 1 - first >= min_lines && breaks.len() - index > min_lines)
        .map(|index| breaks[index])
}

/// 定栏数和栏高：`max` 是一栏最高（页高减去上下页边），`floor` 是一栏最矮，`size(栏数, 栏高)` 是成图的宽高，`pixels` 是一页最多
/// 多少像素。放不下的（有一行字比一栏还高）交回空的。
pub fn balanced(
    shapes: &[Shape],
    max: u32,
    floor: u32,
    size: impl Fn(usize, u32) -> (u32, u32),
    pixels: u64,
) -> Option<Vec<Column>> {
    let fewest = fill(shapes, max)?;
    let total: u64 = shapes.iter().map(|shape| u64::from(shape.height())).sum();
    let mut best: Option<(Vec<Column>, f32)> = None;
    for count in fewest.len()..=fewest.len() + MAX_EXTRA_COLUMNS {
        let low = u32::try_from(total.div_ceil(count as u64))
            .unwrap_or(max)
            .clamp(floor.min(max), max);
        let Some(plan) = shortest(shapes, count, low, max) else {
            continue;
        };
        let used = plan.iter().map(|column| column.used).max().unwrap_or(0);
        let (width, height) = size(plan.len(), used);
        if u64::from(width) * u64::from(height) > pixels {
            break;
        }
        let ratio = super::units::float(width) / super::units::float(height.max(1));
        let distance = ratio.ln().abs();
        if best
            .as_ref()
            .is_none_or(|(_, best)| distance + 0.01 < *best)
        {
            best = Some((plan, distance));
        }
        if ratio >= 1.0 {
            break;
        }
    }
    Some(best.map_or(fewest, |(plan, _)| plan))
}

/// 在 `[low, high]` 里二分出能装进至多 `count` 栏的最矮栏高；连 `high` 都装不下的交回空的。
fn shortest(shapes: &[Shape], count: usize, low: u32, high: u32) -> Option<Vec<Column>> {
    let mut best = fill(shapes, high).filter(|plan| plan.len() <= count)?;
    let (mut low, mut high) = (low.min(high), high);
    while low < high {
        let mid = low + (high - low) / 2;
        match fill(shapes, mid) {
            Some(plan) if plan.len() <= count => {
                best = plan;
                high = mid;
            }
            _ => low = mid + 1,
        }
    }
    Some(best)
}

#[cfg(test)]
mod tests;
