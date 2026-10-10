use std::cell::LazyCell;

use eframe::egui::Color32;
use taide_lsp::native::protocol::lsp_types::{CompletionItem, Documentation};
use taide_native_editor::completion::CompletionItemKind;
use taide_native_syntax::JsRegex;

const SOURCE: &str = r"(#([\da-fA-F]{3}){1,2}|(rgb|hsl)a\(\s*(\d{1,3}%?\s*,\s*){3}(1|0?\.\d+)\)|(rgb|hsl)\(\s*\d{1,3}%?(\s*,\s*\d{1,3}%?){2}\s*\))";
const RGB_COMPONENTS: usize = 3;
const RGBA_COMPONENTS: usize = RGB_COMPONENTS + 1;
const HEX_COMPONENT_BYTES: usize = 2;
const HEX_SHORT_EXPANSION: u8 = 17;
const PERCENT_MAXIMUM: f64 = 100.0;
const HUE_PERIOD: f64 = 360.0;
const HUE_SECTOR: f64 = 30.0;
const HSL_PERIOD: f64 = 12.0;
const HSL_GREEN_PHASE: f64 = 8.0;
const HSL_BLUE_PHASE: f64 = 4.0;
const HSL_RISING_PHASE: f64 = 3.0;
const HSL_FALLING_PHASE: f64 = 9.0;

struct Extractor {
    strict: JsRegex,
    relaxed: JsRegex,
}

thread_local! {
    static EXTRACTOR: LazyCell<Extractor> = LazyCell::new(|| Extractor {
        strict: JsRegex::new(&format!("^{SOURCE}$"), "i").expect("valid Monaco color expression"),
        relaxed: JsRegex::new(SOURCE, "").expect("valid Monaco color expression"),
    });
}

impl Extractor {
    fn extract<'a>(&self, item: &'a CompletionItem) -> Option<&'a str> {
        if self.strict.is_match(&item.label) {
            return Some(&item.label);
        }
        if let Some(detail) = item
            .detail
            .as_deref()
            .filter(|value| self.strict.is_match(value))
        {
            return Some(detail);
        }
        let documentation = match item.documentation.as_ref()? {
            Documentation::String(value) => value,
            Documentation::MarkupContent(value) => &value.value,
        };
        let range = self.relaxed.first_range(documentation)?;
        if range.start == 0 || range.end == documentation.len() {
            return documentation.get(range);
        }
        None
    }
}

pub(super) fn color(item: &CompletionItem) -> Option<Color32> {
    if item.kind != Some(CompletionItemKind::COLOR) {
        return None;
    }
    EXTRACTOR.with(|extractor| parse(extractor.extract(item)?))
}

fn css_trim(value: &str) -> &str {
    value.trim_matches([' ', '\t', '\r', '\n', '\u{c}'])
}

fn byte(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * f64::from(u8::MAX)).round() as u8
}

fn parse(value: &str) -> Option<Color32> {
    let value = css_trim(value);
    if let Some(hex) = value.strip_prefix('#') {
        if !hex.is_ascii() {
            return None;
        }
        let components = match hex.len() {
            RGB_COMPONENTS => {
                let mut digits = hex.chars();
                let mut channel =
                    || Some(u8::try_from(digits.next()?.to_digit(16)?).ok()? * HEX_SHORT_EXPANSION);
                [channel()?, channel()?, channel()?]
            }
            length if length == RGB_COMPONENTS * HEX_COMPONENT_BYTES => {
                let mut digits = hex.as_bytes().chunks_exact(HEX_COMPONENT_BYTES);
                let mut channel =
                    || u8::from_str_radix(std::str::from_utf8(digits.next()?).ok()?, 16).ok();
                [channel()?, channel()?, channel()?]
            }
            _ => return None,
        };
        return Some(Color32::from_rgb(
            components[0],
            components[1],
            components[2],
        ));
    }
    let value = value.to_ascii_lowercase();
    let (name, body) = value.split_once('(')?;
    let parts = body
        .strip_suffix(')')?
        .split(',')
        .map(css_trim)
        .collect::<Vec<_>>();
    let has_alpha = matches!(name, "rgba" | "hsla");
    let expected = if has_alpha {
        RGBA_COMPONENTS
    } else {
        RGB_COMPONENTS
    };
    if parts.len() != expected {
        return None;
    }
    let alpha = if has_alpha {
        parts[RGB_COMPONENTS].parse::<f64>().ok()?
    } else {
        1.0
    };
    let channels = match name {
        "rgb" | "rgba" => {
            let percent = parts[..RGB_COMPONENTS]
                .iter()
                .all(|value| value.ends_with('%'));
            if !percent
                && parts[..RGB_COMPONENTS]
                    .iter()
                    .any(|value| value.ends_with('%'))
            {
                return None;
            }
            let channel = |value: &str| {
                let maximum = if percent {
                    PERCENT_MAXIMUM
                } else {
                    f64::from(u8::MAX)
                };
                Some(
                    css_trim(value.strip_suffix('%').unwrap_or(value))
                        .parse::<f64>()
                        .ok()?
                        / maximum,
                )
            };
            [channel(parts[0])?, channel(parts[1])?, channel(parts[2])?]
        }
        "hsl" | "hsla" => {
            let hue = parts[0].parse::<f64>().ok()?.rem_euclid(HUE_PERIOD);
            let saturation = parts[1]
                .strip_suffix('%')?
                .parse::<f64>()
                .ok()?
                .clamp(0.0, PERCENT_MAXIMUM)
                / PERCENT_MAXIMUM;
            let lightness = parts[2]
                .strip_suffix('%')?
                .parse::<f64>()
                .ok()?
                .clamp(0.0, PERCENT_MAXIMUM)
                / PERCENT_MAXIMUM;
            let channel = |phase: f64| {
                let k = (phase + hue / HUE_SECTOR) % HSL_PERIOD;
                let amplitude = saturation * lightness.min(1.0 - lightness);
                lightness
                    - amplitude
                        * (k - HSL_RISING_PHASE)
                            .min(HSL_FALLING_PHASE - k)
                            .clamp(-1.0, 1.0)
            };
            [
                channel(0.0),
                channel(HSL_GREEN_PHASE),
                channel(HSL_BLUE_PHASE),
            ]
        }
        _ => return None,
    };
    Some(Color32::from_rgba_unmultiplied(
        byte(channels[0]),
        byte(channels[1]),
        byte(channels[2]),
        byte(alpha),
    ))
}

#[cfg(test)]
#[path = "editor-completion-colors-tests.rs"]
mod tests;
