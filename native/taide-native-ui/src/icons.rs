use egui::{self, Color32, ColorImage, Image, TextureHandle};
use resvg::{tiny_skia, usvg};
use taide_model::error::{AppError, AppResult};

#[cfg(test)]
#[path = "glyph-icons-tests.rs"]
mod glyph_tests;
#[path = "glyph-icons.rs"]
pub mod glyphs;

const ICON_VIEWBOX: f32 = 24.0;
const SMALL_ICON: f32 = 12.0;
const KEYBOARD_ICON: f32 = 14.0;
const CLOSE_ICON: f32 = 16.0;
const LIST_ITEM_ICON: f32 = 16.0;
const MAX_RASTER_SIDE: f32 = 1024.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Reset,
    Unbind,
    Warning,
    Keyboard,
    Close,
    ThemeReset,
    Copy,
    Pencil,
    FileJson,
    Trash,
    Plus,
    FolderOpen,
    Search,
    Terminal,
    File,
    CornerDownLeft,
    Loader,
    #[cfg(feature = "native-host")]
    Braces,
    #[cfg(feature = "native-host")]
    Hash,
}

const ICONS: &[Icon] = &[
    Icon::Reset,
    Icon::Unbind,
    Icon::Warning,
    Icon::Keyboard,
    Icon::Close,
    Icon::ThemeReset,
    Icon::Copy,
    Icon::Pencil,
    Icon::FileJson,
    Icon::Trash,
    Icon::Plus,
    Icon::FolderOpen,
    Icon::Search,
    Icon::Terminal,
    Icon::File,
    Icon::CornerDownLeft,
    Icon::Loader,
    #[cfg(feature = "native-host")]
    Icon::Braces,
    #[cfg(feature = "native-host")]
    Icon::Hash,
];

impl Icon {
    fn index(self) -> usize {
        match self {
            Self::Reset => 0,
            Self::Unbind => 1,
            Self::Warning => 2,
            Self::Keyboard => 3,
            Self::Close => 4,
            Self::ThemeReset => 5,
            Self::Copy => 6,
            Self::Pencil => 7,
            Self::FileJson => 8,
            Self::Trash => 9,
            Self::Plus => 10,
            Self::FolderOpen => 11,
            Self::Search => 12,
            Self::Terminal => 13,
            Self::File => 14,
            Self::CornerDownLeft => 15,
            Self::Loader => 16,
            #[cfg(feature = "native-host")]
            Self::Braces => 17,
            #[cfg(feature = "native-host")]
            Self::Hash => 18,
        }
    }

    pub fn size(self) -> f32 {
        match self {
            #[cfg(feature = "native-host")]
            Self::Braces | Self::Hash => LIST_ITEM_ICON,
            Self::Reset | Self::Unbind | Self::Warning | Self::Loader => SMALL_ICON,
            Self::Search | Self::Terminal | Self::File | Self::CornerDownLeft => LIST_ITEM_ICON,
            Self::Keyboard
            | Self::ThemeReset
            | Self::Copy
            | Self::Pencil
            | Self::FileJson
            | Self::Trash
            | Self::Plus
            | Self::FolderOpen => KEYBOARD_ICON,
            Self::Close => CLOSE_ICON,
        }
    }

    fn source(self) -> &'static [u8] {
        match self {
            Self::Reset | Self::ThemeReset => {
                include_bytes!("../../taide-native-app/resources/keybindings/rotate-ccw.svg")
            }
            Self::Unbind => {
                include_bytes!("../../taide-native-app/resources/keybindings/unlink.svg")
            }
            Self::Warning => {
                include_bytes!("../../taide-native-app/resources/keybindings/triangle-alert.svg")
            }
            Self::Keyboard => {
                include_bytes!("../../taide-native-app/resources/keybindings/keyboard.svg")
            }
            Self::Close => include_bytes!("../../taide-native-app/resources/keybindings/x.svg"),
            Self::Copy => include_bytes!("../../taide-native-app/resources/themes/copy.svg"),
            Self::Pencil => include_bytes!("../../taide-native-app/resources/themes/pencil.svg"),
            Self::FileJson => {
                include_bytes!("../../taide-native-app/resources/themes/file-json.svg")
            }
            Self::Trash => include_bytes!("../../taide-native-app/resources/snippets/trash-2.svg"),
            Self::Plus => include_bytes!("../../taide-native-app/resources/snippets/plus.svg"),
            Self::FolderOpen => {
                include_bytes!("../../taide-native-app/resources/snippets/folder-open.svg")
            }
            Self::Search => include_bytes!("../../taide-native-app/resources/icons/search.svg"),
            Self::Terminal => {
                include_bytes!("../../taide-native-app/resources/problems/terminal.svg")
            }
            Self::File => include_bytes!("../../taide-native-app/resources/problems/file.svg"),
            Self::CornerDownLeft => {
                include_bytes!("../../taide-native-app/resources/icons/corner-down-left.svg")
            }
            Self::Loader => {
                include_bytes!("../../taide-native-app/resources/icons/loader-circle.svg")
            }
            #[cfg(feature = "native-host")]
            Self::Braces => include_bytes!("../../taide-native-app/resources/icons/braces.svg"),
            #[cfg(feature = "native-host")]
            Self::Hash => include_bytes!("../../taide-native-app/resources/icons/hash.svg"),
        }
    }
}

pub struct Icons {
    trees: Vec<usvg::Tree>,
    textures: Vec<TextureHandle>,
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
            trees: ICONS
                .iter()
                .map(|icon| {
                    usvg::Tree::from_data(icon.source(), &options).map_err(|error| {
                        AppError::Internal(format!("native keybinding icon: {error}"))
                    })
                })
                .collect::<AppResult<_>>()?,
            textures: Vec::new(),
            scale: None,
        })
    }

    pub fn prepare(&mut self, context: &egui::Context) -> AppResult<()> {
        let scale = context.pixels_per_point();
        if self.scale == Some(scale) {
            return Ok(());
        }
        let textures = ICONS
            .iter()
            .map(|icon| {
                let pixels = (icon.size() * scale).ceil();
                if !pixels.is_finite() || !(1.0..=MAX_RASTER_SIDE).contains(&pixels) {
                    return Err(AppError::Internal(
                        "native keybinding icon scale is out of range".into(),
                    ));
                }
                let side = pixels as u32;
                let mut pixmap = tiny_skia::Pixmap::new(side, side).ok_or_else(|| {
                    AppError::Internal("native keybinding icon allocation failed".into())
                })?;
                let transform = tiny_skia::Transform::from_scale(
                    icon.size() * scale / ICON_VIEWBOX,
                    icon.size() * scale / ICON_VIEWBOX,
                );
                resvg::render(&self.trees[icon.index()], transform, &mut pixmap.as_mut());
                Ok(context.load_texture(
                    format!("native-keybinding-icon-{}", icon.index()),
                    ColorImage::from_rgba_unmultiplied(
                        [side as usize, side as usize],
                        &pixmap.take_demultiplied(),
                    ),
                    egui::TextureOptions::LINEAR,
                ))
            })
            .collect::<AppResult<Vec<_>>>()?;
        self.textures = textures;
        self.scale = Some(scale);
        Ok(())
    }

    pub fn image(&self, icon: Icon, color: Color32) -> Option<Image<'static>> {
        self.textures.get(icon.index()).map(|texture| {
            Image::new((texture.id(), egui::Vec2::splat(icon.size())))
                .fit_to_exact_size(egui::Vec2::splat(icon.size()))
                .tint(color)
        })
    }

    #[cfg(any(test, feature = "inspection"))]
    pub fn texture_id(&self, icon: Icon) -> Option<egui::TextureId> {
        self.textures.get(icon.index()).map(TextureHandle::id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keybinding_icons는_원본벡터의_배율별_크기와_캐시_교체수명을_보존한다() {
        let context = egui::Context::default();
        let mut icons = Icons::new().unwrap();
        let mut previous = Vec::new();
        for scale in [1.0, 1.25, 2.0] {
            context.set_pixels_per_point(scale);
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                icons.prepare(ui.ctx()).unwrap();
            });
            let ids = icons
                .textures
                .iter()
                .map(TextureHandle::id)
                .collect::<Vec<_>>();
            assert_eq!(ids.len(), ICONS.len());
            assert_eq!(
                output
                    .textures_delta
                    .set
                    .iter()
                    .filter(|(id, _)| ids.contains(id))
                    .count(),
                ICONS.len()
            );
            for (index, icon) in ICONS.iter().enumerate() {
                let side = (icon.size() * scale).ceil() as usize;
                assert_eq!(icons.textures[index].size(), [side, side]);
                let delta = output
                    .textures_delta
                    .set
                    .get(&ids[index])
                    .unwrap()
                    .last()
                    .unwrap();
                let egui::ImageData::Color(image) = &delta.image;
                assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
                assert!(image.pixels.iter().any(|pixel| pixel.a() == 0));
                assert_eq!(
                    icons.image(*icon, Color32::WHITE).unwrap().calc_size(
                        egui::Vec2::splat(ICON_VIEWBOX),
                        Some(egui::Vec2::splat(icon.size()))
                    ),
                    egui::Vec2::splat(icon.size())
                );
            }
            for id in &previous {
                assert!(output.textures_delta.free.contains(id));
            }
            output.textures_delta.clear();
            let mut same = context.run_ui(egui::RawInput::default(), |ui| {
                icons.prepare(ui.ctx()).unwrap()
            });
            assert!(
                !same
                    .textures_delta
                    .set
                    .iter()
                    .any(|(id, _)| ids.contains(id))
            );
            assert_eq!(
                icons
                    .textures
                    .iter()
                    .map(TextureHandle::id)
                    .collect::<Vec<_>>(),
                ids
            );
            same.textures_delta.clear();
            previous = ids;
        }
        drop(icons);
        let mut output = context.run_ui(egui::RawInput::default(), |_| {});
        assert!(
            previous
                .iter()
                .all(|id| output.textures_delta.free.contains(id))
        );
        output.textures_delta.clear();
    }
}
