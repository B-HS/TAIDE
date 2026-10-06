use std::collections::HashSet;

use super::glyphs::{FileColor, Glyph, Icons, file, folder, tab};
use egui::{Color32, Rect};
use taide_model::layout::TabKind;

const ROW_ICON_SIDE: f32 = 14.0;
const SCREEN_SIDE: f32 = 400.0;
const FILE_TYPE_GLYPHS: [Glyph; 28] = [
    Glyph::File,
    Glyph::FileCode,
    Glyph::Component,
    Glyph::FileJson,
    Glyph::BookText,
    Glyph::Palette,
    Glyph::Globe,
    Glyph::Cog,
    Glyph::Coffee,
    Glyph::Terminal,
    Glyph::FileCog,
    Glyph::Lock,
    Glyph::Image,
    Glyph::FileText,
    Glyph::FileArchive,
    Glyph::Package,
    Glyph::Settings2,
    Glyph::GitBranch,
    Glyph::GitFork,
    Glyph::Container,
    Glyph::BookMarked,
    Glyph::Scale,
    Glyph::Key,
    Glyph::Folder,
    Glyph::FolderOpen,
    Glyph::FolderCode,
    Glyph::Box,
    Glyph::FlaskConical,
];
const REPRESENTATIVE_FILE_NAMES: [&str; 16] = [
    "package.json",
    "tsconfig.json",
    "Cargo.toml",
    "Cargo.lock",
    ".gitignore",
    "Dockerfile",
    "README.md",
    "LICENSE",
    ".env.local",
    "main.rs",
    "app.tsx",
    "style.css",
    "photo.png",
    "archive.zip",
    "notes",
    "weird.unknownext",
];
const REPRESENTATIVE_FOLDER_NAMES: [&str; 6] = [
    "src",
    ".git",
    ".github",
    "assets",
    "node_modules",
    "anything-else",
];
const TAB_ONLY_GLYPHS: [Glyph; 4] = [
    Glyph::Settings,
    Glyph::FileDiff,
    Glyph::FileSearchCorner,
    Glyph::Sparkles,
];

fn rasters(glyphs: &[Glyph]) -> HashSet<Vec<Color32>> {
    let context = egui::Context::default();
    let mut icons = Icons::new().unwrap();
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::splat(SCREEN_SIDE),
            )),
            ..Default::default()
        },
        |ui| {
            for glyph in glyphs {
                icons
                    .paint(
                        ui,
                        Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(ROW_ICON_SIDE)),
                        *glyph,
                        Color32::WHITE,
                        0.0,
                    )
                    .unwrap();
            }
        },
    );
    let side = ROW_ICON_SIDE as usize;
    let rasters = output
        .textures_delta
        .set
        .iter()
        .flat_map(|(_, deltas)| deltas.iter())
        .filter_map(|delta| match &delta.image {
            egui::ImageData::Color(image) if image.size == [side, side] => {
                Some(image.pixels.clone())
            }
            _ => None,
        })
        .collect();
    output.textures_delta.clear();
    rasters
}

#[test]
fn 파일_이름은_특수_이름_접두사_확장자_순서로_아이콘과_색을_정한다() {
    for (name, glyph, color) in [
        ("index.ts", Glyph::FileCode, FileColor::Info),
        ("App.tsx", Glyph::Component, FileColor::Info),
        ("index.js", Glyph::FileCode, FileColor::Warning),
        ("App.jsx", Glyph::Component, FileColor::Warning),
        ("data.json", Glyph::FileJson, FileColor::Neutral),
        ("notes.md", Glyph::BookText, FileColor::Info),
        ("style.css", Glyph::Palette, FileColor::Info),
        ("index.html", Glyph::Globe, FileColor::Conflicted),
        ("main.rs", Glyph::Cog, FileColor::Staged),
        ("script.py", Glyph::FileCode, FileColor::Warning),
        ("main.go", Glyph::FileCode, FileColor::Renamed),
        ("Main.java", Glyph::Coffee, FileColor::Error),
        ("build.sh", Glyph::Terminal, FileColor::Success),
        ("ci.yml", Glyph::FileCog, FileColor::Error),
        ("ci.yaml", Glyph::FileCog, FileColor::Error),
        ("rustfmt.toml", Glyph::FileCog, FileColor::Neutral),
        ("bun.lock", Glyph::Lock, FileColor::Neutral),
        ("photo.png", Glyph::Image, FileColor::Staged),
        ("photo.jpg", Glyph::Image, FileColor::Staged),
        ("photo.jpeg", Glyph::Image, FileColor::Staged),
        ("anim.gif", Glyph::Image, FileColor::Staged),
        ("banner.webp", Glyph::Image, FileColor::Staged),
        ("icon.svg", Glyph::Image, FileColor::Staged),
        ("report.pdf", Glyph::FileText, FileColor::Error),
        ("archive.zip", Glyph::FileArchive, FileColor::Warning),
        ("INDEX.TS", Glyph::FileCode, FileColor::Info),
        ("Data.JSON", Glyph::FileJson, FileColor::Neutral),
        ("package.json", Glyph::Package, FileColor::Error),
        ("tsconfig.json", Glyph::Settings2, FileColor::Info),
        ("Cargo.toml", Glyph::Cog, FileColor::Conflicted),
        ("Cargo.lock", Glyph::Lock, FileColor::Neutral),
        (".gitignore", Glyph::GitBranch, FileColor::Conflicted),
        ("Dockerfile", Glyph::Container, FileColor::Info),
        ("README.md", Glyph::BookMarked, FileColor::Info),
        ("README", Glyph::BookMarked, FileColor::Info),
        ("readme.txt", Glyph::BookMarked, FileColor::Info),
        ("LICENSE", Glyph::Scale, FileColor::Warning),
        ("LICENSE.md", Glyph::Scale, FileColor::Warning),
        (".env", Glyph::Key, FileColor::Warning),
        (".env.local", Glyph::Key, FileColor::Warning),
        (".env.production", Glyph::Key, FileColor::Warning),
        ("archive.tar", Glyph::File, FileColor::Neutral),
        ("unknown.xyz", Glyph::File, FileColor::Neutral),
        ("Makefile", Glyph::File, FileColor::Neutral),
        (".prettierrc", Glyph::File, FileColor::Neutral),
        ("untitled", Glyph::File, FileColor::Neutral),
    ] {
        assert_eq!(file(name), (glyph, color), "{name}");
    }
}

#[test]
fn 폴더_이름은_특수_이름을_먼저_보고_나머지는_펼침_여부로_아이콘을_정한다() {
    for (name, glyph, color) in [
        ("src", Glyph::FolderCode, FileColor::Info),
        ("node_modules", Glyph::Package, FileColor::Neutral),
        ("dist", Glyph::Box, FileColor::Warning),
        ("build", Glyph::Box, FileColor::Warning),
        ("test", Glyph::FlaskConical, FileColor::Success),
        ("tests", Glyph::FlaskConical, FileColor::Success),
        ("docs", Glyph::BookMarked, FileColor::Info),
        ("public", Glyph::Globe, FileColor::Renamed),
        ("assets", Glyph::Image, FileColor::Staged),
        (".git", Glyph::GitBranch, FileColor::Conflicted),
        (".github", Glyph::GitFork, FileColor::Info),
        ("SRC", Glyph::FolderCode, FileColor::Info),
        ("Node_Modules", Glyph::Package, FileColor::Neutral),
    ] {
        assert_eq!(folder(name, false), (glyph, color), "{name}");
        assert_eq!(
            folder(name, true),
            (glyph, color),
            "{name} 은 펼쳐도 같은 아이콘을 유지한다"
        );
    }
    assert_eq!(
        folder("components", false),
        (Glyph::Folder, FileColor::Folder)
    );
    assert_eq!(
        folder("components", true),
        (Glyph::FolderOpen, FileColor::Folder)
    );
}

#[test]
fn 파일과_폴더_아이콘_이름은_모두_서로_다른_글리프로_그려진다() {
    let resolved = REPRESENTATIVE_FILE_NAMES
        .iter()
        .map(|name| file(name).0)
        .chain(REPRESENTATIVE_FOLDER_NAMES.iter().flat_map(|name| {
            [false, true]
                .into_iter()
                .map(|is_expanded| folder(name, is_expanded).0)
        }))
        .collect::<Vec<_>>();
    assert!(
        resolved
            .iter()
            .all(|glyph| FILE_TYPE_GLYPHS.contains(glyph))
    );
    assert_eq!(
        FILE_TYPE_GLYPHS.iter().collect::<HashSet<_>>().len(),
        FILE_TYPE_GLYPHS.len()
    );

    let rasters = rasters(&FILE_TYPE_GLYPHS);
    assert_eq!(rasters.len(), FILE_TYPE_GLYPHS.len());
    assert!(
        rasters
            .iter()
            .all(|pixels| pixels.iter().any(|pixel| pixel.a() > 0))
    );
}

#[test]
fn 탭_종류는_ts의_탭_아이콘과_같은_글리프와_색으로_풀린다() {
    for (kind, glyph, color) in [
        (
            r#"{"kind":"file","path":"/synthetic/project/src/main.rs"}"#,
            Glyph::Cog,
            Some(FileColor::Staged),
        ),
        (
            r#"{"kind":"file","path":"/synthetic/project.rs/package.json"}"#,
            Glyph::Package,
            Some(FileColor::Error),
        ),
        (
            r#"{"kind":"file","path":"/synthetic/project.rs/notes"}"#,
            Glyph::File,
            Some(FileColor::Neutral),
        ),
        (
            r#"{"kind":"file","path":"README.md"}"#,
            Glyph::BookMarked,
            Some(FileColor::Info),
        ),
        (
            r#"{"kind":"untitled","index":1}"#,
            Glyph::File,
            Some(FileColor::Neutral),
        ),
        (
            r#"{"kind":"terminal","sessionId":"synthetic-session"}"#,
            Glyph::Terminal,
            None,
        ),
        (r#"{"kind":"settings"}"#, Glyph::Settings, None),
        (
            r#"{"kind":"appFile","target":{"kind":"settings"}}"#,
            Glyph::Settings,
            None,
        ),
        (
            r#"{"kind":"diff","path":"/synthetic/project/src/main.rs","staged":false}"#,
            Glyph::FileDiff,
            None,
        ),
        (
            r#"{"kind":"searchEditor","query":{"text":"synthetic"}}"#,
            Glyph::FileSearchCorner,
            None,
        ),
        (
            r#"{"kind":"claudeDiff","requestId":"synthetic-request","path":"/synthetic/project/src/main.rs"}"#,
            Glyph::Sparkles,
            None,
        ),
        (r#"{"kind":"welcome"}"#, Glyph::Sparkles, None),
    ] {
        let parsed = serde_json::from_str::<TabKind>(kind).unwrap();
        assert_eq!(tab(&parsed), (glyph, color), "{kind}");
    }
}

#[test]
fn 탭_전용_글리프는_파일_타입_글리프와_구분되는_서로_다른_모양으로_그려진다() {
    let glyphs = [FILE_TYPE_GLYPHS.as_slice(), TAB_ONLY_GLYPHS.as_slice()].concat();
    let rasters = rasters(&glyphs);
    assert_eq!(rasters.len(), glyphs.len());
    assert!(
        rasters
            .iter()
            .all(|pixels| pixels.iter().any(|pixel| pixel.a() > 0))
    );
}

#[test]
fn 아이콘_색은_ts_색_클래스와_같은_테마_키로_풀린다() {
    for (color, key) in [
        (FileColor::Info, "statusIndicator.info"),
        (FileColor::Warning, "statusIndicator.warning"),
        (FileColor::Error, "statusIndicator.error"),
        (FileColor::Success, "statusIndicator.success"),
        (FileColor::Renamed, "git.renamed"),
        (FileColor::Conflicted, "git.conflicted"),
        (FileColor::Staged, "git.staged"),
        (FileColor::Neutral, "appSidebar.iconDefault"),
        (FileColor::Folder, "explorer.folderIcon"),
    ] {
        assert_eq!(color.theme_key(), key);
    }
    assert_eq!(
        FileColor::ALL.into_iter().collect::<HashSet<_>>().len(),
        FileColor::ALL.len()
    );
}
