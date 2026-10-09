use egui::{
    Align2, Color32, Context, Event, FontFamily, FontId, Id, Key, PointerButton, Rect, Sense,
    Stroke, StrokeKind, TextFormat, Ui, UiBuilder, Vec2, pos2, vec2,
};
use taide_native_editor::display_map::DisplayMap;
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::problem_navigation::{Command, Coordinate};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;

use crate::editor_diagnostics::DiagnosticColors;

const MESSAGE_LINES: usize = 17;
const DECORATION_LINES: usize = 3;
const MINIMUM_MAX_LINES: f32 = 12.0;
const MAX_VIEWPORT_RATIO: f32 = 0.8;
const HEADER_RATIO: f32 = 1.2;
const ARROW_DIVISOR: f32 = 3.0;
const BORDER_WIDTH: f32 = 1.0;
const HEADER_OPACITY: f32 = 0.1;
const DETAILS_OPACITY: f32 = 0.6;
const ICON_SIZE: f32 = 16.0;
const BUTTON_SIZE: f32 = 22.0;
const BUTTON_COUNT: f32 = 3.0;
const BODY_LEFT: f32 = 20.0;
const BODY_RIGHT: f32 = 12.0;
const BODY_TOP: f32 = 8.0;
const HEADER_MARGIN: f32 = 4.0;
const PREVIOUS_ICON: char = '\u{eaa1}';
const NEXT_ICON: char = '\u{ea9a}';
const CLOSE_ICON: char = '\u{ea76}';
const ERROR_ICON: char = '\u{ea87}';
const WARNING_ICON: char = '\u{ea6c}';
const INFO_ICON: char = '\u{ea74}';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Widget {
    pub coordinate: Coordinate,
    pub position: usize,
    pub title: String,
}

impl Widget {
    pub(crate) fn height(&self, line_height: f32, viewport_height: f32) -> f32 {
        let lines = self
            .coordinate
            .problem
            .marker
            .message
            .text
            .split('\n')
            .count()
            .min(MESSAGE_LINES)
            .max(1);
        let maximum = MINIMUM_MAX_LINES.max(viewport_height / line_height * MAX_VIEWPORT_RATIO);
        ((DECORATION_LINES + lines) as f32).min(maximum) * line_height
    }

    pub(crate) fn zone(
        &self,
        document: &DocumentSnapshot,
        display: &DisplayMap,
        line_height: f32,
        viewport_height: f32,
    ) -> (usize, f32) {
        (
            display.row_of_byte(document, self.position.min(document.rope.len_bytes())),
            self.height(line_height, viewport_height),
        )
    }
}

pub trait Provider {
    fn current(&mut self, store: &EditorStore, view: ViewId) -> Option<Widget>;
    fn execute(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        command: Command,
    ) -> Result<bool, EditorError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub diagnostics: DiagnosticColors,
    pub background: Color32,
    pub heading: Color32,
    pub detail: Color32,
    pub hover: Color32,
    pub focus: Color32,
}

pub(crate) fn shortcut(event: &Event, composing: bool, visible: bool) -> Option<Command> {
    if composing {
        return None;
    }
    let Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    if *key == Key::Escape && modifiers.is_none() && visible {
        return Some(Command::Close);
    }
    if *key != Key::F8 || modifiers.ctrl || modifiers.command || modifiers.mac_cmd {
        return None;
    }
    match (modifiers.alt, modifiers.shift) {
        (true, false) => Some(Command::Next),
        (true, true) => Some(Command::Previous),
        (false, false) => Some(Command::NextInFiles),
        (false, true) => Some(Command::PreviousInFiles),
    }
}

pub(crate) struct Frame<'a> {
    pub(crate) id: Id,
    pub(crate) widget: &'a Widget,
    pub(crate) rect: Rect,
    pub(crate) clip: Rect,
    pub(crate) arrow_x: f32,
    pub(crate) line_height: f32,
    pub(crate) font: &'a FontId,
    pub(crate) colors: Colors,
}

pub(crate) struct Output {
    buttons: Vec<Button>,
}

#[derive(Clone)]
struct Button {
    id: Id,
    rect: Rect,
    command: Command,
}

#[derive(Clone)]
pub(crate) struct Scene {
    pub(crate) document: DocumentSnapshot,
    pub(crate) widget: Widget,
    pub(crate) rect: Rect,
    pub(crate) scroll: Vec2,
    pub(crate) line_height: f32,
    pub(crate) word_wrap: bool,
    pub(crate) folds: Vec<std::ops::Range<usize>>,
}

#[derive(Default, Clone)]
pub(crate) struct State {
    scene: Option<Scene>,
    buttons: Vec<Button>,
    pressed: Option<Id>,
    space: Option<Id>,
}

impl State {
    pub(crate) fn focus_ids(&self) -> Vec<Id> {
        self.buttons.iter().map(|button| button.id).collect()
    }

    pub(crate) fn prepare(&mut self, current: Option<&Scene>) -> bool {
        let valid = self
            .scene
            .as_ref()
            .zip(current)
            .is_some_and(|(before, current)| {
                before.document.id == current.document.id
                    && before.document.key == current.document.key
                    && before.document.revision == current.document.revision
                    && before.widget == current.widget
                    && before.rect == current.rect
                    && before.scroll == current.scroll
                    && before.line_height == current.line_height
                    && before.word_wrap == current.word_wrap
                    && before.folds == current.folds
            });
        if !valid {
            self.clear();
        }
        valid
    }

    pub(crate) fn clear(&mut self) {
        self.scene = None;
        self.buttons.clear();
        self.pressed = None;
        self.space = None;
    }

    pub(crate) fn install(&mut self, scene: Scene, output: Output) {
        self.scene = Some(scene);
        self.buttons = output.buttons;
    }

    pub(crate) fn describes(&self, document: &DocumentSnapshot) -> bool {
        self.scene.as_ref().is_some_and(|scene| {
            scene.document.id == document.id
                && scene.document.key == document.key
                && scene.document.revision == document.revision
        })
    }

    pub(crate) fn zone(
        &self,
        store: &EditorStore,
        document: &DocumentSnapshot,
        line_height: f32,
        viewport_height: f32,
    ) -> Option<(usize, f32)> {
        let scene = self.scene.as_ref()?;
        let mut marker = scene.widget.coordinate.problem.marker.clone();
        marker.bytes = scene.widget.position..scene.widget.position;
        let markers =
            taide_native_editor::diagnostics::MarkerSet::new(&scene.document, vec![marker]).ok()?;
        let tracked = markers.tracked(
            document,
            store.changes_since(document.id, markers.revision()).ok()?,
        )?;
        Some((
            tracked.markers().first()?.bytes.start,
            scene.widget.height(line_height, viewport_height),
        ))
    }

    pub(crate) fn pointer(
        &mut self,
        context: &Context,
        event: &Event,
        raw_index: usize,
    ) -> Option<Command> {
        let Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            ..
        } = event
        else {
            return None;
        };
        let trigger = context.pointer_focus_preserving_trigger_at(raw_index);
        if *pressed {
            self.pressed = self
                .buttons
                .iter()
                .find(|button| Some(button.id) == trigger && button.rect.contains(*pos))
                .map(|button| button.id);
            return None;
        }
        let id = self.pressed.take()?;
        self.buttons
            .iter()
            .find(|button| button.id == id && Some(id) == trigger && button.rect.contains(*pos))
            .map(|button| button.command)
    }

    pub(crate) fn key(&mut self, event: &Event, id: Id) -> Option<Command> {
        let button = self.buttons.iter().find(|button| button.id == id)?;
        let Event::Key {
            key,
            pressed,
            repeat,
            modifiers,
            ..
        } = event
        else {
            return None;
        };
        if !modifiers.is_none() {
            self.space = None;
            return None;
        }
        match (*key, *pressed) {
            (Key::Enter, true) => Some(button.command),
            (Key::Space, true) if !repeat => {
                self.space = Some(id);
                None
            }
            (Key::Space, false) => (self.space.take() == Some(id)).then_some(button.command),
            _ => None,
        }
    }

    pub(crate) fn detach(&mut self) {
        self.space = None;
        self.pressed = None;
    }
}

pub(crate) fn paint(ui: &mut Ui, frame: Frame<'_>) -> Output {
    let mut output = Output {
        buttons: Vec::new(),
    };
    if !frame.rect.intersects(frame.clip) {
        return output;
    }
    let mut ui = ui.new_child(
        UiBuilder::new()
            .id_salt(frame.id.with("problem-widget"))
            .max_rect(frame.rect),
    );
    ui.set_clip_rect(frame.rect.intersect(frame.clip));
    let painter = ui.painter();
    let severity = frame.widget.coordinate.problem.marker.message.severity;
    let border = frame.colors.diagnostics.severity(severity);
    let arrow_height = (frame.line_height / ARROW_DIVISOR)
        .round()
        .min(frame.rect.width().max(0.0) / 2.0);
    let top = frame.rect.top() + arrow_height;
    let body = Rect::from_min_max(pos2(frame.rect.left(), top), frame.rect.max);
    let arrow_x = frame
        .arrow_x
        .clamp(body.left() + arrow_height, body.right() - arrow_height);
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(arrow_x, frame.rect.top()),
            pos2(arrow_x - arrow_height, top),
            pos2(arrow_x + arrow_height, top),
        ],
        border,
        Stroke::NONE,
    ));
    painter.rect_filled(body, 0.0, frame.colors.background);
    painter.rect_stroke(
        body,
        0.0,
        Stroke::new(BORDER_WIDTH, border),
        StrokeKind::Inside,
    );
    let header_height = (frame.line_height * HEADER_RATIO).ceil();
    let header = Rect::from_min_max(
        body.min + vec2(BORDER_WIDTH, BORDER_WIDTH),
        pos2(body.right() - BORDER_WIDTH, top + header_height),
    );
    painter.rect_filled(header, 0.0, border.gamma_multiply(HEADER_OPACITY));
    let icon_font = if ui.fonts(|fonts| {
        fonts.definitions().families.contains_key(&FontFamily::Name(
            crate::editor_find_widget::ICON_FAMILY.into(),
        ))
    }) {
        FontId::new(
            ICON_SIZE,
            FontFamily::Name(crate::editor_find_widget::ICON_FAMILY.into()),
        )
    } else {
        FontId::proportional(ICON_SIZE)
    };
    let glyph = match severity {
        taide_native_editor::diagnostics::Severity::Error => ERROR_ICON,
        taide_native_editor::diagnostics::Severity::Warning => WARNING_ICON,
        _ => INFO_ICON,
    };
    painter.text(
        pos2(
            header.left() + HEADER_MARGIN + ICON_SIZE / 2.0,
            header.center().y,
        ),
        Align2::CENTER_CENTER,
        glyph,
        icon_font.clone(),
        border,
    );
    let label_rect = Rect::from_min_max(
        pos2(
            header.left() + ICON_SIZE + HEADER_MARGIN * 2.0,
            header.top(),
        ),
        pos2(header.right() - BUTTON_SIZE * BUTTON_COUNT, header.bottom()),
    );
    let count = &frame.widget.coordinate;
    let detail = format!(
        "{} of {} {}",
        count.index,
        count.total,
        if count.total == 1 {
            "problem"
        } else {
            "problems"
        }
    );
    let mut title = egui::text::LayoutJob::default();
    title.append(
        &frame.widget.title,
        0.0,
        TextFormat {
            font_id: frame.font.clone(),
            color: frame.colors.heading,
            ..Default::default()
        },
    );
    title.append(
        &detail,
        HEADER_MARGIN,
        TextFormat {
            font_id: frame.font.clone(),
            color: frame.colors.detail,
            ..Default::default()
        },
    );
    let title_painter = painter.with_clip_rect(label_rect.intersect(ui.clip_rect()));
    let galley = title_painter.layout_job(title);
    title_painter.galley(
        pos2(
            label_rect.left(),
            label_rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        frame.colors.heading,
    );
    for (index, (command, glyph, label)) in [
        (Command::Next, NEXT_ICON, "Go to Next Problem"),
        (Command::Previous, PREVIOUS_ICON, "Go to Previous Problem"),
        (Command::Close, CLOSE_ICON, "Close"),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = Rect::from_center_size(
            pos2(
                header.right() - BUTTON_SIZE * (BUTTON_COUNT - index as f32 - 0.5),
                header.center().y,
            ),
            vec2(BUTTON_SIZE, BUTTON_SIZE),
        );
        let response = ui.interact(
            rect,
            frame.id.with(("problem-action", index)),
            Sense::click(),
        );
        ui.ctx()
            .register_pointer_preserves_keyboard_focus(response.id);
        output.buttons.push(Button {
            id: response.id,
            rect: rect.intersect(ui.clip_rect()),
            command,
        });
        if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, frame.colors.hover);
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                0.0,
                Stroke::new(BORDER_WIDTH, frame.colors.focus),
                StrokeKind::Inside,
            );
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            glyph,
            icon_font.clone(),
            frame.colors.heading,
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
        });
        response.on_hover_text(label);
    }
    let content = Rect::from_min_max(
        pos2(body.left() + BODY_LEFT, header.bottom() + BODY_TOP),
        pos2(body.right() - BODY_RIGHT, body.bottom() - BORDER_WIDTH),
    );
    let mut message_ui = ui.new_child(
        UiBuilder::new()
            .id_salt("problem-message")
            .max_rect(content),
    );
    message_ui.set_clip_rect(content.intersect(ui.clip_rect()));
    egui::ScrollArea::both()
        .id_salt("problem-message-scroll")
        .auto_shrink([false, false])
        .show(&mut message_ui, |ui| {
            let message = &frame.widget.coordinate.problem.marker.message;
            let mut job = egui::text::LayoutJob::default();
            let format = TextFormat {
                font_id: frame.font.clone(),
                color: frame.colors.diagnostics.foreground,
                line_height: Some(frame.line_height),
                ..Default::default()
            };
            job.append(
                &message.text.replace("\r\n", "\n").replace('\r', "\n"),
                0.0,
                format.clone(),
            );
            job.append(
                &crate::editor_diagnostics::message_details(message),
                HEADER_MARGIN,
                TextFormat {
                    color: frame
                        .colors
                        .diagnostics
                        .foreground
                        .gamma_multiply(DETAILS_OPACITY),
                    ..format
                },
            );
            ui.add(
                egui::Label::new(job)
                    .selectable(true)
                    .wrap_mode(egui::TextWrapMode::Extend),
            );
        });
    output
}
