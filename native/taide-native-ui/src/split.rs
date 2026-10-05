use egui::{Rect, pos2};
use taide_model::layout::SplitDir;

pub const MIN_PANE_SIZE: f32 = 120.0;
pub const RESIZE_HIT_SIZE: f32 = 8.0;
pub const PERCENT_TOTAL: f32 = 100.0;
pub const KEYBOARD_RESIZE_STEP: f32 = 8.0;

pub fn normalized_sizes(sizes: &[f32], count: usize) -> Vec<f32> {
    if count == 0 {
        return Vec::new();
    }
    if sizes.len() != count || sizes.iter().any(|size| !size.is_finite() || *size <= 0.0) {
        return vec![PERCENT_TOTAL / count as f32; count];
    }
    let sum = sizes.iter().copied().sum::<f32>();
    if !sum.is_finite() || sum <= 0.0 {
        return vec![PERCENT_TOTAL / count as f32; count];
    }
    sizes
        .iter()
        .map(|size| *size / sum * PERCENT_TOTAL)
        .collect()
}

pub fn child_rects(rect: Rect, dir: SplitDir, sizes: &[f32], thickness: f32) -> Vec<Rect> {
    let count = sizes.len();
    if count == 0 {
        return Vec::new();
    }
    let axis_size = match dir {
        SplitDir::Horizontal => rect.width(),
        SplitDir::Vertical => rect.height(),
    };
    let gap = thickness.max(0.0).min(axis_size.max(0.0) / count as f32);
    let available = (axis_size - gap * count.saturating_sub(1) as f32).max(0.0);
    let mut offset = 0.0;
    normalized_sizes(sizes, count)
        .into_iter()
        .enumerate()
        .map(|(index, percent)| {
            let extent = available * percent / PERCENT_TOTAL;
            let result = match dir {
                SplitDir::Horizontal => Rect::from_min_max(
                    pos2(rect.left() + offset, rect.top()),
                    pos2(
                        (rect.left() + offset + extent).min(rect.right()),
                        rect.bottom(),
                    ),
                ),
                SplitDir::Vertical => Rect::from_min_max(
                    pos2(rect.left(), rect.top() + offset),
                    pos2(
                        rect.right(),
                        (rect.top() + offset + extent).min(rect.bottom()),
                    ),
                ),
            };
            offset += extent;
            if index + 1 < count {
                offset += gap;
            }
            result
        })
        .collect()
}

pub fn resized_pair(
    sizes: &[f32],
    divider: usize,
    delta: f32,
    content_extent: f32,
) -> Option<Vec<f32>> {
    if divider + 1 >= sizes.len()
        || !delta.is_finite()
        || !content_extent.is_finite()
        || content_extent <= 0.0
    {
        return None;
    }
    let mut result = normalized_sizes(sizes, sizes.len());
    let pair_total = result[divider] + result[divider + 1];
    let minimum = (MIN_PANE_SIZE / content_extent * PERCENT_TOTAL).min(pair_total / 2.0);
    let left = (result[divider] + delta / content_extent * PERCENT_TOTAL)
        .clamp(minimum, pair_total - minimum);
    result[divider] = left;
    result[divider + 1] = pair_total - left;
    Some(result)
}
