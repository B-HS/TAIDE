const HEX_DIGIT_COUNTS: [usize; 3] = [3, 6, 8];
const SHORT_HEX_LENGTH: usize = 4;
pub const HEX_ALPHA_LENGTH: usize = 9;
pub const OPAQUE_HEX_LENGTH: usize = 7;
pub const ALPHA_CHANNEL_MAX: f64 = 255.0;
const RGB_CHANNEL_MAX: f64 = 255.0;
const HEX_RADIX: u32 = 16;
const RED_RANGE: std::ops::Range<usize> = 1..3;
const GREEN_RANGE: std::ops::Range<usize> = 3..5;
const BLUE_RANGE: std::ops::Range<usize> = 5..7;

const CIE_SRGB_LINEAR_THRESHOLD: f64 = 0.04045;
const CIE_SRGB_LINEAR_DIVISOR: f64 = 12.92;
const CIE_SRGB_GAMMA_OFFSET: f64 = 0.055;
const CIE_SRGB_GAMMA_DIVISOR: f64 = 1.055;
const CIE_SRGB_GAMMA_EXPONENT: f64 = 2.4;
const CIE_XYZ_MATRIX: [[f64; 3]; 3] = [
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.072175],
    [0.0193339, 0.119192, 0.9503041],
];
const CIE_D65_WHITE_POINT: [f64; 3] = [0.95047, 1.0, 1.08883];
const CIE_LAB_EPSILON: f64 = 216.0 / 24389.0;
const CIE_LAB_KAPPA: f64 = 24389.0 / 27.0;
const CIE_LAB_L_SCALE: f64 = 116.0;
const CIE_LAB_L_OFFSET: f64 = 16.0;
const CIE_LAB_A_SCALE: f64 = 500.0;
const CIE_LAB_B_SCALE: f64 = 200.0;

const SRGB_LINEAR_THRESHOLD: f64 = 0.03928;
const SRGB_LINEAR_DIVISOR: f64 = 12.92;
const SRGB_GAMMA_OFFSET: f64 = 0.055;
const SRGB_GAMMA_DIVISOR: f64 = 1.055;
const SRGB_GAMMA_EXPONENT: f64 = 2.4;
const LUMINANCE_WEIGHTS: [f64; 3] = [0.2126, 0.7152, 0.0722];
const CONTRAST_RATIO_OFFSET: f64 = 0.05;

/// An sRGB color with channels in `0..=255`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

/// Length of a string as JavaScript counts it (UTF-16 code units).
pub fn js_length(value: &str) -> usize {
    value.encode_utf16().count()
}

/// The first `count` characters of a string.
pub fn prefix(value: &str, count: usize) -> &str {
    value.char_indices().nth(count).map_or(value, |(index, _)| &value[..index])
}

/// The characters of a string from position `start`.
pub fn suffix(value: &str, start: usize) -> &str {
    value.char_indices().nth(start).map_or("", |(index, _)| &value[index..])
}

/// Parses the leading hexadecimal digits like JavaScript `Number.parseInt(value, 16)`.
pub fn parse_hex_prefix(value: &str) -> Option<f64> {
    let digits = value
        .trim_start()
        .chars()
        .map_while(|character| character.to_digit(HEX_RADIX))
        .collect::<Vec<_>>();
    if digits.is_empty() {
        return None;
    }
    Some(
        digits
            .into_iter()
            .fold(0.0, |total, digit| total * f64::from(HEX_RADIX) + f64::from(digit)),
    )
}

pub fn is_hex_color(value: &str) -> bool {
    value
        .trim()
        .strip_prefix('#')
        .is_some_and(|hex| HEX_DIGIT_COUNTS.contains(&hex.len()) && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

pub fn hex_to_rgb(value: &str) -> Option<Rgb> {
    if !is_hex_color(value) {
        return None;
    }
    let normalized = value.trim().to_ascii_lowercase();
    let expanded = if normalized.len() == SHORT_HEX_LENGTH {
        normalized.chars().skip(1).fold(String::from("#"), |mut expanded, digit| {
            expanded.push(digit);
            expanded.push(digit);
            expanded
        })
    } else {
        normalized
    };
    let channel = |range: std::ops::Range<usize>| u8::from_str_radix(&expanded[range], HEX_RADIX).ok().map(f64::from);
    Some(Rgb {
        r: channel(RED_RANGE)?,
        g: channel(GREEN_RANGE)?,
        b: channel(BLUE_RANGE)?,
    })
}

fn hex_channel(channel: f64) -> String {
    format!("{:02x}", channel.clamp(0.0, RGB_CHANNEL_MAX).round() as u8)
}

pub fn rgb_to_hex(rgb: Rgb) -> String {
    format!("#{}{}{}", hex_channel(rgb.r), hex_channel(rgb.g), hex_channel(rgb.b))
}

/// Composites an 8-digit `#rrggbbaa` color over an opaque background; any other value passes through.
pub fn composite_over_background(foreground: &str, background: &str) -> String {
    if js_length(foreground) != HEX_ALPHA_LENGTH {
        return foreground.into();
    }
    let opaque = prefix(foreground, OPAQUE_HEX_LENGTH);
    let (Some(foreground_rgb), Some(background_rgb)) = (hex_to_rgb(opaque), hex_to_rgb(background)) else {
        return opaque.into();
    };
    let alpha = parse_hex_prefix(suffix(foreground, OPAQUE_HEX_LENGTH)).unwrap_or(f64::NAN) / ALPHA_CHANNEL_MAX;
    let mix = |front: f64, back: f64| front * alpha + back * (1.0 - alpha);
    rgb_to_hex(Rgb {
        r: mix(foreground_rgb.r, background_rgb.r),
        g: mix(foreground_rgb.g, background_rgb.g),
        b: mix(foreground_rgb.b, background_rgb.b),
    })
}

/// Moves each channel of `from` toward `to` by `ratio` (`0` keeps `from`, `1` reaches `to`).
pub fn mix(from: &str, to: &str, ratio: f64) -> Option<String> {
    let from = hex_to_rgb(from)?;
    let to = hex_to_rgb(to)?;
    let step = |start: f64, end: f64| start + (end - start) * ratio;
    Some(rgb_to_hex(Rgb {
        r: step(from.r, to.r),
        g: step(from.g, to.g),
        b: step(from.b, to.b),
    }))
}

fn srgb_channel_to_cie_linear(channel: f64) -> f64 {
    let normalized = channel / RGB_CHANNEL_MAX;
    if normalized <= CIE_SRGB_LINEAR_THRESHOLD {
        normalized / CIE_SRGB_LINEAR_DIVISOR
    } else {
        ((normalized + CIE_SRGB_GAMMA_OFFSET) / CIE_SRGB_GAMMA_DIVISOR).powf(CIE_SRGB_GAMMA_EXPONENT)
    }
}

fn cie_lab_f(t: f64) -> f64 {
    if t > CIE_LAB_EPSILON {
        t.cbrt()
    } else {
        (CIE_LAB_KAPPA * t + CIE_LAB_L_OFFSET) / CIE_LAB_L_SCALE
    }
}

fn hex_to_lab(value: &str) -> Option<[f64; 3]> {
    let rgb = hex_to_rgb(value)?;
    let linear = [
        srgb_channel_to_cie_linear(rgb.r),
        srgb_channel_to_cie_linear(rgb.g),
        srgb_channel_to_cie_linear(rgb.b),
    ];
    let [fx, fy, fz] = [0, 1, 2].map(|axis| {
        let row = CIE_XYZ_MATRIX[axis];
        cie_lab_f((linear[0] * row[0] + linear[1] * row[1] + linear[2] * row[2]) / CIE_D65_WHITE_POINT[axis])
    });
    Some([
        CIE_LAB_L_SCALE * fy - CIE_LAB_L_OFFSET,
        CIE_LAB_A_SCALE * (fx - fy),
        CIE_LAB_B_SCALE * (fy - fz),
    ])
}

/// CIE76 perceptual distance between two hex colors; the alpha channel is ignored.
pub fn delta_e76(hex_a: &str, hex_b: &str) -> Option<f64> {
    let a = hex_to_lab(hex_a)?;
    let b = hex_to_lab(hex_b)?;
    Some(((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt())
}

fn srgb_channel_to_linear(channel: f64) -> f64 {
    let normalized = channel / RGB_CHANNEL_MAX;
    if normalized <= SRGB_LINEAR_THRESHOLD {
        normalized / SRGB_LINEAR_DIVISOR
    } else {
        ((normalized + SRGB_GAMMA_OFFSET) / SRGB_GAMMA_DIVISOR).powf(SRGB_GAMMA_EXPONENT)
    }
}

/// WCAG relative luminance of a hex color.
pub fn relative_luminance(hex: &str) -> Option<f64> {
    let rgb = hex_to_rgb(hex)?;
    Some(
        LUMINANCE_WEIGHTS[0] * srgb_channel_to_linear(rgb.r)
            + LUMINANCE_WEIGHTS[1] * srgb_channel_to_linear(rgb.g)
            + LUMINANCE_WEIGHTS[2] * srgb_channel_to_linear(rgb.b),
    )
}

/// WCAG contrast ratio between two hex colors.
pub fn contrast_ratio(hex_a: &str, hex_b: &str) -> Option<f64> {
    let a = relative_luminance(hex_a)?;
    let b = relative_luminance(hex_b)?;
    Some((a.max(b) + CONTRAST_RATIO_OFFSET) / (a.min(b) + CONTRAST_RATIO_OFFSET))
}

/// Contrast ratio of a possibly translucent foreground drawn over its background.
pub fn foreground_contrast_ratio(foreground: &str, background: &str) -> Option<f64> {
    contrast_ratio(&composite_over_background(foreground, background), background)
}
