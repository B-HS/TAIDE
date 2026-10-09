use std::collections::{HashMap, hash_map::Entry};

use eframe::egui::{self, Color32, ColorImage, Rect, TextureHandle, Ui};
use resvg::{tiny_skia, usvg};
use taide_model::error::{AppError, AppResult};
use taide_native_editor::document_symbols::SymbolKind;

const VIEWBOX: f32 = 24.0;
const MAX_RASTER_SIDE: f32 = 1024.0;
const ROTATION_CENTER: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Icon {
    File,
    Package,
    Box,
    Component,
    Braces,
    Hash,
    Parentheses,
    Function,
    Variable,
    Circle,
    ChevronRight,
    ListTree,
    FolderTree,
}

const EXTRA_SOURCES: &[(Icon, &[u8])] = &[
    (
        Icon::Braces,
        include_bytes!("../resources/icons/braces.svg"),
    ),
    (Icon::Hash, include_bytes!("../resources/icons/hash.svg")),
    (
        Icon::Parentheses,
        include_bytes!("../resources/icons/parentheses.svg"),
    ),
    (
        Icon::Function,
        include_bytes!("../resources/icons/square-function.svg"),
    ),
    (
        Icon::Variable,
        include_bytes!("../resources/icons/variable.svg"),
    ),
    (
        Icon::Circle,
        include_bytes!("../resources/icons/circle.svg"),
    ),
    (
        Icon::ListTree,
        include_bytes!("../resources/icons/list-tree.svg"),
    ),
    (
        Icon::FolderTree,
        include_bytes!("../resources/icons/folder-tree.svg"),
    ),
];

impl Icon {
    pub(crate) fn symbol(kind: SymbolKind) -> Self {
        match kind {
            SymbolKind::FILE => Self::File,
            SymbolKind::MODULE | SymbolKind::NAMESPACE | SymbolKind::PACKAGE => Self::Package,
            SymbolKind::CLASS | SymbolKind::STRUCT => Self::Box,
            SymbolKind::INTERFACE => Self::Component,
            SymbolKind::ENUM => Self::Braces,
            SymbolKind::ENUM_MEMBER | SymbolKind::CONSTANT => Self::Hash,
            SymbolKind::CONSTRUCTOR => Self::Parentheses,
            SymbolKind::METHOD | SymbolKind::FUNCTION => Self::Function,
            SymbolKind::PROPERTY | SymbolKind::FIELD | SymbolKind::VARIABLE => Self::Variable,
            _ => Self::Circle,
        }
    }

    fn base(self) -> Option<crate::problems_icons::Glyph> {
        use crate::problems_icons::Glyph;
        match self {
            Self::File => Some(Glyph::File),
            Self::Package => Some(Glyph::Package),
            Self::Box => Some(Glyph::Box),
            Self::Component => Some(Glyph::Component),
            Self::ChevronRight => Some(Glyph::ChevronRight),
            _ => None,
        }
    }
}

pub(crate) struct Icons {
    base: crate::problems_icons::Icons,
    trees: HashMap<Icon, usvg::Tree>,
    textures: HashMap<(Icon, u32), TextureHandle>,
    scale: Option<f32>,
}

impl Icons {
    pub(crate) fn new() -> AppResult<Self> {
        let options = usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_string: Box::new(|_, _| None),
                resolve_data: Box::new(|_, _, _| None),
            },
            ..Default::default()
        };
        Ok(Self {
            base: crate::problems_icons::Icons::new()?,
            trees: EXTRA_SOURCES
                .iter()
                .map(|(icon, source)| {
                    Ok((
                        *icon,
                        usvg::Tree::from_data(source, &options).map_err(|error| {
                            AppError::Internal(format!("native navigation icon: {error}"))
                        })?,
                    ))
                })
                .collect::<AppResult<_>>()?,
            textures: HashMap::new(),
            scale: None,
        })
    }

    pub(crate) fn paint(
        &mut self,
        ui: &Ui,
        rect: Rect,
        icon: Icon,
        color: Color32,
        angle: f32,
    ) -> AppResult<()> {
        if let Some(base) = icon.base() {
            return self.base.paint(ui, rect, base, color, angle);
        }
        let scale = ui.ctx().pixels_per_point();
        let pixels = (rect.width() * scale).ceil();
        if !pixels.is_finite() || !(1.0..=MAX_RASTER_SIDE).contains(&pixels) {
            return Err(AppError::Internal(
                "native navigation icon scale is out of range".into(),
            ));
        }
        if self.scale != Some(scale) {
            self.textures.clear();
            self.scale = Some(scale);
        }
        let side = pixels as u32;
        let key = (icon, side);
        if let Entry::Vacant(entry) = self.textures.entry(key) {
            let tree = self.trees.get(&icon).ok_or_else(|| {
                AppError::Internal("native navigation icon source is missing".into())
            })?;
            let mut pixmap = tiny_skia::Pixmap::new(side, side).ok_or_else(|| {
                AppError::Internal("native navigation icon allocation failed".into())
            })?;
            resvg::render(
                tree,
                tiny_skia::Transform::from_scale(
                    rect.width() * scale / VIEWBOX,
                    rect.height() * scale / VIEWBOX,
                ),
                &mut pixmap.as_mut(),
            );
            entry.insert(ui.ctx().load_texture(
                format!("native-navigation-{icon:?}-{side}"),
                ColorImage::from_rgba_unmultiplied(
                    [side as usize, side as usize],
                    &pixmap.take_demultiplied(),
                ),
                egui::TextureOptions::LINEAR,
            ));
        }
        egui::Image::new((self.textures[&key].id(), rect.size()))
            .tint(color)
            .rotate(angle, egui::Vec2::splat(ROTATION_CENTER))
            .paint_at(ui, rect);
        Ok(())
    }
}
