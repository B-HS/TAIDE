use eframe::egui::Color32;
use serde::Deserialize;
use serde_json::{Value, json};
use taide_lsp::native::protocol::lsp_types::{CompletionItem, CompletionItemKind};

use super::color;

const MONACO_COLOR: u64 = 19;
const MONACO_FILE: u64 = 20;
const MONACO_FOLDER: u64 = 23;
const COLOR_CHANNELS: usize = 3;
const RGBA_CHANNELS: usize = COLOR_CHANNELS + 1;
const FIXTURE: &str = include_str!("fixtures/completion-colors-reference.json");

#[derive(Deserialize)]
struct Reference {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    completion: Value,
    css: Option<String>,
}

fn expected(value: &str) -> Color32 {
    let body = value
        .strip_prefix("rgb(")
        .or_else(|| value.strip_prefix("rgba("))
        .unwrap()
        .strip_suffix(')')
        .unwrap();
    let parts = body
        .split(',')
        .map(|value| value.trim().parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    let alpha = if parts.len() == RGBA_CHANNELS {
        parts[COLOR_CHANNELS]
    } else {
        1.0
    };
    Color32::from_rgba_unmultiplied(
        parts[0].round() as u8,
        parts[1].round() as u8,
        parts[2].round() as u8,
        (alpha * f64::from(u8::MAX)).round() as u8,
    )
}

#[test]
fn 원본_색_추출_우선순위와_실제_브라우저의_css_색을_모든_표본에서_보존한다() {
    let reference: Reference = serde_json::from_str(FIXTURE).unwrap();
    for (index, mut case) in reference.cases.into_iter().enumerate() {
        let kind = match case.completion["kind"].as_u64().unwrap() {
            MONACO_COLOR => CompletionItemKind::COLOR,
            MONACO_FILE => CompletionItemKind::FILE,
            MONACO_FOLDER => CompletionItemKind::FOLDER,
            _ => CompletionItemKind::METHOD,
        };
        case.completion["kind"] = serde_json::to_value(kind).unwrap();
        if case.completion["documentation"].is_object() {
            case.completion["documentation"]["kind"] = json!("markdown");
        }
        let item: CompletionItem = serde_json::from_value(case.completion).unwrap();
        assert_eq!(
            color(&item),
            case.css.as_deref().map(expected),
            "case {index}: {item:?}"
        );
    }
}

#[test]
fn css가_거절하는_혼합단위와_유니코드_공백은_원본_색_문자열_판별을_통과해도_견본을_만들지_않는다() {
    for label in [
        "rgb(1,2%,3)",
        "hsl(0%,100%,50%)",
        "hsl(0,100,50)",
        "rgb(\u{a0}1,2,3)",
    ] {
        let item = CompletionItem {
            label: label.into(),
            kind: Some(CompletionItemKind::COLOR),
            ..Default::default()
        };
        assert_eq!(color(&item), None);
    }
}
