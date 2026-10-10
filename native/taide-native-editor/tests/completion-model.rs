use lsp_types::{CompletionItem, CompletionItemKind};
use taide_model::ids::TabId;
use taide_native_editor::completion::Candidate;
use taide_native_editor::completion_filter::Score;
use taide_native_editor::completion_model::{Model, Ranked};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::lsp::{LspRange, Position};
use taide_native_editor::store::{EditorLimits, EditorStore};

const BYTE_LIMIT: usize = 4096;
const HEX_PAIR_SIZE: usize = 2;
const HEX_RADIX: u32 = 16;
const FIELD_COUNT: usize = 6;

fn snapshot(text: &str) -> DocumentSnapshot {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_untitled(TabId::new(), text, "rust".into())
        .unwrap();
    store.documents().snapshot(document).unwrap()
}

fn text(hex: &str) -> String {
    let bytes = hex
        .as_bytes()
        .chunks_exact(HEX_PAIR_SIZE)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), HEX_RADIX).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

fn optional(hex: &str) -> Option<String> {
    (hex != "-").then(|| text(hex))
}

fn candidate(document: &DocumentSnapshot, item: CompletionItem, overwrite: usize) -> Candidate {
    let column = document.rope.len_utf16_cu();
    let position = Position::new(0, column.try_into().unwrap());
    Candidate::new(
        document,
        position,
        LspRange::new(
            Position::new(0, (column - overwrite).try_into().unwrap()),
            position,
        ),
        item,
    )
    .unwrap()
}

fn kind(value: &str) -> CompletionItemKind {
    match value {
        "0" => CompletionItemKind::METHOD,
        "1" => CompletionItemKind::FUNCTION,
        "18" => CompletionItemKind::TEXT,
        "28" => CompletionItemKind::SNIPPET,
        other => panic!("unknown fixture kind {other}"),
    }
}

#[test]
fn 후보모델의_정렬_필터_증분_강조와_대형공급은_실제_monaco와_일치한다() {
    let mut active = String::new();
    let mut model = None;
    let mut count = 0;
    for row in include_str!("fixtures/completion-model-reference.tsv").lines() {
        let fields = row.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), FIELD_COUNT);
        if active != fields[0] {
            active = fields[0].to_owned();
            let document = snapshot(&text(fields[1]));
            let items = fields[2]
                .split(';')
                .enumerate()
                .map(|(index, encoded)| {
                    let values = encoded.split(',').collect::<Vec<_>>();
                    candidate(
                        &document,
                        CompletionItem {
                            label: text(values[0]),
                            filter_text: optional(values[1]),
                            sort_text: optional(values[2]),
                            kind: Some(kind(values[3])),
                            data: Some(index.to_string().into()),
                            ..Default::default()
                        },
                        values[4].parse().unwrap(),
                    )
                })
                .collect();
            model = Some(Model::new(&document, items).unwrap());
        }
        let model = model.as_mut().unwrap();
        let leading = text(fields[3]);
        let delta = fields[4].parse().unwrap();
        let actual = model.filter(&leading, delta).to_vec();
        let expected = fields[5]
            .split(';')
            .filter(|item| !item.is_empty())
            .map(|encoded| {
                let values = encoded.split(',').collect::<Vec<_>>();
                (
                    values[0].to_owned(),
                    Score {
                        value: values[1].parse().unwrap(),
                        word_start: values[2].parse().unwrap(),
                        positions: values[3]
                            .split('|')
                            .filter(|position| !position.is_empty())
                            .map(|position| position.parse().unwrap())
                            .collect(),
                    },
                )
            })
            .collect::<Vec<_>>();
        let actual = actual
            .into_iter()
            .map(|item| {
                let candidate = model.candidate(item.candidate).unwrap();
                (
                    candidate
                        .item
                        .data
                        .as_ref()
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    item.score,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual, expected,
            "scenario {active}, leading {leading:?}, delta {delta}"
        );
        count += 1;
    }
    assert_eq!(count, 17);
}

#[test]
fn 혼합_sort_text는_누락된_값을_label로_보완하여_입력순서와_무관하게_정렬한다() {
    let document = snapshot("");
    let entries = [("a", Some("3")), ("b", Some("1")), ("ab", None)];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let items = order
            .into_iter()
            .map(|index| {
                let (label, sort_text) = entries[index];
                candidate(
                    &document,
                    CompletionItem {
                        label: label.into(),
                        sort_text: sort_text.map(str::to_owned),
                        ..Default::default()
                    },
                    0,
                )
            })
            .collect();
        let mut model = Model::new(&document, items).unwrap();
        let sorted = model
            .filter("", 0)
            .to_vec()
            .into_iter()
            .map(|item| model.candidate(item.candidate).unwrap().item.label.clone())
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec!["b", "a", "ab"]);
    }
}

#[test]
fn 후보모델은_이전_revision과_잘린_unicode_범위를_거절한다() {
    let document = snapshot("한\u{1f600}con");
    let item = candidate(
        &document,
        CompletionItem {
            label: "console".into(),
            ..Default::default()
        },
        3,
    );
    let mut changed = document.clone();
    changed.revision += 1;
    assert!(matches!(
        Model::new(&changed, vec![item.clone()]),
        Err(EditorError::StaleRevision)
    ));
    let mut split = item.clone();
    split.insert.start = "한".len() + 1;
    assert!(matches!(
        Model::new(&document, vec![split]),
        Err(EditorError::InvalidBoundary)
    ));
    let mut split = item;
    split.requested_byte = "한".len() + 1;
    assert!(matches!(
        Model::new(&document, vec![split]),
        Err(EditorError::InvalidBoundary)
    ));
}

#[test]
fn 후보필터의_빈후보와_반복호출은_문서와_원래_후보를_변경하지_않는다() {
    let document = snapshot("con");
    let item = candidate(
        &document,
        CompletionItem {
            label: "console".into(),
            ..Default::default()
        },
        3,
    );
    let original = item.clone();
    let rope = document.rope.clone();
    let mut model = Model::new(&document, vec![item]).unwrap();
    let first: Vec<Ranked> = model.filter("con", 0).to_vec();
    assert_eq!(model.filter("con", 0), first);
    assert!(model.filter("xyz", 0).is_empty());
    assert_eq!(model.filter("con", 0), first);
    assert_eq!(model.candidate(0), Some(&original));
    assert_eq!(document.rope, rope);
    let mut empty = Model::new(&document, Vec::new()).unwrap();
    assert!(empty.is_empty());
    assert!(empty.filter("con", 0).is_empty());
}
