use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use resvg::usvg::fontdb::{Database, Family, ID, Query, Source, Weight};
use taide_model::error::{AppError, AppResult};
use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;

use crate::editor_fonts::Families;

const TERMINAL_FAMILY: &str = "taide-terminal";
const FONT_BYTES: usize = 64 * 1024 * 1024;
const TOTAL_FONT_BYTES: usize = 128 * 1024 * 1024;
const FAMILY_BYTES: usize = 512;
pub(crate) const FONT_FALLBACKS: [&str; 3] = ["SFMono-Regular", "Menlo", "Apple SD Gothic Neo"];

pub(crate) fn family() -> FontFamily {
    FontFamily::Name(TERMINAL_FAMILY.into())
}

pub(crate) struct Prepared {
    pub definitions: FontDefinitions,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Requested {
    Default,
    Named(String),
    Invalid,
}

pub(crate) fn requested(family: Option<&str>) -> Requested {
    match family.filter(|name| !name.is_empty()) {
        None => Requested::Default,
        Some(name) if name.len() > FAMILY_BYTES || name.chars().any(char::is_control) => {
            Requested::Invalid
        }
        Some(name) => Requested::Named(name.into()),
    }
}

struct Pending {
    families: Families,
    cancelled: bool,
    reply: oneshot::Receiver<AppResult<Prepared>>,
}

pub(crate) struct Loader {
    attempted: Option<Families>,
    pending: Option<Pending>,
}

impl Loader {
    pub fn new(families: Families) -> Self {
        Self {
            attempted: Some(families),
            pending: None,
        }
    }

    pub fn update(
        &mut self,
        families: Families,
        context: &egui::Context,
        tasks: &TaskSupervisor,
    ) -> Option<AppResult<Vec<String>>> {
        let mut completed = None;
        if let Some(pending) = &mut self.pending {
            let result = match pending.reply.try_recv() {
                Ok(result) => Some(result),
                Err(oneshot::error::TryRecvError::Empty) => None,
                Err(oneshot::error::TryRecvError::Closed) => Some(Err(AppError::Internal(
                    "native terminal font worker stopped".into(),
                ))),
            };
            if let Some(result) = result {
                if !pending.cancelled && pending.families == families {
                    completed = Some(result.map(|prepared| {
                        context.set_fonts(prepared.definitions);
                        context.request_repaint();
                        prepared.warnings
                    }));
                }
                self.pending = None;
            }
        }
        if self.pending.is_some() || self.attempted.as_ref() == Some(&families) {
            return completed;
        }
        self.attempted = Some(families.clone());
        let (sender, reply) = oneshot::channel();
        let requested = families.clone();
        let repaint = context.clone();
        let supervisor = tasks.clone();
        if !tasks.spawn_transient("native-terminal-font-request", async move {
            let result = supervisor
                .run_blocking_result("native-terminal-font-load", move || load(&requested))
                .await;
            drop(sender.send(result));
            repaint.request_repaint();
        }) {
            return Some(Err(AppError::Forbidden(
                "native terminal font worker is stopping".into(),
            )));
        }
        self.pending = Some(Pending {
            families,
            reply,
            cancelled: false,
        });
        completed
    }

    pub fn cancel(&mut self) {
        self.attempted = None;
        if let Some(pending) = &mut self.pending {
            pending.cancelled = true;
            if !matches!(
                pending.reply.try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            ) {
                self.pending = None;
            }
        }
    }
}

pub(crate) fn load(families: &Families) -> AppResult<Prepared> {
    prepare_requested(&crate::system_fonts::database(), families)
}

pub(crate) fn named(database: &Database, name: &str, weight: Weight) -> Option<ID> {
    if let Some(id) = database.query(&Query {
        families: &[Family::Name(name)],
        weight,
        ..Default::default()
    }) {
        return Some(id);
    }
    let face = database.faces().find(|face| {
        face.families
            .iter()
            .any(|family| family.0.eq_ignore_ascii_case(name))
    });
    if let Some(face) = face {
        return database.query(&Query {
            families: &[Family::Name(&face.families.first()?.0)],
            weight,
            ..Default::default()
        });
    }
    let face = database
        .faces()
        .find(|face| face.post_script_name.eq_ignore_ascii_case(name))?;
    if weight == Weight::NORMAL {
        return Some(face.id);
    }
    database.query(&Query {
        families: &[Family::Name(&face.families.first()?.0)],
        weight,
        ..Default::default()
    })
}

pub(crate) fn ui_monospace(database: &Database) -> Option<ID> {
    #[cfg(target_os = "macos")]
    const NAMES: [&str; 3] = ["SFMono-Regular", "SF Mono", "Menlo"];
    #[cfg(target_os = "windows")]
    const NAMES: [&str; 1] = ["Consolas"];
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    const NAMES: [&str; 2] = ["DejaVu Sans Mono", "Liberation Mono"];
    NAMES
        .iter()
        .find_map(|name| named(database, name, Weight::NORMAL))
}

pub(crate) fn prepare_requested(database: &Database, requested: &Families) -> AppResult<Prepared> {
    let mut warnings = Vec::new();
    let mut selected = Vec::new();
    match &requested.terminal {
        Requested::Default => {}
        Requested::Invalid => {
            warnings.push("native terminal font family is invalid; using fallback fonts".into())
        }
        Requested::Named(name) => {
            if let Some(id) = named(database, name, Weight::NORMAL) {
                selected.push(id);
            } else {
                warnings.push(
                    "native terminal requested font is unavailable; using fallback fonts".into(),
                );
            }
        }
    }
    selected.extend(ui_monospace(database));
    selected.extend(
        FONT_FALLBACKS
            .iter()
            .filter_map(|name| named(database, name, Weight::NORMAL)),
    );
    selected.extend(database.query(&Query {
        families: &[Family::Monospace],
        ..Default::default()
    }));
    let mut definitions = FontDefinitions::default();
    let mut chain = Vec::new();
    let mut loaded = Vec::new();
    let mut faces = Vec::new();
    let mut remaining = TOTAL_FONT_BYTES;
    for id in selected {
        if loaded.contains(&id) {
            continue;
        }
        loaded.push(id);
        match face_data(database, id, remaining) {
            Ok(data) => {
                let bytes = match &data.font {
                    std::borrow::Cow::Owned(bytes) => bytes.capacity(),
                    std::borrow::Cow::Borrowed(bytes) => bytes.len(),
                };
                remaining = remaining
                    .checked_sub(bytes)
                    .ok_or_else(|| invalid("native terminal fonts exceed their byte budget"))?;
                let name = format!("{TERMINAL_FAMILY}/{}", chain.len());
                definitions.font_data.insert(name.clone(), Arc::new(data));
                faces.push((id, name.clone()));
                chain.push(name);
            }
            Err(_) => warnings
                .push("native terminal system font was refused; using fallback fonts".into()),
        }
    }
    chain.extend(
        definitions
            .families
            .get(&FontFamily::Monospace)
            .cloned()
            .unwrap_or_default(),
    );
    definitions.families.insert(family(), chain);
    warnings.extend(crate::ui_fonts::prepare(
        database,
        &mut definitions,
        &mut remaining,
    ));
    warnings.extend(crate::editor_fonts::prepare(
        database,
        &mut definitions,
        &mut remaining,
        &requested.editor,
        &faces,
    ));
    Ok(Prepared {
        definitions,
        warnings,
    })
}

fn invalid(message: &str) -> AppError {
    AppError::InvalidArgument(message.into())
}

pub(crate) fn face_data(database: &Database, id: ID, remaining: usize) -> AppResult<FontData> {
    let (source, index) = database
        .face_source(id)
        .ok_or_else(|| invalid("native terminal font face is unavailable"))?;
    let limit = remaining.min(FONT_BYTES);
    let bytes = match source {
        Source::File(path) => {
            #[cfg(unix)]
            let mut file = {
                use rustix::fs::{Mode, OFlags};
                File::from(
                    rustix::fs::open(
                        &path,
                        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(std::io::Error::from)?,
                )
            };
            #[cfg(not(unix))]
            let mut file = File::open(path)?;
            let metadata = file.metadata()?;
            let size = usize::try_from(metadata.len())
                .map_err(|_| invalid("native terminal font size is invalid"))?;
            if !metadata.is_file() || size == 0 || size > limit {
                return Err(invalid("native terminal font exceeds its byte budget"));
            }
            let mut bytes = vec![0; size];
            file.read_exact(&mut bytes)?;
            let mut extra = [0];
            if file.read(&mut extra)? != 0 {
                return Err(invalid("native terminal font changed during read"));
            }
            bytes
        }
        Source::Binary(data) => {
            let data = data.as_ref().as_ref();
            if data.is_empty() || data.len() > limit {
                return Err(invalid("native terminal font exceeds its byte budget"));
            }
            data.to_vec()
        }
    };
    if bytes.capacity() > limit {
        return Err(invalid(
            "native terminal font allocation exceeds its byte budget",
        ));
    }
    skrifa::FontRef::from_index(&bytes, index)
        .map_err(|_| invalid("native terminal font data is invalid"))?;
    let mut data = FontData::from_owned(bytes);
    data.index = index;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use skrifa::MetadataProvider;

    const FONT_SIZE: f32 = 13.0;
    const TTC_HEADER_BYTES: usize = 20;
    const TTF_HEADER_BYTES: usize = 12;
    const TABLE_RECORD_BYTES: usize = 16;
    const TABLE_OFFSET_BYTES: usize = 8;
    const COUNT_BYTES: usize = 4;

    fn families(family: Option<&str>) -> Families {
        Families {
            terminal: requested(family),
            editor: Requested::Default,
        }
    }

    fn prepare(database: &Database, family: Option<&str>) -> AppResult<Prepared> {
        prepare_requested(database, &families(family))
    }

    fn terminal_warnings(warnings: &[String]) -> Vec<&str> {
        warnings
            .iter()
            .map(String::as_str)
            .filter(|warning| *warning != "native system UI font is unavailable; using font stack")
            .collect()
    }

    fn collection(font: &[u8]) -> Vec<u8> {
        let mut bytes = b"ttcf\x00\x01\x00\x00\x00\x00\x00\x02".to_vec();
        bytes.extend_from_slice(&(TTC_HEADER_BYTES as u32).to_be_bytes());
        bytes.extend_from_slice(&(TTC_HEADER_BYTES as u32).to_be_bytes());
        bytes.extend_from_slice(font);
        let tables = u16::from_be_bytes([font[COUNT_BYTES], font[COUNT_BYTES + 1]]);
        for table in 0..usize::from(tables) {
            let offset = TTC_HEADER_BYTES
                + TTF_HEADER_BYTES
                + table * TABLE_RECORD_BYTES
                + TABLE_OFFSET_BYTES;
            let value = u32::from_be_bytes(bytes[offset..offset + COUNT_BYTES].try_into().unwrap());
            bytes[offset..offset + COUNT_BYTES]
                .copy_from_slice(&(value + TTC_HEADER_BYTES as u32).to_be_bytes());
        }
        bytes
    }

    fn frame(context: &egui::Context) {
        let mut output = context.run_ui(Default::default(), |ui| {
            let font = egui::FontId::new(FONT_SIZE, family());
            ui.fonts(|fonts| {
                let definitions = fonts.definitions();
                let data = &definitions.font_data[&definitions.families[&family()][0]];
                let face = skrifa::FontRef::from_index(&data.font, data.index).unwrap();
                assert!(
                    "Mtest"
                        .chars()
                        .all(|glyph| face.charmap().map(glyph).is_some())
                );
            });
            let galley = ui.fonts_mut(|fonts| {
                fonts.layout_no_wrap("Mtest".into(), font, egui::Color32::WHITE)
            });
            assert!(
                galley
                    .rows
                    .iter()
                    .any(|row| !row.row.visuals.mesh.indices.is_empty())
            );
            ui.painter()
                .galley(ui.cursor().min, galley, egui::Color32::WHITE);
        });
        output.textures_delta.clear();
    }

    #[tokio::test]
    async fn terminal_fonts는_합성_binary_file_ttc_상한_fallback과_오래된_reply를_보존한다() {
        let defaults = FontDefinitions::default();
        let builtin = &defaults.font_data[&defaults.families[&FontFamily::Monospace][0]];
        let font_bytes = builtin.font.to_vec();
        let mut database = Database::new();
        database.load_font_data(font_bytes.clone());
        let face = database.faces().next().unwrap().clone();
        let name = face.families[0].0.clone();
        assert_eq!(
            named(&database, &name.to_lowercase(), Weight::NORMAL),
            Some(face.id)
        );
        assert_eq!(
            named(&database, &face.post_script_name, Weight::NORMAL),
            Some(face.id)
        );
        assert!(face_data(&database, face.id, font_bytes.len() - 1).is_err());
        let prepared = prepare(&database, Some(&name)).unwrap();
        assert!(terminal_warnings(&prepared.warnings).is_empty());
        assert_eq!(
            prepared.definitions.families[&FontFamily::Monospace],
            defaults.families[&FontFamily::Monospace]
        );
        assert_eq!(
            prepared.definitions.families[&FontFamily::Proportional],
            defaults.families[&FontFamily::Proportional]
        );
        assert_eq!(prepared.definitions.font_data["taide-terminal/0"].index, 0);
        let context = egui::Context::default();
        context.set_fonts(prepared.definitions);
        frame(&context);

        let mut ttc = Database::new();
        ttc.load_font_data(collection(&font_bytes));
        assert_eq!(ttc.faces().count(), 2);
        let mut second = ttc.faces().find(|face| face.index == 1).unwrap().clone();
        second.families[0].0 = "Synthetic TTC".into();
        let second_id = ttc.push_face_info(second.clone());
        let prepared = prepare(&ttc, Some("Synthetic TTC")).unwrap();
        assert!(terminal_warnings(&prepared.warnings).is_empty());
        assert_eq!(prepared.definitions.font_data["taide-terminal/0"].index, 1);
        context.set_fonts(prepared.definitions);
        frame(&context);
        let mut invalid_index = second.clone();
        invalid_index.index = u32::MAX;
        let id = ttc.push_face_info(invalid_index);
        assert!(face_data(&ttc, id, TOTAL_FONT_BYTES).is_err());
        let mut invalid_bytes = second;
        invalid_bytes.index = 0;
        invalid_bytes.source = Source::Binary(Arc::new(vec![0]));
        let id = ttc.push_face_info(invalid_bytes);
        assert!(face_data(&ttc, id, TOTAL_FONT_BYTES).is_err());
        assert_eq!(
            face_data(&ttc, second_id, TOTAL_FONT_BYTES).unwrap().index,
            1
        );

        let path = std::env::temp_dir().join(format!(
            "taide-native-font-{}.ttf",
            taide_model::ids::ProjectId::new()
        ));
        std::fs::write(&path, &font_bytes).unwrap();
        let mut file_face = face.clone();
        file_face.source = Source::File(path.clone());
        let id = database.push_face_info(file_face);
        assert_eq!(
            face_data(&database, id, font_bytes.len())
                .unwrap()
                .font
                .as_ref(),
            font_bytes
        );
        assert!(face_data(&database, id, font_bytes.len() - 1).is_err());
        std::fs::write(&path, [0]).unwrap();
        assert!(face_data(&database, id, TOTAL_FONT_BYTES).is_err());
        std::fs::remove_file(path).unwrap();

        let empty = Database::new();
        let missing = prepare(&empty, Some("Synthetic missing font")).unwrap();
        assert_eq!(terminal_warnings(&missing.warnings).len(), 1);
        assert_eq!(
            missing.definitions.families[&family()],
            defaults.families[&FontFamily::Monospace]
        );
        assert_eq!(
            terminal_warnings(
                &prepare(&empty, Some(&"x".repeat(FAMILY_BYTES + 1)))
                    .unwrap()
                    .warnings
            )
            .len(),
            1
        );
        assert_eq!(
            terminal_warnings(&prepare(&empty, Some("invalid\nfont")).unwrap().warnings).len(),
            1
        );

        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let mut loader = Loader::new(families(Some(&name)));
        let (sender, reply) = oneshot::channel();
        loader.pending = Some(Pending {
            families: families(Some(&name)),
            reply,
            cancelled: false,
        });
        assert!(sender.send(prepare(&database, Some(&name))).is_ok());
        assert!(
            terminal_warnings(
                &loader
                    .update(families(Some(&name)), &context, &tasks)
                    .unwrap()
                    .unwrap()
            )
            .is_empty()
        );
        frame(&context);
        assert_eq!(
            context.fonts(|fonts| fonts.definitions().font_data["taide-terminal/0"].index),
            0
        );
        let (sender, reply) = oneshot::channel();
        loader.pending = Some(Pending {
            families: families(Some(&name)),
            reply,
            cancelled: false,
        });
        loader.cancel();
        assert!(loader.pending.as_ref().unwrap().cancelled);
        tasks.stop_all();
        assert!(sender.send(prepare(&ttc, Some("Synthetic TTC"))).is_ok());
        assert!(
            loader
                .update(families(Some(&name)), &context, &tasks)
                .unwrap()
                .is_err()
        );
        frame(&context);
        assert_eq!(
            context.fonts(|fonts| fonts.definitions().font_data["taide-terminal/0"].index),
            0
        );
        assert!(loader.pending.is_none());
        assert!(
            loader
                .update(families(Some(&name)), &context, &tasks)
                .is_none()
        );
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
    }
}
