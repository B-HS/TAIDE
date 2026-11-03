use super::color::{composite_over_background, delta_e76, foreground_contrast_ratio, is_hex_color, mix};
use super::mapping::{is_distinct_from_body_foreground, is_opaque_foreground_candidate};
use super::tables::{ComponentPair, ContrastPair, COMPONENT_CONTRAST_PAIRS, CONTRAST_PAIRS, STATE_MIN_DISTINCT_DELTA_E};
use super::Colors;

const MIN_CONTRAST_RATIO: f64 = 3.0;
const APP_BACKGROUND_KEY: &str = "app.background";
const MATCH_HIGHLIGHT_FOREGROUND_KEY: &str = "panel.matchHighlight";
const MATCH_HIGHLIGHT_REPAIR_FALLBACK_NOTICE: &str = ", 본문 전경과 동일색 — 구별 가능한 후보 없음";
const CONTRAST_MIX_STEPS: u32 = 255;
const CONTRAST_REPAIR_POLES: [&str; 2] = ["#000000", "#ffffff"];
const RATIO_DECIMALS: usize = 2;

fn repair_background_candidates(key: &str) -> &'static [&'static str] {
    match key {
        "tooltip.background" => &["editorWidget.background", "menu.background", "dropdown.background"],
        _ => &[],
    }
}

fn repair_foreground_candidates(key: &str) -> &'static [&'static str] {
    match key {
        "app.foreground" | "panel.sectionHeader" | "list.foreground" => &["editor.foreground"],
        "panel.matchHighlight" => &[
            "textLink.foreground",
            "button.background",
            "focusBorder",
            "activityBarBadge.background",
            "badge.background",
            "tab.activeBorderTop",
            "editor.foreground",
            "foreground",
        ],
        _ => &[],
    }
}

fn advisory_protected_background_keys(label: &str) -> &'static [&'static str] {
    match label {
        "selectionForeground" => &["list.background", "list.hoverBackground"],
        _ => &[],
    }
}

fn component_background_substitutes(key: &str) -> Option<&'static [&'static str]> {
    match key {
        "menu.itemHover" => Some(&["list.hoverBackground"]),
        _ => None,
    }
}

fn get<'a>(colors: &'a Colors, key: &str) -> &'a str {
    colors.get(key).map_or("", String::as_str)
}

fn hex<'a>(colors: &'a Colors, key: &str) -> Option<&'a str> {
    colors
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty() && is_hex_color(value))
}

fn meets_min_contrast(foreground: &str, background: &str) -> bool {
    foreground_contrast_ratio(foreground, background).unwrap_or(0.0) >= MIN_CONTRAST_RATIO
}

fn passes(ratio: Option<f64>) -> bool {
    ratio.is_some_and(|ratio| ratio >= MIN_CONTRAST_RATIO)
}

fn repair_pair(pair: &ContrastPair, colors: &mut Colors, vscode_colors: &Colors, protected_background_keys: &[&str]) -> Option<String> {
    let foreground = get(colors, pair.foreground).to_owned();
    let background = get(colors, pair.background).to_owned();
    if passes(foreground_contrast_ratio(&foreground, &background)) {
        return None;
    }
    let repaired_background = repair_background_candidates(pair.background)
        .iter()
        .filter_map(|key| vscode_colors.get(*key))
        .find(|value| !value.is_empty() && meets_min_contrast(&foreground, value))
        .cloned();
    if let Some(repaired) = repaired_background {
        let repair = format!("{}: {background} -> {repaired} ({} 대비 확보)", pair.background, pair.label);
        colors.insert(pair.background.into(), repaired);
        return Some(repair);
    }
    let protected_backgrounds = protected_background_keys
        .iter()
        .map(|key| get(colors, key).to_owned())
        .collect::<Vec<_>>();
    let satisfies_all = |value: &str| {
        meets_min_contrast(value, &background) && protected_backgrounds.iter().all(|protected| meets_min_contrast(value, protected))
    };
    let candidates = repair_foreground_candidates(pair.foreground)
        .iter()
        .map(|key| vscode_colors.get(*key).map(String::as_str))
        .collect::<Vec<_>>();
    let is_match_highlight = pair.foreground == MATCH_HIGHLIGHT_FOREGROUND_KEY;
    let body_foreground = get(colors, "app.foreground");
    let distinct = if is_match_highlight {
        candidates.iter().flatten().copied().find(|value| {
            is_opaque_foreground_candidate(Some(value)) && satisfies_all(value) && is_distinct_from_body_foreground(value, body_foreground)
        })
    } else {
        None
    };
    let allows_non_distinct = !is_match_highlight || pair.is_blocking;
    let repaired = distinct
        .or_else(|| {
            allows_non_distinct
                .then(|| {
                    candidates
                        .iter()
                        .flatten()
                        .copied()
                        .find(|value| !value.is_empty() && satisfies_all(value))
                })
                .flatten()
        })?
        .to_owned();
    let notice = if is_match_highlight && distinct.is_none() {
        MATCH_HIGHLIGHT_REPAIR_FALLBACK_NOTICE
    } else {
        ""
    };
    let repair = format!("{}: {foreground} -> {repaired} ({} 대비 확보{notice})", pair.foreground, pair.label);
    colors.insert(pair.foreground.into(), repaired);
    Some(repair)
}

fn surface(colors: &Colors, key: &str) -> Option<String> {
    let surface = hex(colors, key)?;
    if key == APP_BACKGROUND_KEY {
        return Some(surface.into());
    }
    Some(match hex(colors, APP_BACKGROUND_KEY) {
        Some(root) => composite_over_background(surface, root),
        None => surface.into(),
    })
}

fn component_background(pair: &ComponentPair, colors: &Colors) -> Option<String> {
    let background = hex(colors, pair.background)?;
    Some(match surface(colors, pair.surface) {
        Some(surface) => composite_over_background(background, &surface),
        None => background.into(),
    })
}

fn component_ratio(pair: &ComponentPair, colors: &Colors) -> Option<Option<f64>> {
    let foreground = hex(colors, pair.foreground)?;
    let background = component_background(pair, colors)?;
    Some(foreground_contrast_ratio(foreground, &background))
}

fn derive_contrasting_foreground(foreground: &str, backgrounds: &[String]) -> Option<String> {
    let base = composite_over_background(foreground, backgrounds.first()?);
    let mut best: Option<(String, f64)> = None;
    for pole in CONTRAST_REPAIR_POLES {
        for step in 1..=CONTRAST_MIX_STEPS {
            let Some(candidate) = mix(&base, pole, f64::from(step) / f64::from(CONTRAST_MIX_STEPS)) else {
                continue;
            };
            if !backgrounds.iter().all(|background| meets_min_contrast(&candidate, background)) {
                continue;
            }
            let distance = delta_e76(&candidate, &base).unwrap_or(f64::INFINITY);
            if best.as_ref().is_none_or(|(_, best)| distance < *best) {
                best = Some((candidate, distance));
            }
            break;
        }
    }
    best.map(|(value, _)| value)
}

fn repair_component_backgrounds(colors: &mut Colors) -> Vec<String> {
    let mut repairs = Vec::new();
    for pair in COMPONENT_CONTRAST_PAIRS {
        let Some(substitutes) = component_background_substitutes(pair.background) else {
            continue;
        };
        if component_ratio(pair, colors).is_none_or(passes) {
            continue;
        }
        let rendered_surface = surface(colors, pair.surface);
        let foreground = get(colors, pair.foreground).to_owned();
        let replacement = substitutes
            .iter()
            .filter_map(|key| colors.get(*key))
            .find(|candidate| {
                if candidate.is_empty() || !is_hex_color(candidate) {
                    return false;
                }
                let rendered = rendered_surface
                    .as_ref()
                    .map_or_else(|| candidate.to_string(), |surface| composite_over_background(candidate, surface));
                meets_min_contrast(&foreground, &rendered)
                    && rendered_surface
                        .as_ref()
                        .is_none_or(|surface| delta_e76(&rendered, surface).unwrap_or(0.0) >= STATE_MIN_DISTINCT_DELTA_E)
            })
            .cloned();
        let Some(replacement) = replacement else {
            continue;
        };
        repairs.push(format!(
            "{}: {} -> {replacement} ({} 대비 확보)",
            pair.background,
            get(colors, pair.background),
            pair.label
        ));
        colors.insert(pair.background.into(), replacement);
    }
    repairs
}

pub(super) fn repair_component_contrast(colors: &mut Colors) -> Vec<String> {
    let mut repairs = repair_component_backgrounds(colors);
    let mut foreground_keys = Vec::<&str>::new();
    for pair in COMPONENT_CONTRAST_PAIRS {
        if !foreground_keys.contains(&pair.foreground) {
            foreground_keys.push(pair.foreground);
        }
    }
    for key in foreground_keys {
        let Some(foreground) = hex(colors, key).map(str::to_owned) else {
            continue;
        };
        let surfaces = COMPONENT_CONTRAST_PAIRS
            .iter()
            .filter(|pair| pair.foreground == key && component_background_substitutes(pair.background).is_none())
            .filter_map(|pair| component_background(pair, colors))
            .filter(|surface| is_hex_color(surface))
            .collect::<Vec<_>>();
        if surfaces.is_empty() || surfaces.iter().all(|surface| meets_min_contrast(&foreground, surface)) {
            continue;
        }
        let legacy = CONTRAST_PAIRS
            .iter()
            .filter(|pair| pair.foreground == key)
            .filter_map(|pair| hex(colors, pair.background).map(str::to_owned));
        let backgrounds = surfaces.into_iter().chain(legacy).collect::<Vec<_>>();
        let Some(replacement) = derive_contrasting_foreground(&foreground, &backgrounds).filter(|replacement| *replacement != foreground)
        else {
            continue;
        };
        let labels = COMPONENT_CONTRAST_PAIRS
            .iter()
            .filter(|pair| pair.foreground == key)
            .map(|pair| pair.label)
            .collect::<Vec<_>>()
            .join("/");
        repairs.push(format!("{key}: {foreground} -> {replacement} ({labels} 대비 확보)"));
        colors.insert(key.into(), replacement);
    }
    repairs
}

/// Repairs blocking and advisory contrast pairs, then component foregrounds and backgrounds.
pub fn repair_contrast_pairs(colors: &mut Colors, vscode_colors: &Colors) -> Vec<String> {
    let mut repairs = Vec::new();
    let blocking = CONTRAST_PAIRS.iter().filter(|pair| pair.is_blocking).collect::<Vec<_>>();
    for pair in &blocking {
        repairs.extend(repair_pair(pair, colors, vscode_colors, &[]));
    }
    for pair in CONTRAST_PAIRS.iter().filter(|pair| !pair.is_blocking) {
        let protected = blocking
            .iter()
            .filter(|blocking| blocking.foreground == pair.foreground)
            .map(|blocking| blocking.background)
            .chain(advisory_protected_background_keys(pair.label).iter().copied())
            .collect::<Vec<_>>();
        repairs.extend(repair_pair(pair, colors, vscode_colors, &protected));
    }
    repairs.extend(repair_component_contrast(colors));
    repairs
}

/// Errors that make a converted theme unusable: identical body colors or a failing blocking pair.
pub fn validate_output_colors(colors: &Colors) -> Vec<String> {
    let mut errors = Vec::new();
    if colors.get("app.foreground") == colors.get("app.background") {
        errors.push(format!(
            "app.foreground와 app.background가 동일한 색({})입니다",
            colors.get("app.foreground").map_or("undefined", String::as_str)
        ));
    }
    for pair in CONTRAST_PAIRS.iter().filter(|pair| pair.is_blocking) {
        let foreground = get(colors, pair.foreground);
        let background = get(colors, pair.background);
        let ratio = foreground_contrast_ratio(foreground, background);
        if passes(ratio) {
            continue;
        }
        errors.push(format!(
            "{} 대비 부족: {}({foreground}) vs {}({background}) = {} (최소 {MIN_CONTRAST_RATIO})",
            pair.label,
            pair.foreground,
            pair.background,
            ratio.map_or_else(|| "N/A".into(), |ratio| format!("{ratio:.RATIO_DECIMALS$}"))
        ));
    }
    errors
}
