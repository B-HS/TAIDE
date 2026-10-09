use egui::{
    Color32, ColorImage, Id, Pos2, Rect, Response, Sense, TextureHandle, TextureOptions, Ui, pos2,
};
use taide_model::file::FileSizeTier;
use taide_native_editor::display_map::{DisplayMap, WrapSettings};
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::line_breaks::is_full_width_character;
use taide_native_editor::line_tokens::TokenStyleTable;
use taide_native_editor::view::SelectionSet;

use crate::editor_minimap_layout::{
    MINIMAP_GUTTER_WIDTH, MinimapDimensions, MinimapLayout, MinimapViewport, glyph_intensity,
    glyph_pixel,
};
use crate::editor_surface::{EditorAppearance, EditorDisplayOptions, EditorTokens};

const CANVAS_OPACITY: f32 = 0.9;
const HALF_OPACITY: f32 = 0.5;
const LIGHT_BACKGROUND_LUMA: f64 = 0.5;
const CHANNEL_MAX: f64 = 255.0;
const LINEAR_THRESHOLD: f64 = 0.03928;
const LINEAR_DIVISOR: f64 = 12.92;
const GAMMA_OFFSET: f64 = 0.055;
const GAMMA_DIVISOR: f64 = 1.055;
const GAMMA_EXPONENT: f64 = 2.4;
const LUMA_WEIGHTS: [f64; 3] = [0.2126, 0.7152, 0.0722];
const GLYPH_HEIGHT: usize = 2;
const SLIDER_TRANSITION_SECONDS: f64 = 0.1;
const POINTER_DRAG_RESET_DISTANCE: f32 = 140.0;
const SHADOW_WIDTH: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorMinimapColors {
    pub background: Color32,
    pub selection: Color32,
    pub slider: Color32,
    pub slider_hover: Color32,
    pub slider_active: Color32,
    pub shadow: Color32,
}

pub(crate) fn measure(
    ui: &Ui,
    document: &DocumentSnapshot,
    appearance: &EditorAppearance,
    options: &EditorDisplayOptions,
    rect: Rect,
    scrollbar_width: f32,
) -> Option<MinimapDimensions> {
    if !options.minimap
        || options.minimap_colors.is_none()
        || document.metadata.tier != FileSizeTier::Normal
    {
        return None;
    }
    let gutter = crate::editor_gutter::Gutter::measure(
        ui.painter(),
        document.rope.len_lines(),
        appearance,
        options.folding,
    );
    let character_width = ui
        .painter()
        .layout_no_wrap("n".into(), appearance.font.clone(), appearance.foreground)
        .size()
        .x;
    let dimensions = MinimapDimensions::new(
        f64::from((rect.width() - gutter.width()).max(0.0)),
        f64::from(rect.height()),
        f64::from(character_width),
        f64::from(scrollbar_width),
        f64::from(ui.ctx().pixels_per_point()),
    );
    (dimensions.image_width > 0 && dimensions.image_height > 0).then_some(dimensions)
}

#[derive(Clone, PartialEq)]
struct ImageKey {
    document: DocumentId,
    revision: u64,
    token_generation: Option<u64>,
    dimensions: MinimapDimensions,
    rows: std::ops::Range<usize>,
    tab_size: u32,
    wrap: Option<WrapSettings>,
    hidden: Vec<std::ops::Range<usize>>,
    background: Color32,
    foreground: Color32,
    max_texture_side: usize,
}

#[derive(Clone, Default)]
pub(crate) struct MinimapState {
    image_key: Option<ImageKey>,
    styles: Option<TokenStyleTable>,
    textures: Vec<MinimapTile>,
    layout: Option<MinimapLayout>,
    document: Option<(DocumentId, u64, MinimapDimensions)>,
    drag: Option<Drag>,
    touch: Option<(egui::TouchDeviceId, egui::TouchId)>,
    opacity: f32,
    opacity_from: f32,
    opacity_started: f64,
    opacity_visible: bool,
}

#[derive(Clone)]
struct Drag {
    position: Pos2,
    layout: MinimapLayout,
}

#[derive(Clone)]
struct MinimapTile {
    origin: [usize; 2],
    texture: TextureHandle,
}

pub(crate) struct MinimapInput {
    pub(crate) response: Response,
    pub(crate) requested_scroll: Option<(f32, bool)>,
    pub(crate) event_index: Option<usize>,
}

impl MinimapState {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn layout(
        &mut self,
        document: &DocumentSnapshot,
        dimensions: MinimapDimensions,
        viewport: MinimapViewport,
    ) -> MinimapLayout {
        let tag = (document.id, document.revision, dimensions);
        if self.document != Some(tag) {
            self.layout = None;
            self.drag = None;
            self.touch = None;
            self.document = Some(tag);
        }
        let layout = MinimapLayout::new(dimensions, viewport, self.layout.as_ref());
        self.layout = Some(layout.clone());
        layout
    }

    pub(crate) fn interact(
        &mut self,
        ui: &Ui,
        id: Id,
        rect: Rect,
        layout: &MinimapLayout,
        dimensions: MinimapDimensions,
        viewport_height: f32,
        line_height: f32,
        row_count: usize,
    ) -> MinimapInput {
        let response = ui.interact(rect, id.with("minimap"), Sense::CLICK | Sense::DRAG);
        ui.ctx()
            .register_pointer_preserves_keyboard_focus(response.id);
        let slider = slider_rect(rect, layout);
        let mut requested_scroll = None;
        let mut event_index = None;
        if !ui.is_enabled() {
            self.drag = None;
            self.touch = None;
            return MinimapInput {
                response,
                requested_scroll,
                event_index,
            };
        }
        let pointer = ui.input(|input| input.pointer.clone());
        let events = ui.input(|input| input.raw.events.clone());
        for (index, event) in events.iter().enumerate() {
            let admitted = ui.ctx().pointer_focus_preserving_trigger_at(index) == Some(response.id);
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } if admitted && self.touch.is_none() && rect.contains(*pos) => {
                    self.drag = None;
                    event_index = Some(index);
                    if layout.slider_needed && slider.contains(*pos) {
                        self.drag = Some(Drag {
                            position: *pos,
                            layout: layout.clone(),
                        });
                        requested_scroll = Some((layout.scroll_top as f32, true));
                    } else {
                        let row = ((pos.y - rect.top()) as f64 / dimensions.line_height())
                            .floor()
                            .max(0.0) as usize
                            + layout.rows.start;
                        let row = row.min(row_count.saturating_sub(1));
                        requested_scroll = Some((
                            (row as f32 + HALF_OPACITY) * line_height
                                - viewport_height * HALF_OPACITY,
                            false,
                        ));
                    }
                }
                egui::Event::PointerMoved(position) if self.touch.is_none() => {
                    if let Some(drag) = self.drag.as_ref() {
                        let perpendicular = (position.x - drag.position.x)
                            .abs()
                            .min((position.x - rect.left()).abs())
                            .min((position.x - rect.right()).abs());
                        let scroll = if ui.ctx().os() == egui::os::OperatingSystem::Windows
                            && perpendicular > POINTER_DRAG_RESET_DISTANCE
                        {
                            drag.layout.scroll_top
                        } else {
                            drag.layout
                                .scroll_from_delta(f64::from(position.y - drag.position.y))
                        };
                        requested_scroll = Some((scroll as f32, true));
                        event_index = Some(index);
                    }
                }
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                }
                | egui::Event::PointerGone => self.drag = None,
                egui::Event::Touch {
                    device_id,
                    id,
                    phase: egui::TouchPhase::Start,
                    pos,
                    ..
                } if admitted && self.touch.is_none() && rect.contains(*pos) => {
                    self.touch = Some((*device_id, *id));
                    self.drag = None;
                    requested_scroll = Some((
                        layout.scroll_from_touch(f64::from(pos.y - rect.top())) as f32,
                        true,
                    ));
                    event_index = Some(index);
                }
                egui::Event::Touch {
                    device_id,
                    id,
                    phase,
                    pos,
                    ..
                } if self.touch == Some((*device_id, *id)) => match phase {
                    egui::TouchPhase::Move => {
                        requested_scroll = Some((
                            layout.scroll_from_touch(f64::from(pos.y - rect.top())) as f32,
                            true,
                        ));
                        event_index = Some(index);
                    }
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => self.touch = None,
                    egui::TouchPhase::Start => {}
                },
                egui::Event::WindowFocused(false) => {
                    self.drag = None;
                    self.touch = None;
                }
                _ => {}
            }
        }
        if !pointer.primary_down() {
            self.drag = None;
        }
        let time = ui.input(|input| input.time);
        let visible = response.hovered() || self.drag.is_some() || self.touch.is_some();
        let progress =
            ((time - self.opacity_started) / SLIDER_TRANSITION_SECONDS).clamp(0.0, 1.0) as f32;
        let target = if self.opacity_visible { 1.0 } else { 0.0 };
        self.opacity = self.opacity_from + (target - self.opacity_from) * progress;
        if visible != self.opacity_visible {
            self.opacity_visible = visible;
            self.opacity_from = self.opacity;
            self.opacity_started = time;
        }
        if progress < 1.0 || visible != (self.opacity >= 1.0) {
            ui.ctx().request_repaint();
        }
        MinimapInput {
            response,
            requested_scroll,
            event_index,
        }
    }
}

pub(crate) struct MinimapFrame<'a> {
    pub(crate) id: Id,
    pub(crate) document: &'a DocumentSnapshot,
    pub(crate) display: &'a DisplayMap,
    pub(crate) dimensions: MinimapDimensions,
    pub(crate) layout: &'a MinimapLayout,
    pub(crate) rect: Rect,
    pub(crate) appearance: &'a EditorAppearance,
    pub(crate) colors: EditorMinimapColors,
    pub(crate) tokens: Option<EditorTokens<'a>>,
    pub(crate) tab_size: u32,
    pub(crate) selection: &'a SelectionSet,
    pub(crate) hovered: bool,
    pub(crate) horizontal_overflow: bool,
}

pub(crate) fn paint(ui: &Ui, state: &mut MinimapState, frame: MinimapFrame<'_>) {
    let key = ImageKey {
        document: frame.document.id,
        revision: frame.document.revision,
        token_generation: frame.tokens.map(|tokens| tokens.lines.generation()),
        dimensions: frame.dimensions,
        rows: frame.layout.rows.clone(),
        tab_size: frame.tab_size,
        wrap: frame.display.wrap_settings().cloned(),
        hidden: frame.display.hidden_lines().to_vec(),
        background: frame.colors.background,
        foreground: frame.appearance.foreground,
        max_texture_side: ui.input(|input| input.max_texture_side).max(1),
    };
    let styles_match = match (state.styles.as_ref(), frame.tokens) {
        (Some(previous), Some(current)) => previous == current.styles,
        (None, None) => true,
        _ => false,
    };
    if state.image_key.as_ref() != Some(&key) || !styles_match {
        let image = render_image(&frame);
        let side = key.max_texture_side;
        let tiles = if image.width() <= side && image.height() <= side {
            vec![([0, 0], image)]
        } else {
            (0..image.height())
                .step_by(side)
                .flat_map(|y| (0..image.width()).step_by(side).map(move |x| [x, y]))
                .map(|origin| {
                    let size = [
                        (image.width() - origin[0]).min(side),
                        (image.height() - origin[1]).min(side),
                    ];
                    (origin, image.region_by_pixels(origin, size))
                })
                .collect()
        };
        let count = tiles.len();
        for (index, (origin, image)) in tiles.into_iter().enumerate() {
            if let Some(tile) = state.textures.get_mut(index) {
                tile.origin = origin;
                tile.texture.set(image, TextureOptions::NEAREST);
            } else {
                state.textures.push(MinimapTile {
                    origin,
                    texture: ui.ctx().load_texture(
                        format!("native-minimap-{:?}-{index}", frame.id),
                        image,
                        TextureOptions::NEAREST,
                    ),
                });
            }
        }
        state.textures.truncate(count);
        state.image_key = Some(key);
        state.styles = frame.tokens.map(|tokens| tokens.styles.clone());
    }
    let painter = ui.painter().with_clip_rect(frame.rect);
    painter.rect_filled(frame.rect, 0.0, frame.colors.background);
    for tile in &state.textures {
        let pixel_ratio = frame.dimensions.pixel_ratio as f32;
        let texture = &tile.texture;
        let image_rect = Rect::from_min_size(
            frame.rect.min + egui::vec2(tile.origin[0] as f32, tile.origin[1] as f32) / pixel_ratio,
            egui::vec2(texture.size()[0] as f32, texture.size()[1] as f32) / pixel_ratio,
        );
        painter.image(
            texture.id(),
            image_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE.gamma_multiply(CANVAS_OPACITY),
        );
    }
    paint_selections(&painter, &frame);
    if frame.horizontal_overflow {
        let mut mesh = egui::Mesh::default();
        let x1 = frame.rect.left() - SHADOW_WIDTH;
        let x2 = frame.rect.left();
        mesh.colored_vertex(pos2(x1, frame.rect.top()), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(x2, frame.rect.top()), frame.colors.shadow);
        mesh.colored_vertex(pos2(x1, frame.rect.bottom()), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(x2, frame.rect.bottom()), frame.colors.shadow);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 2, 3);
        ui.painter().add(mesh);
    }
    if frame.layout.slider_needed && state.opacity > 0.0 {
        let slider = slider_rect(frame.rect, frame.layout);
        let hovered = frame.hovered
            && ui.input(|input| {
                input
                    .pointer
                    .hover_pos()
                    .is_some_and(|point| slider.contains(point))
            });
        let color = if state.drag.is_some() || state.touch.is_some() {
            frame.colors.slider_active
        } else if hovered {
            frame.colors.slider_hover
        } else {
            frame.colors.slider
        };
        painter.rect_filled(slider, 0.0, color.gamma_multiply(state.opacity));
    }
}

fn slider_rect(rect: Rect, layout: &MinimapLayout) -> Rect {
    Rect::from_min_max(
        pos2(rect.left(), rect.top() + layout.slider_top as f32),
        pos2(
            rect.right(),
            rect.top() + (layout.slider_top + layout.slider_height) as f32,
        ),
    )
    .intersect(rect)
}

fn render_image(frame: &MinimapFrame<'_>) -> ColorImage {
    let size = [frame.dimensions.image_width, frame.dimensions.image_height];
    let mut image = ColorImage::filled(size, frame.colors.background);
    let background = frame.colors.background.to_srgba_unmultiplied();
    let background = [background[0], background[1], background[2]];
    let light = relative_luminance(background) >= LIGHT_BACKGROUND_LUMA;
    let scale = frame.dimensions.scale;
    for row in frame.layout.rows.clone() {
        let y = (row - frame.layout.rows.start) * frame.dimensions.line_pixels();
        if y + scale * GLYPH_HEIGHT > size[1] {
            break;
        }
        let segment = frame.display.segment(frame.document, row);
        let line_start = frame.document.rope.line_to_byte(segment.line);
        let spans = frame
            .tokens
            .map(|tokens| tokens.lines.spans(segment.line))
            .unwrap_or_default();
        let mut span_index = 0;
        let mut x = MINIMAP_GUTTER_WIDTH + segment.indent_columns as usize * scale;
        let mut visible_column = segment.indent_columns as usize;
        let mut byte = segment.bytes.start;
        for character in frame.document.rope.byte_slice(segment.bytes).chars() {
            while span_index + 3 < spans.len()
                && spans[span_index + 2] as usize <= byte - line_start
            {
                span_index += 2;
            }
            let foreground = frame.tokens.map_or(
                frame.appearance.foreground.to_srgba_unmultiplied(),
                |tokens| {
                    let style = spans
                        .get(span_index + 1)
                        .copied()
                        .map(|id| tokens.styles.style(id));
                    style.map_or(
                        frame.appearance.foreground.to_srgba_unmultiplied(),
                        |style| style.foreground,
                    )
                },
            );
            byte += character.len_utf8();
            let foreground_alpha = foreground[3];
            let foreground = [foreground[0], foreground[1], foreground[2]];
            let mut utf16 = [0; 2];
            for code in character.encode_utf16(&mut utf16) {
                if x + scale > size[0] {
                    break;
                }
                if *code == u16::from(b'\t') {
                    let tab = frame.tab_size.max(1) as usize;
                    let width = tab - visible_column % tab;
                    visible_column += width;
                    x += width * scale;
                    continue;
                }
                visible_column += 1;
                if *code == u16::from(b' ') {
                    x += scale;
                    continue;
                }
                let count = if char::from_u32(u32::from(*code)).is_some_and(is_full_width_character)
                {
                    2
                } else {
                    1
                };
                for _ in 0..count {
                    if x + scale > size[0] {
                        break;
                    }
                    for dy in 0..scale * GLYPH_HEIGHT {
                        for dx in 0..scale {
                            let intensity = glyph_intensity(scale, *code, dy * scale + dx, light);
                            let rgba =
                                glyph_pixel(intensity, foreground, background, foreground_alpha);
                            image[(x + dx, y + dy)] = Color32::from_rgb(rgba[0], rgba[1], rgba[2]);
                        }
                    }
                    x += scale;
                }
            }
            if x + scale > size[0] {
                break;
            }
        }
    }
    image
}

fn relative_luminance(rgb: [u8; 3]) -> f64 {
    rgb.into_iter()
        .zip(LUMA_WEIGHTS)
        .map(|(channel, weight)| {
            let channel = f64::from(channel) / CHANNEL_MAX;
            let linear = if channel <= LINEAR_THRESHOLD {
                channel / LINEAR_DIVISOR
            } else {
                ((channel + GAMMA_OFFSET) / GAMMA_DIVISOR).powf(GAMMA_EXPONENT)
            };
            linear * weight
        })
        .sum()
}

fn paint_selections(painter: &egui::Painter, frame: &MinimapFrame<'_>) {
    let line_height = frame.dimensions.line_height() as f32;
    let pixel_ratio = frame.dimensions.pixel_ratio as f32;
    for selection in &frame.selection.selections {
        if selection.anchor == selection.head {
            continue;
        }
        let bytes = selection.anchor.min(selection.head)..selection.anchor.max(selection.head);
        let first = frame.display.row_of_byte(frame.document, bytes.start);
        let last = frame.display.row_of_byte(frame.document, bytes.end);
        for row in first.max(frame.layout.rows.start)..(last + 1).min(frame.layout.rows.end) {
            let segment = frame.display.segment(frame.document, row);
            let x_at = |byte: usize| {
                let mut column = segment.indent_columns as usize;
                let mut x = MINIMAP_GUTTER_WIDTH + column * frame.dimensions.scale;
                for character in frame
                    .document
                    .rope
                    .byte_slice(
                        segment.bytes.start..byte.clamp(segment.bytes.start, segment.bytes.end),
                    )
                    .chars()
                {
                    if character == '\t' {
                        let width = frame.tab_size.max(1) as usize
                            - column % frame.tab_size.max(1) as usize;
                        column += width;
                        x += width * frame.dimensions.scale;
                        continue;
                    }
                    column += character.len_utf16();
                    x += frame.dimensions.scale
                        * if is_full_width_character(character) {
                            2
                        } else {
                            character.len_utf16()
                        };
                }
                x.min(frame.dimensions.image_width) as f32 / pixel_ratio
            };
            let y = frame.rect.top() + (row - frame.layout.rows.start) as f32 * line_height;
            if row < last {
                painter.rect_filled(
                    Rect::from_min_max(
                        pos2(
                            frame.rect.left() + MINIMAP_GUTTER_WIDTH as f32 / pixel_ratio,
                            y,
                        ),
                        pos2(frame.rect.right(), y + line_height),
                    ),
                    0.0,
                    frame
                        .colors
                        .selection
                        .gamma_multiply(HALF_OPACITY * CANVAS_OPACITY),
                );
            }
            let x1 = frame.rect.left() + x_at(bytes.start);
            let x2 = frame.rect.left() + x_at(bytes.end);
            if x2 > x1 {
                painter.rect_filled(
                    Rect::from_min_max(pos2(x1, y), pos2(x2, y + line_height)),
                    0.0,
                    frame.colors.selection.gamma_multiply(CANVAS_OPACITY),
                );
            }
        }
    }
}
