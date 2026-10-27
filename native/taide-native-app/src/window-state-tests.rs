use super::*;

const INITIAL_SIZE: [f32; 2] = [1400.0, 900.0];
const MOVED_SIZE: [f32; 2] = [1100.0, 760.0];
const MAXIMIZED_SIZE: [f32; 2] = [1728.0, 1080.0];
const ZOOM: f32 = 1.25;
const MOVED_ORIGIN: [f32; 2] = [220.0, 140.0];
const PRIMARY: Monitor = Monitor {
    position: [0.0, 0.0],
    size: [1728.0, 1117.0],
};
const SECONDARY: Monitor = Monitor {
    position: [1728.0, -200.0],
    size: [2560.0, 1440.0],
};
const DETACHED_ORIGIN: [f32; 2] = [5000.0, 3000.0];
const OFF_LEFT_ORIGIN: [f32; 2] = [-900.0, 100.0];
const SECONDARY_EDGE_ORIGIN: [f32; 2] = [4288.0, 100.0];
const RETINA_SCALE: f64 = 2.0;
const STANDARD_SCALE: f64 = 1.0;
const LEFT_RETINA_PHYSICAL_ORIGIN: [i32; 2] = [-3456, -400];
const LEFT_RETINA_PHYSICAL_SIZE: [u32; 2] = [3456, 2234];
const LEFT_RETINA: Monitor = Monitor {
    position: [-1728.0, -200.0],
    size: [1728.0, 1117.0],
};
const SECONDARY_PHYSICAL_ORIGIN: [i32; 2] = [1728, -200];
const SECONDARY_PHYSICAL_SIZE: [u32; 2] = [2560, 1440];

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

fn placed_info(size: [f32; 2], origin: [f32; 2]) -> egui::ViewportInfo {
    let rect = egui::Rect::from_min_size(
        egui::pos2(origin[0], origin[1]),
        egui::vec2(size[0], size[1]),
    );
    egui::ViewportInfo {
        inner_rect: Some(rect),
        outer_rect: Some(rect),
        ..info(size)
    }
}

fn default_builder() -> egui::ViewportBuilder {
    egui::ViewportBuilder::default().with_inner_size(INITIAL_SIZE)
}

fn saved_at(origin: [f32; 2]) -> WindowState {
    WindowState {
        width: MOVED_SIZE[0],
        height: MOVED_SIZE[1],
        position: Some(origin),
        maximized: false,
        fullscreen: false,
    }
}

#[test]
fn 저장된_상태가_없으면_기본_창_크기를_그대로_쓰고_저장하지_않는다() {
    let directory = directory("empty");
    let builder = main_viewport(default_builder(), &directory, &[PRIMARY]);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(INITIAL_SIZE[0], INITIAL_SIZE[1]))
    );
    assert_eq!(builder.maximized, None);
    assert_eq!(builder.fullscreen, None);
    assert_eq!(builder.position, None);

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
            position: None,
            maximized: false,
            fullscreen: false,
        }
    );

    let builder = main_viewport(default_builder(), &directory, &[PRIMARY]);
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
    let builder = main_viewport(default_builder(), &directory, &[]);
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
    let builder = main_viewport(default_builder(), &directory, &[]);
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
    let builder = main_viewport(default_builder(), &directory, &[PRIMARY]);
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

#[test]
fn 메인_창_위치를_추적해_저장_사각형이_연결된_모니터와_겹칠_때만_복원한다() {
    let directory = directory("position");
    let mut tracker = Tracker::new(&directory);
    tracker.observe(&placed_info(INITIAL_SIZE, [40.0, 30.0]), 1.0);
    tracker.observe(&placed_info(MOVED_SIZE, MOVED_ORIGIN), 1.0);
    tracker.save().unwrap();
    assert_eq!(load(&directory), Some(saved_at(MOVED_ORIGIN)));

    let builder = main_viewport(default_builder(), &directory, &[PRIMARY, SECONDARY]);
    assert_eq!(
        builder.position,
        Some(egui::pos2(MOVED_ORIGIN[0], MOVED_ORIGIN[1]))
    );
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(MOVED_SIZE[0], MOVED_SIZE[1]))
    );

    let builder = main_viewport(default_builder(), &directory, &[SECONDARY]);
    assert_eq!(builder.position, None);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(MOVED_SIZE[0], MOVED_SIZE[1]))
    );
    let builder = main_viewport(default_builder(), &directory, &[]);
    assert_eq!(builder.position, None);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn 모니터_교차는_원본처럼_네_모서리_중_하나가_반열린_모니터_영역_안에_있을_때만_참이다() {
    let place = |origin: [f32; 2], monitors: &[Monitor]| {
        saved_at(origin).apply(default_builder(), monitors).position
    };
    assert_eq!(
        place(OFF_LEFT_ORIGIN, &[PRIMARY]),
        Some(egui::pos2(OFF_LEFT_ORIGIN[0], OFF_LEFT_ORIGIN[1]))
    );
    assert_eq!(
        place([PRIMARY.size[0] - 1.0, 0.0], &[PRIMARY]),
        Some(egui::pos2(PRIMARY.size[0] - 1.0, 0.0))
    );
    assert_eq!(place([PRIMARY.size[0], 0.0], &[PRIMARY]), None);
    assert_eq!(
        place([PRIMARY.size[0], 0.0], &[PRIMARY, SECONDARY]),
        Some(egui::pos2(PRIMARY.size[0], 0.0))
    );
    assert_eq!(place(SECONDARY_EDGE_ORIGIN, &[PRIMARY, SECONDARY]), None);
    assert_eq!(place(DETACHED_ORIGIN, &[PRIMARY, SECONDARY]), None);
}

#[test]
fn 모니터의_물리_위치와_크기는_그_모니터_배율로_나눈_논리_point가_된다() {
    let monitor = |[x, y]: [i32; 2], [width, height]: [u32; 2], scale_factor| {
        Monitor::from_physical(
            winit::dpi::PhysicalPosition::new(x, y),
            winit::dpi::PhysicalSize::new(width, height),
            scale_factor,
        )
    };
    assert_eq!(
        monitor(
            LEFT_RETINA_PHYSICAL_ORIGIN,
            LEFT_RETINA_PHYSICAL_SIZE,
            RETINA_SCALE
        ),
        LEFT_RETINA
    );
    assert_eq!(
        monitor(
            SECONDARY_PHYSICAL_ORIGIN,
            SECONDARY_PHYSICAL_SIZE,
            STANDARD_SCALE
        ),
        SECONDARY
    );
}

#[test]
fn 최대화_전체_화면_최소화_동안에는_직전_일반_창_위치를_유지한다() {
    let directory = directory("position-maximized");
    let mut tracker = Tracker::new(&directory);
    tracker.observe(&placed_info(MOVED_SIZE, MOVED_ORIGIN), 1.0);
    let mut maximized = placed_info(MAXIMIZED_SIZE, [0.0, 0.0]);
    maximized.maximized = Some(true);
    tracker.observe(&maximized, 1.0);
    let mut fullscreen = placed_info(MAXIMIZED_SIZE, [0.0, 0.0]);
    fullscreen.fullscreen = Some(true);
    tracker.observe(&fullscreen, 1.0);
    let mut minimized = placed_info(MOVED_SIZE, DETACHED_ORIGIN);
    minimized.minimized = Some(true);
    tracker.observe(&minimized, 1.0);
    assert_eq!(
        tracker.current.and_then(|state| state.position),
        Some(MOVED_ORIGIN)
    );

    let mut unplaced = info(MOVED_SIZE);
    unplaced.outer_rect = None;
    tracker.observe(&unplaced, 1.0);
    assert_eq!(
        tracker.current.and_then(|state| state.position),
        Some(MOVED_ORIGIN)
    );

    tracker.observe(&placed_info(MOVED_SIZE, MOVED_ORIGIN), ZOOM);
    assert_eq!(
        tracker.current.and_then(|state| state.position),
        Some([MOVED_ORIGIN[0] * ZOOM, MOVED_ORIGIN[1] * ZOOM])
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn 위치가_없는_이전_상태_파일도_크기를_복원한다() {
    let directory = directory("legacy");
    std::fs::write(
        directory.join(FILE_NAME),
        br#"{"main":{"width":1100.0,"height":760.0,"maximized":false,"fullscreen":true}}"#,
    )
    .unwrap();
    let builder = main_viewport(default_builder(), &directory, &[PRIMARY]);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(MOVED_SIZE[0], MOVED_SIZE[1]))
    );
    assert_eq!(builder.fullscreen, Some(true));
    assert_eq!(builder.position, None);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn 창_생성_hook은_이벤트_루프가_나중에_채운_모니터_목록으로_위치를_정한다() {
    let directory = directory("hook");
    let mut tracker = Tracker::new(&directory);
    tracker.observe(&placed_info(MOVED_SIZE, MOVED_ORIGIN), 1.0);
    tracker.save().unwrap();

    let unprobed = Monitors::default();
    let builder = main_window_hook(directory.clone(), unprobed)(default_builder());
    assert_eq!(builder.position, None);
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(MOVED_SIZE[0], MOVED_SIZE[1]))
    );

    let monitors = Monitors::default();
    let hook = main_window_hook(directory.clone(), monitors.clone());
    monitors.set(vec![PRIMARY]).unwrap();
    let builder = hook(default_builder());
    assert_eq!(
        builder.position,
        Some(egui::pos2(MOVED_ORIGIN[0], MOVED_ORIGIN[1]))
    );
    std::fs::remove_dir_all(directory).unwrap();
}
