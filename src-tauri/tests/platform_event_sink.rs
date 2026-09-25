use std::sync::Mutex;

use taide_lib::domain::layout::service::{default_layout, finish_mutation};
use taide_lib::paths::AppPaths;
use taide_lib::state::AppState;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_runtime::EventSink;

#[derive(Default)]
struct RecordingEventSink(Mutex<Vec<AppEvent>>);

impl EventSink for RecordingEventSink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[test]
fn layout_이벤트는_tauri_없는_port로_발행된다() {
    let sink = RecordingEventSink::default();
    let event = AppEvent::LayoutChanged {
        project_id: ProjectId::from("prj-event-sink".to_string()),
        revision: 1,
    };

    sink.publish(event.clone());

    assert_eq!(sink.0.lock().unwrap().as_slice(), &[event]);
}

#[test]
fn layout_변경_완료는_주입된_port에_현재_revision을_발행한다() {
    let sink = RecordingEventSink::default();
    let state = AppState::new(AppPaths::new(std::env::temp_dir()));
    let project_id = ProjectId::from("prj-layout-event-sink".to_string());
    let mut layout = default_layout();
    layout.revision += 1;

    let snapshot = finish_mutation(&sink, &state, &project_id, &mut layout);

    assert_eq!(snapshot.revision, layout.revision);
    assert!(state.dirty_layouts.read().contains(&project_id));
    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[AppEvent::LayoutChanged {
            project_id,
            revision: snapshot.revision,
        }]
    );
}

#[test]
fn 앱과_두_layout_발행_경로는_platform_adapter를_사용한다() {
    let app_source = include_str!("../src/lib.rs");
    let production_app_source = app_source.split_once("#[cfg(test)]\nmod tests").unwrap().0;
    let normalized_app_source = production_app_source.split_whitespace().collect::<Vec<_>>().join("");
    let layout_source = include_str!("../src/domain/layout/service.rs");
    let adapter_source = include_str!("../src/platform/event_sink.rs");

    assert!(normalized_app_source.contains("TauriEventSink(&app).publish(AppEvent::LayoutChanged"));
    assert!(normalized_app_source.contains("layout_service::finish_mutation(&TauriEventSink(&app)"));
    assert!(layout_source.contains("pub fn finish_mutation(sink: &dyn EventSink"));
    assert!(layout_source.contains("sink.publish(AppEvent::LayoutChanged"));
    assert!(adapter_source.contains("LayoutChanged { project_id, revision }.emit(self.0)"));
}
