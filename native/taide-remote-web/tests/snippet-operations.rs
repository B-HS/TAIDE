use std::collections::BTreeMap;

use egui::{Event, PointerButton, Pos2, RawInput, Rect};
use serde_json::{Value, json};
use taide_model::{
    error::AppError,
    ids::{PaneId, ProjectId, TabId},
    locale::ResolvedLocale,
    snippet::SnippetFile,
    theme::{ResolvedTheme, ThemeType},
};
use taide_native_ui::{
    icons::Icons,
    settings_owner::Owner,
    settings_view::Appearance,
    snippet_edit::{Kind, Outcome, Reply, Request},
    snippet_editor::{Editor, Output},
    theme_editor_tokens::COLORS,
};
use taide_remote_web::{ResponsePayload, snippet_operations::SnippetOperations};

const SCREEN: [f32; 2] = [1000.0, 900.0];
const FRAME_STEP: f64 = 0.1;
const LIST_SEQ: u32 = 11;
const SAVE_SEQ: u32 = 12;
const RETRY_SEQ: u32 = 13;
const DELETE_SEQ: u32 = 14;
const UNKNOWN_SEQ: u32 = 15;
const SUCCESS_SEQ: u32 = 16;

struct Surface {
    context: egui::Context,
    editor: Editor,
    appearance: Appearance,
    snippet_appearance: taide_native_ui::snippet_editor::Appearance,
    locale: ResolvedLocale,
    icons: Icons,
    time: f64,
}

impl Surface {
    fn new() -> Self {
        let theme = ResolvedTheme {
            id: "taide-dark".into(),
            name: "Synthetic".into(),
            theme_type: ThemeType::Dark,
            colors: COLORS
                .iter()
                .flat_map(|(namespace, keys)| {
                    keys.iter()
                        .map(move |key| (format!("{namespace}.{key}"), "#123456".into()))
                })
                .collect(),
            syntax: BTreeMap::new(),
            terminal: BTreeMap::new(),
            token_colors: None,
            syntax_overrides: Vec::new(),
            warnings: Vec::new(),
            author: None,
            license: None,
            source: None,
        };
        Self {
            context: egui::Context::default(),
            editor: Editor::new(Owner {
                project: ProjectId::new(),
                pane: PaneId::new(),
                tab: TabId::new(),
            }),
            appearance: Appearance::new(&theme).unwrap(),
            snippet_appearance: taide_native_ui::snippet_editor::Appearance::new(&theme).unwrap(),
            locale: ResolvedLocale {
                id: "en".into(),
                name: "Synthetic".into(),
                messages: BTreeMap::new(),
                warnings: Vec::new(),
            },
            icons: Icons::new().unwrap(),
            time: 0.0,
        }
    }

    fn show(&mut self, events: Vec<Event>) -> Output {
        self.time += FRAME_STEP;
        let mut output = Output::default();
        let mut drawing = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(SCREEN[0], SCREEN[1]),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                self.icons.prepare(ui.ctx()).unwrap();
                let mut pass = self.editor.show(
                    ui,
                    &self.locale,
                    &self.appearance,
                    &self.snippet_appearance,
                    &self.icons,
                );
                output.requests.append(&mut pass.requests);
                output.notices.append(&mut pass.notices);
                output.traces = pass.traces;
                output.interactions = pass.interactions;
            },
        );
        drawing.textures_delta.clear();
        output
    }

    fn click(&mut self, output: &Output, field: &str) -> Output {
        let (_, id, rect) = output
            .traces
            .iter()
            .find(|(name, _, _)| name == field)
            .unwrap();
        let point = rect.center();
        assert!(
            output
                .interactions
                .get(id)
                .is_some_and(|rect| rect.contains(point)),
            "missing or clipped {field}: {rect:?}"
        );
        self.show(vec![
            Event::PointerMoved(point),
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ])
    }

    fn save(&mut self) -> Request {
        let output = self.show(Vec::new());
        self.click(&output, "save").requests.pop().unwrap()
    }
}

fn response<T: serde::Serialize>(value: T) -> Result<ResponsePayload, Value> {
    Ok(ResponsePayload::Json(serde_json::to_value(value).unwrap()))
}

#[test]
fn snippet_wire는_목록_저장_거절_명시retry_삭제_단절과_late_ack를_한번씩_소비한다() {
    let mut surface = Surface::new();
    let list = surface.show(Vec::new()).requests.pop().unwrap();
    let mut operations = SnippetOperations::default();
    assert_eq!(SnippetOperations::call(&list).command, "snippet_list");
    operations.sent(list.clone(), LIST_SEQ);
    assert!(operations.is_pending(&list));
    assert!(!operations.has_pending_mutations());
    assert!(!operations.response(UNKNOWN_SEQ, &response(())));
    let file = SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::new(),
    };
    assert!(operations.response(LIST_SEQ, &response(vec![file.clone()])));
    assert!(
        surface
            .editor
            .accept(operations.take_replies().pop().unwrap())
    );
    surface
        .editor
        .state_mut()
        .request_select(file.file_name.clone());
    let save = surface.save();
    assert!(matches!(save.kind(), Kind::Save { content, create: false, .. } if content == "{}"));
    assert_eq!(
        SnippetOperations::call(&save).args,
        json!({"fileName":"rust.json","content":"{}"})
    );
    operations.sent(save.clone(), SAVE_SEQ);
    assert!(operations.has_pending_mutations());
    let error = AppError::Io("Synthetic refusal".into());
    assert!(operations.response(SAVE_SEQ, &Err(serde_json::to_value(&error).unwrap())));
    assert!(
        surface
            .editor
            .accept(operations.take_replies().pop().unwrap())
    );
    assert_eq!(operations.take_failures().len(), 1);
    assert!(!operations.response(SAVE_SEQ, &response(file.clone())));
    let retry = surface.save();
    assert!(!retry.same_request(&save));
    operations.sent(retry, RETRY_SEQ);
    let wrong = SnippetFile {
        file_name: "other.json".into(),
        ..file.clone()
    };
    assert!(operations.response(RETRY_SEQ, &response(wrong)));
    let reply = operations.take_replies().pop().unwrap();
    assert!(matches!(&reply.result, Err(AppError::InvalidArgument(_))));
    assert!(surface.editor.accept(reply));
    operations.take_failures();
    let save = surface.save();
    operations.sent(save.clone(), SUCCESS_SEQ);
    assert!(operations.response(SUCCESS_SEQ, &response(file.clone())));
    assert!(
        surface
            .editor
            .accept(operations.take_replies().pop().unwrap())
    );
    let refresh = surface.show(Vec::new()).requests.pop().unwrap();
    assert!(surface.editor.accept(Reply {
        request: refresh,
        result: Ok(Outcome::Listed(vec![file]))
    }));
    surface.editor.state_mut().delete_file_open = true;
    surface.show(Vec::new());
    let modal = surface.show(Vec::new());
    let delete = surface
        .click(&modal, "dialog-confirm")
        .requests
        .pop()
        .unwrap();
    assert_eq!(SnippetOperations::call(&delete).command, "snippet_delete");
    operations.sent(delete.clone(), DELETE_SEQ);
    assert!(operations.has_pending_mutations());
    drop(surface);
    assert!(!delete.is_active());
    operations.disconnected();
    assert!(!operations.has_pending_mutations());
    assert_eq!(operations.take_replies().len(), 1);
    assert_eq!(operations.failures().len(), 1);
    assert!(!operations.response(DELETE_SEQ, &response(())));
    operations.disconnected();
    assert!(operations.take_replies().is_empty());
}
