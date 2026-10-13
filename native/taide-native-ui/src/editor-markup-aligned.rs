use egui::{Align, Label, Stroke, Ui};
use taide_native_editor::documentation::Inline;

use super::{Colors, Provider, TextWeight, inline_font};
use crate::editor_surface::EditorAppearance;

pub(super) fn show(
    ui: &mut Ui,
    contents: &[Inline],
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
    size: f32,
    weight: TextWeight,
) -> bool {
    let alignment = ui.layout().horizontal_align();
    if alignment == Align::Min
        || contents.iter().any(|inline| match inline {
            Inline::Text(span) => span.style.code,
            Inline::Image { .. } => true,
            Inline::Break => false,
        })
    {
        return false;
    }
    let mut job = egui::text::LayoutJob::default();
    let mut links = Vec::new();
    let mut character_count = 0;
    for inline in contents {
        match inline {
            Inline::Text(span) => {
                let mut format = egui::TextFormat {
                    font_id: inline_font(ui, span.style, appearance, size, weight, true),
                    color: colors.foreground,
                    italics: span.style.italic,
                    line_height: Some(appearance.line_height),
                    ..Default::default()
                };
                if span.style.strike {
                    format.strikethrough = Stroke::new(1.0, colors.foreground)
                }
                if span.link.is_some() {
                    format.color = colors.link;
                    format.underline = Stroke::new(1.0, colors.link)
                }
                job.append(&span.text, 0.0, format);
                let end = character_count + span.text.chars().count();
                if let Some(link) = &span.link {
                    links.push((character_count..end, link));
                }
                character_count = end;
            }
            Inline::Break => {
                job.append(
                    "\n",
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(size),
                        line_height: Some(appearance.line_height),
                        ..Default::default()
                    },
                );
                character_count += 1;
            }
            Inline::Image { .. } => unreachable!(),
        }
    }
    job.halign = alignment;
    job.wrap.max_width = ui.available_width();
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let has_links = !links.is_empty();
    let mut label = Label::new(galley.clone()).selectable(true);
    if has_links {
        label = label.sense(egui::Sense::click())
    }
    let response = ui.add(label);
    if !has_links {
        return true;
    }
    ui.ctx()
        .register_pointer_preserves_keyboard_focus(response.id);
    let Some(position) = ui.input(|input| input.pointer.hover_pos()) else {
        return true;
    };
    if !response.contains_pointer() {
        return true;
    }
    let origin = match alignment {
        Align::Min => response.rect.left_top(),
        Align::Center => response.rect.center_top(),
        Align::Max => response.rect.right_top(),
    };
    let mut character = 0;
    for row in &galley.rows {
        for glyph in &row.glyphs {
            let rect = glyph
                .logical_rect()
                .translate(origin.to_vec2() + row.pos.to_vec2());
            if rect.contains(position)
                && let Some((_, link)) = links.iter().find(|(range, _)| range.contains(&character))
            {
                if response.clicked() {
                    provider.open_link(ui, &link.target);
                }
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if !link.title.is_empty() {
                    response.on_hover_text(&link.title);
                }
                return true;
            }
            character += 1;
        }
        if row.ends_with_newline {
            character += 1
        }
    }
    true
}
