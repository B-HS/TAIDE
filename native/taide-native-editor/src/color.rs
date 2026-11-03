const CHANNEL_MAX: f64 = 255.0;
const HUE_MAX: f64 = 360.0;
const ROUNDING_SCALE: f64 = 1000.0;
const FIXED_SCALE: f64 = 100.0;
const PERCENT: f64 = 100.0;
const HUE_SECTOR: f64 = 60.0;
const HUE_SECTORS: f64 = 6.0;
const HEX_RADIX: u32 = 16;

/// A color with channels in the range `0..=1`, as exchanged by document color providers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DocumentColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

/// An 8-bit RGB color with a three-decimal alpha, normalized like the Monaco `RGBA` value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
}

/// A hue/saturation/lightness color normalized like the Monaco `HSLA` value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsla {
    pub h: u16,
    pub s: f64,
    pub l: f64,
    pub a: f64,
}

fn round3(value: f64) -> f64 {
    (value * ROUNDING_SCALE).round() / ROUNDING_SCALE
}

fn unit(value: f64) -> f64 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

fn channel(value: f64) -> u8 {
    if value.is_nan() {
        return 0;
    }
    value.clamp(0.0, CHANNEL_MAX).trunc() as u8
}

fn fixed2(value: f64) -> (u64, u64) {
    let hundredths = (value * FIXED_SCALE).round().max(0.0) as u64;
    let scale = FIXED_SCALE as u64;
    (hundredths / scale, hundredths % scale)
}

impl Rgba {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self {
            r: channel(r),
            g: channel(g),
            b: channel(b),
            a: round3(unit(a)),
        }
    }

    pub fn from_document(color: DocumentColor) -> Self {
        Self::new(
            (CHANNEL_MAX * color.red).round(),
            (CHANNEL_MAX * color.green).round(),
            (CHANNEL_MAX * color.blue).round(),
            color.alpha,
        )
    }

    pub fn to_document(self) -> DocumentColor {
        DocumentColor {
            red: f64::from(self.r) / CHANNEL_MAX,
            green: f64::from(self.g) / CHANNEL_MAX,
            blue: f64::from(self.b) / CHANNEL_MAX,
            alpha: self.a,
        }
    }

    pub fn from_hsla(hsla: Hsla) -> Self {
        let hue = f64::from(hsla.h) / HUE_MAX;
        let (r, g, b) = if hsla.s == 0.0 {
            (hsla.l, hsla.l, hsla.l)
        } else {
            let q = if hsla.l < 0.5 {
                hsla.l * (1.0 + hsla.s)
            } else {
                hsla.l + hsla.s - hsla.l * hsla.s
            };
            let p = 2.0 * hsla.l - q;
            (
                hue_to_rgb(p, q, hue + 1.0 / 3.0),
                hue_to_rgb(p, q, hue),
                hue_to_rgb(p, q, hue - 1.0 / 3.0),
            )
        };
        Self::new(
            (r * CHANNEL_MAX).round(),
            (g * CHANNEL_MAX).round(),
            (b * CHANNEL_MAX).round(),
            hsla.a,
        )
    }
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

impl Hsla {
    pub fn new(h: f64, s: f64, l: f64, a: f64) -> Self {
        let h = if h.is_nan() {
            0.0
        } else {
            h.clamp(0.0, HUE_MAX)
        };
        Self {
            h: h.trunc() as u16,
            s: round3(unit(s)),
            l: round3(unit(l)),
            a: round3(unit(a)),
        }
    }

    pub fn from_rgba(rgba: Rgba) -> Self {
        let r = f64::from(rgba.r) / CHANNEL_MAX;
        let g = f64::from(rgba.g) / CHANNEL_MAX;
        let b = f64::from(rgba.b) / CHANNEL_MAX;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (min + max) / 2.0;
        let chroma = max - min;
        let mut h = 0.0;
        let mut s = 0.0;
        if chroma > 0.0 {
            s = if l <= 0.5 {
                chroma / (2.0 * l)
            } else {
                chroma / (2.0 - 2.0 * l)
            }
            .min(1.0);
            h = if max == r {
                (g - b) / chroma + if g < b { HUE_SECTORS } else { 0.0 }
            } else if max == g {
                (b - r) / chroma + 2.0
            } else {
                (r - g) / chroma + 4.0
            };
            h = (h * HUE_SECTOR).round();
        }
        Self::new(h, s, l, rgba.a)
    }
}

fn two_digit_hex(value: u8) -> String {
    format!("{value:02x}")
}

pub fn format_rgb(rgba: Rgba) -> String {
    if rgba.a == 1.0 {
        return format!("rgb({}, {}, {})", rgba.r, rgba.g, rgba.b);
    }
    let (whole, fraction) = fixed2(rgba.a);
    let alpha = match fraction {
        0 => whole.to_string(),
        fraction if fraction % 10 == 0 => format!("{whole}.{}", fraction / 10),
        fraction => format!("{whole}.{fraction:02}"),
    };
    format!("rgba({}, {}, {}, {alpha})", rgba.r, rgba.g, rgba.b)
}

pub fn format_hsl(rgba: Rgba) -> String {
    let hsla = Hsla::from_rgba(rgba);
    let saturation = (hsla.s * PERCENT).round();
    let lightness = (hsla.l * PERCENT).round();
    if hsla.a == 1.0 {
        return format!("hsl({}, {saturation}%, {lightness}%)", hsla.h);
    }
    let (whole, fraction) = fixed2(hsla.a);
    format!(
        "hsla({}, {saturation}%, {lightness}%, {whole}.{fraction:02})",
        hsla.h
    )
}

pub fn format_hex(rgba: Rgba, compact: bool) -> String {
    let mut text = format!(
        "#{}{}{}",
        two_digit_hex(rgba.r),
        two_digit_hex(rgba.g),
        two_digit_hex(rgba.b)
    );
    if !compact || rgba.a != 1.0 {
        text.push_str(&two_digit_hex(channel((rgba.a * CHANNEL_MAX).round())));
    }
    text
}

/// Parses `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA` (non-hex digits count as zero, like Monaco).
pub fn parse_hex(text: &str) -> Option<Rgba> {
    let digits = text.strip_prefix('#')?;
    let units = text.encode_utf16().count();
    let value = |character: char| f64::from(character.to_digit(HEX_RADIX).unwrap_or(0));
    let values = digits.chars().map(value).collect::<Vec<_>>();
    let pair = |index: usize| 16.0 * values[index] + values[index + 1];
    let single = |index: usize| 16.0 * values[index] + values[index];
    match (units, values.len()) {
        (7, 6) => Some(Rgba::new(pair(0), pair(2), pair(4), 1.0)),
        (9, 8) => Some(Rgba::new(pair(0), pair(2), pair(4), pair(6) / CHANNEL_MAX)),
        (4, 3) => Some(Rgba::new(single(0), single(1), single(2), 1.0)),
        (5, 4) => Some(Rgba::new(
            single(0),
            single(1),
            single(2),
            single(3) / CHANNEL_MAX,
        )),
        _ => None,
    }
}

/// Returns the default provider presentations (`rgb`, `hsl`, `hex`) in Monaco order.
pub fn default_presentations(color: DocumentColor) -> [String; 3] {
    let rgba = Rgba::from_document(color);
    [format_rgb(rgba), format_hsl(rgba), format_hex(rgba, true)]
}
