use axum::extract::State;
use axum::http::{StatusCode, Uri, header};
use http_body_util::BodyExt;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_runtime::AppState;

use super::*;
use crate::bootstrap;
use crate::remote_http::{Context, Ports};
use crate::remote_serving::serve_static;

const PUBLIC: &[(&str, &[u8], &str)] = &[
    (
        "index.html",
        b"<!doctype html><title>synthetic</title>",
        "text/html",
    ),
    (
        "assets/main.js",
        b"export const synthetic = true",
        "text/javascript",
    ),
    (
        "assets/style.css",
        b"body { color: black }",
        "text/css; charset=utf-8",
    ),
    (
        "assets/data.json",
        b"{\"synthetic\":true}",
        "application/json",
    ),
    ("assets/app.wasm", b"\0asm\x01\0\0\0", "application/wasm"),
    ("assets/font.ttf", b"synthetic font", "font/ttf"),
];

struct Directory(PathBuf);

struct Sink;

impl taide_runtime::EventSink for Sink {
    fn publish(&self, _event: taide_model::app_event::AppEvent) {}
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn 생산용_bundle은_경로_manifest_entry_상한을_검증하고_메타데이터를_제공하지_않는다() {
    let directory =
        Directory(std::env::temp_dir().join(format!("taide-native-bundle-{}", ProjectId::new())));
    let executable = directory.0.join("bin/taide");
    let packaged = Catalog::packaged(&executable).unwrap();
    let root = directory.0.join("bin/remote-public");
    assert_eq!(packaged.root, root);
    let mac = Catalog::packaged(&directory.0.join("TAIDE.app/Contents/MacOS/taide")).unwrap();
    assert_eq!(
        mac.root,
        directory
            .0
            .join("TAIDE.app/Contents/Resources/remote-public")
    );
    for invalid_path in [
        Path::new("taide"),
        Path::new("/"),
        Path::new("/taide"),
        Path::new("/synthetic/../taide"),
    ] {
        assert!(Catalog::packaged(invalid_path).is_err());
    }
    std::fs::create_dir_all(root.join("assets")).unwrap();
    for (name, bytes, _) in PUBLIC {
        std::fs::write(root.join(name), bytes).unwrap();
    }
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let entries = || {
        PUBLIC
            .iter()
            .map(|(name, _, _)| (*name).to_string())
            .collect::<Vec<_>>()
    };
    let metadata = || serde_json::json!({"format":BUNDLE_FORMAT,"entryWasm":"assets/app.wasm","files":entries()});
    let fresh = || {
        Arc::new(Catalog::bundle(
            root.clone(),
            Limits {
                payload_bytes: PACKAGED_PAYLOAD_BYTES,
                count: PACKAGED_ASSET_COUNT,
            },
        ))
    };
    let catalog = fresh();
    assert!(catalog.clone().prepare(tasks.clone()).await.is_err());
    assert!(catalog.resolver()(INDEX_DOCUMENT).is_none());
    for bad in [
        serde_json::json!({"format":BUNDLE_FORMAT,"entryWasm":"assets/app.wasm","files":entries(),"unknown":true}),
        serde_json::json!({"format":"legacy","entryWasm":"assets/app.wasm","files":entries()}),
        serde_json::json!({"format":BUNDLE_FORMAT,"entryWasm":"assets/main.js","files":entries()}),
        serde_json::json!({"format":BUNDLE_FORMAT,"entryWasm":"missing.wasm","files":entries()}),
        serde_json::json!({"format":BUNDLE_FORMAT,"entryWasm":"../outside.wasm","files":[INDEX_DOCUMENT,"../outside.wasm"]}),
        serde_json::json!({"format":BUNDLE_FORMAT,"entryWasm":"assets/app.wasm","files":[INDEX_DOCUMENT,INDEX_DOCUMENT,"assets/app.wasm"]}),
        serde_json::json!({"format":BUNDLE_FORMAT,"entryWasm":"assets/app.wasm","files":[INDEX_DOCUMENT,"assets/app.wasm",BUNDLE_MANIFEST]}),
    ] {
        std::fs::write(
            root.join(BUNDLE_MANIFEST),
            serde_json::to_vec(&bad).unwrap(),
        )
        .unwrap();
        let catalog = fresh();
        assert!(matches!(
            catalog.clone().prepare(tasks.clone()).await,
            Err(AppError::InvalidArgument(_))
        ));
        assert!(catalog.resolver()(INDEX_DOCUMENT).is_none());
    }
    let sparse = std::fs::File::create(root.join(BUNDLE_MANIFEST)).unwrap();
    sparse.set_len((MANIFEST_BYTES + 1) as u64).unwrap();
    drop(sparse);
    assert!(fresh().prepare(tasks.clone()).await.is_err());
    std::fs::write(
        root.join(BUNDLE_MANIFEST),
        serde_json::to_vec(&metadata()).unwrap(),
    )
    .unwrap();
    std::fs::write(root.join("assets/app.wasm"), b"not wasm").unwrap();
    assert!(fresh().prepare(tasks.clone()).await.is_err());
    std::fs::write(root.join("assets/app.wasm"), WASM_HEADER).unwrap();
    let outside = directory.0.join("outside-manifest.json");
    std::fs::rename(root.join(BUNDLE_MANIFEST), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.join(BUNDLE_MANIFEST)).unwrap();
    assert!(fresh().prepare(tasks.clone()).await.is_err());
    std::fs::remove_file(root.join(BUNDLE_MANIFEST)).unwrap();
    std::fs::rename(outside, root.join(BUNDLE_MANIFEST)).unwrap();
    let catalog = Arc::new(packaged);
    catalog.clone().prepare(tasks.clone()).await.unwrap();
    let resolver = catalog.resolver();
    for (name, bytes, _) in PUBLIC {
        assert_eq!(resolver(name).unwrap().bytes, *bytes);
    }
    assert!(resolver(BUNDLE_MANIFEST).is_none());
    tasks.shutdown().await;
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 공개_manifest는_상한과_anchor를_검사하고_고정된_자산만_http에_제공한다() {
    let directory = Directory(
        std::env::temp_dir().join(format!("taide-native-remote-assets-{}", ProjectId::new())),
    );
    let root = directory.0.join("public");
    std::fs::create_dir_all(root.join("assets")).unwrap();
    for (name, bytes, _) in PUBLIC {
        std::fs::write(root.join(name), bytes).unwrap();
    }
    std::fs::write(root.join("unlisted.txt"), b"synthetic unlisted").unwrap();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let manifest = || PUBLIC.iter().map(|(name, _, _)| (*name).into()).collect();
    let payload_bytes = PUBLIC.iter().map(|(_, bytes, _)| bytes.len()).sum();
    let limits = || Limits {
        payload_bytes,
        count: PUBLIC.len(),
    };
    let resolver = load(&tasks, root.clone(), manifest(), limits())
        .await
        .unwrap();
    for (name, bytes, mime) in PUBLIC {
        let asset = resolver(name).unwrap();
        assert_eq!(asset.bytes, *bytes);
        assert_eq!(asset.mime, *mime);
    }
    for name in ["unlisted.txt", "unknown", "../index.html", "/index.html"] {
        assert!(resolver(name).is_none());
    }
    for bad in [
        "../outside.html",
        "/absolute.html",
        "assets//file.js",
        "assets/./file.js",
        "assets\\file.js",
        "file.js?query",
        "file.js#hash",
        "file.js\0",
        "disk:file.js",
        "unsupported.xyz",
    ] {
        assert!(matches!(
            load(
                &tasks,
                directory.0.join("absent"),
                vec![INDEX_DOCUMENT.into(), bad.into()],
                limits()
            )
            .await,
            Err(AppError::InvalidArgument(_))
        ));
    }
    for entries in [
        Vec::new(),
        vec!["assets/main.js".into()],
        vec![INDEX_DOCUMENT.into(), INDEX_DOCUMENT.into()],
    ] {
        assert!(matches!(
            load(&tasks, root.clone(), entries, limits()).await,
            Err(AppError::InvalidArgument(_))
        ));
    }
    for quota in [
        Limits {
            payload_bytes: payload_bytes - 1,
            count: PUBLIC.len(),
        },
        Limits {
            payload_bytes,
            count: PUBLIC.len() - 1,
        },
        Limits {
            payload_bytes: 0,
            count: PUBLIC.len(),
        },
    ] {
        assert!(matches!(
            load(&tasks, root.clone(), manifest(), quota).await,
            Err(AppError::InvalidArgument(_))
        ));
    }
    std::fs::write(directory.0.join("outside.js"), b"synthetic outside").unwrap();
    std::os::unix::fs::symlink(directory.0.join("outside.js"), root.join("linked.js")).unwrap();
    std::os::unix::fs::symlink(root.join("assets"), root.join("linked-directory")).unwrap();
    for name in ["linked.js", "linked-directory/main.js", "missing.js"] {
        assert!(
            load(
                &tasks,
                root.clone(),
                vec![INDEX_DOCUMENT.into(), name.into()],
                limits()
            )
            .await
            .is_err()
        );
    }
    std::fs::write(root.join("index.html"), b"changed source").unwrap();
    std::fs::rename(&root, directory.0.join("old-public")).unwrap();
    assert!(
        load(&tasks, root.clone(), manifest(), limits())
            .await
            .is_err()
    );
    let services = bootstrap::services(
        AppState::new(AppPaths::new(directory.0.join("data"))),
        tasks.clone(),
        Arc::new(Sink),
    );
    let owner = Arc::downgrade(&services);
    let asset_owner = Arc::downgrade(&resolver);
    let context = Context {
        services,
        ports: Arc::new(Ports {
            assets: resolver.clone(),
            socket: Arc::new(|_, _, _| {
                Box::pin(async { panic!("static handler must not use socket") })
            }),
        }),
    };
    for (uri, bytes, mime) in PUBLIC
        .iter()
        .map(|(name, bytes, mime)| (format!("/{name}"), *bytes, *mime))
        .chain([
            ("/".into(), PUBLIC[0].1, PUBLIC[0].2),
            ("/unlisted.txt".into(), PUBLIC[0].1, PUBLIC[0].2),
        ])
    {
        let response = serve_static(State(context.clone()), uri.parse::<Uri>().unwrap()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], mime);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(
            response.headers()[header::CONTENT_SECURITY_POLICY]
                .to_str()
                .unwrap()
                .contains("script-src 'self'")
        );
        assert_eq!(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .as_ref(),
            bytes
        );
    }
    drop(context);
    drop(resolver);
    assert!(asset_owner.upgrade().is_none());
    assert!(owner.upgrade().is_none());
    tasks.shutdown().await;
    assert_eq!(tasks.tracked_count(), 0);
    assert!(load(&tasks, root, manifest(), limits()).await.is_err());
}
