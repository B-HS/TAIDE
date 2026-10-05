use std::path::PathBuf;

use egui::{Color32, FontId};
use serde_json::{Value, json};
use taide_model::file::{EditorConfigOptions, FileSizeTier, MirrorEntry, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DiskChoice, EditorError};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::save_cleanup::CleanupFlags;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_native_ui::conflict_banner::{BannerAction, BannerVariant};
use taide_native_ui::editor_surface::{EditorAppearance, NativeEditor};
use taide_remote_web::files::{FileEvent, FileFailure, FileState, FileSurface, FileViews};
use taide_remote_web::shell::Failure;
use taide_remote_web::{InvokeError, ResponsePayload};

const PATH: &str = r"C:\synthetic\file.rs";
const SECOND_PATH: &str = r"C:\synthetic\second.rs";
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 128;
const OPEN_SEQ: u32 = 1;
const OLD_SEQ: u32 = 2;
const SAVE_SEQ: u32 = 3;
const FRESH_SEQ: u32 = 4;
const FONT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 8.0;
const SURFACE_WIDTH: f32 = 900.0;
const SURFACE_HEIGHT: f32 = 500.0;

fn limits() -> EditorLimits {
    EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    }
}

fn file(path: &str, content: &str) -> OpenedFile {
    OpenedFile {
        path: path.into(),
        content: content.into(),
        language_id: "rust".into(),
        byte_size: content.len().try_into().unwrap(),
        line_count: content.lines().count().try_into().unwrap(),
        tier: FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 1.0,
        editor_config: EditorConfigOptions::default(),
    }
}

fn owner(pane: &str) -> ViewKey {
    ViewKey {
        window: "remote".into(),
        pane: PaneId(pane.into()),
        tab: TabId("tab-synthetic".into()),
    }
}

fn opened(views: &mut FileViews, path: &str, seq: u32, file: OpenedFile) {
    views.read_sent(path.into(), seq);
    assert!(views.response(
        seq,
        &Ok(ResponsePayload::Json(serde_json::to_value(file).unwrap()))
    ));
    assert!(matches!(views.state(path), FileState::Ready(_)));
}

fn editor() -> NativeEditor {
    NativeEditor {
        appearance: EditorAppearance {
            font: FontId::monospace(FONT_SIZE),
            line_height: LINE_HEIGHT,
            horizontal_padding: PADDING,
            background: Color32::BLACK,
            foreground: Color32::WHITE,
            muted: Color32::GRAY,
            selection: Color32::BLUE,
            cursor: Color32::WHITE,
            current_line: Color32::BLACK,
            line_numbers: true,
            indent: "\t".into(),
        },
    }
}

#[test]
fn 종료_대기는_조회가_아닌_저장과_대기중_또는_전송중_선택을_소유한다() {
    let key = owner("first");
    let mut views = FileViews::new(limits()).unwrap();
    views.bind(key.clone(), PATH).unwrap();
    views.read_sent(PATH.into(), OPEN_SEQ);
    assert!(!views.has_pending_operations());
    assert!(views.response(
        OPEN_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(file(PATH, "disk")).unwrap()
        ))
    ));
    let view = views.view(&key).unwrap();
    let save = views.prepare_save(view).unwrap().unwrap();
    views.save_sent(save, SAVE_SEQ);
    assert!(views.has_pending_operations());
    assert!(views.response(SAVE_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
    assert!(!views.has_pending_operations());
    let choice = views
        .prepare_choice(view, DiskChoice::KeepMine)
        .unwrap()
        .unwrap();
    assert!(views.queue_choice(choice.clone()).unwrap());
    assert!(views.has_pending_operations());
    views.choice_sent(choice, FRESH_SEQ);
    assert!(views.has_pending_operations());
    assert!(views.response(
        FRESH_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(file(PATH, "disk")).unwrap()
        ))
    ));
    assert!(!views.has_pending_operations());
    let choice = views
        .prepare_choice(view, DiskChoice::ViewDisk)
        .unwrap()
        .unwrap();
    assert!(views.queue_choice(choice).unwrap());
    views.disconnected();
    assert!(!views.has_pending_operations());
    assert!(
        views
            .take_events()
            .iter()
            .any(|event| matches!(event, FileEvent::ChoiceFinished { result: Err(_), .. }))
    );
}

#[test]
fn 미러_복원은_진행중_저장과_디스크_선택을_침범하지_않는다() {
    let key = owner("first");
    let mirror = MirrorEntry {
        path: PATH.into(),
        content: "mirror".into(),
        saved_at_ms: 1.0,
        disk_modified_ms: Some(1.0),
        conflict: false,
        source_missing: false,
    };
    let mut views = FileViews::new(limits()).unwrap();
    views.bind(key.clone(), PATH).unwrap();
    opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
    let view = views.view(&key).unwrap();
    let request = views.prepare_save(view).unwrap().unwrap();
    views.save_sent(request, SAVE_SEQ);
    assert_eq!(views.restore_mirror(PATH, Some(&mirror)), Ok(false));
    assert!(views.response(SAVE_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
    assert_eq!(views.restore_mirror(PATH, Some(&mirror)), Ok(false));

    let mut selected = FileViews::new(limits()).unwrap();
    selected.bind(key.clone(), PATH).unwrap();
    opened(&mut selected, PATH, OPEN_SEQ, file(PATH, "disk"));
    let view = selected.view(&key).unwrap();
    let request = selected
        .prepare_choice(view, DiskChoice::ViewDisk)
        .unwrap()
        .unwrap();
    selected.queue_choice(request.clone()).unwrap();
    assert_eq!(selected.restore_mirror(PATH, Some(&mirror)), Ok(false));
    selected.choice_sent(request, FRESH_SEQ);
    assert_eq!(selected.restore_mirror(PATH, Some(&mirror)), Ok(false));
    assert!(selected.response(
        FRESH_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(file(PATH, "disk")).unwrap()
        ))
    ));
    assert_eq!(selected.restore_mirror(PATH, Some(&mirror)), Ok(false));
}

#[test]
fn 늦은_미러_복원은_초기_revision_공유_view_저장_편집과_용량을_보호한다() {
    let key = owner("first");
    let second = owner("second");
    let mirror = MirrorEntry {
        path: PATH.into(),
        content: "restored".into(),
        saved_at_ms: 1.0,
        disk_modified_ms: Some(1.0),
        conflict: true,
        source_missing: false,
    };
    let mut views = FileViews::new(limits()).unwrap();
    views.bind(key.clone(), PATH).unwrap();
    assert_eq!(views.restore_mirror(PATH, Some(&mirror)), Ok(false));
    opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
    views.bind(second.clone(), PATH).unwrap();
    views.take_events();
    assert_eq!(views.restore_mirror(PATH, Some(&mirror)), Ok(true));
    let view = views.view(&key).unwrap();
    let document = views.store().views().get(view).unwrap().document;
    let snapshot = views.store().documents().snapshot(document).unwrap();
    assert!(snapshot.dirty);
    assert_eq!(snapshot.rope.to_string(), "restored");
    assert_eq!(snapshot.revision, 1);
    assert_eq!(views.store().views().for_document(document).count(), 2);
    assert_eq!(
        views.take_events(),
        [FileEvent::Restored {
            path: PATH.into(),
            document
        }]
    );
    assert_eq!(views.restore_mirror(PATH, Some(&mirror)), Ok(false));
    assert_eq!(
        views.store_mut().choose_disk(
            document,
            snapshot.revision,
            std::path::Path::new(PATH),
            file(PATH, "disk"),
            DiskChoice::ViewDisk
        ),
        Ok(false)
    );

    let mut edited = FileViews::new(limits()).unwrap();
    edited.bind(key.clone(), PATH).unwrap();
    opened(&mut edited, PATH, OPEN_SEQ, file(PATH, "disk"));
    let view = edited.view(&key).unwrap();
    let document = edited.store().views().get(view).unwrap().document;
    replace_selections(edited.store_mut(), view, "typed", None).unwrap();
    edited.store_mut().undo(document).unwrap();
    assert!(edited.store().documents().snapshot(document).unwrap().dirty);
    let saved_snapshot = edited.store_mut().save_snapshot(document).unwrap();
    edited
        .store_mut()
        .mark_saved(saved_snapshot, Some(1.0))
        .unwrap();
    assert!(!edited.store().documents().snapshot(document).unwrap().dirty);
    assert_eq!(edited.restore_mirror(PATH, Some(&mirror)), Ok(false));
    assert_eq!(
        edited
            .store()
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "disk"
    );

    let mut saved = FileViews::new(limits()).unwrap();
    saved.bind(key.clone(), PATH).unwrap();
    opened(&mut saved, PATH, OPEN_SEQ, file(PATH, "disk"));
    let view = saved.view(&key).unwrap();
    let request = saved.prepare_save(view).unwrap().unwrap();
    saved.save_sent(request, SAVE_SEQ);
    assert!(saved.response(SAVE_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
    assert_eq!(saved.restore_mirror(PATH, Some(&mirror)), Ok(false));

    let mut readonly = FileViews::new(limits()).unwrap();
    readonly.bind(key.clone(), PATH).unwrap();
    let mut original = file(PATH, "disk");
    original.read_only = true;
    opened(&mut readonly, PATH, OPEN_SEQ, original);
    let mut oversized = mirror.clone();
    oversized.content = "x".repeat(BYTE_LIMIT + 1);
    assert_eq!(
        readonly.restore_mirror(PATH, Some(&oversized)),
        Err(EditorError::Capacity)
    );
    let view = readonly.view(&key).unwrap();
    let document = readonly.store().views().get(view).unwrap().document;
    assert!(
        !readonly
            .store()
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
    );
    readonly.retry_mirror_restore(PATH);
    assert_eq!(readonly.restore_mirror(PATH, Some(&mirror)), Ok(true));
    assert!(
        readonly
            .store()
            .documents()
            .snapshot(document)
            .unwrap()
            .metadata
            .read_only
    );
}

#[test]
fn 원격_choice_flush_대기는_실제_입력을_잠그고_조회_저장_해제와_closed를_소유한다() {
    let mut views = FileViews::new(limits()).unwrap();
    let key = owner("first");
    views.bind(key.clone(), PATH).unwrap();
    opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
    let view = views.view(&key).unwrap();
    let document = views.store().views().get(view).unwrap().document;
    replace_selections(views.store_mut(), view, "draft ", None).unwrap();
    views.take_events();
    let request = views
        .prepare_choice(view, DiskChoice::ViewDisk)
        .unwrap()
        .unwrap();
    assert_eq!(views.queue_choice(request.clone()), Ok(true));
    assert_eq!(views.validate_choice(&request), Ok(true));
    assert_eq!(views.waiting_choice(), Some(&request));
    assert_eq!(
        views.bound_tabs().into_iter().collect::<Vec<_>>(),
        std::slice::from_ref(&key.tab)
    );
    views.retry(PATH);
    assert!(views.next_reads().is_empty());
    assert!(views.prepare_save(view).unwrap().is_none());
    assert!(
        views
            .prepare_choice(view, DiskChoice::KeepMine)
            .unwrap()
            .is_none()
    );
    let editor = editor();
    let locale =
        serde_json::from_value(json!({"id":"ko","name":"synthetic","messages":{}})).unwrap();
    let banner = taide_native_ui::conflict_banner::BannerAppearance {
        error: Color32::RED,
        warning: Color32::YELLOW,
    };
    let context = egui::Context::default();
    let mut output = context.run_ui(
        egui::RawInput {
            events: vec![egui::Event::Text("blocked".into())],
            ..Default::default()
        },
        |ui| {
            assert!(
                !views
                    .show_surface(
                        ui,
                        &key,
                        FileSurface {
                            editor: &editor,
                            locale: &locale,
                            banner: &banner
                        },
                        true
                    )
                    .unwrap()
                    .editor
                    .unwrap()
                    .changed
            );
        },
    );
    output.textures_delta.clear();
    assert_eq!(
        views
            .store()
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "draft disk"
    );
    views.unbind(&key).unwrap();
    assert_eq!(views.store().documents().len(), 1);
    assert_eq!(views.validate_choice(&request), Err(EditorError::NotFound));
    views.disconnected();
    assert!(views.waiting_choice().is_none());
    assert_eq!(
        views.take_events(),
        [FileEvent::ChoiceFinished {
            request,
            result: Err(FileFailure::Rpc(Failure::Invocation(InvokeError::Closed)))
        }]
    );
    assert!(views.store().documents().snapshot(document).unwrap().dirty);
}

#[test]
fn 원격_restore_notice는_성공한_저장뒤_같은_문서의_모든_view에서_해제된다() {
    let locale =
        serde_json::from_value(json!({"id":"ko","name":"synthetic","messages":{}})).unwrap();
    let banner = taide_native_ui::conflict_banner::BannerAppearance {
        error: Color32::RED,
        warning: Color32::YELLOW,
    };
    let editor = editor();
    let context = egui::Context::default();
    let mut views = FileViews::new(limits()).unwrap();
    let keys = [owner("first"), owner("second")];
    for key in &keys {
        views.bind(key.clone(), PATH).unwrap();
    }
    opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
    for key in &keys {
        views
            .restore_notice(key.clone(), BannerVariant::MirrorRestored)
            .unwrap();
    }
    let banners = |views: &mut FileViews| {
        keys.iter()
            .map(|key| {
                let mut has_banner = false;
                let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                    has_banner = views
                        .show_surface(
                            ui,
                            key,
                            FileSurface {
                                editor: &editor,
                                locale: &locale,
                                banner: &banner,
                            },
                            false,
                        )
                        .unwrap()
                        .banner
                        .is_some();
                });
                output.textures_delta.clear();
                has_banner
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(banners(&mut views), [true, true]);
    let view = views.view(&keys[0]).unwrap();
    replace_selections(views.store_mut(), view, "draft ", None).unwrap();
    let request = views.prepare_save(view).unwrap().unwrap();
    views.save_sent(request, SAVE_SEQ);
    assert!(views.response(SAVE_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
    assert_eq!(banners(&mut views), [false, false]);
}

#[test]
fn 원격_disk_choice는_최신_조회와_공유_문서의_초안_또는_디스크_선택을_보존한다() {
    for choice in [DiskChoice::KeepMine, DiskChoice::ViewDisk] {
        let mut views = FileViews::new(limits()).unwrap();
        let first = owner("first");
        let second = owner("second");
        views.bind(first.clone(), PATH).unwrap();
        views.bind(second.clone(), PATH).unwrap();
        opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
        let view = views.view(&first).unwrap();
        let document = views.store().views().get(view).unwrap().document;
        replace_selections(views.store_mut(), view, "draft ", None).unwrap();
        opened(&mut views, PATH, OLD_SEQ, file(PATH, "observed disk"));
        assert!(views.store().has_disk_conflict(document).unwrap());
        views.take_events();
        views
            .restore_notice(first.clone(), BannerVariant::MirrorRestoredConflict)
            .unwrap();
        views
            .restore_notice(second.clone(), BannerVariant::MirrorRestoredConflict)
            .unwrap();
        let request = views.prepare_choice(view, choice).unwrap().unwrap();
        assert_eq!(request.document(), document);
        assert_eq!(request.choice(), choice);
        assert_eq!(request.call().command, "file_open");
        assert_eq!(request.call().args, json!({"path": PATH}));
        assert_eq!(views.validate_choice(&request), Ok(true));
        views.choice_sent(request.clone(), FRESH_SEQ);
        assert!(views.prepare_save(view).unwrap().is_none());
        assert!(views.prepare_choice(view, choice).unwrap().is_none());
        let mut fresh = file(PATH, "fresh disk");
        fresh.modified_ms = f64::from(FRESH_SEQ);
        assert!(views.response(
            FRESH_SEQ,
            &Ok(ResponsePayload::Json(serde_json::to_value(fresh).unwrap()))
        ));
        let expected_dirty = choice == DiskChoice::KeepMine;
        assert_eq!(
            views.take_events(),
            [FileEvent::ChoiceFinished {
                request,
                result: Ok(expected_dirty)
            }]
        );
        let snapshot = views.store().documents().snapshot(document).unwrap();
        assert_eq!(snapshot.dirty, expected_dirty);
        assert_eq!(
            snapshot.metadata.disk_modified_ms,
            Some(f64::from(FRESH_SEQ))
        );
        assert!(!views.store().has_disk_conflict(document).unwrap());
        if choice == DiskChoice::KeepMine {
            assert_eq!(snapshot.rope.to_string(), "draft disk");
            assert!(views.store_mut().undo(document).unwrap());
            assert_eq!(
                views
                    .store()
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                "disk"
            );
        } else {
            assert_eq!(snapshot.rope.to_string(), "fresh disk");
            assert!(!views.store_mut().undo(document).unwrap());
        }
        assert_eq!(views.store().views().len(), 2);
        assert_eq!(
            views
                .store()
                .views()
                .get(views.view(&second).unwrap())
                .unwrap()
                .document,
            document
        );
        assert!(views.next_reads().is_empty());
    }
}

#[test]
fn 원격_disk_choice는_늦은_편집_무효화_오류_해제와_disconnect를_적용하지_않는다() {
    let failures = [
        Ok(ResponsePayload::Binary(vec![])),
        Err(json!({"code": "SYNTHETIC"})),
    ];
    for failure in failures {
        let mut views = FileViews::new(limits()).unwrap();
        let key = owner("first");
        views.bind(key.clone(), PATH).unwrap();
        opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
        let view = views.view(&key).unwrap();
        let document = views.store().views().get(view).unwrap().document;
        let old = views
            .prepare_choice(view, DiskChoice::ViewDisk)
            .unwrap()
            .unwrap();
        replace_selections(views.store_mut(), view, "draft ", None).unwrap();
        assert_eq!(views.validate_choice(&old), Err(EditorError::StaleRevision));
        let request = views
            .prepare_choice(view, DiskChoice::ViewDisk)
            .unwrap()
            .unwrap();
        views.take_events();
        views.choice_sent(request.clone(), FRESH_SEQ);
        assert!(views.response(FRESH_SEQ, &failure));
        let expected = match failure {
            Ok(_) => FileFailure::Rpc(Failure::MalformedResponse),
            Err(value) => FileFailure::Rpc(Failure::Remote(value)),
        };
        assert_eq!(
            views.take_events(),
            [FileEvent::ChoiceFinished {
                request,
                result: Err(expected)
            }]
        );
        let request = views
            .prepare_choice(view, DiskChoice::ViewDisk)
            .unwrap()
            .unwrap();
        views.choice_sent(request.clone(), FRESH_SEQ);
        views.event(
            "fs:changed",
            &json!({"change":taide_model::file::FsChange {
                kind: taide_model::file::FsChangeKind::Modified,
                paths: vec![PATH.into()],
                from_app: false,
            }})
            .to_string(),
        );
        assert!(views.next_reads().is_empty());
        assert!(views.response(
            FRESH_SEQ,
            &Ok(ResponsePayload::Json(
                serde_json::to_value(file(PATH, "late disk")).unwrap()
            ))
        ));
        assert_eq!(
            views.take_events(),
            [FileEvent::ChoiceFinished {
                request,
                result: Err(FileFailure::Editor(EditorError::StaleRevision))
            }]
        );
        assert_eq!(views.next_reads(), [PATH]);
        let request = views
            .prepare_choice(view, DiskChoice::ViewDisk)
            .unwrap()
            .unwrap();
        views.choice_sent(request.clone(), FRESH_SEQ);
        views.disconnected();
        assert_eq!(
            views.take_events(),
            [FileEvent::ChoiceFinished {
                request,
                result: Err(FileFailure::Rpc(Failure::Invocation(InvokeError::Closed)))
            }]
        );
        assert!(!views.response(FRESH_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
        assert!(views.next_reads().is_empty());
        views.refresh();
        assert_eq!(views.next_reads(), [PATH]);
        let request = views
            .prepare_choice(view, DiskChoice::ViewDisk)
            .unwrap()
            .unwrap();
        views.choice_sent(request.clone(), FRESH_SEQ);
        views.unbind(&key).unwrap();
        assert_eq!(views.store().documents().len(), 1);
        assert!(views.response(
            FRESH_SEQ,
            &Ok(ResponsePayload::Json(
                serde_json::to_value(file(PATH, "late disk")).unwrap()
            ))
        ));
        assert_eq!(
            views.take_events(),
            [FileEvent::ChoiceFinished {
                request,
                result: Err(FileFailure::Editor(EditorError::NotFound))
            }]
        );
        let snapshot = views.store().documents().snapshot(document).unwrap();
        assert_eq!(snapshot.rope.to_string(), "draft disk");
        assert!(snapshot.dirty);
    }
}

#[test]
fn 원격_file_surface는_실제_배너_click_잠금_loading_readonly_오류를_그린다() {
    let locale = serde_json::from_value(json!({"id":"ko","name":"synthetic","messages":{
        "editor.changedOnDisk":"changed", "editor.keepMine":"mine", "editor.viewDiskContent":"disk choice",
        "editor.mirrorRestored":"restored", "common.close":"dismiss", "editor.openFailed":"open failure",
        "editor.readOnlyLossyEncoding":"lossy readonly", "editor.readOnlyLargeFile":"large readonly"
    }})).unwrap();
    let theme = serde_json::from_value(
        json!({"id":"synthetic","name":"synthetic","type":"dark","colors":{
        "statusIndicator.error":"#ff0000", "statusIndicator.warning":"#ffff00"
    },"syntax":{},"terminal":{}}),
    )
    .unwrap();
    let banner = taide_native_ui::presentation::banner_appearance(&theme).unwrap();
    assert_eq!(banner.error, Color32::RED);
    assert_eq!(banner.warning, Color32::YELLOW);
    let editor = editor();
    let context = egui::Context::default();
    let mut views = FileViews::new(limits()).unwrap();
    let key = owner("first");
    views.bind(key.clone(), PATH).unwrap();
    let draw = |views: &mut FileViews, events| {
        let mut surface = None;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SURFACE_WIDTH, SURFACE_HEIGHT),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                surface = Some(views.show_surface(
                    ui,
                    &key,
                    FileSurface {
                        editor: &editor,
                        locale: &locale,
                        banner: &banner,
                    },
                    true,
                ))
            },
        );
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        output.textures_delta.clear();
        (surface.unwrap(), texts)
    };
    let (loading, texts) = draw(&mut views, vec![]);
    assert!(loading.unwrap().editor.is_none());
    assert!(texts.is_empty());
    assert!(views.store().documents().is_empty());
    opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
    let view = views.view(&key).unwrap();
    let document = views.store().views().get(view).unwrap().document;
    replace_selections(views.store_mut(), view, "draft ", None).unwrap();
    opened(&mut views, PATH, OLD_SEQ, file(PATH, "external"));
    views.take_events();
    let (surface, texts) = draw(&mut views, vec![]);
    assert!(texts.contains(&"changed".into()));
    assert!(texts.contains(&"draft disk".into()));
    let button = surface
        .unwrap()
        .banner
        .unwrap()
        .actions
        .into_iter()
        .find(|(action, _)| *action == BannerAction::ViewDisk)
        .unwrap()
        .1
        .center();
    let pointer = |pressed| egui::Event::PointerButton {
        pos: button,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    draw(
        &mut views,
        vec![egui::Event::PointerMoved(button), pointer(true)],
    )
    .0
    .unwrap();
    let (clicked, _) = draw(&mut views, vec![pointer(false)]);
    assert_eq!(
        clicked.unwrap().banner.unwrap().action,
        Some(BannerAction::ViewDisk)
    );
    let events = views.take_events();
    let [FileEvent::ChoiceRequested(request)] = events.as_slice() else {
        panic!("{events:?}")
    };
    let request = request.clone();
    views.choice_sent(request, FRESH_SEQ);
    draw(&mut views, vec![egui::Event::Text("blocked".into())])
        .0
        .unwrap();
    assert_eq!(
        views
            .store()
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "draft disk"
    );
    assert!(views.response(
        FRESH_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(file(PATH, "external")).unwrap()
        ))
    ));
    let (surface, _) = draw(&mut views, vec![]);
    assert!(surface.unwrap().banner.is_none());
    for lossy in [true, false] {
        let mut dto = file(PATH, "readonly");
        dto.read_only = true;
        dto.encoding_lossy = lossy;
        opened(&mut views, PATH, OPEN_SEQ, dto);
        let (surface, texts) = draw(&mut views, vec![egui::Event::Text("forbidden".into())]);
        assert!(!surface.unwrap().editor.unwrap().changed);
        assert!(texts.contains(&if lossy {
            "lossy readonly".into()
        } else {
            "large readonly".into()
        }));
        assert_eq!(
            views
                .store()
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "readonly"
        );
    }
    views.read_sent(PATH.into(), OLD_SEQ);
    assert!(views.response(OLD_SEQ, &Ok(ResponsePayload::Binary(vec![]))));
    let (surface, texts) = draw(&mut views, vec![]);
    assert!(matches!(
        surface,
        Err(FileFailure::Rpc(Failure::MalformedResponse))
    ));
    assert!(texts.contains(&"open failure".into()));
}

#[test]
fn 원격_저장_준비는_공유_cleanup과_editorconfig_false_및_clean_noop을_보존한다() {
    for (config, expected) in [
        (EditorConfigOptions::default(), "draft disk\r\nlast\r\n"),
        (
            EditorConfigOptions {
                trim_trailing_whitespace: Some(false),
                insert_final_newline: Some(false),
                ..Default::default()
            },
            "draft disk  \r\nlast  ",
        ),
    ] {
        let mut views = FileViews::new(limits()).unwrap();
        let key = owner("first");
        views.bind(key.clone(), PATH).unwrap();
        let mut dto = file(PATH, "disk  \r\nlast  ");
        dto.language_id = "plaintext".into();
        dto.editor_config = config;
        opened(&mut views, PATH, OPEN_SEQ, dto);
        let view = views.view(&key).unwrap();
        let flags = CleanupFlags {
            trim_trailing_whitespace: true,
            insert_final_newline: true,
        };
        assert!(
            views
                .prepare_save_after_format(view, flags, false)
                .unwrap()
                .is_none()
        );
        views.take_events();
        replace_selections(views.store_mut(), view, "draft ", None).unwrap();
        let request = views
            .prepare_save_after_format(view, flags, false)
            .unwrap()
            .unwrap();
        assert_eq!(
            request.call().args,
            json!({"path": PATH, "content": expected})
        );
        let document = views.store().views().get(view).unwrap().document;
        let snapshot = views.store().documents().snapshot(document).unwrap();
        assert_eq!(snapshot.rope.to_string(), expected);
        assert!(snapshot.dirty);
        let edits = views.take_events();
        if config.trim_trailing_whitespace == Some(false) {
            assert!(edits.is_empty());
        } else {
            assert_eq!(edits, [FileEvent::Edited { document, view }]);
        }
        views.save_sent(request, SAVE_SEQ);
        assert!(
            views
                .prepare_save_after_format(view, flags, false)
                .unwrap()
                .is_none()
        );
        assert!(views.response(SAVE_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
        assert_eq!(
            views.take_events(),
            [FileEvent::SaveFinished {
                path: PATH.into(),
                document,
                result: Ok(false),
            }]
        );
        assert!(!views.store().documents().snapshot(document).unwrap().dirty);
    }
}

#[test]
fn 원격_파일은_서버_경로와_같은_core_view_renderer_및_마지막_해제를_보존한다() {
    let mut views = FileViews::new(limits()).unwrap();
    let first = owner("first");
    let second = owner("second");
    assert!(views.bind(first.clone(), PATH).unwrap().is_none());
    assert!(views.bind(second.clone(), PATH).unwrap().is_none());
    assert_eq!(views.next_reads(), [PATH]);
    assert_eq!(FileViews::read_call(PATH).args, json!({"path": PATH}));
    opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
    let first_view = views.view(&first).unwrap();
    let second_view = views.view(&second).unwrap();
    assert_ne!(first_view, second_view);
    let document = views.store().views().get(first_view).unwrap().document;
    assert_eq!(
        views.store().views().get(second_view).unwrap().document,
        document
    );
    let editor = NativeEditor {
        appearance: EditorAppearance {
            font: FontId::monospace(FONT_SIZE),
            line_height: LINE_HEIGHT,
            horizontal_padding: PADDING,
            background: Color32::BLACK,
            foreground: Color32::WHITE,
            muted: Color32::GRAY,
            selection: Color32::BLUE,
            cursor: Color32::WHITE,
            current_line: Color32::BLACK,
            line_numbers: true,
            indent: "\t".into(),
        },
    };
    let context = egui::Context::default();
    let mut output = context.run_ui(egui::RawInput::default(), |ui| {
        assert!(views.show(ui, &editor, &first, true).unwrap().is_some());
    });
    assert!(output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "disk")
    ));
    output.textures_delta.clear();
    views.unbind(&first).unwrap();
    assert_eq!(views.store().documents().len(), 1);
    assert_eq!(views.store().views().len(), 1);
    views.unbind(&second).unwrap();
    assert!(views.store().documents().is_empty());
    assert!(views.store().views().is_empty());
    assert!(views.next_reads().is_empty());
    let mut native = EditorStore::new(limits()).unwrap();
    assert_eq!(
        native.open_file(PathBuf::from("relative.rs"), file(PATH, "x")),
        Err(EditorError::InvalidIdentity)
    );
    assert_eq!(
        native.open_file(PathBuf::from("/synthetic/../file.rs"), file(PATH, "x")),
        Err(EditorError::InvalidIdentity)
    );
    let mut invalid = file("", "x");
    assert_eq!(
        native.open_remote_file(invalid.clone(), None),
        Err(EditorError::InvalidIdentity)
    );
    invalid.path = "\0".into();
    assert_eq!(
        native.open_remote_file(invalid, None),
        Err(EditorError::InvalidIdentity)
    );
}

#[test]
fn 원격_파일_save_settle은_저장중_추가_edit와_늦은_read를_보존한다() {
    let mut views = FileViews::new(limits()).unwrap();
    let key = owner("first");
    views.bind(key.clone(), PATH).unwrap();
    opened(&mut views, PATH, OPEN_SEQ, file(PATH, "disk"));
    let view = views.view(&key).unwrap();
    let document = views.store().views().get(view).unwrap().document;
    views.take_events();
    views.retry(PATH);
    views.read_sent(PATH.into(), OLD_SEQ);
    replace_selections(views.store_mut(), view, "first ", None).unwrap();
    let save = views.prepare_save(view).unwrap().unwrap();
    assert_eq!(save.call().command, "file_save");
    assert_eq!(
        save.call().args,
        json!({"path": PATH, "content": "first disk"})
    );
    views.save_sent(save, SAVE_SEQ);
    assert!(views.prepare_save(view).unwrap().is_none());
    replace_selections(views.store_mut(), view, "later ", None).unwrap();
    assert!(views.response(SAVE_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
    let after = views.store().documents().snapshot(document).unwrap();
    assert_eq!(after.rope.to_string(), "first later disk");
    assert!(after.dirty);
    assert_eq!(after.metadata.disk_modified_ms, None);
    assert_eq!(
        views.take_events(),
        [FileEvent::SaveFinished {
            path: PATH.into(),
            document,
            result: Ok(true)
        }]
    );
    assert!(views.response(
        OLD_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(file(PATH, "stale disk")).unwrap()
        ))
    ));
    assert_eq!(
        views.store().documents().snapshot(document).unwrap().rope,
        after.rope
    );
    assert_eq!(views.next_reads(), [PATH]);
    opened(&mut views, PATH, FRESH_SEQ, file(PATH, "first disk"));
    assert!(!views.store().has_disk_conflict(document).unwrap());
    assert_eq!(
        views.store().documents().snapshot(document).unwrap().rope,
        after.rope
    );
    views.unbind(&key).unwrap();
    assert!(views.store().views().is_empty());
    assert_eq!(views.store().documents().len(), 1);
    views.bind(key.clone(), PATH).unwrap();
    assert_eq!(
        views
            .store()
            .views()
            .get(views.view(&key).unwrap())
            .unwrap()
            .document,
        document
    );
}

#[test]
fn 원격_파일은_readonly_오류_이전소유자_폐기와_재연결의_새_조회를_보존한다() {
    let mut views = FileViews::new(limits()).unwrap();
    let key = owner("first");
    views.bind(key.clone(), PATH).unwrap();
    views.read_sent(PATH.into(), OPEN_SEQ);
    assert!(views.response(OPEN_SEQ, &Ok(ResponsePayload::Binary(vec![]))));
    assert!(matches!(
        views.state(PATH),
        FileState::Failed(FileFailure::Rpc(Failure::MalformedResponse))
    ));
    assert!(views.next_reads().is_empty());
    views.retry(PATH);
    let mut readonly = file(PATH, "lossy");
    readonly.encoding_lossy = true;
    opened(&mut views, PATH, FRESH_SEQ, readonly);
    assert!(matches!(
        views.prepare_save(views.view(&key).unwrap()),
        Err(EditorError::ReadOnly)
    ));
    views.retry(PATH);
    views.read_sent(PATH.into(), OLD_SEQ);
    views.bind(key.clone(), SECOND_PATH).unwrap();
    assert!(!views.response(
        OLD_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(file(PATH, "late")).unwrap()
        ))
    ));
    assert!(views.store().documents().is_empty());
    opened(
        &mut views,
        SECOND_PATH,
        FRESH_SEQ,
        file(SECOND_PATH, "disk"),
    );
    let view = views.view(&key).unwrap();
    replace_selections(views.store_mut(), view, "draft ", None).unwrap();
    let save = views.prepare_save(view).unwrap().unwrap();
    views.save_sent(save, SAVE_SEQ);
    views.disconnected();
    assert!(views.next_reads().is_empty());
    assert!(!views.response(SAVE_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
    let events = views.take_events();
    assert!(matches!(
        events.last(),
        Some(FileEvent::SaveFinished {
            result: Err(FileFailure::Rpc(Failure::Invocation(InvokeError::Closed))),
            ..
        })
    ));
    let document = views.store().views().get(view).unwrap().document;
    assert!(views.store().documents().snapshot(document).unwrap().dirty);
    views.refresh();
    assert_eq!(views.next_reads(), [SECOND_PATH]);
}
