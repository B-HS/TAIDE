use super::*;

const SIZES: [f32; 3] = [12.0, 14.0, 20.0];
const RETINA_SCALE: f32 = 2.0;
const SCREEN_SIZE: f32 = 800.0;

#[test]
fn problems_원본_파일_분류와_svg는_크기별_재사용과_dpi_해제를_보존한다() {
    for (name, glyph, color) in [
        ("PACKAGE.JSON", Glyph::Package, FileColor::Error),
        ("TsConfig.Json", Glyph::Settings2, FileColor::Info),
        ("Cargo.toml", Glyph::Cog, FileColor::Conflicted),
        ("Cargo.lock", Glyph::Lock, FileColor::Neutral),
        (".GITIGNORE", Glyph::GitBranch, FileColor::Conflicted),
        ("Dockerfile", Glyph::Container, FileColor::Info),
        ("README", Glyph::BookMarked, FileColor::Info),
        ("readme.json", Glyph::BookMarked, FileColor::Info),
        ("LICENSE-MIT", Glyph::Scale, FileColor::Warning),
        ("LICENCE.md", Glyph::Scale, FileColor::Warning),
        (".env.production.json", Glyph::Key, FileColor::Warning),
        ("file.TS", Glyph::FileCode, FileColor::Info),
        ("file.tsx", Glyph::Component, FileColor::Info),
        ("file.js", Glyph::FileCode, FileColor::Warning),
        ("file.jsx", Glyph::Component, FileColor::Warning),
        ("file.json", Glyph::FileJson, FileColor::Neutral),
        ("file.md", Glyph::BookText, FileColor::Info),
        ("file.css", Glyph::Palette, FileColor::Info),
        ("file.html", Glyph::Globe, FileColor::Conflicted),
        ("file.rs", Glyph::Cog, FileColor::Staged),
        ("file.py", Glyph::FileCode, FileColor::Warning),
        ("file.go", Glyph::FileCode, FileColor::Renamed),
        ("file.java", Glyph::Coffee, FileColor::Error),
        ("file.sh", Glyph::Terminal, FileColor::Success),
        ("file.yml", Glyph::FileCog, FileColor::Error),
        ("file.yaml", Glyph::FileCog, FileColor::Error),
        ("file.toml", Glyph::FileCog, FileColor::Neutral),
        ("file.lock", Glyph::Lock, FileColor::Neutral),
        ("file.png", Glyph::Image, FileColor::Staged),
        ("file.jpg", Glyph::Image, FileColor::Staged),
        ("file.jpeg", Glyph::Image, FileColor::Staged),
        ("file.gif", Glyph::Image, FileColor::Staged),
        ("file.webp", Glyph::Image, FileColor::Staged),
        ("file.svg", Glyph::Image, FileColor::Staged),
        ("file.pdf", Glyph::FileText, FileColor::Error),
        ("file.zip", Glyph::FileArchive, FileColor::Warning),
        ("file.tar.JSON", Glyph::FileJson, FileColor::Neutral),
        (".json", Glyph::File, FileColor::Neutral),
        ("file.", Glyph::File, FileColor::Neutral),
        ("unknown.ext", Glyph::File, FileColor::Neutral),
        ("", Glyph::File, FileColor::Neutral),
        ("한글.RS", Glyph::Cog, FileColor::Staged),
    ] {
        assert_eq!(file(name), (glyph, color), "{name}");
    }
    let context = egui::Context::default();
    let mut icons = Icons::new().unwrap();
    assert_eq!(icons.trees.len(), SOURCES.len());
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::splat(SCREEN_SIZE),
            )),
            ..Default::default()
        },
        |ui| {
            for (glyph, _) in SOURCES {
                for size in SIZES {
                    icons
                        .paint(
                            ui,
                            Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(size)),
                            *glyph,
                            Color32::RED,
                            0.0,
                        )
                        .unwrap();
                }
            }
        },
    );
    assert_eq!(icons.textures.len(), SOURCES.len() * SIZES.len());
    let initial_ids = icons
        .textures
        .values()
        .map(TextureHandle::id)
        .collect::<Vec<_>>();
    for (key, texture) in &icons.textures {
        assert_eq!(texture.size(), [key.1 as usize; 2]);
        let updates = &output.textures_delta.set[&texture.id()];
        assert!(updates.iter().any(|update| {
            match &update.image {
                egui::ImageData::Color(image) => image
                    .pixels
                    .iter()
                    .any(|pixel| pixel.a() > 0 && pixel.r() == pixel.g() && pixel.g() == pixel.b()),
            }
        }));
    }
    output.textures_delta.clear();
    let mut reused = context.run_ui(Default::default(), |ui| {
        for size in SIZES {
            icons
                .paint(
                    ui,
                    Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(size)),
                    Glyph::CircleX,
                    Color32::BLUE,
                    0.0,
                )
                .unwrap();
        }
    });
    assert!(
        reused
            .textures_delta
            .set
            .keys()
            .all(|id| !initial_ids.contains(id))
    );
    assert_eq!(icons.textures.len(), SOURCES.len() * SIZES.len());
    reused.textures_delta.clear();
    context.set_pixels_per_point(RETINA_SCALE);
    let mut retina = context.run_ui(Default::default(), |ui| {
        icons
            .paint(
                ui,
                Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(SIZES[1])),
                Glyph::Cog,
                Color32::GREEN,
                0.0,
            )
            .unwrap();
    });
    assert!(
        initial_ids
            .iter()
            .all(|id| retina.textures_delta.free.contains(id))
    );
    assert_eq!(icons.textures.len(), 1);
    let texture = icons.textures.values().next().unwrap();
    assert_eq!(texture.size(), [(SIZES[1] * RETINA_SCALE) as usize; 2]);
    let retina_id = texture.id();
    retina.textures_delta.clear();
    drop(icons);
    let mut dropped = context.run_ui(Default::default(), |_| {});
    assert!(dropped.textures_delta.free.contains(&retina_id));
    dropped.textures_delta.clear();
}
