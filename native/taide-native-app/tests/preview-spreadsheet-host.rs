use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use eframe::egui::{self, Color32, Context, RawInput};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind};
use taide_model::ids::{ProjectId, TabId};
use taide_model::locale::ResolvedLocale;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::presentation::message;
use taide_native_app::preview::{Failure, MAX_TEXTURE_BYTES};
use taide_native_app::preview_spreadsheet::{Cell, Request, Sheet, Workbook};
use taide_native_app::preview_spreadsheet_cache::Cache;
use taide_native_app::preview_spreadsheet_surface::{self, Appearance};
use taide_native_app::preview_status;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;
use zip::write::SimpleFileOptions;

const TIMEOUT: Duration = Duration::from_secs(5);
const WIDTH: f32 = 640.0;
const HEIGHT: f32 = 360.0;
const PATH: &str = "synthetic.xlsx";
const BODY: &str = "A  \tB\n<tag>\u{a0}C";

#[test]
fn xlml_html_host는_xls_내용과_source_ready_공유_cache를_연결한다() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/spreadsheet-xlml-reference.json")).unwrap();
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-xlml-host-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("synthetic.xls").to_str().unwrap().to_owned();
    let invalid = root.join("doctype.xls").to_str().unwrap().to_owned();
    let html = root.join("table.xls").to_str().unwrap().to_owned();
    std::fs::write(&path, reference[0]["source"].as_str().unwrap()).unwrap();
    std::fs::write(
        &invalid,
        b"<!DOCTYPE Workbook SYSTEM \"file:///never-opened\"><Workbook/>",
    )
    .unwrap();
    std::fs::write(&html, b"<table><tr><td>HTML body</td><td>TRUE</td></tr></table><table><tr><td>7</td></tr></table>").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic XML".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect(
        services(state, tasks.clone(), Arc::new(Sink)),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    let mut cache = Cache::default();
    let tab = TabId::new();
    runtime.block_on(async {
        let request = cache.begin(&tab, &path).unwrap();
        let result = reply(&mut host, &ready, request.clone(), &mut cache).await;
        cache.accept(request, result, 0);
        let workbook = cache.workbook(&tab).unwrap();
        assert_eq!(
            workbook
                .sheets
                .iter()
                .map(|sheet| sheet.name.as_str())
                .collect::<Vec<_>>(),
            ["타입", "Offset", "Empty", "Tall"]
        );
        assert_eq!(workbook.sheets[0].rows[0][0], Cell::Text("한글".into()));
        assert_eq!(workbook.sheets[0].rows[0][2], Cell::Number(42.5));
        assert_eq!(workbook.sheets[3].rows.len(), 500);
        assert_eq!(workbook.sheets[3].total_row_count, 503);
        let peer = TabId::new();
        assert!(cache.begin(&peer, &path).is_none());
        assert!(Arc::ptr_eq(
            cache.workbook(&tab).unwrap(),
            cache.workbook(&peer).unwrap()
        ));
        let bad_tab = TabId::new();
        let request = cache.begin(&bad_tab, &invalid).unwrap();
        let result = reply(&mut host, &ready, request.clone(), &mut cache).await;
        assert!(matches!(result, Err(Failure::Decode(_))));
        cache.accept(request, result, 0);
        assert!(cache.workbook(&bad_tab).is_none());
        let html_tab = TabId::new();
        let request = cache.begin(&html_tab, &html).unwrap();
        let result = reply(&mut host, &ready, request.clone(), &mut cache).await;
        cache.accept(request, result, 0);
        let workbook = cache.workbook(&html_tab).unwrap();
        assert_eq!(workbook.sheets.len(), 2);
        assert_eq!(workbook.sheets[0].name, "Sheet1");
        assert_eq!(workbook.sheets[1].name, "Sheet2");
        assert_eq!(
            workbook.sheets[0].rows[0],
            [Cell::Text("HTML body".into()), Cell::Boolean(true)]
        );
        assert_eq!(workbook.sheets[1].rows[0], [Cell::Number(7.0)]);
        cache.reconcile(&HashMap::new());
        assert_eq!(cache.retained_bytes(), 0);
        tokio::time::timeout(TIMEOUT, host.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn xls_host는_원본_ole와_raw_biff의_승인_read_progress와_공유_cache를_연결한다() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/spreadsheet-xls-reference.json")).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(reference[1]["encoded"].as_str().unwrap())
        .unwrap();
    let stream = taide_native_app::preview_spreadsheet_xls::workbook_stream(&bytes).unwrap();
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-xls-host-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("synthetic.xls").to_str().unwrap().to_owned();
    let raw = root.join("synthetic.biff").to_str().unwrap().to_owned();
    let outside = fixture.0.join("outside.xls").to_str().unwrap().to_owned();
    std::fs::write(&path, &bytes).unwrap();
    std::fs::write(&raw, &stream).unwrap();
    std::fs::write(&outside, &bytes).unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic XLS".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect(
        services(state, tasks.clone(), Arc::new(Sink)),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    let mut cache = Cache::default();
    let tab = TabId::new();
    let raw_tab = TabId::new();
    runtime.block_on(async {
        for (tab, path) in [(&tab, &path), (&raw_tab, &raw)] {
            let request = cache.begin(tab, path).unwrap();
            let result = reply(&mut host, &ready, request.clone(), &mut cache).await;
            cache.accept(request, result, 0);
            let workbook = cache.workbook(tab).unwrap();
            assert_eq!(
                workbook
                    .sheets
                    .iter()
                    .map(|sheet| sheet.name.as_str())
                    .collect::<Vec<_>>(),
                ["타입", "offset", "empty", "blank", "tall"]
            );
            assert_eq!(
                workbook.sheets[0].rows[0][0],
                Cell::Text("한글 <tag>".into())
            );
            assert_eq!(workbook.sheets[0].rows[0][1], Cell::Boolean(true));
            assert_eq!(workbook.sheets[4].rows.len(), 500);
            assert_eq!(workbook.sheets[4].total_row_count, 503);
        }
        assert_eq!(
            cache.workbook(&tab).unwrap().as_ref(),
            cache.workbook(&raw_tab).unwrap().as_ref()
        );
        let peer = TabId::new();
        assert!(cache.begin(&peer, &path).is_none());
        assert!(Arc::ptr_eq(
            cache.workbook(&tab).unwrap(),
            cache.workbook(&peer).unwrap()
        ));
        let request = Request {
            path: outside,
            token: 0,
        };
        assert!(matches!(
            reply(&mut host, &ready, request, &mut cache).await,
            Err(Failure::Read(_))
        ));
        cache.reconcile(&HashMap::new());
        assert_eq!(cache.retained_bytes(), 0);
        tokio::time::timeout(TIMEOUT, host.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn csv_host의_내용판별과_공유_열폭_수명_accesskit_tab_table을_검사한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-csv-host-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("synthetic.csv").to_str().unwrap().to_owned();
    std::fs::write(&path, b"PKlabel,TRUE\n0xFF,\"<tag>\"\n").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic CSV".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect(
        services(state.clone(), tasks.clone(), Arc::new(Sink)),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    let mut cache = Cache::default();
    let tab = TabId::new();
    runtime.block_on(async {
        let request = cache.begin(&tab, &path).unwrap();
        let result = reply(&mut host, &ready, request.clone(), &mut cache).await;
        cache.accept(request, result, 0);
        assert_eq!(
            cache.workbook(&tab).unwrap().sheets[0].rows,
            [
                vec![Cell::Text("PKlabel".into()), Cell::Boolean(true)],
                vec![Cell::Number(255.0), Cell::Text("<tag>".into())],
            ]
        );
        let reserved = cache.retained_bytes();
        assert!(reserved > cache.workbook(&tab).unwrap().retained_bytes());
        let context = Context::default();
        context.enable_accesskit();
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(WIDTH, HEIGHT),
                )),
                ..Default::default()
            },
            |ui| {
                preview_spreadsheet_surface::show(
                    ui,
                    &mut cache,
                    &tab,
                    &path,
                    "synthetic.csv",
                    &locale("en"),
                    &Appearance {
                        status: preview_status::Appearance {
                            background: Color32::BLACK,
                            border: Color32::GRAY,
                            foreground: Color32::WHITE,
                            muted: Color32::LIGHT_GRAY,
                        },
                        header: Color32::DARK_GRAY,
                        selected_background: Color32::BLUE,
                        selected_foreground: Color32::WHITE,
                        inactive_foreground: Color32::LIGHT_GRAY,
                        hover: Color32::GRAY,
                        cell_border: Color32::GRAY,
                        warning: Color32::YELLOW,
                    },
                );
            },
        );
        let nodes = &output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        assert!(
            nodes
                .iter()
                .any(|(_, node)| node.role() == egui::accesskit::Role::TabList)
        );
        assert!(
            nodes
                .iter()
                .any(|(_, node)| node.role() == egui::accesskit::Role::Tab
                    && node.label() == Some("Sheet1")
                    && node.is_selected() == Some(true))
        );
        assert!(
            nodes
                .iter()
                .any(|(_, node)| node.role() == egui::accesskit::Role::Table
                    && node.row_count() == Some(2)
                    && node.column_count() == Some(2))
        );
        assert_eq!(
            nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::accesskit::Role::Cell)
                .count(),
            4
        );
        output.textures_delta.clear();
        assert_eq!(cache.retained_bytes(), reserved);
        cache.reconcile(&HashMap::new());
        assert_eq!(cache.retained_bytes(), 0);
        assert!(cache.workbook(&tab).is_none());
        assert!(taide_native_app::preview_spreadsheet::decode(b"\x09\x08legacy BIFF").is_err());
        assert!(
            taide_native_app::preview_spreadsheet::decode(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1")
                .is_err()
        );
        assert!(taide_native_app::preview_spreadsheet::decode(b"PK\x03\x04broken").is_err());
        tokio::time::timeout(TIMEOUT, host.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

fn locale(id: &str) -> ResolvedLocale {
    let source = match id {
        "ko" => include_str!("../../../crates/taide-locale/resources/locales/ko.json"),
        "ja" => include_str!("../../../crates/taide-locale/resources/locales/ja.json"),
        _ => include_str!("../../../crates/taide-locale/resources/locales/en.json"),
    };
    ResolvedLocale {
        id: id.into(),
        name: id.into(),
        messages: serde_json::from_str(source).unwrap(),
        warnings: Vec::new(),
    }
}

fn surface(
    context: &Context,
    cache: &mut Cache,
    tab: &TabId,
    locale: &ResolvedLocale,
    key: Option<egui::Id>,
) -> (bool, Vec<egui::epaint::TextShape>) {
    let events = if let Some(id) = key {
        context.memory_mut(|memory| memory.request_focus(id));
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }]
    } else {
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Default::default(),
        }]
    };
    let mut opened = false;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(WIDTH, HEIGHT),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            opened = preview_spreadsheet_surface::show(
                ui,
                cache,
                tab,
                PATH,
                PATH,
                locale,
                &Appearance {
                    status: preview_status::Appearance {
                        background: Color32::BLACK,
                        border: Color32::GRAY,
                        foreground: Color32::WHITE,
                        muted: Color32::LIGHT_GRAY,
                    },
                    header: Color32::DARK_GRAY,
                    selected_background: Color32::BLUE,
                    selected_foreground: Color32::WHITE,
                    inactive_foreground: Color32::LIGHT_GRAY,
                    hover: Color32::GRAY,
                    cell_border: Color32::GRAY,
                    warning: Color32::YELLOW,
                },
            );
        },
    );
    let text = output
        .shapes
        .iter()
        .filter_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect();
    output.textures_delta.clear();
    (opened, text)
}

struct Fixture(std::path::PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

fn document(value: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in [
        ("[Content_Types].xml", r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/></Types>"#.to_owned()),
        ("_rels/.rels", r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_owned()),
        ("xl/workbook.xml", r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="First" sheetId="1" r:id="rId1"/><sheet name="Empty" sheetId="2" r:id="rId2"/><sheet name="Last" sheetId="3" r:id="rId3"/></sheets></workbook>"#.to_owned()),
        ("xl/_rels/workbook.xml.rels", r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet3.xml"/></Relationships>"#.to_owned()),
        ("xl/worksheets/sheet1.xml", format!(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>{value}</t></is></c></row></sheetData></worksheet>"#)),
        ("xl/worksheets/sheet2.xml", r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></worksheet>"#.to_owned()),
        ("xl/worksheets/sheet3.xml", r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>3</v></c></row></sheetData></worksheet>"#.to_owned()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(data.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn book() -> Workbook {
    Workbook {
        sheets: vec![
            Sheet {
                name: "First".into(),
                rows: vec![vec![
                    Cell::Text(BODY.into()),
                    Cell::Number(1e21),
                    Cell::Boolean(true),
                    Cell::Null,
                ]],
                total_row_count: 1,
                truncated: false,
            },
            Sheet {
                name: "Empty".into(),
                rows: Vec::new(),
                total_row_count: 0,
                truncated: false,
            },
        ],
    }
}

async fn reply(
    host: &mut HostBridge,
    ready: &Notify,
    request: Request,
    cache: &mut Cache,
) -> Result<Workbook, Failure> {
    host.submit(HostCommand::ReadSpreadsheetPreview(request.clone()))
        .unwrap();
    tokio::time::timeout(TIMEOUT, async {
        let mut saw_source = false;
        loop {
            match host.poll() {
                Some(HostReply::SpreadsheetSourceReady(progress)) => {
                    assert_eq!(progress, request);
                    assert!(!saw_source);
                    saw_source = true;
                    cache.source_ready(&progress);
                }
                Some(HostReply::SpreadsheetPreview {
                    request: returned,
                    result,
                }) => {
                    assert_eq!(returned, request);
                    assert_eq!(saw_source, !matches!(result, Err(Failure::Read(_))));
                    return result;
                }
                Some(_) => panic!("unexpected spreadsheet reply"),
                None => ready.notified().await,
            }
        }
    })
    .await
    .unwrap()
}

#[test]
fn 실제_host_spreadsheet의_승인_progress_공유_data_선택유지_stale_close를_검사한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-spreadsheet-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join(PATH).to_str().unwrap().to_owned();
    let moved = root.join("moved.xlsx").to_str().unwrap().to_owned();
    let outside = fixture.0.join("outside.xlsx").to_str().unwrap().to_owned();
    let broken = root.join("broken.xlsx").to_str().unwrap().to_owned();
    std::fs::write(&path, document("first")).unwrap();
    std::fs::write(&outside, document("outside")).unwrap();
    std::fs::write(&broken, b"PK\x03\x04broken").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic spreadsheet".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect(
        services(state.clone(), tasks.clone(), Arc::new(Sink)),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    let mut cache = Cache::default();
    let tab = TabId::new();
    let side = TabId::new();
    runtime.block_on(async {
        let request = cache.begin(&tab, &path).unwrap();
        assert!(cache.begin(&side, &path).is_none());
        assert!(!cache.has_source(&tab));
        let result = reply(&mut host, &ready, request.clone(), &mut cache).await;
        assert!(cache.has_source(&tab));
        cache.accept(request, result, 0);
        assert_eq!(cache.workbook(&tab).unwrap().sheets.len(), 3);
        assert!(Arc::ptr_eq(
            cache.workbook(&tab).unwrap(),
            cache.workbook(&side).unwrap()
        ));
        assert!(cache.retained_bytes() >= cache.workbook(&tab).unwrap().retained_bytes());
        assert!(cache.select(&tab, 2));
        assert_eq!(cache.selected(&side), Some(0));
        assert!(!cache.select(&tab, 3));
        let old = cache.workbook(&tab).unwrap().clone();
        std::fs::write(&path, document("replacement")).unwrap();
        cache.invalidate_root(fixture.0.join("roo").to_str().unwrap());
        assert!(cache.begin(&tab, &path).is_none());
        cache.invalidate(&path);
        assert_eq!(cache.selected(&tab), Some(2));
        let stale = cache.begin(&tab, &path).unwrap();
        let stale_result = reply(&mut host, &ready, stale.clone(), &mut cache).await;
        assert!(Arc::ptr_eq(&old, cache.workbook(&tab).unwrap()));
        cache.invalidate(&path);
        cache.accept(stale.clone(), stale_result, 0);
        assert!(Arc::ptr_eq(&old, cache.workbook(&tab).unwrap()));
        let current = cache.begin(&tab, &path).unwrap();
        assert!(current.token > stale.token);
        cache.source_ready(&stale);
        assert!(!cache.has_source(&tab));
        let result = reply(&mut host, &ready, current.clone(), &mut cache).await;
        assert_eq!(cache.selected(&tab), Some(2));
        cache.accept(current, result, 0);
        assert_eq!(
            cache.workbook(&tab).unwrap().sheets[0].rows,
            [vec![Cell::Text("replacement".into())]]
        );
        assert_eq!(cache.selected(&tab), Some(2));
        assert_eq!(cache.selected(&side), Some(0));
        cache.invalidate_root(root.to_str().unwrap());
        let cancelled = cache.begin(&tab, &path).unwrap();
        cache.cancelled(&cancelled);
        let retry = cache.begin(&tab, &path).unwrap();
        cache.accept(cancelled, Ok(book()), 0);
        assert!(cache.begin(&side, &path).is_none());
        cache.reset_pending();
        cache.source_ready(&retry);
        assert!(!cache.has_source(&tab));
        let moving = cache.begin(&tab, &path).unwrap();
        let result = reply(&mut host, &ready, moving.clone(), &mut cache).await;
        std::fs::rename(&path, &moved).unwrap();
        cache.reconcile(&HashMap::from([(tab.clone(), moved.clone())]));
        cache.accept(moving, result, 0);
        assert!(cache.workbook(&tab).is_none());
        assert!(cache.workbook(&side).is_none());
        assert_eq!(cache.retained_bytes(), 0);
        let moved_request = cache.begin(&tab, &moved).unwrap();
        let result = reply(&mut host, &ready, moved_request.clone(), &mut cache).await;
        cache.accept(moved_request, result, 0);
        assert_eq!(
            cache.workbook(&tab).unwrap().sheets[0].rows,
            [vec![Cell::Text("replacement".into())]]
        );
        for (path, expected_read) in [(outside, true), (PATH.into(), true), (broken, false)] {
            let request = Request { path, token: 1 };
            match (
                reply(&mut host, &ready, request, &mut cache).await,
                expected_read,
            ) {
                (Err(Failure::Read(error)), true) => {
                    assert_eq!(error.kind(), AppErrorKind::Forbidden)
                }
                (Err(Failure::Decode(error)), false) => {
                    assert_eq!(error.kind(), AppErrorKind::InvalidArgument)
                }
                result => panic!("wrong failure stage: {result:?}"),
            }
        }
        cache.invalidate_all();
        let closed = cache.begin(&tab, &moved).unwrap();
        state.projects.write().remove(&project);
        let result = reply(&mut host, &ready, closed.clone(), &mut cache).await;
        assert!(
            matches!(&result, Err(Failure::Read(error)) if error.kind() == AppErrorKind::Forbidden)
        );
        cache.reconcile(&HashMap::new());
        cache.accept(closed, result, 0);
        assert_eq!(cache.retained_bytes(), 0);
        tokio::time::timeout(TIMEOUT, host.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn spreadsheet_surface의_키보드_표_empty_locale_선택유지_clamp_실패와_admission을_검사한다() {
    for id in ["en", "ko", "ja"] {
        let context = Context::default();
        let mut cache = Cache::default();
        let tab = TabId::new();
        let locale = locale(id);
        let (_, raw) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(raw.is_empty());
        let request = cache.begin(&tab, PATH).unwrap();
        cache.source_ready(&request);
        assert!(
            surface(&context, &mut cache, &tab, &locale, None)
                .1
                .is_empty()
        );
        cache.accept(request, Ok(book()), 0);
        let (_, ready) = surface(&context, &mut cache, &tab, &locale, None);
        let body = ready
            .iter()
            .find(|text| text.galley.text() == "A B <tag>\u{a0}C")
            .unwrap();
        assert_eq!(body.galley.job.sections[0].format.font_id.size, 12.0);
        assert!(ready.iter().any(|text| text.galley.text() == "1e+21"));
        assert!(ready.iter().any(|text| text.galley.text() == "true"));
        assert!(ready.iter().any(|text| text.galley.text() == "First"));
        assert!(ready.iter().any(|text| text.galley.text() == "Empty"));
        let second = preview_spreadsheet_surface::sheet_id(egui::ViewportId::ROOT, &tab, 1);
        surface(&context, &mut cache, &tab, &locale, Some(second));
        assert_eq!(cache.selected(&tab), Some(1));
        let (_, empty) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(
            empty.iter().any(|text| text.galley.text()
                == message(&locale, "preview.spreadsheet.emptySheet", &[]))
        );
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        let mut smaller = book();
        smaller.sheets.truncate(1);
        cache.accept(request, Ok(smaller), 0);
        assert_eq!(cache.selected(&tab), Some(0));
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.accept(request, Ok(book()), 0);
        assert_eq!(cache.selected(&tab), Some(1));
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.accept(request, Ok(Workbook { sheets: Vec::new() }), 0);
        let (_, no_sheets) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(no_sheets.iter().any(
            |text| text.galley.text() == message(&locale, "preview.spreadsheet.noSheets", &[])
        ));
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.accept(request, Ok(book()), 0);
        assert_eq!(cache.selected(&tab), Some(1));
        cache.select(&tab, 0);
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.accept(
            request,
            Ok(Workbook {
                sheets: vec![Sheet {
                    name: "Large".into(),
                    rows: vec![
                        vec![Cell::Text("visible".into())];
                        taide_native_app::preview_spreadsheet::MAX_PREVIEW_ROWS
                    ],
                    total_row_count: 503,
                    truncated: true,
                }],
            }),
            0,
        );
        let (_, large) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(large.iter().any(|text| text.galley.text()
            == message(
                &locale,
                "preview.spreadsheet.truncatedNotice",
                &[("shown", "500"), ("total", "503")]
            )));
        let visible_rows = large
            .iter()
            .filter(|text| text.galley.text() == "visible")
            .count();
        assert!(visible_rows > 0 && visible_rows < 20);
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.accept(
            request,
            Err(Failure::Decode(AppError::InvalidArgument(
                "synthetic".into(),
            ))),
            0,
        );
        let (_, failed) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(
            failed.iter().any(|text| text.galley.text()
                == message(&locale, "preview.spreadsheet.loadFailed", &[]))
        );
        let external = preview_status::control_id(
            egui::ViewportId::ROOT,
            &tab,
            preview_status::EXTERNAL_ACTION_KEY,
        );
        assert!(surface(&context, &mut cache, &tab, &locale, Some(external)).0);
        surface(&context, &mut cache, &tab, &locale, None);
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.accept(
            request,
            Err(Failure::Read(AppError::Forbidden("synthetic".into()))),
            0,
        );
        let (_, failed) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(
            failed
                .iter()
                .any(|text| text.galley.text() == message(&locale, "preview.notSupported", &[]))
        );
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.source_ready(&request);
        assert!(cache.error(&tab).is_none());
        cache.accept(request, Ok(book()), MAX_TEXTURE_BYTES);
        assert!(matches!(cache.error(&tab), Some(Failure::Decode(_))));
        assert_eq!(cache.retained_bytes(), 0);
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        let mut invalid = book();
        invalid.sheets[0].total_row_count = 0;
        cache.accept(request, Ok(invalid), 0);
        assert!(cache.workbook(&tab).is_none());
    }
}
