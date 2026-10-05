use std::sync::Arc;

use eframe::egui::{FontData, FontDefinitions, FontFamily};
use resvg::usvg::fontdb::{Database, Family, Query, Weight};
#[cfg(target_os = "macos")]
use taide_model::error::{AppError, AppResult};

pub(crate) use taide_native_ui::font_families::medium;
use taide_native_ui::font_families::{MEDIUM_FAMILY, SEMIBOLD_FAMILY};
const MEDIUM_WEIGHT: u16 = 500;
const UI_FONT_STACK: [&str; 5] = [
    "Segoe UI",
    "Roboto",
    "Helvetica Neue",
    "Arial",
    "Noto Sans KR",
];
#[cfg(target_os = "macos")]
const SYSTEM_FONT_SIZE: f64 = 12.0;
#[cfg(target_os = "macos")]
const FONT_HEAD_BYTES: usize = 54;

pub(crate) fn prepare(
    database: &Database,
    definitions: &mut FontDefinitions,
    remaining: &mut usize,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let fallback = definitions.families[&FontFamily::Proportional].clone();
    for (weight, family) in [
        (Weight::NORMAL, FontFamily::Proportional),
        (
            Weight(MEDIUM_WEIGHT),
            FontFamily::Name(MEDIUM_FAMILY.into()),
        ),
        (Weight::SEMIBOLD, FontFamily::Name(SEMIBOLD_FAMILY.into())),
    ] {
        #[cfg(target_os = "macos")]
        let mut system = match system_face(database, weight, *remaining) {
            Ok(data) => Some(data),
            Err(_) => {
                warnings.push("native system UI font is unavailable; using font stack".into());
                None
            }
        };
        #[cfg(not(target_os = "macos"))]
        let mut system = None::<FontData>;
        let mut selected = Vec::new();
        for name in UI_FONT_STACK {
            if let Some(id) = crate::terminal_fonts::named(database, name, weight)
                && !selected.contains(&id)
            {
                selected.push(id);
            }
        }
        if let Some(id) = database.query(&Query {
            families: &[Family::SansSerif],
            weight,
            ..Default::default()
        }) && !selected.contains(&id)
        {
            selected.push(id);
        }
        let mut chain = Vec::new();
        for id in std::iter::once(None).chain(selected.into_iter().map(Some)) {
            let result = if let Some(id) = id {
                crate::terminal_fonts::face_data(database, id, *remaining).map(|mut data| {
                    if data
                        .variation_axes()
                        .iter()
                        .any(|axis| axis.tag.into_bytes() == *b"wght")
                    {
                        data.tweak.coords.push(b"wght", f32::from(weight.0));
                    }
                    data
                })
            } else {
                let Some(data) = system.take() else {
                    continue;
                };
                Ok(data)
            };
            let Ok(data) = result else {
                warnings.push("native UI font was refused; using fallback fonts".into());
                continue;
            };
            let bytes = match &data.font {
                std::borrow::Cow::Owned(bytes) => bytes.capacity(),
                std::borrow::Cow::Borrowed(bytes) => bytes.len(),
            };
            let Some(next) = remaining.checked_sub(bytes) else {
                warnings
                    .push("native UI fonts exceed their byte budget; using fallback fonts".into());
                continue;
            };
            *remaining = next;
            let name = format!("taide-ui/{}/{}", weight.0, chain.len());
            definitions.font_data.insert(name.clone(), Arc::new(data));
            chain.push(name);
        }
        chain.extend(fallback.clone());
        definitions.families.insert(family, chain);
    }
    warnings
}

#[cfg(target_os = "macos")]
fn system_face(database: &Database, weight: Weight, remaining: usize) -> AppResult<FontData> {
    use objc2_app_kit::{NSFont, NSFontWeightMedium, NSFontWeightRegular, NSFontWeightSemibold};
    use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFType, CFURL};
    use objc2_core_text::{CTFont, CTFontTableOptions, kCTFontURLAttribute};
    use resvg::usvg::fontdb::Source;

    let weight = match weight {
        Weight::NORMAL => unsafe { NSFontWeightRegular },
        Weight(MEDIUM_WEIGHT) => unsafe { NSFontWeightMedium },
        Weight::SEMIBOLD => unsafe { NSFontWeightSemibold },
        _ => {
            return Err(AppError::InvalidArgument(
                "system UI font weight is unsupported".into(),
            ));
        }
    };
    let font = NSFont::systemFontOfSize_weight(SYSTEM_FONT_SIZE, weight);
    let font: &CTFont = font.as_ref();
    let attribute = unsafe { font.attribute(kCTFontURLAttribute) }
        .ok_or_else(|| AppError::Internal("system UI font has no URL".into()))?;
    let path = attribute
        .downcast_ref::<CFURL>()
        .and_then(CFURL::to_file_path)
        .ok_or_else(|| AppError::InvalidArgument("system UI font URL is invalid".into()))?;
    let head = unsafe { font.table(u32::from_be_bytes(*b"head"), CTFontTableOptions::empty()) }
        .ok_or_else(|| AppError::InvalidArgument("system UI font has no head table".into()))?;
    if head.len() != FONT_HEAD_BYTES {
        return Err(AppError::InvalidArgument(
            "system UI font head size is invalid".into(),
        ));
    }
    let head = head.to_vec();
    let mut selected = None;
    for face in database.faces() {
        if !matches!(&face.source, Source::File(source) if source == &path) {
            continue;
        }
        let data = crate::terminal_fonts::face_data(database, face.id, remaining)?;
        let parsed = skrifa::FontRef::from_index(&data.font, data.index)
            .map_err(|_| AppError::InvalidArgument("system UI font data is invalid".into()))?;
        let Some(table) = parsed.table_data(skrifa::raw::types::Tag::new(b"head")) else {
            continue;
        };
        if table.as_bytes() == head {
            selected = Some(data);
            break;
        }
    }
    let mut data = selected
        .ok_or_else(|| AppError::InvalidArgument("system UI font face was not found".into()))?;
    if let Some(variation) = unsafe { font.variation() } {
        let variation: CFRetained<CFDictionary<CFType, CFType>> =
            unsafe { CFRetained::cast_unchecked(variation) };
        let (keys, values) = variation.to_vecs();
        for (key, value) in keys.iter().zip(&values) {
            let tag = key
                .downcast_ref::<CFNumber>()
                .and_then(CFNumber::as_i64)
                .and_then(|tag| u32::try_from(tag).ok())
                .ok_or_else(|| {
                    AppError::InvalidArgument("system UI variation tag is invalid".into())
                })?;
            let value = value
                .downcast_ref::<CFNumber>()
                .and_then(CFNumber::as_f64)
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    AppError::InvalidArgument("system UI variation value is invalid".into())
                })?;
            let value = value as f32;
            let axis = data
                .variation_axes()
                .into_iter()
                .find(|axis| axis.tag.into_bytes() == tag.to_be_bytes());
            if !value.is_finite() || !axis.is_some_and(|axis| axis.range.contains(value)) {
                return Err(AppError::InvalidArgument(
                    "system UI variation is outside its font axis".into(),
                ));
            }
            data.tweak.coords.push(tag.to_be_bytes(), value);
        }
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    use skrifa::MetadataProvider;

    #[cfg(target_os = "macos")]
    const FONT_BUDGET: usize = 128 * 1024 * 1024;
    #[cfg(target_os = "macos")]
    const FONT_SIZE: f32 = 12.0;

    #[test]
    fn ui_fonts는_실제_weight_face선택과_공유budget_거절을_보존한다() {
        let defaults = FontDefinitions::default();
        let builtin = &defaults.font_data[&defaults.families[&FontFamily::Proportional][0]];
        let mut database = Database::new();
        database.load_font_data(builtin.font.to_vec());
        let mut regular = database.faces().next().unwrap().clone();
        regular.families[0].0 = "Roboto".into();
        regular.weight = Weight::NORMAL;
        let regular_id = database.push_face_info(regular.clone());
        let mut bold = regular.clone();
        bold.weight = Weight::BOLD;
        bold.post_script_name = "SyntheticBold".into();
        let bold_id = database.push_face_info(bold);
        assert_eq!(
            crate::terminal_fonts::named(&database, "SyntheticBold", Weight::NORMAL),
            Some(bold_id)
        );
        regular.weight = Weight(MEDIUM_WEIGHT);
        regular.post_script_name = "SyntheticMedium".into();
        let medium_id = database.push_face_info(regular.clone());
        regular.weight = Weight::SEMIBOLD;
        regular.post_script_name = "SyntheticSemibold".into();
        let semibold_id = database.push_face_info(regular);
        assert_eq!(
            crate::terminal_fonts::named(&database, "roboto", Weight::NORMAL),
            Some(regular_id)
        );
        assert_eq!(
            crate::terminal_fonts::named(&database, "Roboto", Weight(MEDIUM_WEIGHT)),
            Some(medium_id)
        );
        assert_eq!(
            crate::terminal_fonts::named(&database, "SyntheticMedium", Weight(MEDIUM_WEIGHT)),
            Some(medium_id)
        );
        assert_eq!(
            crate::terminal_fonts::named(&database, "Roboto", Weight::SEMIBOLD),
            Some(semibold_id)
        );
        let mut definitions = defaults.clone();
        let mut budget = builtin.font.len() * 3;
        prepare(&database, &mut definitions, &mut budget);
        assert_eq!(budget, 0);
        assert!(definitions.families[&FontFamily::Proportional][0].starts_with("taide-ui/400/"));
        assert!(
            definitions.families[&FontFamily::Name(MEDIUM_FAMILY.into())][0]
                .starts_with("taide-ui/500/")
        );
        assert!(
            definitions.families[&FontFamily::Name(SEMIBOLD_FAMILY.into())][0]
                .starts_with("taide-ui/600/")
        );
        let medium_font = &definitions.font_data
            [&definitions.families[&FontFamily::Name(MEDIUM_FAMILY.into())][0]];
        let parsed = skrifa::FontRef::from_index(&medium_font.font, medium_font.index).unwrap();
        assert!(parsed.charmap().map('M').is_some());
        let mut refused = defaults.clone();
        let mut budget = 0;
        let warnings = prepare(&database, &mut refused, &mut budget);
        assert!(!warnings.is_empty());
        assert_eq!(budget, 0);
        assert_eq!(
            refused.families[&FontFamily::Proportional],
            defaults.families[&FontFamily::Proportional]
        );
        assert_eq!(refused.font_data, defaults.font_data);
        assert_eq!(
            refused.families[&FontFamily::Name(MEDIUM_FAMILY.into())],
            defaults.families[&FontFamily::Proportional]
        );
        assert_eq!(
            refused.families[&FontFamily::Name(SEMIBOLD_FAMILY.into())],
            defaults.families[&FontFamily::Proportional]
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn ui_fonts는_os가_선택한_regular_medium_semibold_face와_variation을_실제_renderer에_연결한다()
    {
        let database = crate::system_fonts::database();
        let regular = system_face(&database, Weight::NORMAL, FONT_BUDGET).unwrap();
        let medium_font = system_face(&database, Weight(MEDIUM_WEIGHT), FONT_BUDGET).unwrap();
        let semibold_font = system_face(&database, Weight::SEMIBOLD, FONT_BUDGET).unwrap();
        assert!(!regular.tweak.coords.as_ref().is_empty());
        assert_ne!(regular.tweak.coords, medium_font.tweak.coords);
        assert_ne!(medium_font.tweak.coords, semibold_font.tweak.coords);
        let mut definitions = FontDefinitions::default();
        let mut budget = FONT_BUDGET;
        let warnings = prepare(&database, &mut definitions, &mut budget);
        assert!(
            !warnings
                .iter()
                .any(|warning| warning.contains("system UI font is unavailable"))
        );
        let regular_prepared =
            &definitions.font_data[&definitions.families[&FontFamily::Proportional][0]];
        let medium_prepared = &definitions.font_data
            [&definitions.families[&FontFamily::Name(MEDIUM_FAMILY.into())][0]];
        let semibold_prepared = &definitions.font_data
            [&definitions.families[&FontFamily::Name(SEMIBOLD_FAMILY.into())][0]];
        assert_eq!(regular_prepared.as_ref(), &regular);
        assert_eq!(medium_prepared.as_ref(), &medium_font);
        assert_eq!(semibold_prepared.as_ref(), &semibold_font);
        let context = egui::Context::default();
        context.set_fonts(definitions);
        let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
            std::env::temp_dir().join(format!("taide-ui-fonts-{}", uuid::Uuid::new_v4())),
        ));
        let appearance = crate::problems::Appearance::new(
            &taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap(),
        )
        .unwrap();
        let locale = taide_model::locale::ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: serde_json::from_str(include_str!(
                "../../../crates/taide-locale/resources/locales/en.json"
            ))
            .unwrap(),
        };
        let mut problems = crate::problems::Views::new("en-US").unwrap();
        let slot = taide_model::ids::ShellSlotId::new();
        problems.toggle(&slot);
        let mut output = context.run_ui(Default::default(), |ui| {
            let medium = medium(ui);
            assert_eq!(medium, FontFamily::Name(MEDIUM_FAMILY.into()));
            let semibold = taide_native_ui::font_families::semibold(ui);
            assert_eq!(semibold, FontFamily::Name(SEMIBOLD_FAMILY.into()));
            ui.label(
                egui::RichText::new("Semibold fixture")
                    .size(FONT_SIZE)
                    .family(semibold),
            );
            assert!(
                problems
                    .show_panel(
                        ui,
                        &slot,
                        &crate::diagnostics::Store::default(),
                        &locale,
                        &appearance
                    )
                    .unwrap()
                    .is_empty()
            );
        });
        let galley = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Problems" => Some(&text.galley),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            galley.job.sections[0].format.font_id.family,
            FontFamily::Name(MEDIUM_FAMILY.into())
        );
        assert_eq!(galley.job.sections[0].format.font_id.size, FONT_SIZE);
        assert!(
            galley
                .rows
                .iter()
                .any(|row| !row.row.visuals.mesh.indices.is_empty())
        );
        let semibold_galley = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Semibold fixture" => {
                    Some(&text.galley)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(
            semibold_galley.job.sections[0].format.font_id.family,
            FontFamily::Name(SEMIBOLD_FAMILY.into())
        );
        assert!(
            semibold_galley
                .rows
                .iter()
                .any(|row| !row.row.visuals.mesh.indices.is_empty())
        );
        output.textures_delta.clear();
    }
}
