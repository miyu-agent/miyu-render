//! 表格的列宽（旧版 `table_width.rs` 起头，施工 R-14 改成三档）：每一列量三个宽，加起来恰好 `total`。
//!
//! - 三档（[`Need`]）：拆不开的最小宽（表头里的一个词、正文里的一个数连单位、一个英文词）、正文不折行的宽（表头只算它最小的
//!   那个宽，表头可以折）、连表头也不折行的宽。
//! - 连表头都不折的装得下：照它给，富余按各列的字多少分。装不下就退一档：正文不折的装得下时，每列先给正文不折的宽，表头要多的
//!   照一条上限线给（超过线的削到线上，线以下的原样）；还装不下再退一档，每列先给拆不开的宽，正文要多的照线给。线以下的原样
//!   （max-min 公平，不按比例削）：窄的序号列、数字列不受牵连。
//! - 列多到连每列拆不开的宽都放不下时，照旧只削最宽的几列，最窄也有 `min_content`；再不够退回等分。

/// 一列的三个宽，都不算格子留的空。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Need {
    /// 拆不开的最小宽：这一列每一格里最长的、不能在中间断开的那一段。
    pub least: u32,
    /// 正文不折行的宽：正文每一格整行的宽，和表头拆不开的宽，取大的。
    pub body: u32,
    /// 连表头也不折行的宽。
    pub full: u32,
}

/// 每列多宽：`needs` 是每一列的三个宽，`pad` 是格子一边留的空，`min_content` 是每列至少留给字的宽（列太多时自动压低）。和恒等于
/// `total`。
pub fn widths(needs: &[Need], total: u32, pad: u32, min_content: u32) -> Vec<u32> {
    let count = u32::try_from(needs.len()).unwrap_or(u32::MAX);
    if count == 0 {
        return Vec::new();
    }
    let min_outer = (pad * 2 + min_content).min(total / count);
    let outer = |width: u32| (width + pad * 2).max(min_outer);
    let least: Vec<u32> = needs.iter().map(|need| outer(need.least)).collect();
    let body: Vec<u32> = needs
        .iter()
        .map(|need| outer(need.body.max(need.least)))
        .collect();
    let full: Vec<u32> = needs
        .iter()
        .map(|need| outer(need.full.max(need.body).max(need.least)))
        .collect();
    let fits = |list: &[u32]| sum(list) <= u64::from(total);
    let mut widths = if fits(&full) {
        let natural: Vec<u32> = needs.iter().map(|need| need.full).collect();
        grow(&full, &natural, total)
    } else if fits(&body) {
        capped(&body, &full, total)
    } else if fits(&least) {
        capped(&least, &body, total)
    } else {
        let floor = vec![min_outer; least.len()];
        capped(&floor, &least, total)
    };
    settle(&mut widths, total);
    widths
}

fn sum(list: &[u32]) -> u64 {
    list.iter().map(|width| u64::from(*width)).sum()
}

/// 装得下：富余照各列的字宽分（只装一个字的序号列不白白撑大）；全是空格子的均分。
fn grow(desired: &[u32], natural: &[u32], total: u32) -> Vec<u32> {
    let surplus = u64::from(total).saturating_sub(sum(desired));
    let weights: u64 = natural.iter().map(|width| u64::from(*width)).sum();
    let count = desired.len() as u64;
    desired
        .iter()
        .zip(natural)
        .map(|(width, weight)| {
            let share = (surplus * u64::from(*weight))
                .checked_div(weights)
                .unwrap_or(surplus / count);
            width + u32::try_from(share).unwrap_or(u32::MAX)
        })
        .collect()
}

/// 每列先给 `low`，想要 `high`：二分出一条上限线，每列给 `high` 削到线上、但不少于 `low` 的宽，和不超过 `total`。
fn capped(low: &[u32], high: &[u32], total: u32) -> Vec<u32> {
    let at = |cap: u32| -> Vec<u32> {
        low.iter()
            .zip(high)
            .map(|(low, high)| (*high).min(cap).max(*low))
            .collect()
    };
    let (mut bottom, mut top) = (0, high.iter().copied().max().unwrap_or(0));
    while bottom < top {
        let mid = bottom + (top - bottom).div_ceil(2);
        if sum(&at(mid)) <= u64::from(total) {
            bottom = mid;
        } else {
            top = mid - 1;
        }
    }
    at(bottom)
}

/// 把和硬掰到 `total`：多退少补都落在最宽的那一列上，看不出来，也不会把哪一列挤没。
fn settle(widths: &mut [u32], total: u32) {
    loop {
        let sum = sum(widths);
        let widest = widths
            .iter()
            .enumerate()
            .max_by_key(|(index, width)| (**width, std::cmp::Reverse(*index)))
            .map_or(0, |(index, _)| index);
        match sum.cmp(&u64::from(total)) {
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Less => {
                widths[widest] += u32::try_from(u64::from(total) - sum).unwrap_or(0);
            }
            std::cmp::Ordering::Greater => {
                let excess = u32::try_from(sum - u64::from(total)).unwrap_or(u32::MAX);
                let take = excess.min(widths[widest].saturating_sub(1));
                if take == 0 {
                    return;
                }
                widths[widest] -= take;
            }
        }
    }
}

#[cfg(test)]
mod tests;
