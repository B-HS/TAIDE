use std::borrow::Cow;
use std::sync::Arc;

use eframe::egui::{FontData, FontDefinitions, FontFamily};
use resvg::usvg::fontdb::{Database, Family, ID, Query, Weight};
use taide_model::error::{AppError, AppResult};
use taide_model::settings::Settings;
use taide_native_ui::font_families::{EDITOR_BOLD_FAMILY, EDITOR_FAMILY};

use crate::terminal_fonts::{FONT_FALLBACKS, Requested, face_data, named, requested, ui_monospace};

const WEIGHT_AXIS: &[u8; 4] = b"wght";

pub(crate) fn family() -> FontFamily {
    FontFamily::Name(EDITOR_FAMILY.into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Families {
    pub terminal: Requested,
    pub editor: Requested,
}

impl Families {
    pub(crate) fn new(settings: &Settings) -> Self {
        Self {
            terminal: requested(settings.terminal_font_family.as_deref()),
            editor: requested(settings.editor_font_family.as_deref()),
        }
    }
}

struct Face {
    id: ID,
    instance: Option<Weight>,
    name: String,
}

struct Registry<'a> {
    database: &'a Database,
    definitions: &'a mut FontDefinitions,
    remaining: &'a mut usize,
    faces: Vec<Face>,
}

impl Registry<'_> {
    fn face(&mut self, id: ID, weight: Weight, name: String) -> AppResult<String> {
        if let Some(face) = self
            .faces
            .iter()
            .find(|face| face.id == id && face.instance.is_none_or(|instance| instance == weight))
        {
            return Ok(face.name.clone());
        }
        let mut data = face_data(self.database, id, *self.remaining)?;
        let is_variable = has_weight_axis(&data);
        if is_variable {
            data.tweak.coords.push(WEIGHT_AXIS, f32::from(weight.0));
        }
        let bytes = match &data.font {
            Cow::Owned(bytes) => bytes.capacity(),
            Cow::Borrowed(bytes) => bytes.len(),
        };
        *self.remaining = self.remaining.checked_sub(bytes).ok_or_else(|| {
            AppError::InvalidArgument("native editor fonts exceed their byte budget".into())
        })?;
        self.definitions
            .font_data
            .insert(name.clone(), Arc::new(data));
        self.faces.push(Face {
            id,
            instance: is_variable.then_some(weight),
            name: name.clone(),
        });
        Ok(name)
    }
}

fn has_weight_axis(data: &FontData) -> bool {
    data.variation_axes()
        .iter()
        .any(|axis| axis.tag.into_bytes() == *WEIGHT_AXIS)
}

fn bold_face(database: &Database, regular: ID) -> ID {
    database
        .face(regular)
        .and_then(|face| {
            database.query(&Query {
                families: &[Family::Name(&face.families.first()?.0)],
                weight: Weight::BOLD,
                stretch: face.stretch,
                style: face.style,
            })
        })
        .unwrap_or(regular)
}

pub(crate) fn prepare(
    database: &Database,
    definitions: &mut FontDefinitions,
    remaining: &mut usize,
    requested: &Requested,
    shared: &[(ID, String)],
) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut selected = Vec::new();
    match requested {
        Requested::Default => {}
        Requested::Invalid => {
            warnings.push("native editor font family is invalid; using fallback fonts".into())
        }
        Requested::Named(name) => {
            if let Some(id) = named(database, name, Weight::NORMAL) {
                selected.push(id);
            } else {
                warnings.push(
                    "native editor requested font is unavailable; using fallback fonts".into(),
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
    let fallback = definitions
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    let faces = shared
        .iter()
        .filter(|(_, name)| {
            definitions
                .font_data
                .get(name)
                .is_some_and(|data| !has_weight_axis(data))
        })
        .map(|(id, name)| Face {
            id: *id,
            instance: None,
            name: name.clone(),
        })
        .collect();
    let mut registry = Registry {
        database,
        definitions,
        remaining,
        faces,
    };
    let mut attempted = Vec::new();
    let mut slots: Vec<(ID, String)> = Vec::new();
    for id in selected {
        if attempted.contains(&id) {
            continue;
        }
        attempted.push(id);
        match registry.face(
            id,
            Weight::NORMAL,
            format!("{EDITOR_FAMILY}/{}", slots.len()),
        ) {
            Ok(name) => slots.push((id, name)),
            Err(_) => {
                warnings.push("native editor system font was refused; using fallback fonts".into())
            }
        }
    }
    let mut bold: Vec<String> = Vec::new();
    for (id, regular) in &slots {
        let name = registry
            .face(
                bold_face(database, *id),
                Weight::BOLD,
                format!("{EDITOR_BOLD_FAMILY}/{}", bold.len()),
            )
            .unwrap_or_else(|_| {
                warnings.push("native editor bold font was refused; using regular weight".into());
                regular.clone()
            });
        if !bold.contains(&name) {
            bold.push(name);
        }
    }
    let regular = slots.into_iter().map(|(_, name)| name).collect();
    for (name, mut chain) in [(EDITOR_FAMILY, regular), (EDITOR_BOLD_FAMILY, bold)] {
        chain.extend(fallback.clone());
        registry
            .definitions
            .families
            .insert(FontFamily::Name(name.into()), chain);
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    use resvg::usvg::fontdb::Source;
    use taide_runtime::TaskSupervisor;

    const FONT_SIZE: f32 = 13.0;
    const USER_FAMILY: &str = "Synthetic Code";
    const GENERIC_FAMILY: &str = "Synthetic Generic";
    const STACK: [(&str, &str, Weight); 7] = [
        (USER_FAMILY, "SyntheticCode-Regular", Weight::NORMAL),
        (USER_FAMILY, "SyntheticCode-Bold", Weight::BOLD),
        ("SF Mono", "SFMono-Regular", Weight::NORMAL),
        ("Menlo", "Menlo-Regular", Weight::NORMAL),
        ("Menlo", "Menlo-Bold", Weight::BOLD),
        (
            "Apple SD Gothic Neo",
            "AppleSDGothicNeo-Regular",
            Weight::NORMAL,
        ),
        (GENERIC_FAMILY, "SyntheticGeneric-Regular", Weight::NORMAL),
    ];
    const REGULAR_FACES: [usize; 5] = [0, 2, 3, 5, 6];
    const BOLD_FACES: [usize; 5] = [1, 2, 4, 5, 6];
    const FALLBACK_FACES: [usize; 4] = [2, 3, 5, 6];
    const FALLBACK_BOLD_FACES: [usize; 4] = [2, 4, 5, 6];
    const WEIGHTLESS_SLOTS: [usize; 3] = [1, 3, 4];
    const REFUSED_BOLD_FACES: usize = 2;
    const VARIABLE_INSTANCES: usize = 2;
    const NORMAL_WEIGHT: f32 = 400.0;
    const BOLD_WEIGHT: f32 = 700.0;
    const SFNT_HEADER_BYTES: usize = 12;
    const TABLE_COUNT_OFFSET: usize = 4;
    const TABLE_COUNT_BYTES: usize = 2;
    const TABLE_RECORD_BYTES: usize = 16;
    const TABLE_OFFSET_FIELD: usize = 8;
    const TABLE_LENGTH_FIELD: usize = 12;
    const TABLE_ALIGNMENT: usize = 4;
    const FVAR_TAG: [u8; 4] = *b"fvar";
    const FVAR_HEADER: [u16; 8] = [1, 0, 16, 2, 1, 20, 0, 8];
    const FVAR_WEIGHT_RANGE: [u32; 3] = [100, 400, 900];
    const FVAR_FIXED_ONE: u32 = 1 << 16;
    const FVAR_AXIS_FLAGS: u16 = 0;
    const FVAR_AXIS_NAME_ID: u16 = 256;

    fn builtin() -> Vec<u8> {
        let defaults = FontDefinitions::default();
        defaults.font_data[&defaults.families[&FontFamily::Monospace][0]]
            .font
            .to_vec()
    }

    fn bold_family() -> FontFamily {
        FontFamily::Name(EDITOR_BOLD_FAMILY.into())
    }

    fn face(
        database: &mut Database,
        font: &[u8],
        padding: usize,
        (family, post_script, weight): (&str, &str, Weight),
    ) -> ID {
        let mut bytes = font.to_vec();
        bytes.resize(font.len() + padding, 0);
        let loaded = database.load_font_source(Source::Binary(Arc::new(bytes)));
        let mut info = database.face(loaded[0]).unwrap().clone();
        database.remove_face(loaded[0]);
        info.families[0].0 = family.into();
        info.post_script_name = post_script.into();
        info.weight = weight;
        database.push_face_info(info)
    }

    fn stack(font: &[u8]) -> Database {
        let mut database = Database::new();
        for (padding, names) in STACK.into_iter().enumerate() {
            face(&mut database, font, padding, names);
        }
        database.set_monospace_family(GENERIC_FAMILY);
        database
    }

    fn bytes(font: &[u8], paddings: &[usize]) -> usize {
        paddings.iter().map(|padding| font.len() + padding).sum()
    }

    fn chain(definitions: &FontDefinitions, family: &FontFamily) -> Vec<String> {
        let defaults = FontDefinitions::default();
        let fallback = &defaults.families[&FontFamily::Monospace];
        let names = &definitions.families[family];
        let (faces, tail) = names.split_at(names.len() - fallback.len());
        assert_eq!(tail, fallback);
        faces.to_vec()
    }

    fn paddings(definitions: &FontDefinitions, names: &[String], font: &[u8]) -> Vec<usize> {
        names
            .iter()
            .map(|name| definitions.font_data[name].font.len() - font.len())
            .collect()
    }

    fn weights(definitions: &FontDefinitions, name: &str) -> Vec<f32> {
        definitions.font_data[name]
            .tweak
            .coords
            .as_ref()
            .iter()
            .filter(|(tag, _)| tag.into_bytes() == *WEIGHT_AXIS)
            .map(|(_, value)| *value)
            .collect()
    }

    fn variable(font: &[u8]) -> Vec<u8> {
        let count_end = TABLE_COUNT_OFFSET + TABLE_COUNT_BYTES;
        let tables = u16::from_be_bytes(font[TABLE_COUNT_OFFSET..count_end].try_into().unwrap());
        let directory_end = SFNT_HEADER_BYTES + usize::from(tables) * TABLE_RECORD_BYTES;
        let mut records: Vec<Vec<u8>> = font[SFNT_HEADER_BYTES..directory_end]
            .chunks(TABLE_RECORD_BYTES)
            .map(|record| {
                let mut record = record.to_vec();
                let offset = u32::from_be_bytes(
                    record[TABLE_OFFSET_FIELD..TABLE_LENGTH_FIELD]
                        .try_into()
                        .unwrap(),
                ) + TABLE_RECORD_BYTES as u32;
                record[TABLE_OFFSET_FIELD..TABLE_LENGTH_FIELD]
                    .copy_from_slice(&offset.to_be_bytes());
                record
            })
            .collect();
        let mut fvar: Vec<u8> = FVAR_HEADER
            .into_iter()
            .flat_map(u16::to_be_bytes)
            .chain(*WEIGHT_AXIS)
            .collect();
        fvar.extend(
            FVAR_WEIGHT_RANGE
                .into_iter()
                .flat_map(|value| (value * FVAR_FIXED_ONE).to_be_bytes()),
        );
        fvar.extend(FVAR_AXIS_FLAGS.to_be_bytes());
        fvar.extend(FVAR_AXIS_NAME_ID.to_be_bytes());
        let fvar_offset = (font.len() + TABLE_RECORD_BYTES).next_multiple_of(TABLE_ALIGNMENT);
        let mut fvar_record = FVAR_TAG.to_vec();
        fvar_record.extend(0u32.to_be_bytes());
        fvar_record.extend((fvar_offset as u32).to_be_bytes());
        fvar_record.extend((fvar.len() as u32).to_be_bytes());
        let position = records.partition_point(|record| record[..FVAR_TAG.len()] < FVAR_TAG[..]);
        records.insert(position, fvar_record);
        let mut bytes = font[..TABLE_COUNT_OFFSET].to_vec();
        bytes.extend((tables + 1).to_be_bytes());
        bytes.extend(&font[count_end..SFNT_HEADER_BYTES]);
        bytes.extend(records.concat());
        bytes.extend(&font[directory_end..]);
        bytes.resize(fvar_offset, 0);
        bytes.extend(fvar);
        bytes
    }

    fn render(definitions: FontDefinitions) {
        let context = egui::Context::default();
        context.set_fonts(definitions);
        let mut output = context.run_ui(Default::default(), |ui| {
            for shown in [family(), bold_family()] {
                let galley = ui.fonts_mut(|fonts| {
                    fonts.layout_no_wrap(
                        "Mtest".into(),
                        egui::FontId::new(FONT_SIZE, shown),
                        egui::Color32::WHITE,
                    )
                });
                assert!(
                    galley
                        .rows
                        .iter()
                        .any(|row| !row.row.visuals.mesh.indices.is_empty())
                );
            }
        });
        output.textures_delta.clear();
    }

    #[test]
    fn editor_fonts는_사용자_글꼴과_ts_fallback_순서로_체인을_만들고_굵은_face를_별도_패밀리에_등록한다()
     {
        let font = builtin();
        let database = stack(&font);
        let defaults = FontDefinitions::default();
        let mut definitions = defaults.clone();
        let every_face: Vec<usize> = (0..STACK.len()).collect();
        let mut remaining = bytes(&font, &every_face);
        let warnings = prepare(
            &database,
            &mut definitions,
            &mut remaining,
            &requested(Some(USER_FAMILY)),
            &[],
        );
        assert!(warnings.is_empty());
        assert_eq!(remaining, 0);
        let regular = chain(&definitions, &family());
        let bold = chain(&definitions, &bold_family());
        assert_eq!(paddings(&definitions, &regular, &font), REGULAR_FACES);
        assert_eq!(paddings(&definitions, &bold, &font), BOLD_FACES);
        for slot in WEIGHTLESS_SLOTS {
            assert_eq!(regular[slot], bold[slot]);
        }
        assert_eq!(
            definitions.font_data.len(),
            defaults.font_data.len() + STACK.len()
        );
        for builtin_family in [FontFamily::Monospace, FontFamily::Proportional] {
            assert_eq!(
                definitions.families[&builtin_family],
                defaults.families[&builtin_family]
            );
        }
        render(definitions);
    }

    #[test]
    fn editor_fonts는_없는_글꼴과_예산_부족을_경고하고_남은_체인으로_대체한다() {
        let font = builtin();
        let database = stack(&font);
        let defaults = FontDefinitions::default();
        let every_face: Vec<usize> = (0..STACK.len()).collect();
        for (name, warned) in [
            (Some("Synthetic missing font"), 1),
            (Some("invalid\nfont"), 1),
            (None, 0),
        ] {
            let mut definitions = defaults.clone();
            let mut remaining = bytes(&font, &every_face);
            let warnings = prepare(
                &database,
                &mut definitions,
                &mut remaining,
                &requested(name),
                &[],
            );
            assert_eq!(warnings.len(), warned);
            let regular = chain(&definitions, &family());
            let bold = chain(&definitions, &bold_family());
            assert_eq!(paddings(&definitions, &regular, &font), FALLBACK_FACES);
            assert_eq!(paddings(&definitions, &bold, &font), FALLBACK_BOLD_FACES);
        }
        let mut definitions = defaults.clone();
        let mut remaining = bytes(&font, &REGULAR_FACES);
        let warnings = prepare(
            &database,
            &mut definitions,
            &mut remaining,
            &requested(Some(USER_FAMILY)),
            &[],
        );
        assert_eq!(warnings.len(), REFUSED_BOLD_FACES);
        assert_eq!(remaining, 0);
        let regular = chain(&definitions, &family());
        assert_eq!(paddings(&definitions, &regular, &font), REGULAR_FACES);
        assert_eq!(chain(&definitions, &bold_family()), regular);
        render(definitions);
        let mut definitions = defaults.clone();
        let mut remaining = 0;
        let warnings = prepare(
            &database,
            &mut definitions,
            &mut remaining,
            &requested(Some(USER_FAMILY)),
            &[],
        );
        assert_eq!(warnings.len(), REGULAR_FACES.len());
        assert!(chain(&definitions, &family()).is_empty());
        assert!(chain(&definitions, &bold_family()).is_empty());
        assert_eq!(definitions.font_data, defaults.font_data);
        render(definitions);
    }

    #[test]
    fn editor_fonts는_가변_글꼴의_굵기를_wght_축으로_고정해_굵기별_face로_등록한다() {
        let font = variable(&builtin());
        let mut database = Database::new();
        face(&mut database, &font, 0, STACK[0]);
        let defaults = FontDefinitions::default();
        let mut definitions = defaults.clone();
        let mut remaining = font.len() * VARIABLE_INSTANCES;
        let warnings = prepare(
            &database,
            &mut definitions,
            &mut remaining,
            &requested(Some(USER_FAMILY)),
            &[],
        );
        assert!(warnings.is_empty());
        assert_eq!(remaining, 0);
        let regular = chain(&definitions, &family());
        let bold = chain(&definitions, &bold_family());
        assert_eq!(regular.len(), 1);
        assert_eq!(bold.len(), 1);
        assert_ne!(regular, bold);
        assert_eq!(weights(&definitions, &regular[0]), [NORMAL_WEIGHT]);
        assert_eq!(weights(&definitions, &bold[0]), [BOLD_WEIGHT]);
        assert_eq!(
            definitions.font_data.len(),
            defaults.font_data.len() + VARIABLE_INSTANCES
        );
        render(definitions);
    }

    #[test]
    fn editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다() {
        let font = builtin();
        let defaults = FontDefinitions::default();
        let definitions = crate::terminal_fonts::prepare_requested(
            &stack(&font),
            &Families {
                terminal: Requested::Default,
                editor: requested(Some(USER_FAMILY)),
            },
        )
        .unwrap()
        .definitions;
        let terminal = chain(&definitions, &crate::terminal_fonts::family());
        let regular = chain(&definitions, &family());
        assert_eq!(paddings(&definitions, &terminal, &font), FALLBACK_FACES);
        assert_eq!(paddings(&definitions, &regular, &font), REGULAR_FACES);
        assert_eq!(regular[1..], terminal[..]);
        assert_eq!(
            paddings(&definitions, &chain(&definitions, &bold_family()), &font),
            BOLD_FACES
        );
        assert_eq!(
            definitions.font_data.len(),
            defaults.font_data.len() + STACK.len()
        );
        render(definitions);
        let font = variable(&font);
        let mut database = Database::new();
        face(&mut database, &font, 0, STACK[0]);
        let definitions = crate::terminal_fonts::prepare_requested(
            &database,
            &Families {
                terminal: requested(Some(USER_FAMILY)),
                editor: requested(Some(USER_FAMILY)),
            },
        )
        .unwrap()
        .definitions;
        let terminal = chain(&definitions, &crate::terminal_fonts::family());
        let regular = chain(&definitions, &family());
        assert_eq!(terminal.len(), 1);
        assert_eq!(regular.len(), 1);
        assert_ne!(terminal, regular);
        assert!(weights(&definitions, &terminal[0]).is_empty());
        assert_eq!(weights(&definitions, &regular[0]), [NORMAL_WEIGHT]);
        assert_eq!(
            definitions.font_data.len(),
            defaults.font_data.len() + terminal.len() + VARIABLE_INSTANCES
        );
        render(definitions);
    }

    #[tokio::test]
    async fn editor_fonts는_편집기_글꼴_설정이_바뀌면_같은_로더에_다시_적재를_요청한다() {
        let mut settings = Settings::default();
        let initial = Families::new(&settings);
        assert_eq!(
            initial,
            Families {
                terminal: Requested::Default,
                editor: Requested::Default,
            }
        );
        settings.editor_font_family = Some(USER_FAMILY.into());
        let changed = Families::new(&settings);
        assert_eq!(changed.terminal, initial.terminal);
        assert_eq!(changed.editor, Requested::Named(USER_FAMILY.into()));
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        tasks.stop_all();
        let context = egui::Context::default();
        let mut loader = crate::terminal_fonts::Loader::new(initial.clone());
        assert!(loader.update(initial, &context, &tasks).is_none());
        assert!(
            loader
                .update(changed.clone(), &context, &tasks)
                .unwrap()
                .is_err()
        );
        assert!(loader.update(changed, &context, &tasks).is_none());
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
    }
}
