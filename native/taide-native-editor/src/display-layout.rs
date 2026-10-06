use std::ops::Range;

const ROW_CENTER: f32 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalLayout {
    line_height: f32,
    row_count: usize,
}

impl VerticalLayout {
    pub fn new(line_height: f32, row_count: usize) -> Self {
        Self {
            line_height,
            row_count,
        }
    }

    pub fn row_top(&self, row: usize) -> f32 {
        row as f32 * self.line_height
    }

    pub fn row_bottom(&self, row: usize) -> f32 {
        self.row_top(row) + self.line_height
    }

    pub fn row_center(&self, row: usize) -> f32 {
        (row as f32 + ROW_CENTER) * self.line_height
    }

    pub fn row_at(&self, y: f32) -> usize {
        ((y / self.line_height).floor().max(0.0) as usize).min(self.row_count.saturating_sub(1))
    }

    pub fn content_height(&self) -> f32 {
        self.row_count as f32 * self.line_height
    }

    pub fn visible_rows(&self, top: f32, height: f32, overscan: usize) -> Range<usize> {
        let first = (top / self.line_height).floor() as usize;
        let end =
            (first + (height / self.line_height).ceil() as usize + overscan).min(self.row_count);
        first.min(end)..end
    }
}
