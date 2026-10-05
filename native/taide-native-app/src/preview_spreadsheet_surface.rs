use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Id, Stroke, Ui, vec2};
use taide_model::ids::TabId;
use taide_model::locale::ResolvedLocale;

use crate::presentation::message;
use crate::preview_spreadsheet::{Cell, Workbook};
use crate::preview_spreadsheet_cache::Cache;

const TEXT_SIZE: f32 = 12.0;
const LINE_HEIGHT: f32 = 16.0;
const PADDING_X: f32 = 8.0;
const PADDING_Y: f32 = 4.0;
const HEADER_PADDING_X: i8 = 8;
const HEADER_PADDING_Y: i8 = 4;
const NOTICE_PADDING_X: i8 = 12;
const GAP: f32 = 4.0;
const BORDER: f32 = 1.0;
const CORNER: u8 = 2;
const ICON_SIZE: f32 = 20.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_STROKE: f32 = 2.0;
const STATUS_TEXT_SIZE: f32 = 14.0;
const STATUS_GAP: f32 = 12.0;

pub struct Appearance {
    pub status: crate::preview_status::Appearance,
    pub header: Color32,
    pub selected_background: Color32,
    pub selected_foreground: Color32,
    pub inactive_foreground: Color32,
    pub hover: Color32,
    pub cell_border: Color32,
    pub warning: Color32,
}

pub fn sheet_id(viewport: egui::ViewportId, tab: &TabId, index: usize) -> Id {
    Id::new(("native-spreadsheet-sheet", viewport, tab, index))
}

fn nowrap(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars() {
        if matches!(character, ' ' | '\t' | '\r' | '\n' | '\u{c}') {
            pending_space = !output.is_empty();
            continue;
        }
        if pending_space {
            output.push(' ');
            pending_space = false;
        }
        output.push(character);
    }
    output
}

fn empty(ui: &mut Ui, locale: &ResolvedLocale, appearance: &Appearance, key: &str) {
    let label = message(locale, key, &[]);
    let text = ui.painter().layout_no_wrap(
        label,
        FontId::proportional(STATUS_TEXT_SIZE),
        appearance.status.foreground,
    );
    let height = ICON_SIZE + STATUS_GAP + text.size().y;
    ui.add_space(((ui.available_height() - height) / 2.0).max(0.0));
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = STATUS_GAP;
        let (_, rect) = ui.allocate_space(vec2(ICON_SIZE, ICON_SIZE));
        let point = |[x, y]: [f32; 2]| rect.min + vec2(x, y) * (ICON_SIZE / ICON_VIEWBOX);
        let stroke = Stroke::new(
            ICON_STROKE * ICON_SIZE / ICON_VIEWBOX,
            appearance.status.foreground,
        );
        let line = |values: &[[f32; 2]]| {
            ui.painter().add(egui::Shape::line(
                values.iter().copied().map(point).collect(),
                stroke,
            ));
        };
        ui.painter().rect_stroke(
            egui::Rect::from_min_max(point([3.0, 3.0]), point([21.0, 21.0])),
            egui::CornerRadius::same(CORNER),
            stroke,
            egui::StrokeKind::Middle,
        );
        line(&[[3.0, 9.0], [21.0, 9.0]]);
        line(&[[3.0, 15.0], [21.0, 15.0]]);
        line(&[[15.0, 3.0], [15.0, 21.0]]);
        ui.add(egui::Label::new(text));
    });
}

fn widths(
    ui: &Ui,
    cache: &mut Cache,
    workbook: &Arc<Workbook>,
    tab: &TabId,
    selected: usize,
) -> Arc<Vec<f32>> {
    if let Some(widths) = cache.column_widths(tab, selected) {
        return widths;
    }
    let sheet = &workbook.sheets[selected];
    let mut widths = vec![PADDING_X * 2.0 + BORDER; sheet.rows.first().map_or(0, Vec::len)];
    for row in &sheet.rows {
        for (index, cell) in row.iter().enumerate() {
            if matches!(cell, Cell::Null) {
                continue;
            }
            let text = ui.painter().layout_no_wrap(
                nowrap(&cell.display()),
                FontId::proportional(TEXT_SIZE),
                Color32::WHITE,
            );
            widths[index] = widths[index].max(text.size().x + PADDING_X * 2.0 + BORDER);
        }
    }
    let widths = Arc::new(widths);
    cache.set_column_widths(tab, selected, widths.clone());
    widths
}

pub fn show(
    ui: &mut Ui,
    cache: &mut Cache,
    tab: &TabId,
    path: &str,
    file_name: &str,
    locale: &ResolvedLocale,
    appearance: &Appearance,
) -> bool {
    cache.ensure(tab, path);
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::ZERO,
        appearance.status.background,
    );
    ui.scope(|ui| {
        ui.visuals_mut().override_text_color = Some(appearance.status.foreground);
        ui.spacing_mut().item_spacing.y = 0.0;
        if let Some(failure) = cache.error(tab) {
            return crate::preview_status::show(
                ui,
                tab,
                Some(failure),
                file_name,
                locale,
                &appearance.status,
                "preview.spreadsheet.loadFailed",
            );
        }
        let Some(workbook) = cache.workbook(tab).cloned() else {
            return false;
        };
        if workbook.sheets.is_empty() {
            empty(ui, locale, appearance, "preview.spreadsheet.noSheets");
            return false;
        }
        let selected = cache.selected(tab).unwrap_or_default();
        egui::Frame::NONE
            .fill(appearance.header)
            .inner_margin(egui::Margin::symmetric(HEADER_PADDING_X, HEADER_PADDING_Y))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.x = GAP;
                egui::ScrollArea::horizontal()
                    .id_salt(("native-spreadsheet-sheet-scroll", tab))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (index, sheet) in workbook.sheets.iter().enumerate() {
                                let is_selected = index == selected;
                                let foreground = if is_selected {
                                    appearance.selected_foreground
                                } else {
                                    appearance.inactive_foreground
                                };
                                let text = ui.painter().layout_no_wrap(
                                    nowrap(&sheet.name),
                                    FontId::proportional(TEXT_SIZE),
                                    foreground,
                                );
                                let (_, rect) = ui.allocate_space(vec2(
                                    text.size().x + PADDING_X * 2.0,
                                    LINE_HEIGHT + PADDING_Y * 2.0,
                                ));
                                let response = ui.interact(
                                    rect,
                                    sheet_id(ui.ctx().viewport_id(), tab, index),
                                    egui::Sense::click(),
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::SelectableLabel,
                                        ui.is_enabled(),
                                        is_selected,
                                        &sheet.name,
                                    )
                                });
                                ui.ctx().accesskit_node_builder(response.id, |node| {
                                    node.set_role(egui::accesskit::Role::Tab);
                                    node.set_selected(is_selected);
                                    node.clear_toggled();
                                });
                                if is_selected {
                                    ui.painter().rect_filled(
                                        rect,
                                        egui::CornerRadius::same(CORNER),
                                        appearance.selected_background,
                                    );
                                } else if response.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        egui::CornerRadius::same(CORNER),
                                        appearance.hover,
                                    );
                                }
                                if response.has_focus() {
                                    ui.painter().rect_stroke(
                                        rect,
                                        egui::CornerRadius::same(CORNER),
                                        ui.visuals().selection.stroke,
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                ui.painter().galley(
                                    egui::pos2(
                                        rect.left() + PADDING_X,
                                        rect.center().y - text.size().y / 2.0,
                                    ),
                                    text,
                                    foreground,
                                );
                                if response.clicked() && cache.select(tab, index) {
                                    ui.ctx().request_repaint();
                                }
                            }
                            ui.ctx().accesskit_node_builder(ui.id(), |node| {
                                node.set_role(egui::accesskit::Role::TabList)
                            });
                        });
                    });
            });
        ui.painter().hline(
            ui.min_rect().x_range(),
            ui.min_rect().bottom() - BORDER / 2.0,
            Stroke::new(BORDER, appearance.status.border),
        );
        let sheet = &workbook.sheets[selected];
        if sheet.truncated {
            egui::Frame::NONE
                .fill(appearance.header)
                .inner_margin(egui::Margin::symmetric(NOTICE_PADDING_X, HEADER_PADDING_Y))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(message(
                                locale,
                                "preview.spreadsheet.truncatedNotice",
                                &[
                                    ("shown", &sheet.rows.len().to_string()),
                                    ("total", &sheet.total_row_count.to_string()),
                                ],
                            ))
                            .size(TEXT_SIZE)
                            .line_height(Some(LINE_HEIGHT))
                            .color(appearance.warning),
                        )
                        .wrap(),
                    );
                });
            ui.painter().hline(
                ui.min_rect().x_range(),
                ui.min_rect().bottom() - BORDER / 2.0,
                Stroke::new(BORDER, appearance.status.border),
            );
        }
        if sheet.rows.is_empty() {
            empty(ui, locale, appearance, "preview.spreadsheet.emptySheet");
            return false;
        }
        let widths = widths(ui, cache, &workbook, tab, selected);
        let natural_width: f32 = widths.iter().sum();
        let width = natural_width.max(ui.available_width());
        let extra = if widths.is_empty() {
            0.0
        } else {
            (width - natural_width) / widths.len() as f32
        };
        let row_height = LINE_HEIGHT + PADDING_Y * 2.0 + BORDER;
        egui::ScrollArea::both()
            .id_salt((
                "native-spreadsheet-table-scroll",
                tab,
                cache.scroll_generation(tab),
            ))
            .show_rows(ui, row_height, sheet.rows.len(), |ui, range| {
                ui.set_min_width(width);
                ui.spacing_mut().item_spacing.y = 0.0;
                for row_index in range {
                    let (_, rect) = ui.allocate_space(vec2(width, row_height));
                    if row_index % 2 == 1 {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::ZERO, appearance.header);
                    }
                    let mut x = rect.left();
                    for (column, cell) in sheet.rows[row_index].iter().enumerate() {
                        let cell_width = widths[column] + extra;
                        let cell_rect = egui::Rect::from_min_size(
                            egui::pos2(x, rect.top()),
                            vec2(cell_width, row_height),
                        );
                        x += cell_width;
                        if !cell_rect.intersects(ui.clip_rect()) {
                            continue;
                        }
                        ui.painter().rect_stroke(
                            cell_rect,
                            egui::CornerRadius::ZERO,
                            Stroke::new(BORDER, appearance.cell_border),
                            egui::StrokeKind::Inside,
                        );
                        let label = nowrap(&cell.display());
                        let response = ui.interact(
                            cell_rect,
                            Id::new((
                                "native-spreadsheet-cell",
                                ui.ctx().viewport_id(),
                                tab,
                                row_index,
                                column,
                            )),
                            egui::Sense::hover(),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Label,
                                ui.is_enabled(),
                                &label,
                            )
                        });
                        ui.ctx().accesskit_node_builder(response.id, |node| {
                            node.set_role(egui::accesskit::Role::Cell);
                            node.set_row_index(row_index);
                            node.set_column_index(column);
                        });
                        let text = ui.painter().layout_no_wrap(
                            label,
                            FontId::proportional(TEXT_SIZE),
                            appearance.status.foreground,
                        );
                        ui.painter().galley(
                            egui::pos2(
                                cell_rect.left() + PADDING_X,
                                cell_rect.center().y - text.size().y / 2.0,
                            ),
                            text,
                            appearance.status.foreground,
                        );
                    }
                }
                ui.ctx().accesskit_node_builder(ui.id(), |node| {
                    node.set_role(egui::accesskit::Role::Table);
                    node.set_row_count(sheet.rows.len());
                    node.set_column_count(widths.len());
                });
            });
        false
    })
    .inner
}
