use egui::emath::Rot2;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2, vec2};
use taide_model::agent::{AgentActivity, BlockedReason, DetectedAgent};
use taide_model::project::{ProjectDisplay, ProjectDisplayPatch};

use crate::icons::glyphs::PROJECT_ICONS;

pub const PROJECT_LABEL_MAX_CODEPOINTS: usize = 4;
pub const PROJECT_GROUP_NAME_MAX_CODEPOINTS: usize = 40;
pub const DEFAULT_PROJECT_ICON_NAME: &str = "folder";
pub const PROJECT_GLYPH_ICON_SIZE: f32 = 20.0;
pub const PROJECT_COLOR_TOKENS: [&str; 12] = [
    "lane1", "lane2", "lane3", "lane4", "lane5", "lane6", "lane7", "lane8", "lane9", "lane10",
    "lane11", "lane12",
];
const PROJECT_LABEL_FONT_SIZES: [f32; PROJECT_LABEL_MAX_CODEPOINTS + 1] =
    [16.0, 16.0, 14.0, 11.0, 9.0];
const PROJECT_LABEL_TIGHT_TRACKING_EM: f32 = -0.025;
const ACTIVITY_PRIORITY: [AgentActivity; 4] = [
    AgentActivity::Unknown,
    AgentActivity::Idle,
    AgentActivity::Working,
    AgentActivity::AwaitingInput,
];
const WORKING_BADGE_SIZE: f32 = 10.0;
const AWAITING_BADGE_SIZE: f32 = 12.0;
const IDLE_BADGE_SIZE: f32 = 8.0;
const BADGE_RING_WIDTH: f32 = 2.0;
const BADGE_BORDER_WIDTH: f32 = 1.0;
const AWAITING_BADGE_ROTATION: f32 = std::f32::consts::FRAC_PI_4;
const PULSE_PERIOD_SECONDS: f64 = 2.0;
const PULSE_KEYFRAME: f32 = 0.5;
const PULSE_MIN_OPACITY: f32 = 0.5;
const PULSE_EASING_X: [f64; 2] = [0.4, 0.6];
const PULSE_EASING_Y: [f64; 2] = [0.0, 1.0];

#[derive(Clone, Copy)]
pub struct RailColors {
    pub item_hover: Color32,
    pub item_active: Color32,
    pub accent: Color32,
    pub agent_working: Color32,
    pub agent_awaiting: Color32,
    pub agent_idle: Color32,
    pub agent_unknown: Color32,
    pub lanes: [Color32; PROJECT_COLOR_TOKENS.len()],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailIcon<'a> {
    Project(Option<&'a str>),
    RootMissing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectGlyph {
    Label(String),
    Icon(String),
    Default,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDisplayResolution {
    pub glyph: ProjectGlyph,
    pub lane: Option<usize>,
}

impl ProjectDisplayResolution {
    pub fn icon(&self) -> Option<&str> {
        match &self.glyph {
            ProjectGlyph::Icon(icon) => Some(icon),
            ProjectGlyph::Label(_) | ProjectGlyph::Default => None,
        }
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// Original `resolveProjectColorToken`: the stored color as an index into
/// [`PROJECT_COLOR_TOKENS`], or `None` when nothing valid is stored.
pub fn resolve_project_color_lane(value: Option<&str>) -> Option<usize> {
    non_empty(value).and_then(|token| PROJECT_COLOR_TOKENS.iter().position(|lane| *lane == token))
}

/// Original `resolveProjectIconName`: an exact catalog id, otherwise the folder default.
pub fn resolve_project_icon_name(name: Option<&str>) -> &'static str {
    PROJECT_ICONS
        .iter()
        .find(|(id, _)| Some(*id) == name)
        .map_or(DEFAULT_PROJECT_ICON_NAME, |(id, _)| id)
}

/// Original `clampProjectLabel`: control characters dropped and the length capped in codepoints,
/// without trimming.
pub fn clamp_project_label(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(PROJECT_LABEL_MAX_CODEPOINTS)
        .collect()
}

/// Original `normalizeProjectLabel`: the backend's `sanitize_display_label` order (strip, trim,
/// then measure).
pub fn normalize_project_label(value: &str) -> String {
    let stripped: String = value
        .chars()
        .filter(|character| !character.is_control())
        .collect();
    stripped
        .trim()
        .chars()
        .take(PROJECT_LABEL_MAX_CODEPOINTS)
        .collect()
}

/// Original `clampProjectGroupName`: what the name input may hold while typing, without trimming.
pub fn clamp_project_group_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(PROJECT_GROUP_NAME_MAX_CODEPOINTS)
        .collect()
}

/// Original `normalizeProjectGroupName`: the backend's `sanitize_group_name` order (strip, trim,
/// then measure).
pub fn normalize_project_group_name(value: &str) -> String {
    let stripped: String = value
        .chars()
        .filter(|character| !character.is_control())
        .collect();
    clamp_project_group_name(stripped.trim())
}

/// Original `isProjectDisplayCustomized`, which gates the menu's "reset to default".
pub fn is_project_display_customized(resolution: &ProjectDisplayResolution) -> bool {
    resolution.glyph != ProjectGlyph::Default || resolution.lane.is_some()
}

/// Original `CLEARED_PROJECT_DISPLAY_PATCH`: `""` on every axis clears it.
pub fn cleared_project_display_patch() -> ProjectDisplayPatch {
    ProjectDisplayPatch {
        icon: Some(String::new()),
        label: Some(String::new()),
        color: Some(String::new()),
    }
}

pub fn resolve_project_display(display: &ProjectDisplay) -> ProjectDisplayResolution {
    let lane = resolve_project_color_lane(display.color.as_deref());
    let glyph = match (
        non_empty(display.label.as_deref()),
        non_empty(display.icon.as_deref()),
    ) {
        (Some(label), _) => {
            ProjectGlyph::Label(label.chars().take(PROJECT_LABEL_MAX_CODEPOINTS).collect())
        }
        (None, Some(icon)) => ProjectGlyph::Icon(icon.to_owned()),
        (None, None) => ProjectGlyph::Default,
    };
    ProjectDisplayResolution { glyph, lane }
}

pub fn label_font_size(label: &str) -> f32 {
    PROJECT_LABEL_FONT_SIZES[label.chars().count().min(PROJECT_LABEL_MAX_CODEPOINTS)]
}

pub fn label_letter_spacing(label: &str) -> f32 {
    if label.chars().count() < PROJECT_LABEL_MAX_CODEPOINTS {
        return 0.0;
    }
    label_font_size(label) * PROJECT_LABEL_TIGHT_TRACKING_EM
}

/// Original `ProjectDisplayGlyph`: the semibold label clipped to `rect`, or the 20px catalog icon.
pub fn paint_project_glyph(
    ui: &Ui,
    painter: &Painter,
    rect: Rect,
    display: &ProjectDisplayResolution,
    color: Color32,
    icon: &mut dyn FnMut(&Ui, Rect, RailIcon<'_>, Color32),
) {
    match &display.glyph {
        ProjectGlyph::Label(text) => {
            let job = egui::text::LayoutJob::single_section(
                text.clone(),
                egui::TextFormat {
                    font_id: egui::FontId::new(
                        label_font_size(text),
                        crate::font_families::semibold(ui),
                    ),
                    color,
                    extra_letter_spacing: label_letter_spacing(text),
                    ..Default::default()
                },
            );
            let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
            painter
                .with_clip_rect(rect)
                .galley(rect.center() - galley.size() / 2.0, galley, color);
        }
        ProjectGlyph::Icon(_) | ProjectGlyph::Default => icon(
            ui,
            Rect::from_center_size(rect.center(), Vec2::splat(PROJECT_GLYPH_ICON_SIZE)),
            RailIcon::Project(display.icon()),
            color,
        ),
    }
}

pub fn agent_status_label_key(
    activity: AgentActivity,
    blocked_reason: Option<BlockedReason>,
) -> &'static str {
    match (activity, blocked_reason) {
        (AgentActivity::AwaitingInput, Some(BlockedReason::Permission)) => {
            "agent.blocked.permission"
        }
        (AgentActivity::AwaitingInput, Some(BlockedReason::Question)) => "agent.blocked.question",
        (AgentActivity::AwaitingInput, Some(BlockedReason::Dialog)) => "agent.blocked.dialog",
        (AgentActivity::AwaitingInput, None) => "agent.status.awaitingInput",
        (AgentActivity::Working, _) => "agent.status.working",
        (AgentActivity::Idle, _) => "agent.status.idle",
        (AgentActivity::Unknown, _) => "agent.status.unknown",
    }
}

fn priority(activity: AgentActivity) -> usize {
    ACTIVITY_PRIORITY
        .iter()
        .position(|candidate| *candidate == activity)
        .unwrap_or_default()
}

pub fn top_priority_agent(agents: &[DetectedAgent]) -> Option<&DetectedAgent> {
    agents.iter().reduce(|top, agent| {
        if priority(agent.activity) > priority(top.activity) {
            agent
        } else {
            top
        }
    })
}

pub fn pulse_opacity(time: f64) -> f32 {
    let phase = (time.rem_euclid(PULSE_PERIOD_SECONDS) / PULSE_PERIOD_SECONDS) as f32;
    let fade = 1.0 - PULSE_MIN_OPACITY;
    if phase < PULSE_KEYFRAME {
        let eased =
            crate::css_motion::cubic_bezier(phase / PULSE_KEYFRAME, PULSE_EASING_X, PULSE_EASING_Y);
        return 1.0 - fade * eased;
    }
    let eased = crate::css_motion::cubic_bezier(
        (phase - PULSE_KEYFRAME) / (1.0 - PULSE_KEYFRAME),
        PULSE_EASING_X,
        PULSE_EASING_Y,
    );
    PULSE_MIN_OPACITY + fade * eased
}

pub fn agent_badge_rect(button: Rect, activity: AgentActivity) -> Rect {
    let size = match activity {
        AgentActivity::Working => WORKING_BADGE_SIZE,
        AgentActivity::AwaitingInput => AWAITING_BADGE_SIZE,
        AgentActivity::Idle | AgentActivity::Unknown => IDLE_BADGE_SIZE,
    };
    Rect::from_min_max(
        button.right_bottom() - Vec2::splat(size),
        button.right_bottom(),
    )
}

pub fn paint_agent_badge(
    painter: &Painter,
    button: Rect,
    activity: AgentActivity,
    colors: &RailColors,
    ring: Color32,
    opacity: f32,
) {
    let badge = agent_badge_rect(button, activity);
    let center = badge.center();
    let radius = badge.width() / 2.0;
    let ring = ring.gamma_multiply(opacity);
    let ring_stroke = Stroke::new(BADGE_RING_WIDTH, ring);
    match activity {
        AgentActivity::Working => {
            painter.circle_stroke(center, radius + BADGE_RING_WIDTH / 2.0, ring_stroke);
            painter.circle_filled(center, radius, colors.agent_working.gamma_multiply(opacity));
        }
        AgentActivity::AwaitingInput => {
            painter.add(Shape::closed_line(
                diamond(center, radius + BADGE_RING_WIDTH / 2.0),
                ring_stroke,
            ));
            painter.add(Shape::convex_polygon(
                diamond(center, radius),
                colors.agent_awaiting.gamma_multiply(opacity),
                Stroke::NONE,
            ));
        }
        AgentActivity::Idle | AgentActivity::Unknown => {
            let border = if activity == AgentActivity::Idle {
                colors.agent_idle
            } else {
                colors.agent_unknown
            };
            painter.circle_stroke(center, radius + BADGE_RING_WIDTH / 2.0, ring_stroke);
            painter.circle_stroke(
                center,
                radius - BADGE_BORDER_WIDTH / 2.0,
                Stroke::new(BADGE_BORDER_WIDTH, border.gamma_multiply(opacity)),
            );
        }
    }
}

fn diamond(center: Pos2, half_side: f32) -> Vec<Pos2> {
    let rotation = Rot2::from_angle(AWAITING_BADGE_ROTATION);
    [
        vec2(-half_side, -half_side),
        vec2(half_side, -half_side),
        vec2(half_side, half_side),
        vec2(-half_side, half_side),
    ]
    .into_iter()
    .map(|corner| center + rotation * corner)
    .collect()
}

#[cfg(test)]
#[path = "project-rail-tests.rs"]
mod tests;
