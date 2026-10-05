use eframe::egui::Context;
use taide_model::app_event::AppEvent;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::ids::ProjectId;
use taide_native_app::events::{MAX_PENDING_PREVIEW_PATHS, TreeChanges};
use taide_native_app::preview::{Cache, Raster};

const MAX_SIDE: usize = 1;

#[test]
fn preview는_self_write도_갱신하고_상한_rescan_root_및_inflight를_분리한다() {
    let changes = TreeChanges::default();
    let project = ProjectId::new();
    changes.record(&AppEvent::FsChanged {
        project_id: project.clone(),
        change: FsChange {
            kind: FsChangeKind::Modified,
            paths: vec!["/root/a.png".into()],
            from_app: true,
        },
    });
    assert!(changes.take([project.clone()]).is_empty());
    let batch = changes.take_previews();
    assert_eq!(
        batch.paths.into_iter().collect::<Vec<_>>(),
        vec!["/root/a.png"]
    );
    assert!(!batch.all);
    assert!(changes.take_previews().paths.is_empty());
    changes.record(&AppEvent::FsRescanRequired {
        project_id: project.clone(),
    });
    assert!(changes.take_previews().projects.contains(&project));
    changes.record(&AppEvent::FsChanged {
        project_id: project,
        change: FsChange {
            kind: FsChangeKind::Created,
            paths: (0..=MAX_PENDING_PREVIEW_PATHS)
                .map(|index| format!("/root/{index}.png"))
                .collect(),
            from_app: false,
        },
    });
    let batch = changes.take_previews();
    assert!(batch.all);
    assert!(batch.paths.is_empty());
    assert!(batch.projects.is_empty());
    let context = Context::default();
    let mut cache = Cache::default();
    let request = cache.begin("/rootish/a.png", MAX_SIDE).unwrap();
    cache.accept(
        &context,
        request,
        Ok(Raster {
            size: [1, 1],
            rgba: vec![0, 0, 0, 0],
            animation: None,
        }),
    );
    let request = cache.begin("/root/a.png", MAX_SIDE).unwrap();
    cache.invalidate_root("/root");
    assert!(cache.begin("/other/a.png", MAX_SIDE).is_none());
    cache.accept(
        &context,
        request,
        Ok(Raster {
            size: [1, 1],
            rgba: vec![0, 0, 0, 0],
            animation: None,
        }),
    );
    assert!(cache.texture("/root/a.png").is_none());
    assert!(cache.texture("/rootish/a.png").is_some());
    assert!(cache.begin("/root/a.png", MAX_SIDE).is_some());
    cache.reset_pending();
    cache.invalidate_all();
    assert!(cache.texture("/rootish/a.png").is_none());
}
