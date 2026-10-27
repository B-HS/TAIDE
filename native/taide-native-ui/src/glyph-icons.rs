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
    Braces,
    Code,
    CodeXml,
    Bug,
    Binary,
    LayoutDashboard,
    PanelsTopLeft,
    Monitor,
    AppWindow,
    Smartphone,
    TabletSmartphone,
    Watch,
    Database,
    Server,
    HardDrive,
    Table2,
    Cloud,
    Network,
    Cpu,
    ShieldCheck,
    Brush,
    PenTool,
    Shapes,
    BookOpen,
    NotebookPen,
    ClipboardList,
    Newspaper,
    Microscope,
    TestTube,
    Bot,
    Brain,
    WandSparkles,
    Gamepad2,
    Dices,
    Swords,
    Wallet,
    CreditCard,
    Banknote,
    PiggyBank,
    Rocket,
    Star,
    Heart,
    Flag,
    Zap,
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
    (
        Glyph::Braces,
        include_bytes!("../../taide-native-app/resources/icons/braces.svg"),
    ),
    (
        Glyph::Code,
        include_bytes!("../../taide-native-app/resources/project-icons/code.svg"),
    ),
    (
        Glyph::CodeXml,
        include_bytes!("../../taide-native-app/resources/project-icons/code-xml.svg"),
    ),
    (
        Glyph::Bug,
        include_bytes!("../../taide-native-app/resources/project-icons/bug.svg"),
    ),
    (
        Glyph::Binary,
        include_bytes!("../../taide-native-app/resources/project-icons/binary.svg"),
    ),
    (
        Glyph::LayoutDashboard,
        include_bytes!("../../taide-native-app/resources/project-icons/layout-dashboard.svg"),
    ),
    (
        Glyph::PanelsTopLeft,
        include_bytes!("../../taide-native-app/resources/project-icons/panels-top-left.svg"),
    ),
    (
        Glyph::Monitor,
        include_bytes!("../../taide-native-app/resources/project-icons/monitor.svg"),
    ),
    (
        Glyph::AppWindow,
        include_bytes!("../../taide-native-app/resources/project-icons/app-window.svg"),
    ),
    (
        Glyph::Smartphone,
        include_bytes!("../../taide-native-app/resources/project-icons/smartphone.svg"),
    ),
    (
        Glyph::TabletSmartphone,
        include_bytes!("../../taide-native-app/resources/project-icons/tablet-smartphone.svg"),
    ),
    (
        Glyph::Watch,
        include_bytes!("../../taide-native-app/resources/project-icons/watch.svg"),
    ),
    (
        Glyph::Database,
        include_bytes!("../../taide-native-app/resources/project-icons/database.svg"),
    ),
    (
        Glyph::Server,
        include_bytes!("../../taide-native-app/resources/project-icons/server.svg"),
    ),
    (
        Glyph::HardDrive,
        include_bytes!("../../taide-native-app/resources/project-icons/hard-drive.svg"),
    ),
    (
        Glyph::Table2,
        include_bytes!("../../taide-native-app/resources/project-icons/table-2.svg"),
    ),
    (
        Glyph::Cloud,
        include_bytes!("../../taide-native-app/resources/project-icons/cloud.svg"),
    ),
    (
        Glyph::Network,
        include_bytes!("../../taide-native-app/resources/project-icons/network.svg"),
    ),
    (
        Glyph::Cpu,
        include_bytes!("../../taide-native-app/resources/project-icons/cpu.svg"),
    ),
    (
        Glyph::ShieldCheck,
        include_bytes!("../../taide-native-app/resources/project-icons/shield-check.svg"),
    ),
    (
        Glyph::Brush,
        include_bytes!("../../taide-native-app/resources/project-icons/brush.svg"),
    ),
    (
        Glyph::PenTool,
        include_bytes!("../../taide-native-app/resources/project-icons/pen-tool.svg"),
    ),
    (
        Glyph::Shapes,
        include_bytes!("../../taide-native-app/resources/project-icons/shapes.svg"),
    ),
    (
        Glyph::BookOpen,
        include_bytes!("../../taide-native-app/resources/project-icons/book-open.svg"),
    ),
    (
        Glyph::NotebookPen,
        include_bytes!("../../taide-native-app/resources/project-icons/notebook-pen.svg"),
    ),
    (
        Glyph::ClipboardList,
        include_bytes!("../../taide-native-app/resources/project-icons/clipboard-list.svg"),
    ),
    (
        Glyph::Newspaper,
        include_bytes!("../../taide-native-app/resources/project-icons/newspaper.svg"),
    ),
    (
        Glyph::Microscope,
        include_bytes!("../../taide-native-app/resources/project-icons/microscope.svg"),
    ),
    (
        Glyph::TestTube,
        include_bytes!("../../taide-native-app/resources/project-icons/test-tube.svg"),
    ),
    (
        Glyph::Bot,
        include_bytes!("../../taide-native-app/resources/project-icons/bot.svg"),
    ),
    (
        Glyph::Brain,
        include_bytes!("../../taide-native-app/resources/project-icons/brain.svg"),
    ),
    (
        Glyph::WandSparkles,
        include_bytes!("../../taide-native-app/resources/project-icons/wand-sparkles.svg"),
    ),
    (
        Glyph::Gamepad2,
        include_bytes!("../../taide-native-app/resources/project-icons/gamepad-2.svg"),
    ),
    (
        Glyph::Dices,
        include_bytes!("../../taide-native-app/resources/project-icons/dices.svg"),
    ),
    (
        Glyph::Swords,
        include_bytes!("../../taide-native-app/resources/project-icons/swords.svg"),
    ),
    (
        Glyph::Wallet,
        include_bytes!("../../taide-native-app/resources/project-icons/wallet.svg"),
    ),
    (
        Glyph::CreditCard,
        include_bytes!("../../taide-native-app/resources/project-icons/credit-card.svg"),
    ),
    (
        Glyph::Banknote,
        include_bytes!("../../taide-native-app/resources/project-icons/banknote.svg"),
    ),
    (
        Glyph::PiggyBank,
        include_bytes!("../../taide-native-app/resources/project-icons/piggy-bank.svg"),
    ),
    (
        Glyph::Rocket,
        include_bytes!("../../taide-native-app/resources/project-icons/rocket.svg"),
    ),
    (
        Glyph::Star,
        include_bytes!("../../taide-native-app/resources/project-icons/star.svg"),
    ),
    (
        Glyph::Heart,
        include_bytes!("../../taide-native-app/resources/project-icons/heart.svg"),
    ),
    (
        Glyph::Flag,
        include_bytes!("../../taide-native-app/resources/project-icons/flag.svg"),
    ),
    (
        Glyph::Zap,
        include_bytes!("../../taide-native-app/resources/project-icons/zap.svg"),
    ),
];

pub const DEFAULT_PROJECT_ICON: Glyph = Glyph::Folder;

pub const PROJECT_ICONS: [(&str, Glyph); 56] = [
    ("code", Glyph::Code),
    ("code-xml", Glyph::CodeXml),
    ("terminal", Glyph::Terminal),
    ("git-branch", Glyph::GitBranch),
    ("git-fork", Glyph::GitFork),
    ("bug", Glyph::Bug),
    ("braces", Glyph::Braces),
    ("binary", Glyph::Binary),
    ("globe", Glyph::Globe),
    ("layout-dashboard", Glyph::LayoutDashboard),
    ("panels-top-left", Glyph::PanelsTopLeft),
    ("monitor", Glyph::Monitor),
    ("app-window", Glyph::AppWindow),
    ("smartphone", Glyph::Smartphone),
    ("tablet-smartphone", Glyph::TabletSmartphone),
    ("watch", Glyph::Watch),
    ("database", Glyph::Database),
    ("server", Glyph::Server),
    ("hard-drive", Glyph::HardDrive),
    ("table-2", Glyph::Table2),
    ("cloud", Glyph::Cloud),
    ("container", Glyph::Container),
    ("network", Glyph::Network),
    ("cpu", Glyph::Cpu),
    ("shield-check", Glyph::ShieldCheck),
    ("palette", Glyph::Palette),
    ("brush", Glyph::Brush),
    ("pen-tool", Glyph::PenTool),
    ("shapes", Glyph::Shapes),
    ("image", Glyph::Image),
    ("file-text", Glyph::FileText),
    ("book-open", Glyph::BookOpen),
    ("notebook-pen", Glyph::NotebookPen),
    ("clipboard-list", Glyph::ClipboardList),
    ("newspaper", Glyph::Newspaper),
    ("flask-conical", Glyph::FlaskConical),
    ("microscope", Glyph::Microscope),
    ("test-tube", Glyph::TestTube),
    ("bot", Glyph::Bot),
    ("brain", Glyph::Brain),
    ("sparkles", Glyph::Sparkles),
    ("wand-sparkles", Glyph::WandSparkles),
    ("gamepad-2", Glyph::Gamepad2),
    ("dices", Glyph::Dices),
    ("swords", Glyph::Swords),
    ("wallet", Glyph::Wallet),
    ("credit-card", Glyph::CreditCard),
    ("banknote", Glyph::Banknote),
    ("piggy-bank", Glyph::PiggyBank),
    ("folder", Glyph::Folder),
    ("rocket", Glyph::Rocket),
    ("star", Glyph::Star),
    ("heart", Glyph::Heart),
    ("flag", Glyph::Flag),
    ("zap", Glyph::Zap),
    ("package", Glyph::Package),
];

pub fn project(name: Option<&str>) -> Glyph {
    name.and_then(|name| PROJECT_ICONS.iter().find(|(id, _)| *id == name))
        .map_or(DEFAULT_PROJECT_ICON, |(_, glyph)| *glyph)
}

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

pub fn tab(kind: &TabKind, has_agent: bool) -> (Glyph, Option<FileColor>) {
    let file_name = match kind {
        TabKind::File { path } => &path[path.rfind('/').map_or(0, |index| index + 1)..],
        TabKind::Untitled { .. } => UNTITLED_TAB_FILE_NAME,
        TabKind::Terminal { .. } if has_agent => return (Glyph::Sparkles, None),
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
        self.prepare(ui.ctx(), rect.size(), glyph)?;
        self.paint_prepared(ui, rect, glyph, color, angle)
    }

    pub fn prepare(
        &mut self,
        context: &egui::Context,
        size: egui::Vec2,
        glyph: Glyph,
    ) -> AppResult<()> {
        let (scale, side) = raster_side(context, size.x)?;
        if self.scale != Some(scale) {
            self.textures.clear();
            self.scale = Some(scale);
        }
        if let Entry::Vacant(entry) = self.textures.entry((glyph, side)) {
            let tree = self
                .trees
                .get(&glyph)
                .ok_or_else(|| AppError::Internal("native glyph icon source is missing".into()))?;
            let mut pixmap = tiny_skia::Pixmap::new(side, side)
                .ok_or_else(|| AppError::Internal("native glyph icon allocation failed".into()))?;
            resvg::render(
                tree,
                tiny_skia::Transform::from_scale(
                    size.x * scale / VIEWBOX,
                    size.y * scale / VIEWBOX,
                ),
                &mut pixmap.as_mut(),
            );
            entry.insert(context.load_texture(
                format!("native-glyph-{glyph:?}-{side}"),
                ColorImage::from_rgba_unmultiplied(
                    [side as usize, side as usize],
                    &pixmap.take_demultiplied(),
                ),
                egui::TextureOptions::LINEAR,
            ));
        }
        Ok(())
    }

    pub fn paint_prepared(
        &self,
        ui: &Ui,
        rect: Rect,
        glyph: Glyph,
        color: Color32,
        angle: f32,
    ) -> AppResult<()> {
        ui.painter()
            .add(self.shape(ui.ctx(), rect, glyph, color, angle)?);
        Ok(())
    }

    pub fn shape(
        &self,
        context: &egui::Context,
        rect: Rect,
        glyph: Glyph,
        color: Color32,
        angle: f32,
    ) -> AppResult<egui::Shape> {
        let (scale, side) = raster_side(context, rect.width())?;
        let texture = self
            .textures
            .get(&(glyph, side))
            .filter(|_| self.scale == Some(scale))
            .ok_or_else(|| AppError::Internal("native glyph icon is not prepared".into()))?;
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
}

fn raster_side(context: &egui::Context, width: f32) -> AppResult<(f32, u32)> {
    let scale = context.pixels_per_point();
    let pixels = (width * scale).ceil();
    if !pixels.is_finite() || !(1.0..=MAX_RASTER_SIDE).contains(&pixels) {
        return Err(AppError::Internal(
            "native glyph icon scale is out of range".into(),
        ));
    }
    Ok((scale, pixels as u32))
}
