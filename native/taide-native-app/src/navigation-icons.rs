use std::collections::{HashMap, hash_map::Entry};

use eframe::egui::{self, Color32, ColorImage, Rect, TextureHandle, Ui, layers::ShapeIdx};
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
    FolderInput,
    Search,
    GitBranch,
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
    (
        Icon::FolderInput,
        include_bytes!("../resources/icons/folder-input.svg"),
    ),
    (
        Icon::Search,
        include_bytes!("../resources/icons/search.svg"),
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
            Self::GitBranch => Some(Glyph::GitBranch),
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
        let key = self.raster(ui.ctx(), rect.size(), icon)?;
        egui::Image::new((self.textures[&key].id(), rect.size()))
            .tint(color)
            .rotate(angle, egui::Vec2::splat(ROTATION_CENTER))
            .paint_at(ui, rect);
        Ok(())
    }

    pub(crate) fn prepare(
        &mut self,
        context: &egui::Context,
        size: egui::Vec2,
        icon: Icon,
    ) -> AppResult<()> {
        match icon.base() {
            Some(base) => self.base.prepare(context, size, base),
            None => self.raster(context, size, icon).map(|_| ()),
        }
    }

    /// The mesh [`Icons::paint`] adds, from a texture [`Icons::prepare`] made.
    pub(crate) fn shape(
        &self,
        context: &egui::Context,
        rect: Rect,
        icon: Icon,
        color: Color32,
        angle: f32,
    ) -> AppResult<egui::Shape> {
        if let Some(base) = icon.base() {
            return self.base.shape(context, rect, base, color, angle);
        }
        let scale = context.pixels_per_point();
        let side = (rect.width() * scale).ceil() as u32;
        let texture = self
            .textures
            .get(&(icon, side))
            .filter(|_| self.scale == Some(scale))
            .ok_or_else(|| AppError::Internal("native navigation icon is not prepared".into()))?;
        let rect = egui::emath::GuiRounding::round_to_pixels(rect, scale);
        let mut mesh = egui::Mesh::with_texture(texture.id());
        mesh.add_rect_with_uv(
            rect,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            color,
        );
        mesh.rotate(
            egui::emath::Rot2::from_angle(angle),
            rect.min + egui::Vec2::splat(ROTATION_CENTER) * rect.size(),
        );
        Ok(egui::Shape::mesh(mesh))
    }

    fn raster(
        &mut self,
        context: &egui::Context,
        size: egui::Vec2,
        icon: Icon,
    ) -> AppResult<(Icon, u32)> {
        let scale = context.pixels_per_point();
        let pixels = (size.x * scale).ceil();
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
                    size.x * scale / VIEWBOX,
                    size.y * scale / VIEWBOX,
                ),
                &mut pixmap.as_mut(),
            );
            entry.insert(context.load_texture(
                format!("native-navigation-{icon:?}-{side}"),
                ColorImage::from_rgba_unmultiplied(
                    [side as usize, side as usize],
                    &pixmap.take_demultiplied(),
                ),
                egui::TextureOptions::LINEAR,
            ));
        }
        Ok(key)
    }
}

pub(crate) use crate::explorer::PaintIcon;

impl PaintIcon<Icon> for Icons {
    fn paint(
        &mut self,
        ui: &Ui,
        rect: Rect,
        icon: Icon,
        color: Color32,
        angle: f32,
    ) -> AppResult<()> {
        Icons::paint(self, ui, rect, icon, color, angle)
    }
}

impl PaintIcon<crate::problems_icons::Glyph> for crate::problems_icons::Icons {
    fn paint(
        &mut self,
        ui: &Ui,
        rect: Rect,
        glyph: crate::problems_icons::Glyph,
        color: Color32,
        angle: f32,
    ) -> AppResult<()> {
        crate::problems_icons::Icons::paint(self, ui, rect, glyph, color, angle)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Source {
    Glyph(crate::problems_icons::Glyph),
    Navigation(Icon),
}

impl From<crate::problems_icons::Glyph> for Source {
    fn from(glyph: crate::problems_icons::Glyph) -> Self {
        Self::Glyph(glyph)
    }
}

impl From<Icon> for Source {
    fn from(icon: Icon) -> Self {
        Self::Navigation(icon)
    }
}

struct Reservation {
    painter: egui::Painter,
    slot: ShapeIdx,
    rect: Rect,
    source: Source,
    color: Color32,
    angle: f32,
}

/// Icons a render that only reads the texture caches asked for: each keeps its place in the paint
/// order and is filled by [`Deferred::fill`] once the render is kept.
#[derive(Default)]
pub(crate) struct Deferred {
    reservations: Vec<Reservation>,
}

impl<T: Into<Source>> PaintIcon<T> for Deferred {
    fn paint(&mut self, ui: &Ui, rect: Rect, icon: T, color: Color32, angle: f32) -> AppResult<()> {
        let painter = ui.painter().clone();
        let slot = painter.add(egui::Shape::Noop);
        self.reservations.push(Reservation {
            painter,
            slot,
            rect,
            source: icon.into(),
            color,
            angle,
        });
        Ok(())
    }
}

impl Deferred {
    pub(crate) fn fill(
        self,
        glyphs: &mut crate::problems_icons::Icons,
        icons: &mut Icons,
    ) -> AppResult<()> {
        let mut failure = None;
        for reservation in self.reservations {
            let context = reservation.painter.ctx();
            let (size, rect) = (reservation.rect.size(), reservation.rect);
            let shape = match reservation.source {
                Source::Glyph(glyph) => glyphs.prepare(context, size, glyph).and_then(|()| {
                    glyphs.shape(context, rect, glyph, reservation.color, reservation.angle)
                }),
                Source::Navigation(icon) => icons.prepare(context, size, icon).and_then(|()| {
                    icons.shape(context, rect, icon, reservation.color, reservation.angle)
                }),
            };
            match shape {
                Ok(shape) => reservation.painter.set(reservation.slot, shape),
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }
}
