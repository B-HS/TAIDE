use super::color::{
    composite_over_background, delta_e76, is_hex_color, js_length, mix, parse_hex_prefix, suffix, ALPHA_CHANNEL_MAX, HEX_ALPHA_LENGTH,
};
use super::tables::{StatePair, APP_SHADOW_MIN_ALPHA, STATE_DISTINCTNESS_PAIRS};
use super::Colors;

const BODY_FOREGROUND_KEY: &str = "app.foreground";
const SHADOW_KEY: &str = "app.shadow";
const SHADOW_REPAIR_RGB: &str = "#000000";
const MIX_RATIO_STEPS: u32 = 255;
const REPAIR_MAX_PASSES: usize = 4;
const ALPHA_DIGITS: usize = 2;
const DISTANCE_DECIMALS: usize = 2;
const ALPHA_DECIMALS: usize = 3;

struct Measurement {
    container: String,
    distance: f64,
    is_distinct: bool,
}

fn value<'a>(colors: &'a Colors, key: &str) -> Option<&'a str> {
    colors.get(key).map(String::as_str)
}

fn hex<'a>(colors: &'a Colors, key: &str) -> Option<&'a str> {
    value(colors, key).filter(|value| !value.is_empty() && is_hex_color(value))
}

fn surfaces(pair: &StatePair, colors: &Colors) -> Option<(String, String)> {
    let container = hex(colors, pair.container)?;
    let surface = pair.surface.and_then(|surface| hex(colors, surface));
    let container = surface.map_or_else(|| container.to_owned(), |surface| composite_over_background(container, surface));
    let state_base = if pair.is_sibling {
        surface.map_or_else(|| container.clone(), str::to_owned)
    } else {
        container.clone()
    };
    Some((container, state_base))
}

fn distance(state: Option<&str>, container: &str, state_base: &str) -> Option<f64> {
    let state = state.filter(|state| !state.is_empty() && is_hex_color(state))?;
    delta_e76(&composite_over_background(state, state_base), container)
}

fn measure(pair: &StatePair, colors: &Colors) -> Option<Measurement> {
    let (container, state_base) = surfaces(pair, colors)?;
    let measured = distance(value(colors, pair.state), &container, &state_base)?;
    let alternative = pair
        .alternative_state
        .and_then(|key| distance(value(colors, key), &container, &state_base));
    Some(Measurement {
        is_distinct: measured >= pair.min_delta_e || alternative.unwrap_or(0.0) >= pair.min_delta_e,
        container,
        distance: measured,
    })
}

fn shadow_alpha(shadow: Option<&str>) -> Option<f64> {
    let shadow = shadow.filter(|shadow| !shadow.is_empty() && is_hex_color(shadow))?;
    if js_length(shadow) != HEX_ALPHA_LENGTH {
        return Some(1.0);
    }
    Some(parse_hex_prefix(suffix(shadow, HEX_ALPHA_LENGTH - ALPHA_DIGITS)).unwrap_or(f64::NAN) / ALPHA_CHANNEL_MAX)
}

fn is_faint(alpha: f64) -> bool {
    !(alpha >= APP_SHADOW_MIN_ALPHA)
}

fn derive_distinct_state(container: &str, foreground: &str, min_delta_e: f64) -> Option<String> {
    let mix_at = |step: u32| mix(container, foreground, f64::from(step) / f64::from(MIX_RATIO_STEPS));
    let satisfies_at =
        |step: u32| mix_at(step).is_some_and(|candidate| distance(Some(&candidate), container, container).unwrap_or(0.0) >= min_delta_e);
    if !satisfies_at(MIX_RATIO_STEPS) {
        return None;
    }
    let mut rejected = 0;
    let mut accepted = MIX_RATIO_STEPS;
    while accepted - rejected > 1 {
        let middle = (rejected + accepted) / 2;
        if satisfies_at(middle) {
            accepted = middle;
        } else {
            rejected = middle;
        }
    }
    mix_at(accepted)
}

fn repair_pair(pair: &StatePair, colors: &mut Colors) -> Option<String> {
    let measurement = measure(pair, colors).filter(|measurement| !measurement.is_distinct)?;
    let repaired_key = pair.alternative_state.unwrap_or(pair.state);
    let replaced = value(colors, repaired_key).map(str::to_owned);
    let foreground = hex(colors, BODY_FOREGROUND_KEY)?;
    let replacement = derive_distinct_state(&measurement.container, foreground, pair.min_delta_e)?;
    if replaced.as_deref() == Some(replacement.as_str()) {
        return None;
    }
    let repair = format!(
        "{repaired_key}: {} -> {replacement} ({} 구별성 확보)",
        replaced.as_deref().unwrap_or("undefined"),
        pair.label
    );
    colors.insert(repaired_key.into(), replacement);
    Some(repair)
}

/// Pulls state colors away from their containers and raises a too-faint shadow; converted themes
/// carry no palette, so replacements are mixed toward the body foreground.
pub fn repair_state_distinctness(colors: &mut Colors) -> Vec<String> {
    let mut repairs = Vec::new();
    for _ in 0..REPAIR_MAX_PASSES {
        let before = repairs.len();
        for pair in STATE_DISTINCTNESS_PAIRS {
            repairs.extend(repair_pair(pair, colors));
        }
        if repairs.len() == before {
            break;
        }
    }
    let shadow = value(colors, SHADOW_KEY).map(str::to_owned);
    if shadow_alpha(shadow.as_deref()).is_some_and(is_faint) {
        let replacement = format!(
            "{SHADOW_REPAIR_RGB}{:02x}",
            (APP_SHADOW_MIN_ALPHA * ALPHA_CHANNEL_MAX).round() as u8
        );
        repairs.push(format!(
            "{SHADOW_KEY}: {} -> {replacement} (appShadow 구별성 확보)",
            shadow.as_deref().unwrap_or("undefined")
        ));
        colors.insert(SHADOW_KEY.into(), replacement);
    }
    repairs
}

/// Describes the state colors that stay indistinguishable from their containers.
pub fn validate_state_distinctness(colors: &Colors) -> Vec<String> {
    let mut errors = STATE_DISTINCTNESS_PAIRS
        .iter()
        .filter_map(|pair| {
            let measurement = measure(pair, colors).filter(|measurement| !measurement.is_distinct)?;
            Some(format!(
                "{} 구별성 부족: {}({}) vs {}({}) = ΔE {:.DISTANCE_DECIMALS$} (최소 {})",
                pair.label,
                pair.state,
                value(colors, pair.state).unwrap_or("undefined"),
                pair.container,
                value(colors, pair.container).unwrap_or("undefined"),
                measurement.distance,
                pair.min_delta_e
            ))
        })
        .collect::<Vec<_>>();
    if let Some(alpha) = shadow_alpha(value(colors, SHADOW_KEY)).filter(|alpha| is_faint(*alpha)) {
        errors.push(format!(
            "appShadow 구별성 부족: {SHADOW_KEY}({}) 알파 {alpha:.ALPHA_DECIMALS$} (최소 {APP_SHADOW_MIN_ALPHA})",
            value(colors, SHADOW_KEY).unwrap_or("undefined")
        ));
    }
    errors
}
