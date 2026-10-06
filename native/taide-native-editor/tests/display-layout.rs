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
