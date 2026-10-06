use std::collections::HashMap;
use std::collections::hash_map::Entry;

use super::egui::{self, Color32, ColorImage, Rect, TextureHandle, Ui};
use resvg::{tiny_skia, usvg};
use taide_model::error::{AppError, AppResult};
use taide_model::layout::TabKind;

const VIEWBOX: f32 = 24.0;
const MAX_RASTER_SIDE: f32 = 1024.0;
const ROTATION_CENTER: f32 = 0.5;
const UNTITLED_TAB_FILE_NAME: &str = "untitled";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
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
    Folder,
    FolderOpen,
    FolderCode,
    Box,
    FlaskConical,
    GitFork,
    Settings,
    FileDiff,
    FileSearchCorner,
    Sparkles,
}

const SOURCES: &[(Glyph, &[u8])] = &[
    (
        Glyph::CircleX,
        include_bytes!("../../taide-native-app/resources/problems/circle-x.svg"),
    ),
    (
        Glyph::TriangleAlert,
        include_bytes!("../../taide-native-app/resources/problems/triangle-alert.svg"),
    ),
    (
        Glyph::Info,
        include_bytes!("../../taide-native-app/resources/problems/info.svg"),
    ),
    (
        Glyph::Lightbulb,
        include_bytes!("../../taide-native-app/resources/problems/lightbulb.svg"),
    ),
    (
        Glyph::CircleCheck,
        include_bytes!("../../taide-native-app/resources/problems/circle-check.svg"),
    ),
    (
        Glyph::X,
        include_bytes!("../../taide-native-app/resources/problems/x.svg"),
    ),
    (
        Glyph::ChevronRight,
        include_bytes!("../../taide-native-app/resources/problems/chevron-right.svg"),
    ),
    (
        Glyph::File,
        include_bytes!("../../taide-native-app/resources/problems/file.svg"),
    ),
    (
        Glyph::FileCode,
        include_bytes!("../../taide-native-app/resources/problems/file-code.svg"),
    ),
    (
        Glyph::Component,
        include_bytes!("../../taide-native-app/resources/problems/component.svg"),
    ),
    (
        Glyph::FileJson,
        include_bytes!("../../taide-native-app/resources/problems/file-json.svg"),
    ),
    (
        Glyph::BookText,
        include_bytes!("../../taide-native-app/resources/problems/book-text.svg"),
    ),
    (
        Glyph::Palette,
        include_bytes!("../../taide-native-app/resources/problems/palette.svg"),
    ),
    (
        Glyph::Globe,
        include_bytes!("../../taide-native-app/resources/problems/globe.svg"),
    ),
    (
        Glyph::Cog,
        include_bytes!("../../taide-native-app/resources/problems/cog.svg"),
    ),
    (
        Glyph::Coffee,
        include_bytes!("../../taide-native-app/resources/problems/coffee.svg"),
    ),
    (
        Glyph::Terminal,
        include_bytes!("../../taide-native-app/resources/problems/terminal.svg"),
    ),
    (
        Glyph::FileCog,
        include_bytes!("../../taide-native-app/resources/problems/file-cog.svg"),
    ),
    (
        Glyph::Lock,
        include_bytes!("../../taide-native-app/resources/problems/lock.svg"),
    ),
    (
        Glyph::Image,
        include_bytes!("../../taide-native-app/resources/problems/image.svg"),
    ),
    (
        Glyph::FileText,
        include_bytes!("../../taide-native-app/resources/problems/file-text.svg"),
    ),
    (
        Glyph::FileArchive,
        include_bytes!("../../taide-native-app/resources/problems/file-archive.svg"),
    ),
    (
        Glyph::Package,
        include_bytes!("../../taide-native-app/resources/problems/package.svg"),
    ),
    (
        Glyph::Settings2,
        include_bytes!("../../taide-native-app/resources/problems/settings-2.svg"),
    ),
    (
        Glyph::GitBranch,
        include_bytes!("../../taide-native-app/resources/problems/git-branch.svg"),
    ),
    (
        Glyph::Container,
        include_bytes!("../../taide-native-app/resources/problems/container.svg"),
    ),
    (
        Glyph::BookMarked,
        include_bytes!("../../taide-native-app/resources/problems/book-marked.svg"),
    ),
    (
        Glyph::Scale,
        include_bytes!("../../taide-native-app/resources/problems/scale.svg"),
    ),
    (
        Glyph::Key,
        include_bytes!("../../taide-native-app/resources/problems/key.svg"),
    ),
    (
        Glyph::Folder,
        include_bytes!("../../taide-native-app/resources/icons/folder.svg"),
    ),
    (
        Glyph::FolderOpen,
        include_bytes!("../../taide-native-app/resources/icons/folder-open.svg"),
    ),
    (
        Glyph::FolderCode,
        include_bytes!("../../taide-native-app/resources/icons/folder-code.svg"),
    ),
    (
        Glyph::Box,
        include_bytes!("../../taide-native-app/resources/icons/box.svg"),
    ),
    (
        Glyph::FlaskConical,
        include_bytes!("../../taide-native-app/resources/icons/flask-conical.svg"),
    ),
    (
        Glyph::GitFork,
        include_bytes!("../../taide-native-app/resources/icons/git-fork.svg"),
    ),
    (
        Glyph::Settings,
        include_bytes!("../../taide-native-app/resources/icons/settings.svg"),
    ),
    (
        Glyph::FileDiff,
        include_bytes!("../../taide-native-app/resources/icons/file-diff.svg"),
    ),
    (
        Glyph::FileSearchCorner,
        include_bytes!("../../taide-native-app/resources/icons/file-search-corner.svg"),
    ),
    (
        Glyph::Sparkles,
        include_bytes!("../../taide-native-app/resources/icons/sparkles.svg"),
    ),
];

pub const SEVERITIES: [Glyph; 4] = [
    Glyph::CircleX,
    Glyph::TriangleAlert,
    Glyph::Info,
    Glyph::Lightbulb,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileColor {
    Info,
    Warning,
    Error,
    Success,
    Renamed,
    Conflicted,
    Staged,
    Neutral,
    Folder,
}

impl FileColor {
    pub const ALL: [Self; 9] = [
        Self::Info,
        Self::Warning,
        Self::Error,
        Self::Success,
        Self::Renamed,
        Self::Conflicted,
        Self::Staged,
        Self::Neutral,
        Self::Folder,
    ];

    pub fn theme_key(self) -> &'static str {
        match self {
            Self::Info => "statusIndicator.info",
            Self::Warning => "statusIndicator.warning",
            Self::Error => "statusIndicator.error",
            Self::Success => "statusIndicator.success",
            Self::Renamed => "git.renamed",
            Self::Conflicted => "git.conflicted",
            Self::Staged => "git.staged",
            Self::Neutral => "appSidebar.iconDefault",
            Self::Folder => "explorer.folderIcon",
        }
    }
}

pub fn file(name: &str) -> (Glyph, FileColor) {
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

pub fn folder(name: &str, is_expanded: bool) -> (Glyph, FileColor) {
    match name.to_lowercase().as_str() {
        "src" => (Glyph::FolderCode, FileColor::Info),
        "node_modules" => (Glyph::Package, FileColor::Neutral),
        "dist" | "build" => (Glyph::Box, FileColor::Warning),
        "test" | "tests" => (Glyph::FlaskConical, FileColor::Success),
        "docs" => (Glyph::BookMarked, FileColor::Info),
        "public" => (Glyph::Globe, FileColor::Renamed),
        "assets" => (Glyph::Image, FileColor::Staged),
        ".git" => (Glyph::GitBranch, FileColor::Conflicted),
        ".github" => (Glyph::GitFork, FileColor::Info),
        _ if is_expanded => (Glyph::FolderOpen, FileColor::Folder),
        _ => (Glyph::Folder, FileColor::Folder),
    }
}

pub fn tab(kind: &TabKind) -> (Glyph, Option<FileColor>) {
    let file_name = match kind {
        TabKind::File { path } => &path[path.rfind('/').map_or(0, |index| index + 1)..],
        TabKind::Untitled { .. } => UNTITLED_TAB_FILE_NAME,
        TabKind::Terminal { .. } => return (Glyph::Terminal, None),
        TabKind::Settings | TabKind::AppFile { .. } => return (Glyph::Settings, None),
        TabKind::Diff { .. } => return (Glyph::FileDiff, None),
        TabKind::SearchEditor { .. } => return (Glyph::FileSearchCorner, None),
        TabKind::ClaudeDiff { .. } | TabKind::Welcome => return (Glyph::Sparkles, None),
    };
    let (glyph, color) = file(file_name);
    (glyph, Some(color))
}

pub struct Icons {
    trees: HashMap<Glyph, usvg::Tree>,
    textures: HashMap<(Glyph, u32), TextureHandle>,
    scale: Option<f32>,
}

impl Icons {
    pub fn new() -> AppResult<Self> {
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
                        AppError::Internal(format!("native glyph icon: {error}"))
                    })?;
                    Ok((*glyph, tree))
                })
                .collect::<AppResult<_>>()?,
            textures: HashMap::new(),
            scale: None,
        })
    }

    pub fn paint(
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
                "native glyph icon scale is out of range".into(),
            ));
        }
        if self.scale != Some(scale) {
            self.textures.clear();
            self.scale = Some(scale);
        }
        let side = pixels as u32;
        let key = (glyph, side);
        if let Entry::Vacant(entry) = self.textures.entry(key) {
            let tree = self
                .trees
                .get(&glyph)
                .ok_or_else(|| AppError::Internal("native glyph icon source is missing".into()))?;
            let mut pixmap = tiny_skia::Pixmap::new(side, side)
                .ok_or_else(|| AppError::Internal("native glyph icon allocation failed".into()))?;
            resvg::render(
                tree,
                tiny_skia::Transform::from_scale(
                    rect.width() * scale / VIEWBOX,
                    rect.height() * scale / VIEWBOX,
                ),
                &mut pixmap.as_mut(),
            );
            entry.insert(ui.ctx().load_texture(
                format!("native-glyph-{glyph:?}-{side}"),
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
