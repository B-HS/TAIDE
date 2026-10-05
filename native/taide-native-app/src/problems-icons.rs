use std::collections::HashMap;
use std::collections::hash_map::Entry;

use eframe::egui::{self, Color32, ColorImage, Rect, TextureHandle, Ui};
use resvg::{tiny_skia, usvg};
use taide_model::error::{AppError, AppResult};

const VIEWBOX: f32 = 24.0;
const MAX_RASTER_SIDE: f32 = 1024.0;
const ROTATION_CENTER: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Glyph {
    CircleX,
    TriangleAlert,
    Info,
    Lightbulb,
    CircleCheck,
    X,
    ChevronRight,
    File,
    FileCode,
    Component,
    FileJson,
    BookText,
    Palette,
    Globe,
    Cog,
    Coffee,
    Terminal,
    FileCog,
    Lock,
    Image,
    FileText,
    FileArchive,
    Package,
    Settings2,
    GitBranch,
    Container,
    BookMarked,
    Scale,
    Key,
}

const SOURCES: &[(Glyph, &[u8])] = &[
    (
        Glyph::CircleX,
        include_bytes!("../resources/problems/circle-x.svg"),
    ),
    (
        Glyph::TriangleAlert,
        include_bytes!("../resources/problems/triangle-alert.svg"),
    ),
    (
        Glyph::Info,
        include_bytes!("../resources/problems/info.svg"),
    ),
    (
        Glyph::Lightbulb,
        include_bytes!("../resources/problems/lightbulb.svg"),
    ),
    (
        Glyph::CircleCheck,
        include_bytes!("../resources/problems/circle-check.svg"),
    ),
    (Glyph::X, include_bytes!("../resources/problems/x.svg")),
    (
        Glyph::ChevronRight,
        include_bytes!("../resources/problems/chevron-right.svg"),
    ),
    (
        Glyph::File,
        include_bytes!("../resources/problems/file.svg"),
    ),
    (
        Glyph::FileCode,
        include_bytes!("../resources/problems/file-code.svg"),
    ),
    (
        Glyph::Component,
        include_bytes!("../resources/problems/component.svg"),
    ),
    (
        Glyph::FileJson,
        include_bytes!("../resources/problems/file-json.svg"),
    ),
    (
        Glyph::BookText,
        include_bytes!("../resources/problems/book-text.svg"),
    ),
    (
        Glyph::Palette,
        include_bytes!("../resources/problems/palette.svg"),
    ),
    (
        Glyph::Globe,
        include_bytes!("../resources/problems/globe.svg"),
    ),
    (Glyph::Cog, include_bytes!("../resources/problems/cog.svg")),
    (
        Glyph::Coffee,
        include_bytes!("../resources/problems/coffee.svg"),
    ),
    (
        Glyph::Terminal,
        include_bytes!("../resources/problems/terminal.svg"),
    ),
    (
        Glyph::FileCog,
        include_bytes!("../resources/problems/file-cog.svg"),
    ),
    (
        Glyph::Lock,
        include_bytes!("../resources/problems/lock.svg"),
    ),
    (
        Glyph::Image,
        include_bytes!("../resources/problems/image.svg"),
    ),
    (
        Glyph::FileText,
        include_bytes!("../resources/problems/file-text.svg"),
    ),
    (
        Glyph::FileArchive,
        include_bytes!("../resources/problems/file-archive.svg"),
    ),
    (
        Glyph::Package,
        include_bytes!("../resources/problems/package.svg"),
    ),
    (
        Glyph::Settings2,
        include_bytes!("../resources/problems/settings-2.svg"),
    ),
    (
        Glyph::GitBranch,
        include_bytes!("../resources/problems/git-branch.svg"),
    ),
    (
        Glyph::Container,
        include_bytes!("../resources/problems/container.svg"),
    ),
    (
        Glyph::BookMarked,
        include_bytes!("../resources/problems/book-marked.svg"),
    ),
    (
        Glyph::Scale,
        include_bytes!("../resources/problems/scale.svg"),
    ),
    (Glyph::Key, include_bytes!("../resources/problems/key.svg")),
];

pub(crate) const SEVERITIES: [Glyph; 4] = [
    Glyph::CircleX,
    Glyph::TriangleAlert,
    Glyph::Info,
    Glyph::Lightbulb,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FileColor {
    Info,
    Warning,
    Error,
    Success,
    Renamed,
    Conflicted,
    Staged,
    Neutral,
}

impl FileColor {
    pub(crate) const ALL: [Self; 8] = [
        Self::Info,
        Self::Warning,
        Self::Error,
        Self::Success,
        Self::Renamed,
        Self::Conflicted,
        Self::Staged,
        Self::Neutral,
    ];

    pub(crate) fn theme_key(self) -> &'static str {
        match self {
            Self::Info => "statusIndicator.info",
            Self::Warning => "statusIndicator.warning",
            Self::Error => "statusIndicator.error",
            Self::Success => "statusIndicator.success",
            Self::Renamed => "git.renamed",
            Self::Conflicted => "git.conflicted",
            Self::Staged => "git.staged",
            Self::Neutral => "appSidebar.iconDefault",
        }
    }
}

pub(crate) fn file(name: &str) -> (Glyph, FileColor) {
    let name = name.to_lowercase();
    match name.as_str() {
        "package.json" => return (Glyph::Package, FileColor::Error),
        "tsconfig.json" => return (Glyph::Settings2, FileColor::Info),
        "cargo.toml" => return (Glyph::Cog, FileColor::Conflicted),
        "cargo.lock" => return (Glyph::Lock, FileColor::Neutral),
        ".gitignore" => return (Glyph::GitBranch, FileColor::Conflicted),
        "dockerfile" => return (Glyph::Container, FileColor::Info),
        _ => {}
    }
    if name.starts_with("readme") {
        return (Glyph::BookMarked, FileColor::Info);
    }
    if name.starts_with("license") || name.starts_with("licence") {
        return (Glyph::Scale, FileColor::Warning);
    }
    if name.starts_with(".env") {
        return (Glyph::Key, FileColor::Warning);
    }
    let extension = name
        .rfind('.')
        .filter(|index| *index > 0)
        .map(|index| &name[index + 1..])
        .unwrap_or_default();
    match extension {
        "ts" => (Glyph::FileCode, FileColor::Info),
        "tsx" => (Glyph::Component, FileColor::Info),
        "js" | "py" => (Glyph::FileCode, FileColor::Warning),
        "jsx" => (Glyph::Component, FileColor::Warning),
        "json" => (Glyph::FileJson, FileColor::Neutral),
        "md" => (Glyph::BookText, FileColor::Info),
        "css" => (Glyph::Palette, FileColor::Info),
        "html" => (Glyph::Globe, FileColor::Conflicted),
        "rs" => (Glyph::Cog, FileColor::Staged),
        "go" => (Glyph::FileCode, FileColor::Renamed),
        "java" => (Glyph::Coffee, FileColor::Error),
        "sh" => (Glyph::Terminal, FileColor::Success),
        "yml" | "yaml" => (Glyph::FileCog, FileColor::Error),
        "toml" => (Glyph::FileCog, FileColor::Neutral),
        "lock" => (Glyph::Lock, FileColor::Neutral),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => (Glyph::Image, FileColor::Staged),
        "pdf" => (Glyph::FileText, FileColor::Error),
        "zip" => (Glyph::FileArchive, FileColor::Warning),
        _ => (Glyph::File, FileColor::Neutral),
    }
}

pub(crate) struct Icons {
    trees: HashMap<Glyph, usvg::Tree>,
    textures: HashMap<(Glyph, u32), TextureHandle>,
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
            trees: SOURCES
                .iter()
                .map(|(glyph, source)| {
                    let tree = usvg::Tree::from_data(source, &options).map_err(|error| {
                        AppError::Internal(format!("native Problems icon: {error}"))
                    })?;
                    Ok((*glyph, tree))
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
        glyph: Glyph,
        color: Color32,
        angle: f32,
    ) -> AppResult<()> {
        let scale = ui.ctx().pixels_per_point();
        let pixels = (rect.width() * scale).ceil();
        if !pixels.is_finite() || !(1.0..=MAX_RASTER_SIDE).contains(&pixels) {
            return Err(AppError::Internal(
                "native Problems icon scale is out of range".into(),
            ));
        }
        if self.scale != Some(scale) {
            self.textures.clear();
            self.scale = Some(scale);
        }
        let side = pixels as u32;
        let key = (glyph, side);
        if let Entry::Vacant(entry) = self.textures.entry(key) {
            let tree = self.trees.get(&glyph).ok_or_else(|| {
                AppError::Internal("native Problems icon source is missing".into())
            })?;
            let mut pixmap = tiny_skia::Pixmap::new(side, side).ok_or_else(|| {
                AppError::Internal("native Problems icon allocation failed".into())
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
                format!("native-problems-{glyph:?}-{side}"),
                ColorImage::from_rgba_unmultiplied(
                    [side as usize, side as usize],
                    &pixmap.take_demultiplied(),
                ),
                egui::TextureOptions::LINEAR,
            ));
        }
        let texture = &self.textures[&key];
        egui::Image::new((texture.id(), rect.size()))
            .tint(color)
            .rotate(angle, egui::Vec2::splat(ROTATION_CENTER))
            .paint_at(ui, rect);
        Ok(())
    }
}

#[cfg(test)]
#[path = "problems-icons-tests.rs"]
mod tests;
