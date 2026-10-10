use taide_native_editor::display_layout::VerticalLayout;

const LINE_HEIGHT: f32 = 20.0;
const ROWS: usize = 50_000;
const VIEWPORT_HEIGHT: f32 = 200.0;
const VIEWPORT_ROWS: usize = 10;
const OVERSCAN: usize = 1;
const SCROLLED_ROW: usize = 40_000;
const PARTIAL_SCROLL: f32 = 130.0;
const PARTIAL_FIRST_ROW: usize = 6;
const HALF_ROW: f32 = 0.5;
const FAR_BELOW: f32 = 1.0e9;
const PREVIEW_HEIGHT: f32 = 40.0;
const MESSAGE_HEIGHT: f32 = 80.0;

#[test]
fn 여러_미리보기_zone은_기존_메시지와_공존하며_원래_줄좌표와_포인터_행을_보존한다() {
    let layout = VerticalLayout::new(LINE_HEIGHT, 4)
        .with_zone(1, MESSAGE_HEIGHT)
        .with_additional_zones([(2, LINE_HEIGHT), (0, PREVIEW_HEIGHT), (1, LINE_HEIGHT)]);
    assert_eq!(layout.zone(), Some(80.0..160.0));
    assert_eq!(layout.additional_zone(0), Some(200.0..220.0));
    assert_eq!(layout.additional_zone(1), Some(20.0..60.0));
    assert_eq!(layout.additional_zone(2), Some(160.0..180.0));
    assert_eq!(layout.content_height(), 240.0);
    for (row, top) in [(0, 0.0), (1, 60.0), (2, 180.0), (3, 220.0)] {
        assert_eq!(layout.row_top(row), top);
        assert_eq!(layout.row_at(top), row);
    }
    for (position, row) in [
        (20.0, 0),
        (59.0, 0),
        (80.0, 1),
        (160.0, 1),
        (200.0, 2),
        (240.0, 3),
    ] {
        assert_eq!(layout.row_at(position), row);
    }
    assert_eq!(
        layout.visible_rows(LINE_HEIGHT, PREVIEW_HEIGHT - LINE_HEIGHT, 0),
        0..1
    );
    assert_eq!(
        layout.visible_rows(layout.content_height(), VIEWPORT_HEIGHT, OVERSCAN),
        4..4
    );
}

#[test]
fn 줄_위치는_줄_번호와_줄_높이의_곱이고_전체_높이는_줄_수를_따른다() {
    let layout = VerticalLayout::new(LINE_HEIGHT, ROWS);
    assert_eq!(layout.row_top(0), 0.0);
    assert_eq!(
        layout.row_top(SCROLLED_ROW),
        SCROLLED_ROW as f32 * LINE_HEIGHT
    );
    assert_eq!(
        layout.row_bottom(SCROLLED_ROW),
        SCROLLED_ROW as f32 * LINE_HEIGHT + LINE_HEIGHT
    );
    assert_eq!(
        layout.row_center(SCROLLED_ROW),
        (SCROLLED_ROW as f32 + HALF_ROW) * LINE_HEIGHT
    );
    assert_eq!(layout.content_height(), ROWS as f32 * LINE_HEIGHT);
    assert_eq!(VerticalLayout::new(LINE_HEIGHT, 0).content_height(), 0.0);
}

#[test]
fn 세로_위치의_줄은_줄_경계에서_바뀌고_범위_밖에서는_첫_줄과_마지막_줄로_고정된다() {
    let layout = VerticalLayout::new(LINE_HEIGHT, ROWS);
    for (y, expected) in [
        (-LINE_HEIGHT, 0),
        (0.0, 0),
        (LINE_HEIGHT - HALF_ROW, 0),
        (LINE_HEIGHT, 1),
        (SCROLLED_ROW as f32 * LINE_HEIGHT + HALF_ROW, SCROLLED_ROW),
        (ROWS as f32 * LINE_HEIGHT, ROWS - 1),
        (FAR_BELOW, ROWS - 1),
        (f32::NAN, 0),
    ] {
        assert_eq!(layout.row_at(y), expected);
    }
    assert_eq!(VerticalLayout::new(LINE_HEIGHT, 0).row_at(LINE_HEIGHT), 0);
}

#[test]
fn 보이는_줄_범위는_첫_줄에서_화면_줄_수와_overscan만큼_이어지고_문서_끝에서_잘린다() {
    let layout = VerticalLayout::new(LINE_HEIGHT, ROWS);
    assert_eq!(
        layout.visible_rows(0.0, VIEWPORT_HEIGHT, OVERSCAN),
        0..VIEWPORT_ROWS + OVERSCAN
    );
    assert_eq!(
        layout.visible_rows(SCROLLED_ROW as f32 * LINE_HEIGHT, VIEWPORT_HEIGHT, OVERSCAN),
        SCROLLED_ROW..SCROLLED_ROW + VIEWPORT_ROWS + OVERSCAN
    );
    assert_eq!(
        layout.visible_rows(PARTIAL_SCROLL, VIEWPORT_HEIGHT, OVERSCAN),
        PARTIAL_FIRST_ROW..PARTIAL_FIRST_ROW + VIEWPORT_ROWS + OVERSCAN
    );
    assert_eq!(
        layout.visible_rows((ROWS - 1) as f32 * LINE_HEIGHT, VIEWPORT_HEIGHT, OVERSCAN),
        ROWS - 1..ROWS
    );
    assert_eq!(
        layout.visible_rows(ROWS as f32 * LINE_HEIGHT, VIEWPORT_HEIGHT, OVERSCAN),
        ROWS..ROWS
    );
    assert_eq!(
        VerticalLayout::new(LINE_HEIGHT, 1).visible_rows(0.0, VIEWPORT_HEIGHT, OVERSCAN),
        0..1
    );
}

#[test]
fn 메시지_zone은_줄_뒤_공간과_전체높이를_확보하고_아래_줄의_좌표와_보이는_범위를_옮긴다() {
    const ZONE_HEIGHT: f32 = 80.0;
    let layout = VerticalLayout::new(LINE_HEIGHT, 5).with_zone(1, ZONE_HEIGHT);
    assert_eq!(layout.zone(), Some(40.0..120.0));
    assert_eq!(layout.row_top(1), 20.0);
    assert_eq!(layout.row_top(2), 120.0);
    assert_eq!(layout.row_center(2), 130.0);
    assert_eq!(layout.content_height(), 180.0);
    for (y, row) in [(39.0, 1), (40.0, 1), (119.0, 1), (120.0, 2), (179.0, 4)] {
        assert_eq!(layout.row_at(y), row);
    }
    assert_eq!(layout.visible_rows(45.0, 20.0, 0), 1..2);
    assert_eq!(layout.visible_rows(120.0, 20.0, 0), 2..4);
    assert_eq!(layout.visible_rows(180.0, 20.0, 0), 5..5);
    assert!(
        VerticalLayout::new(LINE_HEIGHT, 0)
            .with_zone(0, ZONE_HEIGHT)
            .zone()
            .is_none()
    );
    assert!(
        VerticalLayout::new(LINE_HEIGHT, 5)
            .with_zone(5, ZONE_HEIGHT)
            .zone()
            .is_none()
    );
    assert!(
        VerticalLayout::new(LINE_HEIGHT, 5)
            .with_zone(1, f32::NAN)
            .zone()
            .is_none()
    );
}
