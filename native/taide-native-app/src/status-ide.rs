use eframe::egui::{self, Color32, ColorImage, FontId, Response, TextureHandle, Ui, vec2};
use resvg::{tiny_skia, usvg};
use taide_model::{
    error::{AppError, AppResult},
    ide::IdeStatus,
    locale::ResolvedLocale,
    theme::ResolvedTheme,
};

const ICON_SIZE: f32 = 12.0;
const ICON_VIEWBOX: f32 = 24.0;
const TEXT_SIZE: f32 = 11.0;
const ICON_GAP: f32 = 4.0;
const MAX_RASTER_SIDE: f32 = 1024.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Connected,
    Waiting,
    Off,
}

impl State {
    const ALL: [Self; 3] = [Self::Connected, Self::Waiting, Self::Off];

    fn from_status(status: IdeStatus) -> Self {
        if status.connected {
            return Self::Connected;
        }
        if status.running {
            return Self::Waiting;
        }
        Self::Off
    }

    fn index(self) -> usize {
        match self {
            Self::Connected => 0,
            Self::Waiting => 1,
            Self::Off => 2,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Connected => "ide.connected",
            Self::Waiting => "ide.starting",
            Self::Off => "ide.disconnected",
        }
    }

    fn source(self) -> &'static [u8] {
        match self {
            Self::Connected => include_bytes!("../resources/status/plug-zap.svg"),
            Self::Waiting => include_bytes!("../resources/status/plug.svg"),
            Self::Off => include_bytes!("../resources/status/unplug.svg"),
        }
    }
}

pub(crate) struct Appearance {
    success: Color32,
    muted: Color32,
    tooltip: crate::tooltips::Appearance,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            success: crate::presentation::color(theme, "statusIndicator.success")?,
            muted: crate::presentation::color(theme, "appSidebar.iconDefault")?,
            tooltip: crate::tooltips::Appearance::new(theme)?,
        })
    }
}

pub(crate) struct Icons {
    trees: Vec<usvg::Tree>,
    textures: Vec<TextureHandle>,
    scale: Option<f32>,
    tooltips: crate::tooltips::Provider,
}

impl Icons {
    #[cfg(test)]
    pub(crate) fn new() -> AppResult<Self> {
        Self::with_tooltips(crate::tooltips::Provider::default())
    }

    pub(crate) fn with_tooltips(tooltips: crate::tooltips::Provider) -> AppResult<Self> {
        let options = usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_string: Box::new(|_, _| None),
                resolve_data: Box::new(|_, _, _| None),
            },
            ..Default::default()
        };
        Ok(Self {
            trees: State::ALL
                .iter()
                .map(|state| {
                    usvg::Tree::from_data(state.source(), &options).map_err(|error| {
                        AppError::Internal(format!("native IDE status icon: {error}"))
                    })
                })
                .collect::<AppResult<_>>()?,
            textures: Vec::new(),
            scale: None,
            tooltips,
        })
    }

    fn prepare(&mut self, context: &egui::Context) -> AppResult<()> {
        let scale = context.pixels_per_point();
        if self.scale == Some(scale) {
            return Ok(());
        }
        let pixels = (ICON_SIZE * scale).ceil();
        if !pixels.is_finite() || !(1.0..=MAX_RASTER_SIDE).contains(&pixels) {
            return Err(AppError::Internal(
                "native IDE status icon scale is out of range".into(),
            ));
        }
        let side = pixels as u32;
        let textures = self
            .trees
            .iter()
            .enumerate()
            .map(|(index, tree)| {
                let mut pixmap = tiny_skia::Pixmap::new(side, side).ok_or_else(|| {
                    AppError::Internal("native IDE status icon allocation failed".into())
                })?;
                resvg::render(
                    tree,
                    tiny_skia::Transform::from_scale(
                        ICON_SIZE * scale / ICON_VIEWBOX,
                        ICON_SIZE * scale / ICON_VIEWBOX,
                    ),
                    &mut pixmap.as_mut(),
                );
                Ok(context.load_texture(
                    format!("native-status-ide-icon-{index}"),
                    ColorImage::from_rgba_unmultiplied(
                        [side as usize, side as usize],
                        &pixmap.take_demultiplied(),
                    ),
                    egui::TextureOptions::LINEAR,
                ))
            })
            .collect::<AppResult<_>>()?;
        self.textures = textures;
        self.scale = Some(scale);
        Ok(())
    }
}

fn title(locale: &ResolvedLocale, state: State, port: u32) -> Option<String> {
    (state != State::Off)
        .then(|| crate::presentation::message(locale, "ide.title", &[("port", &port.to_string())]))
}

pub(crate) fn show(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    icons: &mut Icons,
    status: IdeStatus,
) -> AppResult<Response> {
    icons.prepare(ui.ctx())?;
    let state = State::from_status(status);
    let color = if state == State::Connected {
        appearance.success
    } else {
        appearance.muted
    };
    let response = ui
        .horizontal(|ui| {
            ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                node.set_role(egui::accesskit::Role::GenericContainer)
            });
            ui.spacing_mut().item_spacing.x = ICON_GAP;
            ui.add(
                egui::Image::new((
                    icons.textures[state.index()].id(),
                    vec2(ICON_SIZE, ICON_SIZE),
                ))
                .fit_to_exact_size(vec2(ICON_SIZE, ICON_SIZE))
                .tint(color),
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new(crate::presentation::message(locale, state.label(), &[]))
                        .font(FontId::proportional(TEXT_SIZE))
                        .color(color),
                )
                .extend()
                .selectable(false),
            );
        })
        .response;
    if let Some(text) = title(locale, state, status.port) {
        icons
            .tooltips
            .show(&response, &text, egui::RectAlign::TOP, &appearance.tooltip);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_ide::store::IdeStore;
    use taide_model::ids::ProjectId;

    const SCREEN_WIDTH: f32 = 320.0;
    const SCREEN_HEIGHT: f32 = 24.0;
    const PORT: u32 = 12345;
    const RETINA_SCALE: f32 = 2.0;

    #[test]
    fn ide_tooltip은_dark_light_원본_테마와_12px_글꼴을_사용한다() {
        const SCREEN: egui::Vec2 = vec2(640.0, 240.0);
        const FONT_SIZE: f32 = 12.0;
        const LINE_HEIGHT: f32 = 16.0;
        const RADIUS: u8 = 6;
        let locale = ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: serde_json::from_str(include_str!(
                "../../../crates/taide-locale/resources/locales/en.json"
            ))
            .unwrap(),
        };
        for name in ["vscode-dark-modern", "vscode-light-modern"] {
            let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
                std::env::temp_dir().join(format!("taide-ide-tooltip-{}", uuid::Uuid::new_v4())),
            ));
            let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
            let background = crate::presentation::color(&theme, "tooltip.background").unwrap();
            let border = crate::presentation::color(&theme, "tooltip.border").unwrap();
            let foreground = crate::presentation::color(&theme, "app.foreground").unwrap();
            let appearance = Appearance::new(&theme).unwrap();
            let context = egui::Context::default();
            context.enable_accesskit();
            let mut icons = Icons::new().unwrap();
            let status = IdeStatus {
                running: true,
                connected: true,
                port: PORT,
                client_count: 1,
            };
            let mut bounds = egui::Rect::NOTHING;
            let mut render = |time, events| {
                let mut output = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        ui.add_space(SCREEN.y / 2.0);
                        bounds = show(ui, &locale, &appearance, &mut icons, status)
                            .unwrap()
                            .rect;
                    },
                );
                output.textures_delta.clear();
                (output, bounds)
            };
            let (_, rect) = render(0.0, Vec::new());
            render(1.0, vec![egui::Event::PointerMoved(rect.center())]);
            render(2.0, Vec::new());
            let (output, _) = render(3.0, Vec::new());
            let frame = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect) if rect.fill == background => Some(rect),
                    _ => None,
                })
                .expect("IDE tooltip must use resolved tooltip.background");
            assert_eq!(frame.stroke, egui::Stroke::new(1.0, border));
            assert_eq!(frame.corner_radius, egui::CornerRadius::same(RADIUS));
            let title = title(&locale, State::Connected, PORT).unwrap();
            let nodes = &output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes;
            let (tooltip_id, tooltip) = nodes
                .iter()
                .find(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
                .unwrap();
            assert_eq!(tooltip.label(), Some(title.as_str()));
            let (_, trigger) = nodes
                .iter()
                .find(|(_, node)| node.described_by() == [*tooltip_id])
                .unwrap();
            assert_eq!(trigger.role(), egui::accesskit::Role::GenericContainer);
            let bounds = trigger.bounds().unwrap();
            assert_eq!(
                bounds,
                egui::accesskit::Rect {
                    x0: rect.left().into(),
                    y0: rect.top().into(),
                    x1: rect.right().into(),
                    y1: rect.bottom().into()
                }
            );
            let label = crate::presentation::message(&locale, "ide.connected", &[]);
            assert!(trigger.children().iter().any(|child| {
                nodes.iter().any(|(id, node)| {
                    id == child
                        && node.role() == egui::accesskit::Role::Label
                        && node.value() == Some(label.as_str())
                })
            }));
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == title => Some(text),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                text.galley.job.sections[0].format.font_id,
                FontId::proportional(FONT_SIZE)
            );
            assert_eq!(
                text.galley.job.sections[0].format.line_height,
                Some(LINE_HEIGHT)
            );
            assert_eq!(text.galley.job.sections[0].format.color, foreground);
            assert!(text.pos.y + text.galley.size().y < rect.top());
        }
    }

    #[tokio::test]
    async fn 실제_ide_store의_세_상태와_표시_아이콘_캐시를_보존한다() {
        let store = IdeStore::default();
        let off = store.status();
        let token = ProjectId::new().to_string();
        let started = store
            .mark_started(
                PORT,
                token.clone(),
                std::env::temp_dir().join(format!("taide-ide-status-{}", ProjectId::new())),
                tokio::spawn(std::future::pending()),
            )
            .unwrap();
        assert_eq!(store.client_connected_for_token(&token), Some(1));
        let connected = store.status();
        assert_eq!(
            State::from_status(IdeStatus {
                connected: true,
                ..Default::default()
            }),
            State::Connected
        );
        let locale = ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: [
                ("ide.connected".into(), "Connected".into()),
                ("ide.starting".into(), "Starting…".into()),
                ("ide.disconnected".into(), "Disconnected".into()),
                ("ide.title".into(), "IDE Integration (port {{port}})".into()),
            ]
            .into(),
        };
        let theme_state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
            std::env::temp_dir().join(format!("taide-ide-status-theme-{}", uuid::Uuid::new_v4())),
        ));
        let theme =
            taide_runtime::theme_actions::theme_get(&theme_state, "vscode-dark-modern".into())
                .unwrap();
        let appearance = Appearance {
            success: Color32::GREEN,
            muted: Color32::GRAY,
            tooltip: crate::tooltips::Appearance::new(&theme).unwrap(),
        };
        let context = egui::Context::default();
        let mut icons = Icons::new().unwrap();
        let mut initial_ids = Vec::new();
        for (status, state, label) in [
            (off, State::Off, "Disconnected"),
            (started, State::Waiting, "Starting…"),
            (connected, State::Connected, "Connected"),
        ] {
            assert_eq!(State::from_status(status), state);
            let expected_title = if state == State::Off {
                None
            } else {
                Some(format!("IDE Integration (port {PORT})"))
            };
            assert_eq!(title(&locale, state, status.port), expected_title);
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
                    )),
                    ..Default::default()
                },
                |ui| {
                    show(ui, &locale, &appearance, &mut icons, status).unwrap();
                },
            );
            let ids = icons
                .textures
                .iter()
                .map(TextureHandle::id)
                .collect::<Vec<_>>();
            let uploads = output
                .textures_delta
                .set
                .iter()
                .filter(|(id, _)| ids.contains(id))
                .count();
            if initial_ids.is_empty() {
                assert_eq!(uploads, State::ALL.len());
                for (id, updates) in output
                    .textures_delta
                    .set
                    .iter()
                    .filter(|(id, _)| ids.contains(id))
                {
                    assert!(ids.contains(id));
                    assert!(updates.iter().any(|delta| match &delta.image {
                        egui::ImageData::Color(image) =>
                            image.pixels.iter().any(|pixel| pixel.a() > 0
                                && pixel.r() == pixel.g()
                                && pixel.g() == pixel.b()),
                    }));
                }
                initial_ids = ids.clone();
            } else {
                assert_eq!(uploads, 0);
                assert_eq!(ids, initial_ids);
            }
            output.textures_delta.clear();
            let expected_color = if state == State::Connected {
                appearance.success
            } else {
                appearance.muted
            };
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => Some(text),
                    _ => None,
                })
                .unwrap();
            assert_eq!(text.galley.job.sections[0].format.font_id.size, TEXT_SIZE);
            assert_eq!(text.galley.job.sections[0].format.color, expected_color);
            let image = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect
                            .brush
                            .as_ref()
                            .is_some_and(|brush| brush.fill_texture_id == ids[state.index()]) =>
                    {
                        Some(rect)
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(image.rect.size(), vec2(ICON_SIZE, ICON_SIZE));
            assert_eq!(image.fill, expected_color);
            assert_eq!(text.pos.x - image.rect.right(), ICON_GAP);
        }
        context.set_pixels_per_point(RETINA_SCALE);
        let mut output = context.run_ui(Default::default(), |ui| {
            show(ui, &locale, &appearance, &mut icons, store.status()).unwrap();
        });
        assert!(
            initial_ids
                .iter()
                .all(|id| output.textures_delta.free.contains(id))
        );
        assert!(
            icons
                .textures
                .iter()
                .all(|texture| texture.size() == [(ICON_SIZE * RETINA_SCALE) as usize; 2])
        );
        output.textures_delta.clear();
        let retina_ids = icons
            .textures
            .iter()
            .map(TextureHandle::id)
            .collect::<Vec<_>>();
        drop(icons);
        let mut dropped = context.run_ui(Default::default(), |_| {});
        assert!(
            retina_ids
                .iter()
                .all(|id| dropped.textures_delta.free.contains(id))
        );
        dropped.textures_delta.clear();
        assert_eq!(store.client_disconnected_for_token(&token), Some(0));
        assert_eq!(State::from_status(store.status()), State::Waiting);
        let stopped = store.take_shutdown_state().unwrap();
        let server = stopped.server_handle.unwrap();
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
        assert_eq!(store.status(), IdeStatus::default());
        assert_eq!(title(&locale, State::Off, store.status().port), None);
    }
}
