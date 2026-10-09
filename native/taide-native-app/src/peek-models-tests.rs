use super::*;
use taide_lsp::native::protocol::lsp_types::{Location, Position, Range};
use taide_model::file::EditorConfigOptions;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::ViewKey;

const BYTE_LIMIT: usize = 4096;
const PATH: &str = "/synthetic/preview.rs";

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: 16,
        max_views: 4,
        max_undo_groups: 2,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap()
}

fn file(path: &str, content: &str) -> OpenedFile {
    OpenedFile {
        path: path.into(),
        content: content.into(),
        language_id: "rust".into(),
        byte_size: content.len() as u32,
        line_count: content.split('\n').count() as u32,
        tier: FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 0.0,
        editor_config: EditorConfigOptions::default(),
    }
}

fn edit(store: &mut EditorStore, document: DocumentId) {
    let current = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: current.revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "dirty ".into(),
                }],
                group: UndoGroup(current.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
}

#[test]
fn preload는_기존문서와_중복을_제외하고_첫8개의_파일만_선택한다() {
    let mut store = store();
    store
        .open_file(PATH.into(), file(PATH, "existing"))
        .unwrap();
    let mut targets = Vec::new();
    for path in [PATH.into(), "/synthetic/0.rs".into()]
        .into_iter()
        .chain((0..PRELOAD_LIMIT + 2).map(|index| format!("/synthetic/{index}.rs")))
    {
        targets.push(Target::from_location(Location::new(
            taide_lsp::service::workspace_folder_uri(&path)
                .parse()
                .unwrap(),
            Range::new(Position::new(0, 0), Position::new(0, 1)),
        )));
    }
    let paths = Models::default().preload_paths(&store, &targets);
    assert_eq!(paths.len(), PRELOAD_LIMIT);
    assert_eq!(paths[0], PathBuf::from("/synthetic/0.rs"));
    assert_eq!(paths.last().unwrap(), &PathBuf::from("/synthetic/7.rs"));
}

#[test]
fn 기존_dirty_본문과_새_미리보기_인수는_디스크값으로_덮어쓰지않는다() {
    let now = Instant::now();
    let mut store = store();
    let document = store
        .open_file(PATH.into(), file(PATH, "existing"))
        .unwrap();
    edit(&mut store, document);
    let mut models = Models::default();
    assert_eq!(
        models.admit(&mut store, file(PATH, "disk"), now).unwrap(),
        Some(document)
    );
    assert!(!models.owns(document));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "dirty existing"
    );
    let path = "/synthetic/second.rs";
    let second = models
        .admit(&mut store, file(path, "preview"), now)
        .unwrap()
        .unwrap();
    edit(&mut store, second);
    assert_eq!(
        models
            .admit(&mut store, file(path, "new disk"), now)
            .unwrap(),
        Some(second)
    );
    models.adopt(second);
    assert!(!models.owns(second));
    models.sweep(&mut store, &HashSet::new(), now + MODEL_TTL);
    assert_eq!(
        store.documents().snapshot(second).unwrap().rope.to_string(),
        "dirty preview"
    );
}

#[test]
fn 미리보기_ttl은_전용_view와_dirty를_보존하고_실제탭_인수를_구분한다() {
    let now = Instant::now();
    let mut store = store();
    let mut models = Models::default();
    let clean = models
        .admit(&mut store, file(PATH, "clean"), now)
        .unwrap()
        .unwrap();
    assert_eq!(models.next_expiry(now), Some(MODEL_TTL));
    assert!(
        models
            .sweep(
                &mut store,
                &HashSet::new(),
                now + MODEL_TTL - Duration::from_nanos(1)
            )
            .is_empty()
    );
    assert_eq!(
        models.sweep(&mut store, &HashSet::new(), now + MODEL_TTL),
        [clean]
    );
    let preview = models
        .admit(&mut store, file(PATH, "preview"), now)
        .unwrap()
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "peek".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            preview,
        )
        .unwrap();
    models.sweep(&mut store, &HashSet::from([view]), now + MODEL_TTL);
    assert!(models.owns(preview));
    models.sweep(&mut store, &HashSet::new(), now + MODEL_TTL);
    assert!(!models.owns(preview));
    store.detach_view(view).unwrap();
    let dirty = models
        .admit(&mut store, file("/synthetic/dirty.rs", "dirty"), now)
        .unwrap()
        .unwrap();
    edit(&mut store, dirty);
    assert!(
        models
            .sweep(&mut store, &HashSet::new(), now + MODEL_TTL)
            .is_empty()
    );
    assert!(models.owns(dirty));
    assert!(store.documents().snapshot(dirty).unwrap().dirty);
}

#[test]
fn 새_대형_읽기전용_거절_lossy_미리보기는_문서로_반입하지않는다() {
    let now = Instant::now();
    let mut store = store();
    let mut models = Models::default();
    for tier in [
        FileSizeTier::Large,
        FileSizeTier::ReadOnly,
        FileSizeTier::Refused,
    ] {
        let mut opened = file(PATH, "large");
        opened.tier = tier;
        assert_eq!(models.admit(&mut store, opened, now).unwrap(), None);
    }
    for lossy in [true, false] {
        let mut opened = file(PATH, "readonly");
        opened.encoding_lossy = lossy;
        opened.read_only = !lossy;
        assert_eq!(models.admit(&mut store, opened, now).unwrap(), None);
    }
    assert!(store.documents().is_empty());
}
