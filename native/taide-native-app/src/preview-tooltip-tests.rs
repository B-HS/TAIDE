use super::*;
use taide_model::{
    ids::{ProjectId, TabId},
    paths::AppPaths,
};
use taide_runtime::AppState;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);
const MAX_SIDE: usize = 64;
const FOCUSED: f64 = 1.0;
const SETTLED: f64 = 2.0;
const CLOSED: f64 = 3.0;
const HOVER: f64 = 4.0;
const HOVER_SETTLED: f64 = 5.0;
const UNMOUNTED: f64 = 6.0;
const BUTTON_SIZE: f32 = 24.0;
const PAGES: usize = 2;

#[test]
fn preview_tooltip은_pdf4_hwp2의_actual_button_owner와_disabled_unmount를_보존한다() {
    use crate::{preview_hwp, preview_hwp_surface, preview_pdf, preview_pdf_surface};

    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-preview-tooltip-{}", ProjectId::new())),
    ));
    let locale = taide_runtime::locale_actions::locale_get(&state, "en".into()).unwrap();
    for name in ["vscode-dark-modern", "vscode-light-modern"] {
        let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
        let appearance = Appearance::new(&theme).unwrap();
        let preview_appearance = preview_pdf_surface::Appearance {
            background: crate::presentation::color(&theme, "editor.background").unwrap(),
            header: crate::presentation::color(&theme, "editor.widgetBackground").unwrap(),
            border: crate::presentation::color(&theme, "editor.widgetBorder").unwrap(),
            foreground: crate::presentation::color(&theme, "editor.foreground").unwrap(),
            muted: crate::presentation::color(&theme, "appSidebar.iconDefault").unwrap(),
        };
        for key in [
            "preview.pdf.previousPage",
            "preview.pdf.nextPage",
            "preview.pdf.zoomOut",
            "preview.pdf.zoomIn",
            "preview.hwp.previousPage",
            "preview.hwp.nextPage",
        ] {
            for disabled in [false, true] {
                let context = Context::default();
                context.enable_accesskit();
                let provider = Provider::default();
                let tab = TabId::new();
                let is_pdf = key.starts_with("preview.pdf.");
                let mut pdf = preview_pdf::Cache::default();
                let mut hwp = preview_hwp::Cache::default();
                let path = if is_pdf {
                    "synthetic.pdf"
                } else {
                    "synthetic.hwp"
                };
                if is_pdf {
                    let request = pdf.begin(&tab, path, MAX_SIDE).unwrap();
                    pdf.accept(
                        &context,
                        request,
                        Ok(preview_pdf::Page::new(
                            PAGES,
                            crate::preview::Raster {
                                size: [1, 1],
                                rgba: Color32::WHITE.to_array().to_vec(),
                                animation: None,
                            },
                        )),
                        0,
                    );
                    let mut selection = preview_pdf::Selection::default();
                    match key {
                        "preview.pdf.previousPage" if !disabled => selection.page = PAGES,
                        "preview.pdf.nextPage" if disabled => selection.page = PAGES,
                        "preview.pdf.zoomOut" if disabled => selection.zoom = preview_pdf::MIN_ZOOM,
                        "preview.pdf.zoomIn" if disabled => selection.zoom = preview_pdf::MAX_ZOOM,
                        _ => {}
                    }
                    pdf.change(&tab, selection);
                } else {
                    let request = hwp.begin(&tab, path, MAX_SIDE).unwrap();
                    hwp.accept(
                        &context,
                        request,
                        Ok(preview_hwp::Page::new(PAGES, None)),
                        0,
                    );
                    let last_page = (key == "preview.hwp.previousPage" && !disabled)
                        || (key == "preview.hwp.nextPage" && disabled);
                    hwp.change(&tab, usize::from(last_page));
                }
                let id = if is_pdf {
                    preview_pdf_surface::control_id(egui::ViewportId::ROOT, &tab, key)
                } else {
                    preview_hwp_surface::control_id(egui::ViewportId::ROOT, &tab, key)
                };
                let label = crate::presentation::message(&locale, key, &[]);
                let render =
                    |pdf: &mut preview_pdf::Cache, hwp: &mut preview_hwp::Cache, time, events| {
                        let mut shown = None;
                        let mut output = context.run_ui(
                            egui::RawInput {
                                time: Some(time),
                                events,
                                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                                ..Default::default()
                            },
                            |ui| {
                                provider.begin_frame(&context);
                                let output = if is_pdf {
                                    preview_pdf_surface::show_with_tooltips(
                                        ui,
                                        pdf,
                                        &tab,
                                        path,
                                        path,
                                        &locale,
                                        &preview_appearance,
                                    )
                                } else {
                                    preview_hwp_surface::show_with_tooltips(
                                        ui,
                                        hwp,
                                        &tab,
                                        path,
                                        path,
                                        &locale,
                                        &preview_appearance,
                                    )
                                };
                                assert!(!output.external);
                                provider.show_triggers(&output.tooltips, &appearance);
                                provider.finish_frame(&context);
                                shown = context.read_response(id);
                            },
                        );
                        output.textures_delta.clear();
                        (output, shown)
                    };
                render(&mut pdf, &mut hwp, 0.0, Vec::new());
                render(
                    &mut pdf,
                    &mut hwp,
                    FOCUSED,
                    vec![Event::AccessKitActionRequest(
                        egui::accesskit::ActionRequest {
                            target_node: id.accesskit_id(),
                            target_tree: egui::accesskit::TreeId::ROOT,
                            action: egui::accesskit::Action::Focus,
                            data: None,
                        },
                    )],
                );
                render(&mut pdf, &mut hwp, FOCUSED, Vec::new());
                let (opened, button) = render(&mut pdf, &mut hwp, SETTLED, Vec::new());
                let button = button.unwrap();
                assert_eq!(button.enabled(), !disabled);
                assert_eq!(button.rect.size(), egui::Vec2::splat(BUTTON_SIZE));
                let nodes = &opened
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes;
                let child = &nodes
                    .iter()
                    .find(|(target, _)| *target == id.accesskit_id())
                    .unwrap()
                    .1;
                assert_eq!(child.role(), egui::accesskit::Role::Button);
                assert_eq!(child.label(), Some(label.as_str()));
                assert_eq!(child.is_disabled(), disabled);
                let tooltip = nodes
                    .iter()
                    .find(|(_, node)| node.role() == egui::accesskit::Role::Tooltip);
                if disabled {
                    assert!(tooltip.is_none());
                    assert!(child.described_by().is_empty());
                } else {
                    let (tooltip_id, tooltip) =
                        tooltip.expect("actual preview button needs Tooltip role");
                    assert_eq!(tooltip.label(), Some(label.as_str()));
                    assert_eq!(child.described_by(), [*tooltip_id]);
                    assert!(tooltip.bounds().unwrap().y0 >= f64::from(button.rect.bottom()));
                    let text = opened
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.text() == label => Some(text),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(
                        text.galley.job.sections[0].format.font_id,
                        egui::FontId::proportional(FONT_SIZE)
                    );
                    assert_eq!(
                        text.galley.job.sections[0].format.color,
                        appearance.foreground
                    );
                }
                let (closed, _) = render(
                    &mut pdf,
                    &mut hwp,
                    CLOSED,
                    vec![Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
                assert_eq!(
                    closed
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip),
                    !disabled
                );
                render(
                    &mut pdf,
                    &mut hwp,
                    HOVER,
                    vec![Event::PointerMoved(button.rect.center())],
                );
                render(&mut pdf, &mut hwp, HOVER_SETTLED, Vec::new());
                let (hovered, _) = render(&mut pdf, &mut hwp, HOVER_SETTLED, Vec::new());
                assert_eq!(
                    hovered
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip),
                    !disabled
                );
                pdf.invalidate(path);
                hwp.invalidate(path);
                let (unmounted, _) = render(&mut pdf, &mut hwp, UNMOUNTED, Vec::new());
                assert!(
                    !unmounted
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
                );
            }
        }
    }
}
