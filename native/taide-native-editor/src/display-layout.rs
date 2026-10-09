use std::ops::Range;

const ROW_CENTER: f32 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalLayout {
    line_height: f32,
    row_count: usize,
    zone: Option<(usize, f32)>,
}

impl VerticalLayout {
    pub fn new(line_height: f32, row_count: usize) -> Self {
        Self {
            line_height,
            row_count,
            zone: None,
        }
    }

    pub fn with_zone(mut self, after_row: usize, height: f32) -> Self {
        self.zone = (after_row < self.row_count && height.is_finite() && height > 0.0)
            .then_some((after_row, height));
        self
    }

    pub fn zone(&self) -> Option<Range<f32>> {
        let (row, height) = self.zone?;
        let top = (row + 1) as f32 * self.line_height;
        Some(top..top + height)
    }

    pub fn row_top(&self, row: usize) -> f32 {
        row as f32 * self.line_height
            + self
                .zone
                .filter(|(after, _)| row > *after)
                .map_or(0.0, |(_, height)| height)
    }

    pub fn row_bottom(&self, row: usize) -> f32 {
        self.row_top(row) + self.line_height
    }

    pub fn row_center(&self, row: usize) -> f32 {
        self.row_top(row) + ROW_CENTER * self.line_height
    }

    pub fn row_at(&self, y: f32) -> usize {
        let y = if let Some(zone) = self.zone() {
            if zone.contains(&y) {
                return self.zone.unwrap().0;
            }
            if y >= zone.end {
                y - (zone.end - zone.start)
            } else {
                y
            }
        } else {
            y
        };
        ((y / self.line_height).floor().max(0.0) as usize).min(self.row_count.saturating_sub(1))
    }

    pub fn content_height(&self) -> f32 {
        self.row_count as f32 * self.line_height + self.zone.map_or(0.0, |(_, height)| height)
    }

    pub fn visible_rows(&self, top: f32, height: f32, overscan: usize) -> Range<usize> {
        if self.zone.is_some() {
            if top >= self.content_height() {
                return self.row_count..self.row_count;
            }
            let first = self.row_at(top);
            let end = (self.row_at(top + height) + 1 + overscan).min(self.row_count);
            return first.min(end)..end;
        }
        let first = (top / self.line_height).floor() as usize;
        let end =
            (first + (height / self.line_height).ceil() as usize + overscan).min(self.row_count);
        first.min(end)..end
    }
}
