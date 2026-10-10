use std::ops::Range;

const ROW_CENTER: f32 = 0.5;

#[derive(Debug, Clone, PartialEq)]
struct AdditionalZone {
    after_row: usize,
    height: f32,
    order: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalLayout {
    line_height: f32,
    row_count: usize,
    zone: Option<(usize, f32)>,
    additional: Vec<AdditionalZone>,
}

impl VerticalLayout {
    pub fn new(line_height: f32, row_count: usize) -> Self {
        Self {
            line_height,
            row_count,
            zone: None,
            additional: Vec::new(),
        }
    }

    pub fn with_zone(mut self, after_row: usize, height: f32) -> Self {
        self.zone = (after_row < self.row_count && height.is_finite() && height > 0.0)
            .then_some((after_row, height));
        self
    }

    pub fn zone(&self) -> Option<Range<f32>> {
        let (row, height) = self.zone?;
        let top = (row + 1) as f32 * self.line_height
            + self
                .additional
                .iter()
                .filter(|zone| zone.after_row < row)
                .map(|zone| zone.height)
                .sum::<f32>();
        Some(top..top + height)
    }

    pub fn with_additional_zones(mut self, zones: impl IntoIterator<Item = (usize, f32)>) -> Self {
        self.additional = zones
            .into_iter()
            .enumerate()
            .filter_map(|(order, (after_row, height))| {
                (after_row < self.row_count && height.is_finite() && height > 0.0).then_some(
                    AdditionalZone {
                        after_row,
                        height,
                        order,
                    },
                )
            })
            .collect();
        self.additional
            .sort_by_key(|zone| (zone.after_row, zone.order));
        self
    }

    pub fn additional_zone(&self, order: usize) -> Option<Range<f32>> {
        let zone = self.additional.iter().find(|zone| zone.order == order)?;
        let top = (zone.after_row + 1) as f32 * self.line_height
            + self
                .zone
                .filter(|(row, _)| *row <= zone.after_row)
                .map_or(0.0, |(_, height)| height)
            + self
                .additional
                .iter()
                .take_while(|candidate| candidate.order != order)
                .map(|zone| zone.height)
                .sum::<f32>();
        Some(top..top + zone.height)
    }

    pub fn row_top(&self, row: usize) -> f32 {
        row as f32 * self.line_height
            + self
                .zone
                .filter(|(after, _)| row > *after)
                .map_or(0.0, |(_, height)| height)
            + self
                .additional
                .iter()
                .filter(|zone| row > zone.after_row)
                .map(|zone| zone.height)
                .sum::<f32>()
    }

    pub fn row_bottom(&self, row: usize) -> f32 {
        self.row_top(row) + self.line_height
    }

    pub fn row_center(&self, row: usize) -> f32 {
        self.row_top(row) + ROW_CENTER * self.line_height
    }

    pub fn row_at(&self, y: f32) -> usize {
        let mut shift = 0.0;
        if let Some(zone) = self.zone() {
            if zone.contains(&y) {
                return self.zone.unwrap().0;
            }
            if y >= zone.end {
                shift += zone.end - zone.start;
            }
        }
        for zone in &self.additional {
            let range = self.additional_zone(zone.order).unwrap();
            if range.contains(&y) {
                return zone.after_row;
            }
            if y >= range.end {
                shift += zone.height;
            }
        }
        let y = y - shift;
        ((y / self.line_height).floor().max(0.0) as usize).min(self.row_count.saturating_sub(1))
    }

    pub fn content_height(&self) -> f32 {
        self.row_count as f32 * self.line_height
            + self.zone.map_or(0.0, |(_, height)| height)
            + self.additional.iter().map(|zone| zone.height).sum::<f32>()
    }

    pub fn visible_rows(&self, top: f32, height: f32, overscan: usize) -> Range<usize> {
        if self.zone.is_some() || !self.additional.is_empty() {
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
