use std::collections::BTreeMap;

use serde_json::json;
use taide_model::{
    error::{AppError, AppErrorKind},
    ids::{PaneId, ProjectId, TabId},
    locale::ResolvedLocale,
    settings::Settings,
    theme::{ResolvedTheme, ThemeType},
};
use taide_native_ui::{
    settings_owner::Owner,
    settings_view::{Appearance, Request, Views},
    theme_editor_tokens::COLORS,
};
use taide_remote_web::{
    InvokeError, ResponsePayload,
    settings_catalog::{CatalogReads, Read, invocation_error, json_payload},
};

const FIRST_THEME: u32 = 1;
const FIRST_LOCALE: u32 = 2;
const STALE_THEME: u32 = 3;
const STALE_LOCALE: u32 = 4;
const NEW_THEME: u32 = 5;
const NEW_LOCALE: u32 = 6;
const UNKNOWN: u32 = 7;
const SCREEN: [f32; 2] = [1000.0, 900.0];

struct Surface {
    views: Views,
    context: egui::Context,
    owner: Owner,
    appearance: Appearance,
    locale: ResolvedLocale,
}

impl Surface {
    fn new() -> Self {
        let theme = ResolvedTheme {
            id: "taide-dark".into(),
            name: "Synthetic theme".into(),
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
            views: Views::default(),
            context: egui::Context::default(),
            owner: Owner {
                project: ProjectId::new(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            appearance: Appearance::new(&theme).unwrap(),
            locale: ResolvedLocale {
                id: "en".into(),
                name: "English".into(),
                messages: BTreeMap::new(),
                warnings: Vec::new(),
            },
        }
    }

    fn show(&mut self) -> Option<Request> {
        self.views.begin_frame();
        let mut request = None;
        let mut drawing = self.context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SCREEN[0], SCREEN[1]),
                )),
                ..Default::default()
            },
            |ui| {
                let output = self.views.show(
                    ui,
                    self.owner.clone(),
                    &Settings::default(),
                    &self.locale,
                    &self.appearance,
                );
                assert!(output.error.is_none());
                request = output.load;
            },
        );
        drawing.textures_delta.clear();
        self.views.finish_frame();
        request
    }
}

fn themes() -> Result<ResponsePayload, serde_json::Value> {
    Ok(ResponsePayload::Json(
        json!([{ "id": "taide-dark", "name": "Synthetic theme", "type": "dark", "builtin": true }]),
    ))
}

fn locales() -> Result<ResponsePayload, serde_json::Value> {
    Ok(ResponsePayload::Json(
        json!([{ "id": "en", "name": "English", "builtin": true }]),
    ))
}

#[test]
fn 실제_settings_목록은_독립오류와_요청소유권_remount_지연응답을_보존한다() {
    let mut surface = Surface::new();
    let first = surface.show().unwrap();
    assert!(surface.show().is_none());
    let mut reads = CatalogReads::default();
    reads.request(first.clone());
    reads.request(first.clone());
    let next = reads.next_reads();
    assert_eq!(next.len(), 2);
    assert_eq!(Read::Themes.call().command, "theme_list");
    assert_eq!(Read::Locales.call().command, "locale_list");
    assert!(Read::Themes.call().args.is_null());
    reads.sent(first.clone(), Read::Themes, FIRST_THEME);
    reads.sent(first.clone(), Read::Locales, FIRST_LOCALE);
    assert!(reads.next_reads().is_empty());
    assert!(!reads.response(UNKNOWN, &themes()));
    assert!(reads.response(FIRST_THEME, &themes()));
    assert!(reads.take_finished().is_empty());
    assert!(reads.response(FIRST_LOCALE, &Ok(ResponsePayload::Binary(Vec::new()))));
    let completed = reads.take_finished().pop().unwrap();
    assert_eq!(
        completed.catalog.themes.as_ref().unwrap()[0].id,
        "taide-dark"
    );
    assert_eq!(
        completed.catalog.locales.as_ref().unwrap_err().kind(),
        AppErrorKind::Internal
    );
    assert!(
        surface
            .views
            .accept(&completed.request, Ok(completed.catalog))
    );
    assert!(surface.show().is_none());
    assert!(!reads.response(FIRST_THEME, &themes()));

    surface.views.invalidate_catalog();
    let stale = surface.show().unwrap();
    assert_eq!(first.mount(), stale.mount());
    assert!(stale.generation() > first.generation());
    reads.request(stale.clone());
    reads.sent(stale.clone(), Read::Themes, STALE_THEME);
    reads.sent(stale.clone(), Read::Locales, STALE_LOCALE);
    surface.views.clear();
    let current = surface.show().unwrap();
    assert!(current.mount() > stale.mount());
    reads.request(current.clone());
    assert_eq!(reads.next_reads().len(), 2);
    reads.sent(current.clone(), Read::Themes, NEW_THEME);
    reads.sent(current.clone(), Read::Locales, NEW_LOCALE);
    assert!(reads.response(STALE_THEME, &themes()));
    assert!(reads.response(STALE_LOCALE, &locales()));
    assert!(reads.take_finished().is_empty());
    assert!(!surface.views.accept(
        &stale,
        Ok(taide_native_ui::settings_view::Catalog {
            themes: Ok(Vec::new()),
            locales: Ok(Vec::new())
        })
    ));
    assert!(reads.response(NEW_LOCALE, &locales()));
    assert!(reads.response(NEW_THEME, &themes()));
    let completed = reads.take_finished().pop().unwrap();
    assert_eq!(completed.request, current);
    assert!(
        surface
            .views
            .accept(&completed.request, Ok(completed.catalog))
    );
    assert!(surface.show().is_none());
}

#[test]
fn 목록은_closed_단일완료_자동재전송없음_사건취소와_원본오류타입을_보존한다() {
    let mut surface = Surface::new();
    let first = surface.show().unwrap();
    let mut reads = CatalogReads::default();
    reads.request(first.clone());
    reads.sent(first.clone(), Read::Themes, FIRST_THEME);
    reads.sent(first.clone(), Read::Locales, FIRST_LOCALE);
    assert!(reads.response(FIRST_THEME, &themes()));
    reads.disconnected();
    assert!(reads.next_reads().is_empty());
    let closed = reads.take_finished().pop().unwrap();
    assert!(closed.catalog.themes.is_ok());
    assert_eq!(
        closed.catalog.locales.as_ref().unwrap_err().kind(),
        AppErrorKind::Forbidden
    );
    assert!(surface.views.accept(&closed.request, Ok(closed.catalog)));
    assert!(surface.show().is_none());
    reads.disconnected();
    assert!(reads.take_finished().is_empty());
    assert!(!reads.response(FIRST_LOCALE, &locales()));
    surface.views.invalidate_catalog();
    let next = surface.show().unwrap();
    reads.request(next.clone());
    reads.sent(next.clone(), Read::Themes, NEW_THEME);
    reads.sent(next, Read::Locales, NEW_LOCALE);
    reads.invalidate();
    assert!(reads.next_reads().is_empty());
    assert!(reads.response(NEW_THEME, &themes()));
    assert!(reads.response(NEW_LOCALE, &locales()));
    assert!(reads.take_finished().is_empty());

    for error in [
        AppError::Io("synthetic".into()),
        AppError::NotFound("synthetic".into()),
        AppError::InvalidArgument("synthetic".into()),
        AppError::Forbidden("synthetic".into()),
        AppError::Internal("synthetic".into()),
        AppError::localized(
            AppErrorKind::Forbidden,
            "synthetic.key",
            "synthetic fallback",
        )
        .with_arg("item", "synthetic"),
    ] {
        let decoded = json_payload(&Err(serde_json::to_value(&error).unwrap())).unwrap_err();
        assert_eq!(
            serde_json::to_value(decoded).unwrap(),
            serde_json::to_value(error).unwrap()
        );
    }
    assert_eq!(
        json_payload(&Err(json!({"code": "invalid", "message": "synthetic"})))
            .unwrap_err()
            .kind(),
        AppErrorKind::Internal
    );
    assert_eq!(
        invocation_error(InvokeError::Closed).kind(),
        AppErrorKind::Forbidden
    );
}
