use taide_native_editor::color::{
    DocumentColor, Hsla, Rgba, default_presentations, format_hex, format_rgb, parse_hex,
};
use taide_native_editor::default_colors::{self, document_colors};
use taide_native_editor::document::EditorError;

const LIMITS: default_colors::Limits = default_colors::Limits {
    max_bytes: 1 << 20,
    max_colors: 64,
};
const EPSILON: f64 = 1e-9;

fn color(red: f64, green: f64, blue: f64, alpha: f64) -> DocumentColor {
    DocumentColor {
        red,
        green,
        blue,
        alpha,
    }
}

fn assert_color(actual: DocumentColor, expected: DocumentColor) {
    for (left, right) in [
        (actual.red, expected.red),
        (actual.green, expected.green),
        (actual.blue, expected.blue),
        (actual.alpha, expected.alpha),
    ] {
        assert!((left - right).abs() < EPSILON, "{actual:?} != {expected:?}");
    }
}

#[test]
fn 기본_색_공급자는_줄단위_hex_rgb_hsl_규칙과_경계를_원본대로_찾는다() {
    let input = "a: #fff;\n#12345678\nx#abc\nrgb(255, 0, 0) rgba(0 0 255 / 0.5) hsl(120, 100%, 50%)\nhsla(240 100% 50% / 1.0) rgba(1,2,3,1.0) RGB(1,2,3) xrgb(1,2,3)\r\n'#abcd' \"#abcdef\" #abcg #00000\nrgb(256,0,0) rgb(01,0,0) hsl(361,0%,0%) hsl(360.5,100%,50%) rgb(1,,2,3)";
    let colors = document_colors(input, LIMITS).unwrap();
    let expected = [
        ("#fff", color(1.0, 1.0, 1.0, 1.0)),
        (
            "#12345678",
            color(18.0 / 255.0, 52.0 / 255.0, 86.0 / 255.0, 0.471),
        ),
        ("rgb(255, 0, 0)", color(1.0, 0.0, 0.0, 1.0)),
        ("rgba(0 0 255 / 0.5)", color(0.0, 0.0, 1.0, 0.5)),
        ("hsl(120, 100%, 50%)", color(0.0, 1.0, 0.0, 1.0)),
        ("hsla(240 100% 50% / 1.0)", color(0.0, 0.0, 1.0, 1.0)),
        (
            "#abcd",
            color(170.0 / 255.0, 187.0 / 255.0, 204.0 / 255.0, 0.867),
        ),
        (
            "#abcdef",
            color(171.0 / 255.0, 205.0 / 255.0, 239.0 / 255.0, 1.0),
        ),
        ("hsl(360.5,100%,50%)", color(1.0, 0.0, 0.0, 1.0)),
    ];
    assert_eq!(
        colors
            .iter()
            .map(|information| &input[information.bytes.clone()])
            .collect::<Vec<_>>(),
        expected.iter().map(|(text, _)| *text).collect::<Vec<_>>()
    );
    for (information, (_, expected)) in colors.iter().zip(expected) {
        assert_color(information.color, expected);
    }
    assert_eq!(
        document_colors(
            "#fff #000",
            default_colors::Limits {
                max_colors: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
}

#[test]
fn 색_표현은_monaco_rgb_hsl_hex_형식과_반올림을_따른다() {
    assert_eq!(
        default_presentations(color(1.0, 0.0, 0.0, 1.0)),
        ["rgb(255, 0, 0)", "hsl(0, 100%, 50%)", "#ff0000"]
    );
    assert_eq!(
        default_presentations(color(0.0, 0.0, 1.0, 0.5)),
        [
            "rgba(0, 0, 255, 0.5)",
            "hsla(240, 100%, 50%, 0.50)",
            "#0000ff80"
        ]
    );
    let translucent = Rgba::new(10.0, 20.0, 30.0, 0.125);
    assert_eq!(format_rgb(translucent), "rgba(10, 20, 30, 0.13)");
    assert_eq!(format_hex(translucent, false), "#0a141e20");
    assert_eq!(
        parse_hex("#abc"),
        Some(Rgba {
            r: 170,
            g: 187,
            b: 204,
            a: 1.0
        })
    );
    assert_eq!(parse_hex("#abcde"), None);
    assert_eq!(parse_hex("abc"), None);
    assert_eq!(
        Hsla::from_rgba(Rgba::new(128.0, 128.0, 128.0, 1.0)),
        Hsla {
            h: 0,
            s: 0.0,
            l: 0.502,
            a: 1.0
        }
    );
    assert_eq!(
        Rgba::from_hsla(Hsla::new(400.0, 2.0, -1.0, 0.33333)),
        Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0.333
        }
    );
}
