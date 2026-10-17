//! 表格的列宽（旧版 `table_width.rs`）：照每一列不折行要多宽分，加起来恰好 `total`。装得下把富余按各列的字多少分；装不下找一条
//! 上限线，超过的削到线上，线以下的原样（max-min 公平，不按比例削）；列多到连每列最小的字宽都放不下时退回等分。

/// 每列多宽：`natural` 是每一列最宽的那一格不折行的字宽（不算留空），`pad` 是格子一边留的空，`min_content` 是每列至少留给字的宽
/// （列太多时自动压低）。和恒等于 `total`。
pub fn widths(natural: &[u32], total: u32, pad: u32, min_content: u32) -> Vec<u32> {
    let count = u32::try_from(natural.len()).unwrap_or(u32::MAX);
    if count == 0 {
        return Vec::new();
    }
    let min_outer = (pad * 2 + min_content).min(total / count);
    let desired: Vec<u32> = natural
        .iter()
        .map(|width| (width + pad * 2).max(min_outer))
        .collect();
    let wanted: u64 = desired.iter().map(|width| u64::from(*width)).sum();
    let mut widths = if wanted <= u64::from(total) {
        grow(&desired, natural, total, wanted)
    } else {
        shrink(&desired, total, min_outer)
    };
    settle(&mut widths, total);
    widths
}

/// 装得下：富余照各列的字宽分（只装一个字的序号列不白白撑大）；全是空格子的均分。
fn grow(desired: &[u32], natural: &[u32], total: u32, wanted: u64) -> Vec<u32> {
    let surplus = u64::from(total) - wanted;
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

/// 装不下：二分出一条上限线，超过它的削到线上，线以下的不动。
fn shrink(desired: &[u32], total: u32, min_outer: u32) -> Vec<u32> {
    let capped = |cap: u32| -> u64 {
        desired
            .iter()
            .map(|width| u64::from((*width).min(cap).max(min_outer)))
            .sum()
    };
    let (mut low, mut high) = (
        min_outer,
        desired
            .iter()
            .copied()
            .max()
            .unwrap_or(min_outer)
            .max(min_outer),
    );
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if capped(mid) <= u64::from(total) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    desired
        .iter()
        .map(|width| (*width).min(low).max(min_outer))
        .collect()
}

/// 把和硬掰到 `total`：多退少补都落在最宽的那一列上，看不出来，也不会把哪一列挤没。
fn settle(widths: &mut [u32], total: u32) {
    loop {
        let sum: u64 = widths.iter().map(|width| u64::from(*width)).sum();
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
