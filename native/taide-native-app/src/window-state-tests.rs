use super::*;

const INITIAL_SIZE: [f32; 2] = [1400.0, 900.0];
const MOVED_SIZE: [f32; 2] = [1100.0, 760.0];
const MAXIMIZED_SIZE: [f32; 2] = [1728.0, 1080.0];
const ZOOM: f32 = 1.25;

fn directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "taide-native-window-state-{name}-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

fn info(size: [f32; 2]) -> egui::ViewportInfo {
    egui::ViewportInfo {
        inner_rect: Some(egui::Rect::from_min_size(
            egui::pos2(40.0, 30.0),
            egui::vec2(size[0], size[1]),
        )),
        minimized: Some(false),
        maximized: Some(false),
        fullscreen: Some(false),
        ..Default::default()
    }
}

fn default_builder() -> egui::ViewportBuilder {
    egui::ViewportBuilder::default().with_inner_size(INITIAL_SIZE)
}

#[test]
fn 저장된_상태가_없으면_기본_창_크기를_그대로_쓰고_저장하지_않는다() {
    let directory = directory("empty");
    let builder = main_viewport(default_builder(), &directory);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(INITIAL_SIZE[0], INITIAL_SIZE[1]))
    );
    assert_eq!(builder.maximized, None);
    assert_eq!(builder.fullscreen, None);

    let mut tracker = Tracker::new(&directory);
    tracker.observe(&egui::ViewportInfo::default(), 1.0);
    tracker.save().unwrap();
    assert!(!path(&directory).exists());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn 메인_창_크기를_추적해_data_dir에_저장하고_다음_실행의_viewport에_복원한다() {
    let directory = directory("roundtrip");
    let mut tracker = Tracker::new(&directory);
    tracker.observe(&info(INITIAL_SIZE), 1.0);
    tracker.observe(&info(MOVED_SIZE), 1.0);
    tracker.save().unwrap();

    let saved: BTreeMap<String, WindowState> =
        serde_json::from_slice(&std::fs::read(directory.join(FILE_NAME)).unwrap()).unwrap();
    assert_eq!(saved.keys().collect::<Vec<_>>(), vec![MAIN_LABEL]);
    assert_eq!(
        saved[MAIN_LABEL],
        WindowState {
            width: MOVED_SIZE[0],
            height: MOVED_SIZE[1],
            maximized: false,
            fullscreen: false,
        }
    );

    let builder = main_viewport(default_builder(), &directory);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(MOVED_SIZE[0], MOVED_SIZE[1]))
    );
    assert_eq!(builder.maximized, Some(false));
    assert_eq!(builder.fullscreen, Some(false));
    assert_eq!(builder.position, None);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn 최대화와_전체_화면_동안에는_이전_창_크기를_지키고_상태만_기록한다() {
    let directory = directory("maximized");
    let mut tracker = Tracker::new(&directory);
    tracker.observe(&info(MOVED_SIZE), 1.0);
    let mut maximized = info(MAXIMIZED_SIZE);
    maximized.maximized = Some(true);
    tracker.observe(&maximized, 1.0);
    tracker.save().unwrap();
    let builder = main_viewport(default_builder(), &directory);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(MOVED_SIZE[0], MOVED_SIZE[1]))
    );
    assert_eq!(builder.maximized, Some(true));

    let mut fullscreen = info(MAXIMIZED_SIZE);
    fullscreen.fullscreen = Some(true);
    tracker.observe(&fullscreen, 1.0);
    let mut minimized = info(MAXIMIZED_SIZE);
    minimized.minimized = Some(true);
    tracker.observe(&minimized, 1.0);
    tracker.save().unwrap();
    let builder = main_viewport(default_builder(), &directory);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(MOVED_SIZE[0], MOVED_SIZE[1]))
    );
    assert_eq!(builder.maximized, Some(false));
    assert_eq!(builder.fullscreen, Some(true));

    let restarted = Tracker::new(&directory);
    assert_eq!(restarted.current, tracker.current);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn 화면_확대_배율을_곱한_논리_크기로_저장하고_빈_크기는_무시한다() {
    let directory = directory("zoom");
    let mut tracker = Tracker::new(&directory);
    tracker.observe(&info(MOVED_SIZE), ZOOM);
    tracker.observe(&info([0.0, MOVED_SIZE[1]]), 1.0);
    assert_eq!(
        tracker.current.map(|state| [state.width, state.height]),
        Some([MOVED_SIZE[0] * ZOOM, MOVED_SIZE[1] * ZOOM])
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn 손상된_상태_파일은_기본_창으로_시작하고_다음_저장이_덮어쓴다() {
    let directory = directory("corrupt");
    std::fs::write(directory.join(FILE_NAME), b"{not json").unwrap();
    let builder = main_viewport(default_builder(), &directory);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(INITIAL_SIZE[0], INITIAL_SIZE[1]))
    );
    let mut tracker = Tracker::new(&directory);
    tracker.observe(&info(MOVED_SIZE), 1.0);
    tracker.save().unwrap();
    assert_eq!(
        load(&directory).map(|state| [state.width, state.height]),
        Some(MOVED_SIZE)
    );
    std::fs::remove_dir_all(directory).unwrap();
}
