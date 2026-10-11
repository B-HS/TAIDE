use eframe::egui::{self, Color32, FontId, Response, Stroke, Ui, vec2};
use taide_lsp::native::{Phase, session::SessionSnapshot};
use taide_model::{error::AppResult, ids::ProjectId, locale::ResolvedLocale, theme::ResolvedTheme};

const TEXT_SIZE: f32 = 11.0;
const ICON_SIZE: f32 = 12.0;
const ICON_GAP: f32 = 4.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_CENTER: f32 = 12.0;
const ICON_RADIUS: f32 = 10.0;
const ICON_STROKE: f32 = 2.0;
pub(crate) const ITEM_GAP: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub running: usize,
    pub total: usize,
    pub has_crashed: bool,
}

pub(super) fn summarize<'a>(
    project: &ProjectId,
    sessions: impl IntoIterator<Item = (&'a ProjectId, &'a SessionSnapshot)>,
) -> Option<Summary> {
    let mut summary = Summary {
        running: 0,
        total: 0,
        has_crashed: false,
    };
    for (_, state) in sessions.into_iter().filter(|(owner, _)| *owner == project) {
        summary.total += 1;
        summary.running += usize::from(state.phase == Phase::Running);
        summary.has_crashed |= state.phase == Phase::Degraded;
    }
    (summary.total > 0).then_some(summary)
}

pub(crate) struct Appearance {
    success: Color32,
    error: Color32,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            success: crate::presentation::color(theme, "statusIndicator.success")?,
            error: crate::presentation::color(theme, "statusIndicator.error")?,
        })
    }
}

pub(crate) fn show(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    summary: Option<Summary>,
) -> Option<Response> {
    let summary = summary?;
    let color = if summary.has_crashed {
        appearance.error
    } else {
        appearance.success
    };
    let text = crate::presentation::message(
        locale,
        "window.lspStatus",
        &[
            ("running", &summary.running.to_string()),
            ("total", &summary.total.to_string()),
        ],
    );
    Some(
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ICON_GAP;
            let (rect, _) =
                ui.allocate_exact_size(vec2(ICON_SIZE, ICON_SIZE), egui::Sense::hover());
            let scale = ICON_SIZE / ICON_VIEWBOX;
            let point = |[x, y]: [f32; 2]| rect.min + vec2(x, y) * scale;
            let stroke = Stroke::new(ICON_STROKE * scale, color);
            ui.painter().circle_stroke(
                point([ICON_CENTER, ICON_CENTER]),
                ICON_RADIUS * scale,
                stroke,
            );
            let line = |points: &[[f32; 2]]| {
                ui.painter().add(egui::Shape::line(
                    points.iter().copied().map(point).collect(),
                    stroke,
                ));
            };
            if summary.has_crashed {
                line(&[[15.0, 9.0], [9.0, 15.0]]);
                line(&[[9.0, 9.0], [15.0, 15.0]]);
            } else {
                line(&[[16.0, 9.0], [10.5, 14.5], [8.0, 12.0]]);
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .font(FontId::proportional(TEXT_SIZE))
                        .color(color),
                )
                .extend()
                .selectable(false),
            )
        })
        .inner,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN_WIDTH: f32 = 320.0;
    const SCREEN_HEIGHT: f32 = 24.0;

    fn snapshot(phase: Phase) -> SessionSnapshot {
        SessionSnapshot {
            phase,
            generation: 0,
            pending: 0,
            server_pending: 0,
            progress_tokens: 0,
            registrations: 0,
            capability_revision: 0,
            document_methods: Default::default(),
            document_signature_options: Default::default(),
            document_completion_options: Default::default(),
            document_on_type_formatting_options: Default::default(),
            pid: None,
            failure: None,
        }
    }

    #[test]
    fn 집계는_포커스_프로젝트와_서버_단위를_보존하고_종료된_publisher를_숨긴다() {
        let project = ProjectId::new();
        let other = ProjectId::new();
        let sessions = vec![
            (project.clone(), snapshot(Phase::Running)),
            (project.clone(), snapshot(Phase::Initializing)),
            (project.clone(), snapshot(Phase::Degraded)),
            (other.clone(), snapshot(Phase::Running)),
        ];
        assert_eq!(
            summarize(
                &project,
                sessions.iter().map(|(owner, state)| (owner, state))
            ),
            Some(Summary {
                running: 1,
                total: 3,
                has_crashed: true
            })
        );
        assert_eq!(
            summarize(&other, sessions.iter().map(|(owner, state)| (owner, state))),
            Some(Summary {
                running: 1,
                total: 1,
                has_crashed: false
            })
        );
        assert_eq!(
            summarize(
                &ProjectId::new(),
                sessions.iter().map(|(owner, state)| (owner, state))
            ),
            None
        );
        let sessions = sessions
            .into_iter()
            .map(|(project, snapshot)| super::super::RegistryState {
                project,
                snapshot,
                name: "synthetic server".into(),
                owner: crate::diagnostics::Owner::new(),
                documents: Default::default(),
                open_documents: Default::default(),
            })
            .collect();
        let (states, receiver) = tokio::sync::watch::channel(sessions);
        let (commands, _) = tokio::sync::mpsc::channel(1);
        let (_, replies) = tokio::sync::mpsc::channel(1);
        let (stop, _) = tokio::sync::watch::channel(false);
        let bridge = super::super::LspBridge {
            commands,
            replies,
            stop,
            worker: None,
            synced: Default::default(),
            models: Default::default(),
            projects: None,
            states: receiver,
        };
        assert_eq!(bridge.summary(&project).unwrap().total, 3);
        drop(states);
        assert_eq!(bridge.summary(&project), None);
    }

    #[test]
    fn 실제_publisher_폐기는_다른_marker를_보존하고_늦은_발행을_거절한다() {
        use crate::diagnostics::{Owner, Store};
        use taide_lsp::native::protocol::lsp_types::{Diagnostic, DiagnosticSeverity};
        use taide_model::ids::TabId;
        use taide_native_editor::document::DocumentKey;
        use taide_native_editor::store::{EditorLimits, EditorStore};

        const BYTE_LIMIT: usize = 1024;
        let mut editor = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let document = editor
            .open_untitled(TabId::new(), "publisher", "rust".into())
            .unwrap();
        let mut model = editor.documents().snapshot(document).unwrap();
        model.key = DocumentKey::File(std::path::PathBuf::from("/synthetic/publisher.rs"));
        let project = ProjectId::new();
        let first = Owner::new();
        let second = Owner::new();
        let registry = |owners: &[Owner]| {
            owners
                .iter()
                .copied()
                .map(|owner| super::super::RegistryState {
                    project: project.clone(),
                    snapshot: snapshot(Phase::Running),
                    name: "synthetic publisher".into(),
                    owner,
                    documents: std::collections::HashSet::from([document]),
                    open_documents: std::collections::HashSet::from([document]),
                })
                .collect()
        };
        let (states, receiver) = tokio::sync::watch::channel(Vec::new());
        let (commands, _) = tokio::sync::mpsc::channel(1);
        let (_, replies) = tokio::sync::mpsc::channel(1);
        let (stop, _) = tokio::sync::watch::channel(false);
        let mut bridge = super::super::LspBridge {
            commands,
            replies,
            stop,
            worker: None,
            synced: Default::default(),
            models: Default::default(),
            projects: None,
            states: receiver,
        };
        let error = Diagnostic::default();
        let warning = Diagnostic {
            severity: Some(DiagnosticSeverity::WARNING),
            ..Default::default()
        };
        let mut markers = Store::default();
        markers.retain_documents(std::collections::HashSet::from([document]));
        states.send_replace(registry(&[first, second]));
        markers.reconcile(bridge.diagnostic_bindings().unwrap());
        markers.publish(first, &model, vec![error.clone()]);
        markers.publish(second, &model, vec![warning.clone()]);
        assert_eq!(markers.counts(), &[1, 1, 0, 0]);
        let revision = markers.revision().clone();
        assert!(bridge.diagnostic_bindings().is_none());
        assert!(std::sync::Arc::ptr_eq(&revision, markers.revision()));
        states.send_replace(registry(&[second]));
        markers.reconcile(bridge.diagnostic_bindings().unwrap());
        assert_eq!(markers.counts(), &[0, 1, 0, 0]);
        markers.publish(first, &model, vec![error]);
        assert_eq!(markers.counts(), &[0, 1, 0, 0]);
        markers.remove_document(document);
        markers.publish(second, &model, vec![warning.clone()]);
        assert_eq!(markers.batches().count(), 0);
        markers.retain_documents(std::collections::HashSet::from([document]));
        markers.publish(second, &model, vec![warning.clone()]);
        assert_eq!(markers.counts(), &[0, 1, 0, 0]);
        states.send_replace(registry(&[first, second]));
        drop(states);
        markers.reconcile(bridge.diagnostic_bindings().unwrap());
        assert_eq!(markers.counts(), &[0, 0, 0, 0]);
        markers.publish(second, &model, vec![warning]);
        assert_eq!(markers.batches().count(), 0);
        let revision = markers.revision().clone();
        markers.reconcile(bridge.diagnostic_bindings().unwrap());
        assert!(std::sync::Arc::ptr_eq(&revision, markers.revision()));
        assert_eq!(bridge.summary(&project), None);
    }

    #[test]
    fn 상태_렌더는_원본_문구와_색상_글꼴_아이콘을_사용한다() {
        let appearance = Appearance {
            success: Color32::GREEN,
            error: Color32::RED,
        };
        for (id, template, expected) in [
            ("ko", "LSP {{running}}/{{total}}", "LSP 1/3"),
            ("ja", "LSP {{running}}/{{total}}", "LSP 1/3"),
            ("en", "{{running}}/{{total}} LSP", "1/3 LSP"),
        ] {
            let locale = ResolvedLocale {
                id: id.into(),
                name: id.into(),
                warnings: Vec::new(),
                messages: [("window.lspStatus".into(), template.into())].into(),
            };
            for has_crashed in [false, true] {
                let context = egui::Context::default();
                let mut text_rect = None;
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        text_rect = show(
                            ui,
                            &locale,
                            &appearance,
                            Some(Summary {
                                running: 1,
                                total: 3,
                                has_crashed,
                            }),
                        )
                        .map(|response| response.rect);
                    },
                );
                output.textures_delta.clear();
                let expected_color = if has_crashed {
                    appearance.error
                } else {
                    appearance.success
                };
                let text = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == expected => Some(text),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(text.galley.job.sections[0].format.font_id.size, TEXT_SIZE);
                assert_eq!(text.galley.job.sections[0].format.color, expected_color);
                let circle = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Circle(circle) => Some(circle),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(circle.radius, ICON_RADIUS * ICON_SIZE / ICON_VIEWBOX);
                assert_eq!(circle.stroke.color, expected_color);
                assert_eq!(
                    text_rect.unwrap().left() - (circle.center.x + ICON_SIZE / 2.0),
                    ICON_GAP
                );
                let paths = output
                    .shapes
                    .iter()
                    .filter(|shape| matches!(shape.shape, egui::Shape::Path(_)))
                    .count();
                assert_eq!(paths, if has_crashed { 2 } else { 1 });
                let mut empty = context.run_ui(Default::default(), |ui| {
                    assert!(show(ui, &locale, &appearance, None).is_none());
                });
                empty.textures_delta.clear();
                assert!(empty.shapes.iter().all(|shape| !matches!(
                    shape.shape,
                    egui::Shape::Text(_) | egui::Shape::Circle(_) | egui::Shape::Path(_)
                )));
            }
        }
    }
}
