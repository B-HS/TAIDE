#![cfg(feature = "native-host")]

use serde_json::Value;
use taide_native_ui::editor_minimap_layout::{
    MinimapDimensions, MinimapLayout, MinimapViewport, glyph_rgba,
};

const REFERENCE: &str = include_str!("fixtures/minimap-reference.txt");
const DIMENSION_COUNT: usize = 36;
const LAYOUT_COUNT: usize = 2592;
const GLYPH_COUNT: usize = 808;
const EPSILON: f64 = 0.000_001;

fn number(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap()
}

fn close(actual: f64, expected: &Value, case: usize, field: &str) {
    if let Some(expected) = expected.as_f64() {
        assert!(
            (actual - expected).abs() < EPSILON,
            "case {case} {field}: {actual} != {expected}"
        );
    } else {
        assert!(actual.is_finite(), "case {case} {field}: {actual}");
    }
}

#[test]
fn 실제_원본_미니맵_36폭_2592스크롤_표본은_줄_창과_slider가_일치한다() {
    let source: Value = serde_json::from_str(REFERENCE).unwrap();
    assert_eq!(source["monaco"], "0.56.0");
    let dimensions = source["dimensions"].as_array().unwrap();
    assert_eq!(dimensions.len(), DIMENSION_COUNT);
    let computed = dimensions
        .iter()
        .enumerate()
        .map(|(index, case)| {
            let input = &case["input"];
            let expected = &case["output"];
            let dimension = MinimapDimensions::new(
                number(input, "remainingWidth"),
                number(input, "outerHeight"),
                number(input, "typicalHalfwidthCharacterWidth"),
                number(input, "verticalScrollbarWidth"),
                number(input, "pixelRatio"),
            );
            close(dimension.width, &expected["minimapWidth"], index, "width");
            assert_eq!(
                dimension.scale,
                expected["minimapScale"].as_u64().unwrap() as usize
            );
            assert_eq!(
                dimension.image_width,
                expected["minimapCanvasInnerWidth"].as_u64().unwrap() as usize
            );
            assert_eq!(
                dimension.image_height,
                expected["minimapCanvasInnerHeight"].as_u64().unwrap() as usize
            );
            close(
                dimension.image_point_width(),
                &expected["minimapCanvasOuterWidth"],
                index,
                "point_width",
            );
            close(
                dimension.image_point_height(),
                &expected["minimapCanvasOuterHeight"],
                index,
                "point_height",
            );
            dimension
        })
        .collect::<Vec<_>>();
    let layouts = source["layouts"].as_array().unwrap();
    assert_eq!(layouts.len(), LAYOUT_COUNT);
    let mut previous = Vec::<MinimapLayout>::new();
    for (index, case) in layouts.iter().enumerate() {
        let dimension_index = case["dimensionIndex"].as_u64().unwrap() as usize;
        let input = &dimensions[dimension_index]["input"];
        let first = case["first"].as_u64().unwrap() as usize - 1;
        let viewport = MinimapViewport {
            line_count: case["count"].as_u64().unwrap() as usize,
            first_row: first,
            last_row: case["last"].as_u64().unwrap() as usize - 1,
            first_row_top: first as f64 * number(input, "lineHeight"),
            height: number(input, "outerHeight"),
            line_height: number(input, "lineHeight"),
            scroll_top: number(case, "scrollTop"),
            scroll_height: number(case, "scrollHeight"),
            beyond_last_line: case["beyond"].as_bool().unwrap(),
        };
        let earlier = case["previousIndex"]
            .as_u64()
            .map(|index| &previous[index as usize]);
        let layout = MinimapLayout::new(computed[dimension_index], viewport, earlier);
        let expected = &case["output"];
        assert_eq!(
            layout.rows.start + 1,
            expected["startLineNumber"].as_u64().unwrap() as usize,
            "case {index} start"
        );
        assert_eq!(
            layout.rows.end,
            expected["endLineNumber"].as_u64().unwrap() as usize,
            "case {index} end"
        );
        assert_eq!(
            layout.slider_needed,
            expected["sliderNeeded"].as_bool().unwrap(),
            "case {index} slider_needed"
        );
        close(
            layout.slider_height,
            &expected["sliderHeight"],
            index,
            "slider_height",
        );
        close(
            layout.slider_top,
            &expected["sliderTop"],
            index,
            "slider_top",
        );
        close(
            layout.slider_ratio,
            &expected["_computedSliderRatio"],
            index,
            "slider_ratio",
        );
        previous.push(layout);
    }
}

#[test]
fn 실제_원본_문자시트_808표본은_축척_unicode_light_alpha_색혼합이_일치한다() {
    let source: Value = serde_json::from_str(REFERENCE).unwrap();
    let glyphs = source["glyphs"].as_array().unwrap();
    assert_eq!(glyphs.len(), GLYPH_COUNT);
    for (index, case) in glyphs.iter().enumerate() {
        let rgb = |key| ["r", "g", "b"].map(|channel| case[key][channel].as_u64().unwrap() as u8);
        let actual = glyph_rgba(
            case["scale"].as_u64().unwrap() as usize,
            case["code"].as_u64().unwrap() as u16,
            rgb("foreground"),
            rgb("background"),
            case["alpha"].as_u64().unwrap() as u8,
            case["light"].as_bool().unwrap(),
        );
        let expected = case["rgba"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_u64().unwrap() as u8)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "glyph case {index}");
    }
}

#[test]
fn 짧은_문서와_잘못된_화면은_비정상_scroll값이나_문자시트_경계_오류를_만들지_않는다() {
    let dimensions = MinimapDimensions::new(400.0, 240.0, 8.0, 14.0, 1.0);
    let viewport = MinimapViewport {
        line_count: 1,
        first_row: 0,
        last_row: 0,
        first_row_top: 0.0,
        height: 240.0,
        line_height: 20.0,
        scroll_top: 0.0,
        scroll_height: 240.0,
        beyond_last_line: true,
    };
    let layout = MinimapLayout::new(dimensions, viewport, None);
    assert!(!layout.slider_needed);
    assert_eq!(layout.scroll_from_delta(10.0), 0.0);
    assert_eq!(layout.scroll_from_touch(120.0), 0.0);
    assert_eq!(glyph_rgba(1, 128, [255; 3], [0; 3], 255, false).len(), 8);
    assert_eq!(
        MinimapLayout::new(
            dimensions,
            MinimapViewport {
                height: f64::NAN,
                ..viewport
            },
            None
        ),
        MinimapLayout::default()
    );
}
